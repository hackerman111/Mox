//! Interactive event loop and key event handling for the tree navigator.

use super::action::{execute_nav_tmux, format_kill_command, format_rename_command, switch_to_item};
use super::model::{TreeItem, fetch_tmux_snapshot, flatten_tree};
use super::state::NavigatorApp;
use super::view::render_navigator;
use crate::tmux::execute_tmux;
use crate::tui::RawModeGuard;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal,
};
use std::io::stdout;

/// Runs the interactive Navigator TUI inside a tmux popup.
pub fn run_navigator(socket: Option<&str>) -> Result<(), String> {
    let _guard = RawModeGuard::new()?;

    let sessions = fetch_tmux_snapshot(socket)?;
    let mut app = NavigatorApp::new(sessions);

    let mut out = stdout();

    loop {
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let term_w = term_w as usize;
        let term_h = term_h as usize;

        // Fetch preview for current selected item if needed
        let filtered = app.filtered_sessions();
        let items = flatten_tree(&filtered);

        if app.selected_index >= items.len() && !items.is_empty() {
            app.selected_index = items.len() - 1;
        }

        let active_pane = if let Some(item) = items.get(app.selected_index) {
            item.active_pane_id().to_string()
        } else {
            String::new()
        };

        if app.show_preview && active_pane != app.cached_preview_pane && !active_pane.is_empty() {
            app.cached_preview_pane = active_pane.clone();
            let preview_raw = execute_tmux(socket, &["capture-pane", "-ep", "-t", &active_pane])
                .unwrap_or_default();
            app.cached_preview_lines = preview_raw.lines().map(|s| s.to_string()).collect();
        }

        render_navigator(&app, &items, term_w, term_h, &mut out)?;

        if let Event::Key(key_event) = event::read().map_err(|e| e.to_string())? {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }

            // Alt-m: always exit popup
            if key_event.modifiers.contains(KeyModifiers::ALT)
                && key_event.code == KeyCode::Char('m')
            {
                break;
            }

            // 1. If Confirmation Dialog is active
            if let Some((_, cmd)) = app.confirm_action.take() {
                match key_event.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        let _ = execute_nav_tmux(socket, &cmd);
                        // Refresh snapshot
                        if let Ok(fresh) = fetch_tmux_snapshot(socket) {
                            app.sessions = fresh;
                        }
                    }
                    _ => {
                        // Cancelled
                    }
                }
                continue;
            }

            // 2. If Rename Input Dialog is active
            if let Some((target_type, target_id)) = app.rename_target.take() {
                match key_event.code {
                    KeyCode::Enter => {
                        let new_name = app.rename_buffer.trim().to_string();
                        if !new_name.is_empty() {
                            if let Some(item) = items.iter().find(|item| match item {
                                TreeItem::Session(s) => {
                                    target_type == "session" && s.id == target_id
                                }
                                TreeItem::Window { window, .. } => {
                                    target_type == "window" && window.id == target_id
                                }
                                TreeItem::Pane { pane, .. } => {
                                    target_type == "pane" && pane.id == target_id
                                }
                            }) && let Some(cmd) = format_rename_command(item, &new_name)
                            {
                                execute_nav_tmux(socket, &cmd)?;
                            }
                            if let Ok(fresh) = fetch_tmux_snapshot(socket) {
                                app.sessions = fresh;
                            }
                        }
                        app.rename_buffer.clear();
                    }
                    KeyCode::Esc => {
                        app.rename_buffer.clear();
                    }
                    KeyCode::Backspace => {
                        app.rename_buffer.pop();
                        app.rename_target = Some((target_type, target_id));
                    }
                    KeyCode::Char(c) => {
                        app.rename_buffer.push(c);
                        app.rename_target = Some((target_type, target_id));
                    }
                    _ => {
                        app.rename_target = Some((target_type, target_id));
                    }
                }
                continue;
            }

            // 3. Search Mode
            if app.in_search_mode {
                match key_event.code {
                    KeyCode::Esc => {
                        if app.search_query.is_empty() {
                            app.in_search_mode = false;
                        } else {
                            app.search_query.clear();
                        }
                        app.selected_index = 0;
                    }
                    KeyCode::Enter => {
                        if let Some(item) = items.get(app.selected_index) {
                            let _ = switch_to_item(socket, item);
                            break;
                        }
                    }
                    KeyCode::Down if !items.is_empty() && app.selected_index + 1 < items.len() => {
                        app.selected_index += 1;
                    }
                    KeyCode::Up if app.selected_index > 0 => {
                        app.selected_index -= 1;
                    }

                    KeyCode::Tab => {
                        app.toggle_selected_collapse(app.selected_index);
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.selected_index = 0;
                    }
                    KeyCode::Char(c) => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'j' {
                            if !items.is_empty() && app.selected_index + 1 < items.len() {
                                app.selected_index += 1;
                            }
                        } else if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'k' {
                            if app.selected_index > 0 {
                                app.selected_index -= 1;
                            }
                        } else {
                            app.search_query.push(c);
                            app.selected_index = 0;
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // 4. Normal Navigation Mode
            match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    break;
                }
                KeyCode::Char('/') => {
                    app.in_search_mode = true;
                }
                KeyCode::Char('?') => {
                    app.show_preview = !app.show_preview;
                }
                KeyCode::Tab => {
                    app.toggle_selected_collapse(app.selected_index);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if !items.is_empty() && app.selected_index + 1 < items.len() {
                        app.selected_index += 1;
                    }
                    app.pending_g = false;
                    app.pending_d = false;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if app.selected_index > 0 {
                        app.selected_index -= 1;
                    }
                    app.pending_g = false;
                    app.pending_d = false;
                }
                KeyCode::Char('g') => {
                    if app.pending_g {
                        app.selected_index = 0;
                        app.pending_g = false;
                    } else {
                        app.pending_g = true;
                    }
                    app.pending_d = false;
                }
                KeyCode::Char('G') => {
                    if !items.is_empty() {
                        app.selected_index = items.len() - 1;
                    }
                    app.pending_g = false;
                    app.pending_d = false;
                }
                KeyCode::Enter => {
                    if let Some(item) = items.get(app.selected_index) {
                        let _ = switch_to_item(socket, item);
                        break;
                    }
                }
                KeyCode::Char('d') => {
                    if app.pending_d {
                        app.pending_d = false;
                        if let Some(item) = items.get(app.selected_index) {
                            let kill_cmd = format_kill_command(item);
                            let target_desc = match item {
                                TreeItem::Session(s) => format!("session '{}'", s.name),
                                TreeItem::Window { window, .. } => {
                                    format!("window '{}'", window.name)
                                }
                                TreeItem::Pane { pane, .. } => format!("pane '{}'", pane.id),
                            };
                            app.confirm_action =
                                Some((format!("Kill {target_desc}? (y/n)"), kill_cmd));
                        }
                    } else {
                        app.pending_d = true;
                    }
                    app.pending_g = false;
                }
                KeyCode::Char('r') => {
                    if let Some(item) = items.get(app.selected_index) {
                        match item {
                            TreeItem::Session(s) => {
                                app.rename_target = Some(("session".to_string(), s.id.clone()));
                                app.rename_buffer = s.name.clone();
                            }
                            TreeItem::Window { window, .. } => {
                                app.rename_target = Some(("window".to_string(), window.id.clone()));
                                app.rename_buffer = window.name.clone();
                            }
                            TreeItem::Pane { pane, .. } => {
                                app.rename_target = Some(("pane".to_string(), pane.id.clone()));
                                app.rename_buffer = pane.title.clone();
                            }
                        }
                    }
                    app.pending_g = false;
                    app.pending_d = false;
                }
                _ => {
                    app.pending_g = false;
                    app.pending_d = false;
                }
            }
        }
    }

    Ok(())
}
