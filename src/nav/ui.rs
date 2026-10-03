//! Interactive dual-pane TUI tree navigator and live preview for tmux.

use crate::nav::fuzzy::filter_sessions;
use crate::nav::model::{SessionInfo, TreeItem, fetch_tmux_snapshot, flatten_tree};
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

/// State for the interactive navigator TUI.
pub struct NavigatorApp {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    search_query: String,
    in_search_mode: bool,
    show_preview: bool,
    pending_g: bool,
    pending_d: bool,
    confirm_action: Option<(String, String)>, // (prompt_text, command)
    rename_target: Option<(String, String)>,  // (target_type, target_id)
    rename_buffer: String,
    cached_preview_pane: String,
    cached_preview_lines: Vec<String>,
    scroll_offset: usize,
}

impl NavigatorApp {
    pub fn new(sessions: Vec<SessionInfo>) -> Self {
        Self {
            sessions,
            selected_index: 0,
            search_query: String::new(),
            in_search_mode: false,
            show_preview: true,
            pending_g: false,
            pending_d: false,
            confirm_action: None,
            rename_target: None,
            rename_buffer: String::new(),
            cached_preview_pane: String::new(),
            cached_preview_lines: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn filtered_sessions(&self) -> Vec<SessionInfo> {
        filter_sessions(&self.sessions, &self.search_query)
    }

    pub fn toggle_selected_collapse(&mut self, selected_idx: usize) {
        let filtered = self.filtered_sessions();
        let items = flatten_tree(&filtered);
        if selected_idx >= items.len() {
            return;
        }

        match &items[selected_idx] {
            TreeItem::Session(s) => {
                let sid = s.id.clone();
                if let Some(session) = self.sessions.iter_mut().find(|sess| sess.id == sid) {
                    session.collapsed = !session.collapsed;
                }
            }
            TreeItem::Window { session, window } => {
                let sid = session.id.clone();
                let wid = window.id.clone();
                if let Some(win) = self
                    .sessions
                    .iter_mut()
                    .find(|sess| sess.id == sid)
                    .and_then(|sess| sess.windows.iter_mut().find(|w| w.id == wid))
                {
                    win.collapsed = !win.collapsed;
                }
            }

            TreeItem::Pane { .. } => {}
        }
    }
}

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
                            let cmd = if target_type == "session" {
                                format!("rename-session -t {target_id} '{new_name}'")
                            } else {
                                format!("rename-window -t {target_id} '{new_name}'")
                            };
                            let _ = execute_nav_tmux(socket, &cmd);
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
                            TreeItem::Pane { .. } => {}
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

pub fn execute_nav_tmux(socket: Option<&str>, cmd_str: &str) -> Result<(), String> {
    crate::tmux::apply_tmux_commands(socket, &[cmd_str.to_string()])
}

fn render_navigator(
    app: &NavigatorApp,
    items: &[TreeItem],
    width: usize,
    height: usize,
    out: &mut std::io::Stdout,
) -> Result<(), String> {
    let _ = execute!(
        out,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All),
        SetBackgroundColor(Color::Rgb {
            r: 46,
            g: 52,
            b: 64
        }),
        SetForegroundColor(Color::Rgb {
            r: 236,
            g: 239,
            b: 244
        })
    );

    let tree_width = if app.show_preview {
        (width * 55 / 100).max(30).min(width.saturating_sub(20))
    } else {
        width
    };
    let preview_width = width.saturating_sub(tree_width).saturating_sub(1);
    let content_height = height.saturating_sub(4);

    // 1. Search Bar
    let _ = execute!(out, cursor::MoveTo(0, 0));
    let mode_badge = if app.in_search_mode {
        "[SEARCH]"
    } else {
        "[NORMAL]"
    };
    let query_display = format!(" / {}", app.search_query);
    let bar_line = truncate_or_pad(&format!("{mode_badge}{query_display}"), tree_width);

    let _ = execute!(
        out,
        SetForegroundColor(Color::Rgb {
            r: 136,
            g: 192,
            b: 208
        }),
        Print(&bar_line),
        ResetColor,
        SetBackgroundColor(Color::Rgb {
            r: 46,
            g: 52,
            b: 64
        })
    );

    if app.show_preview && preview_width > 0 {
        let _ = execute!(
            out,
            cursor::MoveTo(tree_width as u16, 0),
            SetForegroundColor(Color::Rgb {
                r: 129,
                g: 161,
                b: 193
            }),
            Print("│ Preview"),
            ResetColor,
            SetBackgroundColor(Color::Rgb {
                r: 46,
                g: 52,
                b: 64
            })
        );
    }

    // Divider line
    let _ = execute!(out, cursor::MoveTo(0, 1));
    let div_line = "─".repeat(tree_width);
    let _ = execute!(
        out,
        SetForegroundColor(Color::Rgb {
            r: 76,
            g: 86,
            b: 106
        }),
        Print(&div_line)
    );
    if app.show_preview && preview_width > 0 {
        let _ = execute!(
            out,
            cursor::MoveTo(tree_width as u16, 1),
            Print("┼"),
            Print("─".repeat(preview_width))
        );
    }

    // Scroll calculation for tree
    let mut scroll_offset = app.scroll_offset;
    if app.selected_index < scroll_offset {
        scroll_offset = app.selected_index;
    } else if app.selected_index >= scroll_offset + content_height {
        scroll_offset = app.selected_index.saturating_sub(content_height) + 1;
    }

    // 2. Tree Items
    for row in 0..content_height {
        let y = (row + 2) as u16;
        let item_idx = scroll_offset + row;

        let _ = execute!(out, cursor::MoveTo(0, y));

        if item_idx < items.len() {
            let item = &items[item_idx];
            let is_selected = item_idx == app.selected_index;

            let line_text = format_tree_line(item, tree_width);

            if is_selected {
                let _ = execute!(
                    out,
                    SetBackgroundColor(Color::Rgb {
                        r: 67,
                        g: 76,
                        b: 94
                    }),
                    SetForegroundColor(Color::Rgb {
                        r: 236,
                        g: 239,
                        b: 244
                    }),
                    Print(&line_text),
                    ResetColor,
                    SetBackgroundColor(Color::Rgb {
                        r: 46,
                        g: 52,
                        b: 64
                    })
                );
            } else {
                let _ = execute!(
                    out,
                    SetForegroundColor(item_color(item)),
                    Print(&line_text),
                    ResetColor,
                    SetBackgroundColor(Color::Rgb {
                        r: 46,
                        g: 52,
                        b: 64
                    })
                );
            }
        } else {
            let _ = execute!(out, Print(" ".repeat(tree_width)));
        }

        // Preview Divider and Content
        if app.show_preview && preview_width > 0 {
            let _ = execute!(
                out,
                cursor::MoveTo(tree_width as u16, y),
                SetForegroundColor(Color::Rgb {
                    r: 76,
                    g: 86,
                    b: 106
                }),
                Print("│"),
                ResetColor,
                SetBackgroundColor(Color::Rgb {
                    r: 46,
                    g: 52,
                    b: 64
                })
            );

            let preview_line = if row < app.cached_preview_lines.len() {
                truncate_or_pad(&app.cached_preview_lines[row], preview_width)
            } else {
                " ".repeat(preview_width)
            };

            let _ = execute!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: 216,
                    g: 222,
                    b: 233
                }),
                Print(&preview_line),
                ResetColor,
                SetBackgroundColor(Color::Rgb {
                    r: 46,
                    g: 52,
                    b: 64
                })
            );
        }
    }

    // 3. Footer / Help Bar
    let footer_y = (height.saturating_sub(2)) as u16;
    let _ = execute!(
        out,
        cursor::MoveTo(0, footer_y),
        SetForegroundColor(Color::Rgb {
            r: 76,
            g: 86,
            b: 106
        }),
        Print("─".repeat(width))
    );

    let status_y = (height.saturating_sub(1)) as u16;
    let _ = execute!(out, cursor::MoveTo(0, status_y));

    if let Some((prompt, _)) = &app.confirm_action {
        let prompt_str = format!(" [CONFIRM] {prompt} ");
        let _ = execute!(
            out,
            SetBackgroundColor(Color::Rgb {
                r: 191,
                g: 97,
                b: 106
            }),
            SetForegroundColor(Color::Black),
            Print(&prompt_str),
            ResetColor
        );
    } else if let Some((target_type, _)) = &app.rename_target {
        let prompt_str = format!(" Rename {target_type}: {}_ ", app.rename_buffer);
        let _ = execute!(
            out,
            SetBackgroundColor(Color::Rgb {
                r: 235,
                g: 203,
                b: 139
            }),
            SetForegroundColor(Color::Black),
            Print(&prompt_str),
            ResetColor
        );
    } else {
        let help_text =
            " Enter:Switch │ dd:Kill │ r:Rename │ Tab:Fold │ ?:Preview │ /:Search │ Esc:Exit";
        let _ = execute!(
            out,
            SetForegroundColor(Color::Rgb {
                r: 143,
                g: 188,
                b: 187
            }),
            Print(help_text),
            ResetColor
        );
    }

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn format_tree_line(item: &TreeItem, max_w: usize) -> String {
    let raw = match item {
        TreeItem::Session(s) => {
            let icon = if s.collapsed { "▶" } else { "▼" };
            let active = if s.active { " *" } else { "" };
            format!("{icon} {}{active}", s.name)
        }
        TreeItem::Window { window, .. } => {
            let icon = if window.collapsed { "▶" } else { "▼" };
            let agent = if window.is_agent { " [AI]" } else { "" };
            let active = if window.active { " *" } else { "" };
            format!(
                "  ├─ {icon} {}: {}{agent}{active}",
                window.index, window.name
            )
        }
        TreeItem::Pane { pane, .. } => {
            let active = if pane.active { " *" } else { "" };
            let title_info = if !pane.title.is_empty() && pane.title != pane.command {
                format!(" [{}]", pane.title)
            } else {
                String::new()
            };
            format!(
                "  │  └─ {} {}  {}{title_info}{active}",
                pane.id, pane.command, pane.cwd
            )
        }
    };

    truncate_or_pad(&raw, max_w)
}

fn truncate_or_pad(s: &str, target_width: usize) -> String {
    let char_count = s.chars().count();
    if char_count < target_width {
        format!("{s}{}", " ".repeat(target_width - char_count))
    } else {
        s.chars().take(target_width).collect()
    }
}

fn item_color(item: &TreeItem) -> Color {
    match item {
        TreeItem::Session(_) => Color::Rgb {
            r: 136,
            g: 192,
            b: 208,
        },
        TreeItem::Window { window, .. } => {
            if window.is_agent {
                Color::Rgb {
                    r: 180,
                    g: 142,
                    b: 173,
                }
            } else {
                Color::Rgb {
                    r: 235,
                    g: 203,
                    b: 139,
                }
            }
        }
        TreeItem::Pane { .. } => Color::Rgb {
            r: 216,
            g: 222,
            b: 233,
        },
    }
}
