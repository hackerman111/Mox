//! Application process runner, binary validation, and tmux pane focusing.

use crate::apps::model::{AppConfig, AppLaunchMode};
use crate::tmux::execute_tmux;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Validate whether an executable binary is present and responds successfully to `--version`.
pub fn check_binary(binary: &str) -> Result<(), String> {
    let mut cmd = Command::new(binary);
    cmd.arg("--version");
    let output = crate::process::output(&mut cmd, Duration::from_secs(1), 64 * 1024)
        .map_err(|error| format!("{binary} is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{binary} --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// Search across all tmux panes for an active pane matching current session, command, and working directory.
pub fn find_existing_pane(
    socket: Option<&str>,
    current_session: &str,
    command: &str,
    target_cwd: &str,
) -> Result<Option<String>, String> {
    let panes = execute_tmux(
        socket,
        &[
            "list-panes",
            "-a",
            "-F",
            "#{session_id}\t#{pane_id}\t#{pane_current_command}\t#{pane_current_path}",
        ],
    )?;
    let target_cmd = command.split_whitespace().next().unwrap_or(command);
    for line in panes.lines() {
        let mut fields = line.splitn(4, '\t');
        let (Some(session), Some(pane), Some(cmd), Some(path)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if session == current_session && (cmd == command || cmd == target_cmd) && path == target_cwd
        {
            return Ok(Some(pane.to_string()));
        }
    }
    Ok(None)
}

/// Launch or focus a configured application according to its mode, binary check, and cwd.
pub fn run_app(
    socket: Option<&str>,
    app: &AppConfig,
    cwd: Option<&str>,
    force_window: bool,
) -> Result<(), String> {
    let target_cwd = match cwd {
        Some(path) => path.to_string(),
        None => execute_tmux(socket, &["display-message", "-p", "#{pane_current_path}"])?,
    };
    if !Path::new(&target_cwd).is_dir() {
        return Err(format!("working directory does not exist: {target_cwd}"));
    }

    if app.focus_existing
        && let Ok(current_session) =
            execute_tmux(socket, &["display-message", "-p", "#{session_id}"])
    {
        let bin = app
            .command
            .split_whitespace()
            .next()
            .unwrap_or(&app.command);
        if let Some(pane) = find_existing_pane(socket, &current_session, bin, &target_cwd)? {
            execute_tmux(socket, &["select-window", "-t", &pane])?;
            execute_tmux(socket, &["select-pane", "-t", &pane])?;
            return Ok(());
        }
    }

    if app.check_binary {
        let binary = app
            .command
            .split_whitespace()
            .next()
            .unwrap_or(&app.command);
        check_binary(binary)?;
    }

    if force_window || matches!(app.mode, AppLaunchMode::Window) {
        execute_tmux(socket, &["new-window", "-c", &target_cwd, &app.command])?;
    } else if std::io::stdout().is_terminal() {
        let mut parts = app.command.split_whitespace();
        let binary = parts.next().unwrap_or(&app.command);
        let mut cmd = Command::new(binary);
        cmd.args(parts);
        cmd.current_dir(&target_cwd);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = cmd.exec();
            return Err(format!("failed to exec {}: {err}", app.command));
        }
        #[cfg(not(unix))]
        {
            let status = cmd
                .status()
                .map_err(|e| format!("failed to run {}: {e}", app.command))?;
            if !status.success() {
                return Err(format!("{} exited with error", app.command));
            }
        }
    } else {
        let (width, height) = match &app.mode {
            AppLaunchMode::Popup { width, height } => (width.as_str(), height.as_str()),
            AppLaunchMode::Window => ("85%", "85%"),
        };
        let args = [
            "display-popup",
            "-E",
            "-w",
            width,
            "-h",
            height,
            "-b",
            "rounded",
            "-T",
            &app.title,
            "-d",
            &target_cwd,
            &app.command,
        ];
        execute_tmux(socket, &args)?;
    }

    Ok(())
}
