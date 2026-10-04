//! Pane output logging and capture operations.

use crate::tmux::{capture_to_file, execute_tmux, shell_quote};
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

const OWNER_OPTION: &str = "@mox_logging";

/// Append the pane stream provided on stdin to `path`.
pub fn sink(path: &Path) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("open log file {}: {error}", path.display()))?;
    io::copy(&mut io::stdin().lock(), &mut file)
        .map_err(|error| format!("write log file {}: {error}", path.display()))?;
    Ok(())
}

/// Start appending pane output to a file, unless a pipe is already active.
pub fn start(socket: Option<&str>, pane: &str, path: &Path) -> Result<(), String> {
    let path = prepare_log_path(path)?;
    let path_text = path
        .to_str()
        .ok_or_else(|| "log path is not valid UTF-8".to_string())?;
    let owned = execute_tmux(socket, &["show-options", "-pqv", "-t", pane, OWNER_OPTION])?;
    let state = execute_tmux(
        socket,
        &["display-message", "-p", "-t", pane, "#{pane_pipe}"],
    )?;
    if !state.is_empty() && state != "0" {
        if owned.is_empty() {
            return Err(format!("pane {pane} already has an active pipe"));
        }
        return Err(format!("logging is already active for pane {pane}"));
    }
    if !owned.is_empty() {
        // The sink may have exited before tmux cleared the pane's pipe state.
        execute_tmux(socket, &["set-option", "-pu", "-t", pane, OWNER_OPTION])?;
    }

    let executable =
        std::env::current_exe().map_err(|error| format!("find mox executable: {error}"))?;
    let executable = executable
        .to_str()
        .ok_or_else(|| "mox executable path is not valid UTF-8".to_string())?;
    let command = format!(
        "exec {} log sink {}",
        shell_quote(executable),
        shell_quote(path_text)
    );

    // `-o` prevents this process from replacing an existing external pipe.
    execute_tmux(
        socket,
        &[
            "set-option",
            "-p",
            "-o",
            "-t",
            pane,
            OWNER_OPTION,
            path_text,
        ],
    )?;
    if let Err(error) = execute_tmux(socket, &["pipe-pane", "-o", "-t", pane, &command]) {
        let _ = execute_tmux(socket, &["set-option", "-pu", "-t", pane, OWNER_OPTION]);
        return Err(error);
    }
    let state = execute_tmux(
        socket,
        &["display-message", "-p", "-t", pane, "#{pane_pipe}"],
    )?;
    if state != "1" {
        let _ = execute_tmux(socket, &["set-option", "-pu", "-t", pane, OWNER_OPTION]);
        return Err(format!(
            "logging pipe for pane {pane} exited before becoming active"
        ));
    }
    Ok(())
}

fn prepare_log_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("find current directory: {error}"))?
            .join(path)
    };
    if absolute.to_str().is_none() {
        return Err("log path is not valid UTF-8".to_string());
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&absolute)
        .map_err(|error| format!("open log file {}: {error}", absolute.display()))?;
    if !file
        .metadata()
        .map_err(|error| format!("inspect log file {}: {error}", absolute.display()))?
        .is_file()
    {
        return Err(format!(
            "log destination is not a file: {}",
            absolute.display()
        ));
    }
    drop(file);
    let canonical = absolute
        .canonicalize()
        .map_err(|error| format!("resolve log path {}: {error}", absolute.display()))?;
    if canonical.to_str().is_none() {
        return Err("log path is not valid UTF-8".to_string());
    }
    Ok(canonical)
}

/// Stop logging only when this application owns the pane pipe.
pub fn stop(socket: Option<&str>, pane: &str) -> Result<(), String> {
    let owned = execute_tmux(socket, &["show-options", "-pqv", "-t", pane, OWNER_OPTION])?;
    if owned.is_empty() {
        return Err(format!("pane {pane} is not being logged by mox"));
    }
    let state = execute_tmux(
        socket,
        &["display-message", "-p", "-t", pane, "#{pane_pipe}"],
    )?;
    if state != "1" {
        execute_tmux(socket, &["set-option", "-pu", "-t", pane, OWNER_OPTION])?;
        return Err(format!("logging pipe for pane {pane} is no longer active"));
    }
    execute_tmux(socket, &["pipe-pane", "-t", pane])?;
    execute_tmux(socket, &["set-option", "-pu", "-t", pane, OWNER_OPTION])?;
    Ok(())
}

/// Save the visible screen or complete retained pane history.
pub fn save(socket: Option<&str>, pane: &str, path: &Path, full: bool) -> Result<(), String> {
    capture_to_file(socket, pane, path, full)
}

/// Clear the selected pane's retained history.
pub fn clear(socket: Option<&str>, pane: &str) -> Result<(), String> {
    execute_tmux(socket, &["clear-history", "-t", pane]).map(|_| ())
}
