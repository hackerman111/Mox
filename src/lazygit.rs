//! Open or focus LazyGit in the current tmux session.

use crate::tmux::{execute_tmux, popup};
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Focus an existing LazyGit pane at `cwd`, or open LazyGit in a popup/window.
pub fn open(socket: Option<&str>, cwd: Option<&str>, window: bool) -> Result<(), String> {
    let target_cwd = match cwd {
        Some(path) => path.to_string(),
        None => execute_tmux(socket, &["display-message", "-p", "#{pane_current_path}"])?,
    };
    if !Path::new(&target_cwd).is_dir() {
        return Err(format!("working directory does not exist: {target_cwd}"));
    }

    let current_session = execute_tmux(socket, &["display-message", "-p", "#{session_id}"])?;
    let panes = execute_tmux(
        socket,
        &[
            "list-panes",
            "-a",
            "-F",
            "#{session_id}\t#{pane_id}\t#{pane_current_command}\t#{pane_current_path}",
        ],
    )?;
    for line in panes.lines() {
        let mut fields = line.splitn(4, '\t');
        let (Some(session), Some(pane), Some(command), Some(path)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if session == current_session && command == "lazygit" && path == target_cwd {
            execute_tmux(socket, &["select-window", "-t", pane])?;
            execute_tmux(socket, &["select-pane", "-t", pane])?;
            return Ok(());
        }
    }

    let mut version = Command::new("lazygit");
    version.arg("--version");
    let output = crate::process::output(&mut version, Duration::from_secs(1), 64 * 1024)
        .map_err(|error| format!("lazygit is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "lazygit --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    if window {
        execute_tmux(socket, &["new-window", "-c", &target_cwd, "lazygit"])?;
    } else if std::io::stdout().is_terminal() {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = Command::new("lazygit").current_dir(&target_cwd).exec();
            return Err(format!("failed to exec lazygit: {err}"));
        }
        #[cfg(not(unix))]
        {
            let status = Command::new("lazygit")
                .current_dir(&target_cwd)
                .status()
                .map_err(|e| format!("failed to run lazygit: {e}"))?;
            if !status.success() {
                return Err("lazygit exited with error".to_string());
            }
        }
    } else {
        popup(socket, "LazyGit", Some(&target_cwd), "lazygit")?;
    }
    Ok(())
}
