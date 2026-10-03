//! Window lifecycle and toggling operations for AI agent windows.

use super::model::{WindowSummary, calculate_next_agent_name, parse_window_list_for_agents};
use crate::tmux::execute_tmux;

/// Creates a new AI agent window, sets `@mox_is_agent 1`, records previous window,
/// and switches to root (insert) mode.
pub fn create_agent_window(socket: Option<&str>) -> Result<(), String> {
    // 1. Record current window ID as previous window
    if let Ok(current_win_id) = execute_tmux(socket, &["display-message", "-p", "#{window_id}"]) {
        let _ = execute_tmux(
            socket,
            &["set-option", "@mox_prev_window", current_win_id.trim()],
        );
    }

    // 2. Query existing window names in current session
    let existing_raw =
        execute_tmux(socket, &["list-windows", "-F", "#{window_name}"]).unwrap_or_default();
    let existing_names: Vec<String> = existing_raw
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let win_name = calculate_next_agent_name(&existing_names);

    // 3. Check for custom agent command in @mox_agent_cmd
    let custom_cmd =
        execute_tmux(socket, &["show-option", "-qv", "@mox_agent_cmd"]).unwrap_or_default();
    let custom_cmd = custom_cmd.trim();

    // 4. Create new window
    if custom_cmd.is_empty() {
        execute_tmux(socket, &["new-window", "-n", &win_name])?;
    } else {
        execute_tmux(socket, &["new-window", "-n", &win_name, custom_cmd])?;
    }

    // 5. Mark window as AI agent window
    execute_tmux(socket, &["set-option", "-w", "@mox_is_agent", "1"])?;

    // 6. Switch to insert mode (root key-table)
    execute_tmux(socket, &["set-option", "key-table", "root"])?;
    let _ = execute_tmux(socket, &["refresh-client", "-S"]);

    Ok(())
}

/// Toggles between active work window and AI agent window.
pub fn toggle_agent_window(socket: Option<&str>) -> Result<(), String> {
    // Check current window ID and agent status
    let current_info = execute_tmux(
        socket,
        &["display-message", "-p", "#{window_id}\t#{@mox_is_agent}"],
    )?;

    let parts: Vec<&str> = current_info.trim().split('\t').collect();
    let current_win_id = parts.first().copied().unwrap_or("").trim();
    let is_agent = parts.get(1).map(|s| s.trim() == "1").unwrap_or(false);

    if is_agent {
        // We are currently in an agent window -> switch back to previous window
        let prev_win =
            execute_tmux(socket, &["show-option", "-qv", "@mox_prev_window"]).unwrap_or_default();
        let prev_win = prev_win.trim();

        let windows_raw =
            execute_tmux(socket, &["list-windows", "-F", "#{window_id}"]).unwrap_or_default();
        let win_exists = windows_raw
            .lines()
            .any(|w| w.trim() == prev_win && !prev_win.is_empty());

        if win_exists {
            execute_tmux(socket, &["select-window", "-t", prev_win])?;
        } else {
            // Fallback to last-window
            execute_tmux(socket, &["last-window"])?;
        }

        // Switch to insert mode
        execute_tmux(socket, &["set-option", "key-table", "root"])?;
        let _ = execute_tmux(socket, &["refresh-client", "-S"]);
    } else {
        // We are in a normal work window -> remember it and switch to agent window
        if !current_win_id.is_empty() {
            let _ = execute_tmux(socket, &["set-option", "@mox_prev_window", current_win_id]);
        }

        // Find agent windows in current session
        let list_raw = execute_tmux(
            socket,
            &[
                "list-windows",
                "-F",
                "#{window_id}\t#{window_name}\t#{@mox_is_agent}",
            ],
        )?;
        let windows = parse_window_list_for_agents(&list_raw);
        let agent_windows: Vec<&WindowSummary> = windows.iter().filter(|w| w.is_agent).collect();

        if let Some(agent_win) = agent_windows.last() {
            execute_tmux(socket, &["select-window", "-t", &agent_win.id])?;
            execute_tmux(socket, &["set-option", "key-table", "root"])?;
            let _ = execute_tmux(socket, &["refresh-client", "-S"]);
        } else {
            // No agent window exists yet, create one!
            create_agent_window(socket)?;
        }
    }

    Ok(())
}
