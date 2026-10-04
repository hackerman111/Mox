//! Search pattern and entity matcher for Flash navigation.
//!
//! Provides smart-case substring searching, char-motion scanning, and
//! token-to-target projection with zero external dependencies.

use crate::extract::model::{EntityKind, ExtractedToken};
use crate::flash::labeler::MatchTarget;

/// Checks if a slice matches query case-insensitively, returning the byte length of the match.
fn match_ci(slice: &str, query: &str) -> Option<usize> {
    let mut slice_chars = slice.chars();
    let mut matched_bytes = 0;

    for qc in query.chars() {
        let sc = slice_chars.next()?;

        if !sc.to_lowercase().eq(qc.to_lowercase()) {
            return None;
        }

        matched_bytes += sc.len_utf8();
    }

    Some(matched_bytes)
}

/// Finds all occurrences of `query` across `lines` using smart-case semantics.
///
/// If `query` contains any uppercase characters, matching is case-sensitive;
/// otherwise, matching is case-insensitive.
pub fn find_matches(lines: &[String], query: &str, pane_id: &str) -> Vec<MatchTarget> {
    if query.is_empty() {
        return Vec::new();
    }

    let is_case_sensitive = query.chars().any(|c| c.is_uppercase());
    let mut targets = Vec::new();

    for (row, line) in lines.iter().enumerate() {
        if is_case_sensitive {
            let mut byte_offset = 0;
            while byte_offset < line.len() {
                let Some(found_idx) = line[byte_offset..].find(query) else {
                    break;
                };
                let match_start = byte_offset + found_idx;
                let match_end = match_start + query.len();
                let col = line[..match_start].chars().count();
                targets.push(MatchTarget {
                    row,
                    col,
                    pane_id: pane_id.to_string(),
                    matched_text: line[match_start..match_end].to_string(),
                });
                byte_offset = match_start + query.len().max(1);
            }
        } else {
            let mut byte_offset = 0;
            while byte_offset < line.len() {
                if let Some(matched_len) = match_ci(&line[byte_offset..], query) {
                    let match_end = byte_offset + matched_len;
                    let col = line[..byte_offset].chars().count();
                    targets.push(MatchTarget {
                        row,
                        col,
                        pane_id: pane_id.to_string(),
                        matched_text: line[byte_offset..match_end].to_string(),
                    });
                    byte_offset += matched_len.max(1);
                } else if let Some(ch) = line[byte_offset..].chars().next() {
                    byte_offset += ch.len_utf8();
                } else {
                    break;
                }
            }
        }
    }

    targets
}

/// Projects extracted entities into flash match targets, optionally filtering by kind.
pub fn find_token_matches(
    tokens: &[ExtractedToken],
    filter_kind: Option<EntityKind>,
) -> Vec<MatchTarget> {
    tokens
        .iter()
        .filter(|t| filter_kind.is_none() || filter_kind == Some(t.kind))
        .map(|t| MatchTarget {
            row: t.screen_row,
            col: t.col_start,
            pane_id: t.pane_id.clone(),
            matched_text: t.clean_text.clone(),
        })
        .collect()
}

/// Finds single-character motion occurrences (`f`/`F`) forward or backward from the cursor.
///
/// Searches on the line at `cursor_row`. Forward searches find matches where `col > cursor_col`,
/// while backward searches find matches where `col < cursor_col`.
pub fn find_char_motion_matches(
    lines: &[String],
    ch: char,
    forward: bool,
    cursor_row: usize,
    cursor_col: usize,
    pane_id: &str,
) -> Vec<MatchTarget> {
    let Some(line) = lines.get(cursor_row) else {
        return Vec::new();
    };

    let mut targets = Vec::new();
    for (col, c) in line.chars().enumerate() {
        if c == ch {
            let valid = if forward {
                col > cursor_col
            } else {
                col < cursor_col
            };

            if valid {
                targets.push(MatchTarget {
                    row: cursor_row,
                    col,
                    pane_id: pane_id.to_string(),
                    matched_text: ch.to_string(),
                });
            }
        }
    }

    targets
}
