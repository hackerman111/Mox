//! Navigator application state machine, search, and tree manipulation.

use super::fuzzy::filter_sessions;
use super::model::{SessionInfo, TreeItem, flatten_tree};

/// State for the interactive navigator TUI.
pub struct NavigatorApp {
    pub sessions: Vec<SessionInfo>,
    pub selected_index: usize,
    pub search_query: String,
    pub in_search_mode: bool,
    pub show_preview: bool,
    pub pending_g: bool,
    pub pending_d: bool,
    pub confirm_action: Option<(String, String)>, // (prompt_text, command)
    pub rename_target: Option<(String, String)>,  // (target_type, target_id)
    pub rename_buffer: String,
    pub cached_preview_pane: String,
    pub cached_preview_lines: Vec<String>,
    pub scroll_offset: usize,
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
