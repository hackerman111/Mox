//! Interactive event loop and action dispatching for the which-key popup.

use super::view::{prompt_confirm, render_menu};
use crate::keymap::{KeyAction, KeyNode, build_default_keymap};
use crate::tmux::execute_tmux;
use crate::tui::RawModeGuard;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

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
                        KeyAction::Flash(cfg) => {
                            let bin = std::env::current_exe().map_err(|e| e.to_string())?;
                            let command = crate::flash::build_flash_command(
                                &bin.to_string_lossy(),
                                cfg,
                                socket,
                            );
                            launch_after_menu(socket, &format!("{command} --launch-popup"))?;
                            break;
                        }
                        KeyAction::Extract => {
                            let bin = std::env::current_exe().map_err(|e| e.to_string())?;
                            let mut command = format!("'{}' extract --launch-popup", bin.display());
                            if let Some(socket) = socket {
                                command.push_str(&format!(" --socket '{socket}'"));
                            }
                            launch_after_menu(socket, &command)?;
                            break;
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
                            crate::nav::run_navigator(socket)?;
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

/// The background job survives closing the menu; tmux does not support nested popups.
fn launch_after_menu(socket: Option<&str>, command: &str) -> Result<(), String> {
    execute_tmux(socket, &["set-option", "key-table", "root"])?;
    let tmux = socket.map_or_else(|| "tmux".to_string(), |s| format!("tmux -L '{s}'"));
    execute_tmux(
        socket,
        &[
            "run-shell",
            "-b",
            &format!("{tmux} display-popup -C; {command}"),
        ],
    )?;
    Ok(())
}
