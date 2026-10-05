//! Arbitrary tmux window tracking and quick-toggling.

use crate::apps::model::TrackedWindow;
use crate::tmux::execute_tmux;

/// Parses output of `tmux list-windows` format:
/// `#{window_id}\t#{window_name}\t#{session_id}\t#{window_active}\t#{@mox_tracked}`.
///
/// Only windows with `@mox_tracked == "1"` are retained.
pub fn parse_tracked_windows(raw: &str) -> Vec<TrackedWindow> {
    let mut tracked = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        let is_tracked = parts[4].trim() == "1";
        if is_tracked {
            tracked.push(TrackedWindow {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                session_id: parts[2].to_string(),
                active: parts[3].trim() == "1",
            });
        }
    }
    tracked
}

/// Query all tracked windows in the current tmux session.
pub fn list_tracked_windows(socket: Option<&str>) -> Result<Vec<TrackedWindow>, String> {
    let raw = execute_tmux(
        socket,
        &[
            "list-windows",
            "-F",
            "#{window_id}\t#{window_name}\t#{session_id}\t#{window_active}\t#{@mox_tracked}",
        ],
    )?;
    Ok(parse_tracked_windows(&raw))
}

/// Check if a specific window target (or current window if None) is tracked.
pub fn is_window_tracked(
    socket: Option<&str>,
    window_target: Option<&str>,
) -> Result<bool, String> {
    let target = match window_target {
        Some(t) => t.trim().to_string(),
        None => {
            let id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])?;
            id.trim().to_string()
        }
    };
    if target.is_empty() {
        return Ok(false);
    }
    let val = execute_tmux(
        socket,
        &["show-option", "-w", "-t", &target, "-qv", "@mox_tracked"],
    )?;
    Ok(val.trim() == "1")
}

/// Mark a window target (or current window if None) as tracked (`@mox_tracked 1`).
pub fn track_window(socket: Option<&str>, window_target: Option<&str>) -> Result<String, String> {
    let target = match window_target {
        Some(t) => t.trim().to_string(),
        None => {
            let id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])?;
            id.trim().to_string()
        }
    };
    if target.is_empty() {
        return Err("No window target specified or found".to_string());
    }
    execute_tmux(
        socket,
        &["set-option", "-w", "-t", &target, "@mox_tracked", "1"],
    )?;
    Ok(target)
}

/// Unmark a window target (or current window if None) from tracking (`@mox_tracked 0`).
pub fn untrack_window(socket: Option<&str>, window_target: Option<&str>) -> Result<String, String> {
    let target = match window_target {
        Some(t) => t.trim().to_string(),
        None => {
            let id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])?;
            id.trim().to_string()
        }
    };
    if target.is_empty() {
        return Err("No window target specified or found".to_string());
    }
    execute_tmux(
        socket,
        &["set-option", "-w", "-t", &target, "@mox_tracked", "0"],
    )?;
    Ok(target)
}

/// Toggles tracking on the current window.
/// Returns `Ok(true)` if now tracked, `Ok(false)` if untracked.
pub fn toggle_current_window_tracking(socket: Option<&str>) -> Result<bool, String> {
    let out = execute_tmux(
        socket,
        &["display-message", "-p", "#{window_id}\t#{@mox_tracked}"],
    )?;
    let mut parts = out.trim().split('\t');
    let win_id = parts.next().unwrap_or("").trim();
    if win_id.is_empty() {
        return Err("Unable to determine current window id".to_string());
    }
    let is_tracked = parts.next().map(|s| s.trim() == "1").unwrap_or(false);
    if is_tracked {
        untrack_window(socket, Some(win_id))?;
        Ok(false)
    } else {
        track_window(socket, Some(win_id))?;
        Ok(true)
    }
}

/// Toggles between active work window and tracked tmux windows.
///
/// 1. Query current window id and its `@mox_tracked` value.
/// 2. If currently in target window (or in a tracked window when target is None):
///    Switch back to previous window stored in `@mox_prev_window` (fallback to `last-window`).
/// 3. If currently in an untracked window (or switching to a specific target):
///    Save current window id to `@mox_prev_window`.
///    Select target window, or last tracked window if target is None.
/// 4. Reset key-table to "root" and refresh-client.
pub fn toggle_tracked_window(
    socket: Option<&str>,
    target_window_id: Option<&str>,
) -> Result<(), String> {
    // 1. Query current window id and its @mox_tracked value:
    let current_info = execute_tmux(
        socket,
        &["display-message", "-p", "#{window_id}\t#{@mox_tracked}"],
    )?;
    let mut parts = current_info.trim().split('\t');
    let current_win_id = parts.next().unwrap_or("").trim();
    let is_tracked = parts.next().map(|s| s.trim() == "1").unwrap_or(false);

    let is_target = target_window_id.is_some_and(|target| target == current_win_id);
    let should_switch_back = if target_window_id.is_some() {
        is_target
    } else {
        is_tracked
    };

    if should_switch_back {
        // 2. If currently in a tracked window (or current_id == target_window_id):
        // Switch back to previous window stored in session @mox_prev_window (with fallback to last-window).
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
            execute_tmux(socket, &["last-window"])?;
        }
    } else {
        // 3. If currently in an untracked window:
        // Save current window id to @mox_prev_window:
        if !current_win_id.is_empty() {
            execute_tmux(socket, &["set-option", "@mox_prev_window", current_win_id])?;
        }

        if let Some(target) = target_window_id {
            execute_tmux(socket, &["select-window", "-t", target])?;
        } else {
            let tracked = list_tracked_windows(socket)?;
            if let Some(last_win) = tracked.last() {
                execute_tmux(socket, &["select-window", "-t", &last_win.id])?;
            } else {
                return Err("No tracked windows found".to_string());
            }
        }
    }

    // 4. Reset key-table to "root" and refresh-client.
    execute_tmux(socket, &["set-option", "key-table", "root"])?;
    let _ = execute_tmux(socket, &["refresh-client", "-S"]);

    Ok(())
}
