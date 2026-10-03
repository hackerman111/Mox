//! Tmux CLI interaction and configuration generation.

use crate::keymap::{KeyAction, KeyNode};
use crate::scroll;
use std::process::Command;

/// Execute a tmux command with optional socket name.
pub fn execute_tmux(socket: Option<&str>, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("tmux");
    if let Some(s) = socket {
        cmd.args(["-L", s]);
    }
    cmd.args(args);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn tmux: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux command failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Applies a list of tmux configuration lines via `source-file -`.
pub fn apply_tmux_commands(socket: Option<&str>, commands: &[String]) -> Result<(), String> {
    let mut cmd = Command::new("tmux");
    if let Some(s) = socket {
        cmd.args(["-L", s]);
    }
    cmd.args(["source-file", "-"]);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn tmux: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        for line in commands {
            if !line.trim().is_empty() {
                let _ = writeln!(stdin, "{line}");
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for tmux: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux source-file failed: {stderr}"));
    }

    Ok(())
}

/// Generates the complete tmux script for configuring moch.
pub fn generate_init_script(root_keymap: &KeyNode, entry_key: &str, bin_path: &str) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push("# === moch tmux initialization ===".into());

    // 1. Vim scroll configuration
    lines.extend(scroll::generate_scroll_config());

    // 2. Status indicator helper
    lines.push("# Modal status indicator".into());
    lines.push("set -g @moch_status_normal \"#[fg=black,bg=green,bold] MODAL #[default]\"".into());
    lines
        .push("set -g @moch_status_sticky \"#[fg=black,bg=yellow,bold] STICKY #[default]\"".into());

    // 3. Entry binding (default: M-m)
    lines.push(format!(
        "bind-key -n {} {{ set-option key-table moch ; display-message \"── MODAL ──\" }}",
        entry_key
    ));

    // 4. Generate key-table bindings for root and intermediate tables
    generate_table_bindings(root_keymap, "moch", bin_path, &mut lines);

    lines
}

fn generate_table_bindings(
    node: &KeyNode,
    table_name: &str,
    bin_path: &str,
    lines: &mut Vec<String>,
) {
    for child in &node.children {
        if child.key == "q" || child.key == "Escape" || child.key == "M-m" {
            let cmd = match &child.action {
                Some(KeyAction::ExitModal) => {
                    "{ set-option key-table root ; display-message \"── INSERT ──\" }".to_string()
                }
                Some(KeyAction::SwitchTable(target)) => format!("set-option key-table {target}"),
                _ => "{ set-option key-table root ; display-message \"── INSERT ──\" }".to_string(),
            };
            lines.push(format!("bind-key -T {table_name} {} {cmd}", child.key));
            continue;
        }

        if !child.children.is_empty() {
            let subtable_name = format!("moch_{}", child.key);

            // Incomplete sequence / submenu prefix
            let prefix_arg = if node.key.is_empty() {
                child.key.clone()
            } else {
                format!("{} {}", node.key, child.key)
            };

            let title = if child.sticky {
                format!(" moch: {} [STICKY] ", child.label)
            } else {
                format!(" moch: {} ", child.label)
            };

            let popup_cmd = format!(
                "display-popup -w 72 -h 12 -y S -b rounded -T \"{title}\" -E \"'{bin_path}' which-key --prefix '{prefix_arg}'\""
            );

            if child.sticky {
                // For sticky mode, set key-table to subtable and show which-key
                lines.push(format!(
                    "bind-key -T {table_name} {} {{ set-option key-table {subtable_name} ; {popup_cmd} }}",
                    child.key
                ));
            } else {
                // For transient submenu, switch-client and show which-key
                lines.push(format!(
                    "bind-key -T {table_name} {} {{ switch-client -T {subtable_name} ; {popup_cmd} }}",
                    child.key
                ));
            }

            // Also generate sub-table bindings
            generate_table_bindings(child, &subtable_name, bin_path, lines);
        } else if let Some(action) = &child.action {
            match action {
                KeyAction::Tmux(cmd) => {
                    lines.push(format!("bind-key -T {table_name} {} {cmd}", child.key));
                }
                KeyAction::SwitchTable(target) => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} set-option key-table {target}",
                        child.key
                    ));
                }
                KeyAction::Confirm { prompt, command } => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} confirm-before -p \"{prompt}\" \"{command}\"",
                        child.key
                    ));
                }
                KeyAction::CopyMode => {
                    lines.push(format!("bind-key -T {table_name} {} copy-mode", child.key));
                }
                KeyAction::WhichKey => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} display-popup -w 72 -h 14 -y S -b rounded -T \" moch: which-key \" -E \"'{bin_path}' which-key\"",
                        child.key
                    ));
                }
                KeyAction::TeaTimer => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} display-popup -w 46 -h 10 -y C -b rounded -T \" Tea Timer \" -E \"'{bin_path}' tea\"",
                        child.key
                    ));
                }
                KeyAction::ExitModal => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; display-message \"── INSERT ──\" }}",
                        child.key
                    ));
                }
            }
        }
    }
}
