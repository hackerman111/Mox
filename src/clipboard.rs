//! tmux buffer clipboard integration and URL opening.

use crate::tmux::{execute_tmux, load_buffer};
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

/// Copy text into the tmux buffer and the system clipboard when supported.
/// Returns success once the tmux buffer is loaded, even if no system clipboard
/// backend is installed; callers can still paste with tmux's buffer commands.
pub fn copy(socket: Option<&str>, text: &str) -> Result<(), String> {
    let set_clipboard =
        execute_tmux(socket, &["show-options", "-s", "-qv", "set-clipboard"]).unwrap_or_default();
    let termfeatures = execute_tmux(socket, &["display-message", "-p", "#{client_termfeatures}"])
        .unwrap_or_default();
    let clipboard_feature = termfeatures
        .split([',', ' ', '\t'])
        .any(|feature| feature == "clipboard");

    // `external` explicitly permits tmux to send clipboard data to the client.
    let osc52 = matches!(set_clipboard.as_str(), "on" | "external") && clipboard_feature;
    load_buffer(socket, text, osc52)?;
    if osc52 {
        return Ok(());
    }
    match copy_with_system_tool(text) {
        Ok(_) => Ok(()),
        // The tmux buffer is already populated and remains a valid copy target.
        Err(_) => Ok(()),
    }
}

fn copy_with_system_tool(text: &str) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    let candidates: &[(&str, &[&str])] = &[("pbcopy", &[])];
    #[cfg(target_os = "windows")]
    let candidates: &[(&str, &[&str])] = &[("clip.exe", &[])];
    #[cfg(all(unix, not(target_os = "macos")))]
    let candidates: &[(&str, &[&str])] = &[
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("pbcopy", &[]),
    ];
    for (program, arguments) in candidates {
        let mut command = Command::new(program);
        command
            .args(*arguments)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("start {program}: {error}")),
        };
        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            continue;
        };
        let payload = text.as_bytes().to_vec();
        let mut writer = Some(std::thread::spawn(move || {
            let result = stdin.write_all(&payload);
            drop(stdin);
            result
        }));
        let started = Instant::now();
        let status = loop {
            let status = match child.try_wait() {
                Ok(status) => status,
                Err(_error) => {
                    terminate_child(&mut child);
                    let _ = writer.take().map(std::thread::JoinHandle::join);
                    break None;
                }
            };
            if status.is_some()
                && writer
                    .as_ref()
                    .is_some_and(std::thread::JoinHandle::is_finished)
            {
                break status;
            }
            if started.elapsed() < Duration::from_secs(2) {
                std::thread::sleep(Duration::from_millis(5));
            } else {
                terminate_child(&mut child);
                let _ = writer.take().map(std::thread::JoinHandle::join);
                break None;
            }
        };
        let Some(status) = status else { continue };
        let write_result = writer
            .take()
            .ok_or_else(|| format!("{program} writer missing"))?
            .join()
            .map_err(|_| format!("{program} writer stopped unexpectedly"))?;
        if write_result.is_err() {
            continue;
        }
        if status.success() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn terminate_child(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        // SAFETY: this child was placed in a private process group immediately before spawn.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Open only HTTP, HTTPS, or IPFS URLs with the platform URL handler.
pub fn open_url(url: &str) -> Result<(), String> {
    validate_url(url)?;
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    // The browser owns its lifetime. Its launcher may leave inherited pipes
    // open in a browser process, so captured output is not a completion signal.
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("open URL: {e}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("URL opener failed: {status}")),
            Ok(None) if start.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(5))
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("URL opener timed out".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
        }
    }
}

fn validate_url(url: &str) -> Result<(), String> {
    if url
        .bytes()
        .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return Err("URL contains whitespace or control characters".into());
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return Err("URL must use http, https, or ipfs".into());
    };
    if !matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "ipfs"
    ) {
        return Err("URL must use http, https, or ipfs".into());
    }
    if rest.is_empty() {
        return Err("URL destination is empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_url;

    #[test]
    fn url_validation_allows_only_supported_schemes() {
        assert!(validate_url("https://example.com/path").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("ipfs://bafybeigdyr").is_ok());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("https://example.com\nother").is_err());
        assert!(validate_url("https://").is_err());
    }
}
