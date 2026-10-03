//! Which-key popup UI and interactive event loop.

use crate::keymap::{KeyAction, KeyNode, build_default_keymap};
use crate::tmux::execute_tmux;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
        let mut out = stdout();
        let _ = execute!(out, cursor::Hide);
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = execute!(out, cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}

/// Runs the interactive which-key loop inside a tmux popup.
pub fn run_which_key(prefix_str: &str, socket: Option<&str>) -> Result<(), String> {
    let root = build_default_keymap();
    let prefix_tokens: Vec<&str> = prefix_str.split_whitespace().collect();

    let initial_node = root.resolve_path(&prefix_tokens).unwrap_or(&root);

    let _guard = RawModeGuard::new()?;
    let mut current_node = initial_node;
    let mut history_stack: Vec<&KeyNode> = Vec::new();
    let mut status_message: Option<String> = None;

    loop {
        render_menu(
            current_node,
            &history_stack,
            prefix_str,
            status_message.as_deref(),
        )?;
        status_message = None;

        if let Event::Key(key_event) = event::read().map_err(|e| e.to_string())? {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }

            // Alt-m: toggle/exit modal completely
            if key_event.modifiers.contains(KeyModifiers::ALT)
                && key_event.code == KeyCode::Char('m')
            {
                let _ = execute_tmux(socket, &["set-option", "key-table", "root"]);
                let _ = execute_tmux(socket, &["refresh-client", "-S"]);
                break;
            }

            let key_str = match key_event.code {
                KeyCode::Char(' ') => "Space".to_string(),
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Esc => "Escape".to_string(),
                KeyCode::Backspace => "Backspace".to_string(),
                KeyCode::Enter => "Enter".to_string(),
                KeyCode::Tab => "Tab".to_string(),
                _ => continue,
            };

            // Look up key in children
            if let Some(child) = current_node.find_child(&key_str) {
                if !child.children.is_empty() {
                    // Navigate down to child submenu
                    history_stack.push(current_node);
                    current_node = child;
                    continue;
                }

                if let Some(action) = &child.action {
                    match action {
                        KeyAction::Tmux(cmd) => {
                            if cmd.contains("choose-tree") {
                                let _ = execute_tmux(socket, &["set-option", "key-table", "root"]);
                                let _ = execute_tmux(socket, &["refresh-client", "-S"]);
                                let _ = execute_shell_tmux(socket, cmd);
                                break;
                            } else {
                                let _ = execute_shell_tmux(socket, cmd);
                                if current_node.sticky {
                                    status_message = Some(format!("Executed: {}", child.label));
                                    continue;
                                } else {
                                    break;
                                }
                            }
                        }
                        KeyAction::SwitchTable(table) => {
                            let _ = execute_tmux(socket, &["set-option", "key-table", table]);
                            let _ = execute_tmux(socket, &["refresh-client", "-S"]);
                            break;
                        }
                        KeyAction::Confirm { prompt, command } => {
                            if prompt_confirm(prompt)? {
                                let _ = execute_shell_tmux(socket, command);
                                if current_node.sticky {
                                    status_message = Some(format!("Confirmed: {}", command));
                                    continue;
                                } else {
                                    break;
                                }
                            } else {
                                status_message = Some("Cancelled".into());
                                continue;
                            }
                        }
                        KeyAction::CopyMode => {
                            let _ = execute_tmux(socket, &["set-option", "key-table", "root"]);
                            let _ = execute_tmux(socket, &["refresh-client", "-S"]);
                            let _ = execute_tmux(socket, &["copy-mode"]);
                            break;
                        }
                        KeyAction::TeaTimer => {
                            crate::tea::run_tea_timer()?;
                            break;
                        }
                        KeyAction::Navigator => {
                            let _ = execute_tmux(socket, &["set-option", "key-table", "root"]);
                            let _ = execute_tmux(socket, &["refresh-client", "-S"]);
                            let bin_path = std::env::current_exe()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| "mox".to_string());
                            let _ = execute_tmux(
                                socket,
                                &[
                                    "display-popup",
                                    "-w",
                                    "90%",
                                    "-h",
                                    "85%",
                                    "-b",
                                    "rounded",
                                    "-T",
                                    " Mox: Navigator ",
                                    "-s",
                                    "fg=#eceff4,bg=#2e3440",
                                    "-S",
                                    "fg=#81a1c1,bg=default",
                                    "-E",
                                    &format!("'{bin_path}' nav"),
                                ],
                            );
                            break;
                        }
                        KeyAction::AgentToggle => {
                            let _ = crate::agent::toggle_agent_window(socket);
                            break;
                        }
                        KeyAction::AgentCreate => {
                            let _ = crate::agent::create_agent_window(socket);
                            break;
                        }
                        KeyAction::WhichKey => {
                            continue;
                        }

                        KeyAction::ExitModal => {
                            let _ = execute_tmux(socket, &["set-option", "key-table", "root"]);
                            let _ = execute_tmux(socket, &["refresh-client", "-S"]);
                            break;
                        }
                    }
                }
            } else {
                // If not matched, handle Esc/q/Backspace for navigation
                if key_str == "Escape" || key_str == "q" || key_str == "Backspace" {
                    if let Some(prev) = history_stack.pop() {
                        current_node = prev;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn execute_shell_tmux(socket: Option<&str>, cmd_str: &str) -> Result<(), String> {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c");
    if let Some(s) = socket {
        cmd.arg(format!("tmux -L {s} {cmd_str}"));
    } else {
        cmd.arg(format!("tmux {cmd_str}"));
    }
    let _ = cmd.status();
    Ok(())
}

fn build_display_items(node: &KeyNode) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let keys: std::collections::HashSet<&str> =
        node.children.iter().map(|c| c.key.as_str()).collect();

    let has_digits = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
        .iter()
        .all(|d| keys.contains(d));

    let mut skip_keys = std::collections::HashSet::new();
    skip_keys.insert("Backspace");
    skip_keys.insert("q");
    skip_keys.insert("Escape");
    skip_keys.insert("Enter");
    skip_keys.insert("i");
    skip_keys.insert("M-m");

    if has_digits {
        for d in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            skip_keys.insert(d);
        }
    }

    if keys.contains("d") && keys.contains("x") {
        skip_keys.insert("x");
    }
    if keys.contains("D") && keys.contains("X") {
        skip_keys.insert("X");
    }
    if keys.contains("t") && keys.contains("T") {
        skip_keys.insert("T");
    }
    if keys.contains("o") && keys.contains("Tab") {
        skip_keys.insert("Tab");
    }
    if keys.contains("?") && keys.contains("Space") {
        skip_keys.insert("Space");
    }

    for child in &node.children {
        if skip_keys.contains(child.key.as_str()) {
            continue;
        }

        let key_disp = if child.key == "d" && keys.contains("x") {
            "x/d".to_string()
        } else if child.key == "D" && keys.contains("X") {
            "X/D".to_string()
        } else if child.key == "t" && keys.contains("T") {
            "t/T".to_string()
        } else if child.key == "o" && keys.contains("Tab") {
            "o/Tab".to_string()
        } else if child.key == "?" && keys.contains("Space") {
            "?/␣".to_string()
        } else if child.key == "Space" {
            "␣".to_string()
        } else {
            child.key.clone()
        };

        let label = if !child.children.is_empty() {
            format!("{}…", child.label)
        } else {
            child.label.clone()
        };

        items.push((key_disp, label));
    }

    if has_digits {
        items.push(("0-9".to_string(), "select win".to_string()));
    }

    items
}

fn render_menu(
    node: &KeyNode,
    _history: &[&KeyNode],
    _prefix_str: &str,
    status_msg: Option<&str>,
) -> Result<(), String> {
    let mut out = stdout();
    execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0))
        .map_err(|e| format!("Render error: {e}"))?;

    let (cols, _rows) = terminal::size().unwrap_or((82, 15));

    let mode_name = if node.label.is_empty() {
        "NORMAL"
    } else {
        &node.label
    };
    let bg_color = match mode_name.to_uppercase().as_str() {
        "RESIZE" => Color::Magenta,
        "WINDOW" => Color::Yellow,
        _ => Color::Green,
    };

    execute!(
        out,
        Print("  "),
        SetBackgroundColor(bg_color),
        SetForegroundColor(Color::Black),
        Print(format!(" {} ", mode_name.to_uppercase())),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(if node.sticky {
            "  [STICKY]"
        } else {
            "  which-key"
        }),
        ResetColor,
        Print("\r\n\r\n")
    )
    .map_err(|e| e.to_string())?;

    let display_items = build_display_items(node);
    let num_columns = if cols >= 68 && display_items.len() > 6 {
        3
    } else if cols >= 44 && display_items.len() > 3 {
        2
    } else {
        1
    };

    let col_w = (cols as usize / num_columns).max(22);

    for chunk in display_items.chunks(num_columns) {
        let mut line = String::new();
        for (key, label) in chunk {
            let key_styled = format!("\x1b[1;33m{:<5}\x1b[0m", key);
            let max_label_len = col_w.saturating_sub(9).max(8);
            let trunc_label: String = if label.chars().count() > max_label_len {
                label.chars().take(max_label_len).collect()
            } else {
                label.clone()
            };
            let entry = format!(
                "  {key_styled}  {:<width$}",
                trunc_label,
                width = max_label_len
            );
            line.push_str(&entry);
        }
        execute!(out, Print(format!("{}\r\n", line.trim_end()))).map_err(|e| e.to_string())?;
    }

    if let Some(msg) = status_msg {
        execute!(
            out,
            Print("\r\n"),
            SetForegroundColor(Color::Green),
            Print(format!("  > {msg}\r\n")),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
    } else {
        execute!(
            out,
            Print("\r\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("  [key] run    [q/Esc] back    [i] insert\r\n"),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
    }

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn prompt_confirm(prompt: &str) -> Result<bool, String> {
    let mut out = stdout();
    execute!(
        out,
        cursor::MoveToNextLine(1),
        SetForegroundColor(Color::Red),
        Print(format!("{prompt} ")),
        ResetColor
    )
    .map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;

    loop {
        if let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                _ => {}
            }
        }
    }
}
