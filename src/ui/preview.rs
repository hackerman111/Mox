//! Bounded preview providers; commands are displayed, never executed.
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preview {
    Text(String),
    File(PathBuf),
    Directory(PathBuf),
    Pane(String),
}

pub fn load(source: &Preview, socket: Option<&str>) -> Result<Vec<String>, String> {
    let text = match source {
        Preview::Text(text) => text.chars().take(65536).collect(),
        Preview::Pane(pane) => {
            // Actual tmux execution remains owned by its API layer.
            crate::tmux::preview_capture(socket, pane)?
        }
        Preview::Directory(path) => {
            return local_directory(path);
        }
        Preview::File(path) => {
            return local_file(path);
        }
    };
    Ok(to_lines(&text))
}

/// Loads at most 64 KiB from a regular file and returns at most 200 sanitized lines.
pub fn local_file(path: &Path) -> Result<Vec<String>, String> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Preview only supports regular files".into());
    }
    let mut bytes = Vec::new();
    file.take(65_536)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.contains(&0) {
        return Ok(vec!["Binary file".into()]);
    }
    Ok(to_lines(&String::from_utf8_lossy(&bytes)))
}

/// Loads at most 200 directory entries as sorted, sanitized preview lines.
pub fn local_directory(path: &Path) -> Result<Vec<String>, String> {
    let mut entries = std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .take(200)
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    entries.sort();
    Ok(to_lines(&entries.join("\n")))
}

fn to_lines(text: &str) -> Vec<String> {
    BufReader::new(text.as_bytes())
        .lines()
        .take(200)
        .map(|line| {
            line.unwrap_or_default()
                .chars()
                .map(|ch| if ch == '\t' { ' ' } else { ch })
                .filter(|ch| !ch.is_control())
                .collect()
        })
        .collect()
}

fn load_local_process(command_name: &str, path: &Path) -> Result<Vec<String>, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = Command::new(executable);
    command.args([command_name, "--path"]).arg(path);
    let output = crate::process::output(&mut command, Duration::from_millis(500), 1_048_576)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid preview response: {e}"))
}

/// One bounded worker per picker. Drop closes requests and joins its owned thread.
pub struct PreviewWorker {
    sender: Option<std::sync::mpsc::SyncSender<(u64, Preview)>>,
    receiver: std::sync::mpsc::Receiver<(u64, Vec<String>)>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl PreviewWorker {
    pub fn new(socket: Option<&str>) -> Self {
        let (sender, requests) = std::sync::mpsc::sync_channel::<(u64, Preview)>(1);
        let (results, receiver) = std::sync::mpsc::sync_channel(2);
        let socket = socket.map(str::to_owned);
        let thread = std::thread::spawn(move || {
            while let Ok((generation, source)) = requests.recv() {
                let lines = match &source {
                    Preview::File(path) => load_local_process("preview-file", path),
                    Preview::Directory(path) => load_local_process("preview-directory", path),
                    _ => load(&source, socket.as_deref()),
                }
                .unwrap_or_else(|error| vec![error]);
                if let Err(std::sync::mpsc::TrySendError::Disconnected(_)) =
                    results.try_send((generation, lines))
                {
                    break;
                }
            }
        });
        Self {
            sender: Some(sender),
            receiver,
            thread: Some(thread),
        }
    }
    pub fn request(&self, generation: u64, source: &Preview) -> bool {
        self.sender
            .as_ref()
            .is_some_and(|s| s.try_send((generation, source.clone())).is_ok())
    }
    pub fn result(&self, generation: u64) -> Option<Vec<String>> {
        let mut latest = None;
        while let Ok((version, lines)) = self.receiver.try_recv() {
            if version == generation {
                latest = Some(lines);
            }
        }
        latest
    }
}
impl Drop for PreviewWorker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
