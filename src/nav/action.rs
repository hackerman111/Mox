//! Tmux action command generation and execution for tree navigator items.

use super::model::TreeItem;
use crate::tmux::execute_tmux;

/// Switches to the given tree item (session, window, or pane).
pub fn switch_to_item(socket: Option<&str>, item: &TreeItem) -> Result<(), String> {
    match item {
        TreeItem::Session(s) => {
            let _ = execute_tmux(socket, &["switch-client", "-t", &s.id]);
        }
        TreeItem::Window { window, .. } => {
            let _ = execute_tmux(socket, &["select-window", "-t", &window.id]);
            let _ = execute_tmux(socket, &["switch-client", "-t", &window.id]);
        }
        TreeItem::Pane { window, pane, .. } => {
            let _ = execute_tmux(socket, &["select-pane", "-t", &pane.id]);
            let _ = execute_tmux(socket, &["select-window", "-t", &window.id]);
            let _ = execute_tmux(socket, &["switch-client", "-t", &pane.id]);
        }
    }
    Ok(())
}

/// Generates the tmux command string to switch to a given tree item.
pub fn format_switch_command(item: &TreeItem) -> String {
    match item {
        TreeItem::Session(s) => format!("switch-client -t {}", s.id),
        TreeItem::Window { window, .. } => format!("select-window -t {}", window.id),
        TreeItem::Pane { window, pane, .. } => {
            format!(
                "select-pane -t {} ; select-window -t {}",
                pane.id, window.id
            )
        }
    }
}

/// Generates the tmux command string to kill a given tree item.
pub fn format_kill_command(item: &TreeItem) -> String {
    match item {
        TreeItem::Session(s) => format!("kill-session -t {}", s.id),
        TreeItem::Window { window, .. } => format!("kill-window -t {}", window.id),
        TreeItem::Pane { pane, .. } => format!("kill-pane -t {}", pane.id),
    }
}

/// Generates the tmux command string to rename a session or window.
pub fn format_rename_command(item: &TreeItem, new_name: &str) -> Option<String> {
    let clean_name = new_name.replace('\'', "");
    match item {
        TreeItem::Session(s) => Some(format!("rename-session -t {} '{clean_name}'", s.id)),
        TreeItem::Window { window, .. } => {
            Some(format!("rename-window -t {} '{clean_name}'", window.id))
        }
        TreeItem::Pane { .. } => None,
    }
}

/// Executes a tmux command line for navigator actions.
pub fn execute_nav_tmux(socket: Option<&str>, cmd_str: &str) -> Result<(), String> {
    crate::tmux::apply_tmux_commands(socket, &[cmd_str.to_string()])
}
