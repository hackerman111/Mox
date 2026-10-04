//! Navigator application state and tree manipulation.

use super::fuzzy::filter_sessions;
use super::model::{SessionInfo, TreeItem, visible_tree};
use crate::ui::picker::PickerState;

#[derive(Debug)]
pub enum CollapseTarget {
    Session(String),
    Window {
        session_id: String,
        window_id: String,
    },
}

/// State for the interactive navigator TUI.
pub struct NavigatorApp {
    pub sessions: Vec<SessionInfo>,
    pub picker: PickerState,
    pub in_search_mode: bool,
    pub show_preview: bool,
    pub pending_g: bool,
    pub pending_d: bool,
    pub confirm_action: Option<(String, String)>,
    pub rename_target: Option<(String, String)>,
    pub rename_buffer: String,
    pub cached_preview_pane: String,
    pub cached_preview_lines: Vec<String>,
}

impl NavigatorApp {
    pub fn new(sessions: Vec<SessionInfo>) -> Self {
        Self {
            sessions,
            picker: PickerState::default(),
            in_search_mode: false,
            show_preview: true,
            pending_g: false,
            pending_d: false,
            confirm_action: None,
            rename_target: None,
            rename_buffer: String::new(),
            cached_preview_pane: String::new(),
            cached_preview_lines: Vec::new(),
        }
    }

    /// Compatibility helper for callers that need owned filtered models.
    pub fn filtered_sessions(&self) -> Vec<SessionInfo> {
        filter_sessions(&self.sessions, &self.picker.query)
    }

    pub fn visible_items(&self) -> Vec<TreeItem<'_>> {
        visible_tree(&self.sessions, &self.picker.query)
    }

    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        self.picker.selected = 0;
        self.picker.offset = 0;
        self.cached_preview_pane.clear();
        self.cached_preview_lines.clear();
    }

    pub fn toggle_selected_collapse(&mut self, selected_idx: usize) {
        let target = self
            .visible_items()
            .get(selected_idx)
            .and_then(collapse_target);
        if let Some(target) = target {
            self.toggle_collapse(target);
        }
    }

    pub fn toggle_collapse(&mut self, target: CollapseTarget) {
        match target {
            CollapseTarget::Session(session_id) => {
                if let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                {
                    session.collapsed = !session.collapsed;
                }
            }
            CollapseTarget::Window {
                session_id,
                window_id,
            } => {
                if let Some(window) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                    .and_then(|session| {
                        session
                            .windows
                            .iter_mut()
                            .find(|window| window.id == window_id)
                    })
                {
                    window.collapsed = !window.collapsed;
                }
            }
        }
    }
}

pub fn collapse_target(item: &TreeItem<'_>) -> Option<CollapseTarget> {
    match item {
        TreeItem::Session(session) => Some(CollapseTarget::Session(session.id.clone())),
        TreeItem::Window { session, window } => Some(CollapseTarget::Window {
            session_id: session.id.clone(),
            window_id: window.id.clone(),
        }),
        TreeItem::Pane { .. } => None,
    }
}
