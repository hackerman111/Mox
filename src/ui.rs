//! Which-key popup UI and interactive event loop.

use crate::keymap::{KeyAction, KeyNode, build_default_keymap};
use crate::tmux::execute_tmux;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
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
                let _ = execute_tmux(socket, &["switch-client", "-T", "root"]);
                break;
            }

            let key_str = match key_event.code {
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Esc => "Escape".to_string(),
                KeyCode::Backspace => "Backspace".to_string(),
                KeyCode::Enter => "Enter".to_string(),
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
                            let _ = execute_shell_tmux(socket, cmd);
                            if current_node.sticky {
                                status_message = Some(format!("Executed: {}", child.label));
                                continue;
                            } else {
                                break;
                            }
                        }
                        KeyAction::SwitchTable(_table) => {
                            if !history_stack.is_empty() {
                                current_node = history_stack.pop().unwrap();
                                continue;
                            } else {
                                break;
                            }
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
                            let _ = execute_tmux(socket, &["copy-mode"]);
                            break;
                        }
                        KeyAction::TeaTimer => {
                            crate::tea::run_tea_timer()?;
                            break;
                        }
                        KeyAction::WhichKey => {
                            continue;
                        }
                        KeyAction::ExitModal => {
                            let _ = execute_tmux(socket, &["switch-client", "-T", "root"]);
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

fn render_menu(
    node: &KeyNode,
    _history: &[&KeyNode],
    prefix_str: &str,
    status_msg: Option<&str>,
) -> Result<(), String> {
    let mut out = stdout();
    execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0))
        .map_err(|e| format!("Render error: {e}"))?;

    let (cols, _rows) = terminal::size().unwrap_or((80, 24));

    // Header
    let prefix_display = if !node.key.is_empty() {
        format!("{prefix_str} {}", node.key)
    } else if !prefix_str.is_empty() {
        prefix_str.to_string()
    } else {
        "root".to_string()
    };

    if node.sticky {
        execute!(
            out,
            SetForegroundColor(Color::Black),
            crossterm::style::SetBackgroundColor(Color::Yellow),
            Print(format!(" STICKY: {} ", node.label.to_uppercase())),
            ResetColor,
            Print(format!("  Prefix: {}\r\n\r\n", prefix_display))
        )
        .map_err(|e| e.to_string())?;
    } else {
        execute!(
            out,
            SetForegroundColor(Color::Cyan),
            Print(format!("Prefix: {}\r\n\r\n", prefix_display)),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
    }

    // Grid of options
    let items: Vec<_> = node
        .children
        .iter()
        .filter(|c| c.key != "Backspace")
        .collect();

    let num_columns = if cols >= 60 { 2 } else { 1 };
    let col_width = (cols as usize / num_columns).saturating_sub(2);

    for chunk in items.chunks(num_columns) {
        let mut line = String::new();
        for item in chunk {
            let key_styled = format!("[{}]", item.key);
            let label = if !item.children.is_empty() {
                format!("{}...", item.label)
            } else {
                item.label.clone()
            };
            let entry = format!(
                "{:<5} {:<width$}",
                key_styled,
                label,
                width = col_width.saturating_sub(6)
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
            Print(format!("> {msg}\r\n")),
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
