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

/// Strips any previously injected mox or moch indicator from a status-left string.
pub fn clean_status_left(s: &str) -> String {
    let mut cleaned = s.trim().to_string();
    if let Some(pos) = cleaned.find("#{E:@mox_indicator}") {
        cleaned.replace_range(pos..pos + "#{E:@mox_indicator}".len(), "");
    }
    if let Some(pos) = cleaned.find("#{E:@moch_indicator}") {
        cleaned.replace_range(pos..pos + "#{E:@moch_indicator}".len(), "");
    }
    while let Some(start) = cleaned.find("#{?#{==") {
        if cleaned[start..].starts_with("#{?#{==:#{key-table},mox")
            || cleaned[start..].starts_with("#{?#{==:#{key-table},moch")
        {
            let mut depth = 0;
            let mut end = None;
            for (i, c) in cleaned[start..].char_indices() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
            }
            if let Some(end_idx) = end {
                cleaned.replace_range(start..end_idx, "");
            } else {
                break;
            }
        } else {
            break;
        }
    }
    cleaned
}

/// Generates the complete tmux script for configuring mox.
pub fn generate_init_script(
    root_keymap: &KeyNode,
    entry_key: &str,
    bin_path: &str,
    current_status_left: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push("# === Mox tmux initialization ===".into());

    // 1. Vim scroll configuration
    lines.extend(scroll::generate_scroll_config());

    // 2. Status indicator configuration
    lines.push("# Modal status indicator".into());
    lines.push("set -g status-left-length 250".into());

    let raw_status_left = current_status_left.unwrap_or("#[fg=black,bg=blue,bold] #S #[default]");
    let base_status_left = clean_status_left(raw_status_left);

    let has_powerline = base_status_left.contains('');
    let indicator_fmt = if has_powerline {
        "#{?#{==:#{key-table},mox},#[fg=black]#[bg=green]#[bold] NORMAL #[fg=green]#[bg=blue],#{?#{==:#{key-table},mox_r},#[fg=black]#[bg=magenta]#[bold] RESIZE #[fg=magenta]#[bg=blue],#{?#{==:#{key-table},mox_w},#[fg=black]#[bg=yellow]#[bold] WINDOW #[fg=yellow]#[bg=blue],#{?#{m:mox_*,#{key-table}},#[fg=black]#[bg=cyan]#[bold] SUBMENU #[fg=cyan]#[bg=blue],}}}}"
    } else {
        "#{?#{==:#{key-table},mox},#[fg=black]#[bg=green]#[bold] NORMAL #[default] ,#{?#{==:#{key-table},mox_r},#[fg=black]#[bg=magenta]#[bold] RESIZE #[default] ,#{?#{==:#{key-table},mox_w},#[fg=black]#[bg=yellow]#[bold] WINDOW #[default] ,#{?#{m:mox_*,#{key-table}},#[fg=black]#[bg=cyan]#[bold] SUBMENU #[default] ,}}}}"
    };

    lines.push(format!(
        "set -g status-left \"{indicator_fmt}{base_status_left}\""
    ));

    // 3. Popup border styling: transparent border background ensures rounded corners do not bleed opaque pixels
    lines.push("set -g popup-border-style 'fg=#81a1c1,bg=default'".into());

    // 4. Entry binding (default: M-m)
    lines.push(format!(
        "bind-key -n {} {{ set-option key-table mox ; refresh-client -S }}",
        entry_key
    ));

    // 5. Generate key-table bindings for root and sub-tables
    generate_table_bindings(root_keymap, "mox", bin_path, &mut lines);

    lines
}

fn generate_table_bindings(
    node: &KeyNode,
    table_name: &str,
    bin_path: &str,
    lines: &mut Vec<String>,
) {
    for child in &node.children {
        // Exit shortcuts: q, Escape, i, M-m
        if child.key == "i" || child.key == "M-m" {
            lines.push(format!(
                "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S }}",
                child.key
            ));
            continue;
        }

        if child.key == "q" || child.key == "Escape" || child.key == "Enter" {
            let cmd = match &child.action {
                Some(KeyAction::ExitModal) => {
                    "{ set-option key-table root ; refresh-client -S }".to_string()
                }
                Some(KeyAction::SwitchTable(target)) => {
                    format!("{{ set-option key-table {target} ; refresh-client -S }}")
                }
                _ => "{ set-option key-table root ; refresh-client -S }".to_string(),
            };
            lines.push(format!("bind-key -T {table_name} {} {cmd}", child.key));
            continue;
        }

        if !child.children.is_empty() {
            let subtable_name = format!("{table_name}_{}", child.key);
            let prefix_arg = if node.key.is_empty() {
                child.key.clone()
            } else {
                format!("{} {}", node.key, child.key)
            };

            let title = if child.sticky {
                format!(" Mox: {} [STICKY] ", child.label)
            } else {
                format!(" Mox: {} ", child.label)
            };

            let (w, h) = if child.key == "r" || child.key == "s" || child.key == "g" {
                (68, 11)
            } else {
                (78, 14)
            };

            let popup_cmd = format!(
                "display-popup -w {w} -h {h} -y S -b rounded -T \"{title}\" -s \"fg=#eceff4,bg=#2e3440\" -S \"fg=#81a1c1,bg=default\" -E \"'{bin_path}' which-key --prefix '{prefix_arg}'\""
            );

            if child.sticky {
                if child.key == "r" {
                    // For sticky resize: switch table without blocking screen, popup available via ?
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table {subtable_name} ; refresh-client -S }}",
                        child.key
                    ));
                } else {
                    // For sticky window mode (w): switch table and show compact which-key
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table {subtable_name} ; refresh-client -S ; {popup_cmd} }}",
                        child.key
                    ));
                }
            } else {
                // Transient submenu
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
                    if cmd.contains("choose-tree") {
                        lines.push(format!(
                            "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; {cmd} }}",
                            child.key
                        ));
                    } else {
                        lines.push(format!("bind-key -T {table_name} {} {cmd}", child.key));
                    }
                }
                KeyAction::SwitchTable(target) => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table {target} ; refresh-client -S }}",
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
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; copy-mode }}",
                        child.key
                    ));
                }
                KeyAction::WhichKey => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} display-popup -w 82 -h 15 -y S -b rounded -T \" Mox: which-key \" -s \"fg=#eceff4,bg=#2e3440\" -S \"fg=#81a1c1,bg=default\" -E \"'{bin_path}' which-key\"",
                        child.key
                    ));
                }
                KeyAction::TeaTimer => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} display-popup -w 50 -h 10 -y C -b rounded -T \" Tea Timer \" -s \"fg=#eceff4,bg=#2e3440\" -S \"fg=#81a1c1,bg=default\" -E \"'{bin_path}' tea\"",
                        child.key
                    ));
                }
                KeyAction::Navigator => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; display-popup -w 90% -h 85% -b rounded -T \" Mox: Navigator \" -s \"fg=#eceff4,bg=#2e3440\" -S \"fg=#81a1c1,bg=default\" -E \"'{bin_path}' nav\" }}",
                        child.key
                    ));
                }

                KeyAction::AgentToggle => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} run-shell \"'{bin_path}' agent-toggle\"",
                        child.key
                    ));
                }
                KeyAction::AgentCreate => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} run-shell \"'{bin_path}' agent-create\"",
                        child.key
                    ));
                }
                KeyAction::ExitModal => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S }}",
                        child.key
                    ));
                }
            }
        }
    }
}
