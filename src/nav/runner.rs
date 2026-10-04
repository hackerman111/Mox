//! Interactive event loop and key event handling for the tree navigator.

use super::action::{execute_nav_tmux, format_kill_command, format_rename_command, switch_to_item};
use super::model::{TreeItem, fetch_tmux_snapshot, visible_tree};
use super::state::{NavigatorApp, collapse_target};
use super::view::render_navigator;
use crate::tui::RawModeGuard;
use crate::ui::picker::Input;
use crate::ui::preview::{Preview, PreviewWorker};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use std::io::stdout;
use std::time::Duration;

/// Runs the interactive Navigator TUI inside a tmux popup.
pub fn run_navigator(socket: Option<&str>) -> Result<(), String> {
    let _guard = RawModeGuard::new()?;
    let mut app = NavigatorApp::new(fetch_tmux_snapshot(socket)?);
    let preview_worker = PreviewWorker::new(socket);
    let mut preview_generation = 0u64;
    let mut requested_pane = String::new();
    let mut preview_sent = false;
    let mut out = stdout();

    loop {
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let (term_w, term_h) = (term_w as usize, term_h as usize);
        let items = visible_tree(&app.sessions, &app.picker.query);
        app.picker.viewport(items.len(), term_h.saturating_sub(4));

        let active_pane = items
            .get(app.picker.selected)
            .map(|item| item.active_pane_id())
            .filter(|pane| !pane.is_empty())
            .unwrap_or_default();
        if active_pane != requested_pane {
            requested_pane = active_pane.to_owned();
            preview_generation = preview_generation.wrapping_add(1);
            preview_sent = false;
            app.cached_preview_lines.clear();
        }
        if requested_pane.is_empty() {
            app.cached_preview_lines.clear();
            preview_sent = true;
        } else if app.show_preview && !preview_sent {
            preview_sent =
                preview_worker.request(preview_generation, &Preview::Pane(requested_pane.clone()));
        }
        if let Some(lines) = preview_worker.result(preview_generation) {
            app.cached_preview_lines = lines;
        }

        render_navigator(&app, &items, term_w, term_h, &mut out)?;
        if !event::poll(Duration::from_millis(50)).map_err(|error| error.to_string())? {
            continue;
        }
        let Event::Key(key_event) = event::read().map_err(|error| error.to_string())? else {
            continue;
        };
        if key_event.kind != KeyEventKind::Press {
            continue;
        }

        if key_event.modifiers.contains(KeyModifiers::ALT) && key_event.code == KeyCode::Char('m') {
            break;
        }

        if let Some((_, command)) = app.confirm_action.take() {
            if matches!(key_event.code, KeyCode::Char('y' | 'Y')) {
                let _ = execute_nav_tmux(socket, &command);
                drop(items);
                refresh(
                    &mut app,
                    socket,
                    &mut requested_pane,
                    &mut preview_generation,
                    &mut preview_sent,
                );
            }
            continue;
        }

        if let Some((target_type, target_id)) = app.rename_target.take() {
            match key_event.code {
                KeyCode::Enter => {
                    let new_name = app.rename_buffer.trim().to_owned();
                    let command = if new_name.is_empty() {
                        None
                    } else {
                        items.iter().find_map(|item| {
                            let target = match item {
                                TreeItem::Session(session) => {
                                    target_type == "session" && session.id == target_id
                                }
                                TreeItem::Window { window, .. } => {
                                    target_type == "window" && window.id == target_id
                                }
                                TreeItem::Pane { pane, .. } => {
                                    target_type == "pane" && pane.id == target_id
                                }
                            };
                            target
                                .then(|| format_rename_command(item, &new_name))
                                .flatten()
                        })
                    };
                    if let Some(command) = command {
                        execute_nav_tmux(socket, &command)?;
                        drop(items);
                        refresh(
                            &mut app,
                            socket,
                            &mut requested_pane,
                            &mut preview_generation,
                            &mut preview_sent,
                        );
                    }
                    app.rename_buffer.clear();
                }
                KeyCode::Esc => app.rename_buffer.clear(),
                KeyCode::Backspace => {
                    app.rename_buffer.pop();
                    app.rename_target = Some((target_type, target_id));
                }
                KeyCode::Char(character) => {
                    app.rename_buffer.push(character);
                    app.rename_target = Some((target_type, target_id));
                }
                _ => app.rename_target = Some((target_type, target_id)),
            }
            continue;
        }

        if app.in_search_mode {
            match key_event.code {
                KeyCode::Esc => {
                    if app.picker.query.is_empty() {
                        app.in_search_mode = false;
                    } else {
                        app.picker.query.clear();
                    }
                    app.picker.selected = 0;
                    app.picker.offset = 0;
                }
                KeyCode::Tab => {
                    let target = items.get(app.picker.selected).and_then(collapse_target);
                    drop(items);
                    if let Some(target) = target {
                        app.toggle_collapse(target);
                    }
                }
                KeyCode::Enter => {
                    if let Some(item) = items.get(app.picker.selected) {
                        let _ = switch_to_item(socket, item);
                        break;
                    }
                }
                _ => match app.picker.input(key_event, items.len(), true) {
                    Input::Changed => {
                        app.picker.selected = 0;
                        app.picker.offset = 0;
                    }
                    Input::Accept => {
                        if let Some(item) = items.get(app.picker.selected) {
                            let _ = switch_to_item(socket, item);
                            break;
                        }
                    }
                    Input::Cancel | Input::None => {}
                },
            }
            continue;
        }

        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => break,
            KeyCode::Char('/') => app.in_search_mode = true,
            KeyCode::Char('?') => app.show_preview = !app.show_preview,
            KeyCode::Tab => {
                let target = items.get(app.picker.selected).and_then(collapse_target);
                drop(items);
                if let Some(target) = target {
                    app.toggle_collapse(target);
                }
            }
            KeyCode::Char('g') => {
                if app.pending_g {
                    app.picker.selected = 0;
                    app.pending_g = false;
                } else {
                    app.pending_g = true;
                }
                app.pending_d = false;
            }
            KeyCode::Char('G') => {
                if !items.is_empty() {
                    app.picker.selected = items.len() - 1;
                }
                app.pending_g = false;
                app.pending_d = false;
            }
            KeyCode::Char('d') => {
                if app.pending_d {
                    app.pending_d = false;
                    if let Some(item) = items.get(app.picker.selected) {
                        let command = format_kill_command(item);
                        let description = match item {
                            TreeItem::Session(session) => format!("session '{}'", session.name),
                            TreeItem::Window { window, .. } => {
                                format!("window '{}'", window.name)
                            }
                            TreeItem::Pane { pane, .. } => format!("pane '{}'", pane.id),
                        };
                        app.confirm_action = Some((format!("Kill {description}? (y/n)"), command));
                    }
                } else {
                    app.pending_d = true;
                }
                app.pending_g = false;
            }
            KeyCode::Char('r') => {
                if let Some(item) = items.get(app.picker.selected) {
                    match item {
                        TreeItem::Session(session) => {
                            app.rename_target = Some(("session".into(), session.id.clone()));
                            app.rename_buffer.clone_from(&session.name);
                        }
                        TreeItem::Window { window, .. } => {
                            app.rename_target = Some(("window".into(), window.id.clone()));
                            app.rename_buffer.clone_from(&window.name);
                        }
                        TreeItem::Pane { pane, .. } => {
                            app.rename_target = Some(("pane".into(), pane.id.clone()));
                            app.rename_buffer.clone_from(&pane.title);
                        }
                    }
                }
                app.pending_g = false;
                app.pending_d = false;
            }
            KeyCode::Enter => {
                if let Some(item) = items.get(app.picker.selected) {
                    let _ = switch_to_item(socket, item);
                    break;
                }
            }
            _ => {
                match app.picker.input(key_event, items.len(), false) {
                    Input::Accept => {
                        if let Some(item) = items.get(app.picker.selected) {
                            let _ = switch_to_item(socket, item);
                            break;
                        }
                    }
                    Input::Changed | Input::Cancel | Input::None => {}
                }
                app.pending_g = false;
                app.pending_d = false;
            }
        }
    }

    Ok(())
}

fn refresh(
    app: &mut NavigatorApp,
    socket: Option<&str>,
    requested_pane: &mut String,
    generation: &mut u64,
    sent: &mut bool,
) {
    if let Ok(sessions) = fetch_tmux_snapshot(socket) {
        app.set_sessions(sessions);
        requested_pane.clear();
        *generation = generation.wrapping_add(1);
        *sent = false;
    }
}
