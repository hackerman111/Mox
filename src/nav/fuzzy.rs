//! Fuzzy search engine for tmux sessions, windows, and panes.

use crate::nav::model::{PaneInfo, SessionInfo, WindowInfo};

/// Computes a fuzzy subsequence match score between target and query.
/// Returns `Some(score)` if all query characters appear in sequence in target, else `None`.
pub fn fuzzy_match(target: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let target_chars: Vec<char> = target.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    let mut t_idx = 0;
    let mut q_idx = 0;
    let mut score: i64 = 0;
    let mut last_match_idx: Option<usize> = None;

    while q_idx < query_chars.len() && t_idx < target_chars.len() {
        let qc = query_chars[q_idx].to_ascii_lowercase();
        let tc = target_chars[t_idx].to_ascii_lowercase();

        if qc == tc {
            score += 10;

            // Prefix bonus
            if t_idx == 0 {
                score += 20;
            }

            // Word boundary bonus
            if t_idx > 0 {
                let prev = target_chars[t_idx - 1];
                if prev == ' ' || prev == '/' || prev == '-' || prev == '_' || prev == '.' {
                    score += 15;
                }
            }

            // Consecutive match bonus
            if let Some(last) = last_match_idx {
                if last + 1 == t_idx {
                    score += 10;
                }
            }

            last_match_idx = Some(t_idx);
            q_idx += 1;
        }

        t_idx += 1;
    }

    if q_idx == query_chars.len() {
        let extra_chars = target_chars.len().saturating_sub(query_chars.len()) as i64;
        Some(score - extra_chars)
    } else {
        None
    }
}

/// Checks if a pane matches the search query across command, cwd, title, or id.
pub fn pane_matches(pane: &PaneInfo, query: &str) -> bool {
    fuzzy_match(&pane.command, query).is_some()
        || fuzzy_match(&pane.cwd, query).is_some()
        || fuzzy_match(&pane.title, query).is_some()
        || fuzzy_match(&pane.id, query).is_some()
}

/// Filters a list of sessions by query, preserving hierarchical structure and expanding matching branches.
pub fn filter_sessions(sessions: &[SessionInfo], query: &str) -> Vec<SessionInfo> {
    let query = query.trim();
    if query.is_empty() {
        return sessions.to_vec();
    }

    let mut filtered_sessions = Vec::new();

    for session in sessions {
        let session_match = fuzzy_match(&session.name, query).is_some();
        let mut matched_windows: Vec<WindowInfo> = Vec::new();

        for window in &session.windows {
            let window_match = fuzzy_match(&window.name, query).is_some();
            let matched_panes: Vec<PaneInfo> = window
                .panes
                .iter()
                .filter(|p| pane_matches(p, query))
                .cloned()
                .collect();

            if !matched_panes.is_empty() {
                // Panes matched: keep window with only matched panes, uncollapsed
                let mut w = window.clone();
                w.panes = matched_panes;
                w.collapsed = false;
                matched_windows.push(w);
            } else if window_match || session_match {
                // Window or Session matched directly: keep window with all panes
                let mut w = window.clone();
                w.collapsed = false;
                matched_windows.push(w);
            }
        }

        if !matched_windows.is_empty() {
            let mut s = session.clone();
            s.windows = matched_windows;
            s.collapsed = false;
            filtered_sessions.push(s);
        } else if session_match {
            let mut s = session.clone();
            s.collapsed = false;
            filtered_sessions.push(s);
        }
    }

    filtered_sessions
}
