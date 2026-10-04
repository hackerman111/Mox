//! Presentation models and row rendering for command palette entries.
//!
//! Provides the data structures and rendering needed to display actions
//! in a unified List/Preview layout. Source registration and dispatch are
//! deferred to Part 2.

use crate::ui::render::truncate_or_pad;
use crate::ui::theme::Theme;
use crossterm::{
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

/// A presentation entry for the command palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem {
    /// Human-readable command title (e.g. "Split Window Vertically").
    pub title: String,
    /// Associated keybinding hint, if any (e.g. "w s l").
    pub key_hint: Option<String>,
    /// Command category (e.g. "Window", "Session", "Tmux").
    pub category: Option<String>,
    /// Short description or preview details.
    pub description: Option<String>,
}

/// Renders a single palette entry row with category and keybinding hints.
pub fn render_palette_row<W: Write>(
    out: &mut W,
    item: &PaletteItem,
    is_selected: bool,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    let (bg, fg) = if is_selected {
        (theme.selection_bg, theme.selection_fg)
    } else {
        (theme.bg, theme.fg)
    };

    let cat_badge = item
        .category
        .as_ref()
        .map(|c| format!("[{c}] "))
        .unwrap_or_default();

    let hint_str = item
        .key_hint
        .as_ref()
        .map(|k| format!(" ({k})"))
        .unwrap_or_default();

    let full_text = format!("  {}{}{}", cat_badge, item.title, hint_str);
    let line = truncate_or_pad(&full_text, width);

    execute!(
        out,
        SetBackgroundColor(bg),
        SetForegroundColor(if is_selected { theme.warning } else { fg }),
        Print(&line),
        ResetColor,
    )
}
