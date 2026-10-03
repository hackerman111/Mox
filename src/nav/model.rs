//! Data structures and parser for tmux session, window, and pane hierarchy.

use crate::tmux::execute_tmux;

/// Information about a single tmux pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneInfo {
    pub id: String,
    pub index: u32,
    pub active: bool,
    pub command: String,
    pub cwd: String,
    pub title: String,
}

/// Information about a tmux window containing one or more panes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: String,
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub is_agent: bool,
    pub panes: Vec<PaneInfo>,
    pub collapsed: bool,
}

/// Information about a tmux session containing one or more windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub windows: Vec<WindowInfo>,
    pub collapsed: bool,
}

/// A node in the flattened visible tree list for UI navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeItem<'a> {
    Session(&'a SessionInfo),
    Window {
        session: &'a SessionInfo,
        window: &'a WindowInfo,
    },
    Pane {
        session: &'a SessionInfo,
        window: &'a WindowInfo,
        pane: &'a PaneInfo,
    },
}

impl<'a> TreeItem<'a> {
    pub fn session_id(&self) -> &str {
        match self {
            TreeItem::Session(s) => &s.id,
            TreeItem::Window { session, .. } => &session.id,
            TreeItem::Pane { session, .. } => &session.id,
        }
    }

    pub fn window_id(&self) -> Option<&str> {
        match self {
            TreeItem::Session(_) => None,
            TreeItem::Window { window, .. } => Some(&window.id),
            TreeItem::Pane { window, .. } => Some(&window.id),
        }
    }

    pub fn pane_id(&self) -> Option<&str> {
        match self {
            TreeItem::Session(_) | TreeItem::Window { .. } => None,
            TreeItem::Pane { pane, .. } => Some(&pane.id),
        }
    }

    /// Returns the target pane id for preview generation.
    pub fn active_pane_id(&self) -> &str {
        match self {
            TreeItem::Pane { pane, .. } => &pane.id,
            TreeItem::Window { window, .. } => window
                .panes
                .iter()
                .find(|p| p.active)
                .or_else(|| window.panes.first())
                .map(|p| p.id.as_str())
                .unwrap_or(""),
            TreeItem::Session(session) => {
                let active_win = session
                    .windows
                    .iter()
                    .find(|w| w.active)
                    .or_else(|| session.windows.first());
                if let Some(win) = active_win {
                    win.panes
                        .iter()
                        .find(|p| p.active)
                        .or_else(|| win.panes.first())
                        .map(|p| p.id.as_str())
                        .unwrap_or("")
                } else {
                    ""
                }
            }
        }
    }
}

/// Parses the TSV output of `tmux list-panes -a -F '...'` into a hierarchical tree.
pub fn parse_tmux_snapshot(raw: &str) -> Vec<SessionInfo> {
    let mut sessions: Vec<SessionInfo> = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 13 {
            continue;
        }

        let sess_name = parts[0].to_string();
        let sess_id = parts[1].to_string();
        let sess_active = parts[2].trim() == "1";

        let win_index = parts[3].trim().parse::<u32>().unwrap_or(0);
        let win_id = parts[4].to_string();
        let win_name = parts[5].to_string();
        let win_active = parts[6].trim() == "1";

        let pane_id = parts[7].to_string();
        let pane_index = parts[8].trim().parse::<u32>().unwrap_or(0);
        let pane_active = parts[9].trim() == "1";
        let pane_command = parts[10].to_string();
        let pane_cwd = parts[11].to_string();
        let pane_title = parts[12].to_string();

        let is_agent = parts.get(13).map(|s| s.trim() == "1").unwrap_or(false);

        // Find or create session
        let session = if let Some(pos) = sessions.iter().position(|s| s.id == sess_id) {
            &mut sessions[pos]
        } else {
            sessions.push(SessionInfo {
                id: sess_id,
                name: sess_name,
                active: sess_active,
                windows: Vec::new(),
                collapsed: false,
            });
            sessions.last_mut().unwrap()
        };

        // Find or create window
        let window = if let Some(pos) = session.windows.iter().position(|w| w.id == win_id) {
            &mut session.windows[pos]
        } else {
            session.windows.push(WindowInfo {
                id: win_id,
                index: win_index,
                name: win_name,
                active: win_active,
                is_agent,
                panes: Vec::new(),
                collapsed: false,
            });
            session.windows.last_mut().unwrap()
        };

        // Add pane if not already present
        if !window.panes.iter().any(|p| p.id == pane_id) {
            window.panes.push(PaneInfo {
                id: pane_id,
                index: pane_index,
                active: pane_active,
                command: pane_command,
                cwd: pane_cwd,
                title: pane_title,
            });
        }
    }

    sessions
}

/// Flattens hierarchical sessions into visible `TreeItem` list based on `collapsed` flags.
pub fn flatten_tree<'a>(sessions: &'a [SessionInfo]) -> Vec<TreeItem<'a>> {
    let mut items = Vec::new();

    for session in sessions {
        items.push(TreeItem::Session(session));
        if !session.collapsed {
            for window in &session.windows {
                items.push(TreeItem::Window { session, window });
                if !window.collapsed {
                    for pane in &window.panes {
                        items.push(TreeItem::Pane {
                            session,
                            window,
                            pane,
                        });
                    }
                }
            }
        }
    }

    items
}

/// Fetches the current tmux snapshot in a single CLI command.
pub fn fetch_tmux_snapshot(socket: Option<&str>) -> Result<Vec<SessionInfo>, String> {
    let format = "#{session_name}\t#{session_id}\t#{session_attached}\t#{window_index}\t#{window_id}\t#{window_name}\t#{window_active}\t#{pane_id}\t#{pane_index}\t#{pane_active}\t#{pane_current_command}\t#{pane_current_path}\t#{pane_title}\t#{@mox_is_agent}";
    let raw = execute_tmux(socket, &["list-panes", "-a", "-F", format])?;
    Ok(parse_tmux_snapshot(&raw))
}
