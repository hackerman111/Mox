//! Bounded external tool execution, including descendant shutdown and pipe draining.
use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

struct OwnedChild {
    child: Child,
    active: bool,
}
impl OwnedChild {
    fn terminate(&mut self) {
        if !self.active {
            return;
        }
        #[cfg(unix)]
        {
            // SAFETY: process_group(0) created a private group whose ID is this
            // owned child PID. Its descendants inherit the group; no parent does.
            unsafe {
                libc::kill(-(self.child.id() as i32), libc::SIGKILL);
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.active = false;
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.terminate();
    }
}

pub fn output(
    command: &mut Command,
    timeout: Duration,
    max_bytes: usize,
) -> Result<Output, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = OwnedChild {
        child: command.spawn().map_err(|e| e.to_string())?,
        active: true,
    };
    let stdout = child.child.stdout.take().ok_or("Child stdout missing")?;
    let stderr = child.child.stderr.take().ok_or("Child stderr missing")?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        for descriptor in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
            // SAFETY: both descriptors are owned, live pipes. Only their file
            // status flags are changed; ownership remains with the reader thread.
            let configured = unsafe {
                let flags = libc::fcntl(descriptor, libc::F_GETFL);
                flags >= 0 && libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) >= 0
            };
            if !configured {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
    }
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let read_stop = stop.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(2);
    let read =
        move |mut reader: Box<dyn Read + Send>,
              tag: bool,
              sender: std::sync::mpsc::SyncSender<(bool, Result<Vec<u8>, String>)>| {
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 8192];
            let result = loop {
                if read_stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break Err("External command cancelled".into());
                }
                match reader.read(&mut buffer) {
                    Ok(0) => break Ok(bytes),
                    Ok(count) if bytes.len().saturating_add(count) <= max_bytes => {
                        bytes.extend_from_slice(&buffer[..count])
                    }
                    Ok(_) => break Err("External command exceeded output limit".into()),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => break Err(error.to_string()),
                }
            };
            let _ = sender.send((tag, result));
        };
    let first = sender.clone();
    let out_read = read.clone();
    let out_thread = std::thread::spawn(move || out_read(Box::new(stdout), true, first));
    let err_thread = std::thread::spawn(move || read(Box::new(stderr), false, sender));
    let started = Instant::now();
    let mut out = None;
    let mut err = None;
    let result = (|| {
        let mut status = None;
        loop {
            while let Ok((tag, bytes)) = receiver.try_recv() {
                let bytes = bytes?;
                if tag {
                    out = Some(bytes);
                } else {
                    err = Some(bytes);
                }
            }
            if status.is_none() {
                status = child.child.try_wait().map_err(|e| e.to_string())?;
            }
            if let Some(status) = status
                && out.is_some()
                && err.is_some()
            {
                let stdout = out.take().ok_or("Missing stdout")?;
                let stderr = err.take().ok_or("Missing stderr")?;
                if stdout.len().saturating_add(stderr.len()) > max_bytes {
                    return Err("External command exceeded output limit".into());
                }
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            if started.elapsed() >= timeout {
                return Err("External command timed out".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    })();
    if result.is_err() {
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        child.terminate();
    } else {
        child.active = false;
    }
    let _ = out_thread.join();
    let _ = err_thread.join();
    result
}
