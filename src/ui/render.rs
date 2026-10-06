//! Common terminal user interface rendering primitives and widgets.
//!
//! Provides pure presentation functions for lists, trees, previews, search prompts,
//! confirmations, toasts, and hints. Does NOT invoke tmux or external processes.

use crate::ui::theme::Theme;
use crossterm::{
    cursor, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io::Write;

/// Helper to truncate or pad a string to exact terminal cell width.
pub fn truncate_or_pad(s: &str, target_width: usize) -> String {
    let safe = truncate_str(s, target_width);
    let char_count = display_width(&safe);
    if char_count < target_width {
        format!("{safe}{}", " ".repeat(target_width - char_count))
    } else {
        safe
    }
}

/// Helper to safely truncate a string up to max_width terminal cells.
pub fn truncate_str(s: &str, max_w: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut output = String::new();
    let mut width = 0;
    for ch in safe_chars(s).take(max_w.saturating_mul(16)) {
        let cells = ch.width().unwrap_or(0);
        if width + cells > max_w {
            break;
        }
        output.push(ch);
        width += cells;
    }
    output
}

pub fn display_width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

fn safe_chars(s: &str) -> impl Iterator<Item = char> + '_ {
    s.chars()
        .map(|ch| if ch == '\t' { ' ' } else { ch })
        .filter(|ch| !ch.is_control())
}

/// Viewport helper for calculating scroll window and clamped selection.
pub struct Viewport;

impl Viewport {
    /// Adjusts `scroll_offset` so that `selected_index` remains visible within `visible_count` rows.
    pub fn adjust_scroll(
        selected_index: usize,
        scroll_offset: usize,
        visible_count: usize,
    ) -> usize {
        let visible_count = visible_count.max(1);
        if selected_index < scroll_offset {
            selected_index
        } else if selected_index >= scroll_offset + visible_count {
            selected_index.saturating_sub(visible_count) + 1
        } else {
            scroll_offset
        }
    }
}

/// A row in a generic flat list (e.g. Extract tokens, search results, recent files).
pub struct ListRow<'a> {
    pub prefix: &'a str,
    pub badge: Option<(&'a str, Color)>,
    pub text: &'a str,
    pub is_selected: bool,
}

/// Renders a single flat list row.
pub fn render_list_row<W: Write>(
    out: &mut W,
    row: &ListRow,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    let (bg, fg) = if row.is_selected {
        (theme.selection_bg, theme.selection_fg)
    } else {
        (theme.bg, theme.fg)
    };

    let sel_str = if row.is_selected { "> " } else { "  " };
    let sel_color = if row.is_selected {
        theme.warning
    } else {
        theme.muted
    };

    queue!(
        out,
        SetBackgroundColor(bg),
        SetForegroundColor(sel_color),
        Print(sel_str),
    )?;

    let mut used_cols = 2;

    if let Some((badge_text, badge_color)) = row.badge
        && used_cols < width
    {
        let max_badge = width.saturating_sub(used_cols);
        let b_trunc = truncate_str(badge_text, max_badge);
        let b_len = display_width(&b_trunc);
        used_cols += b_len;
        queue!(out, SetForegroundColor(badge_color), Print(&b_trunc),)?;
        if used_cols < width {
            queue!(out, Print(" "))?;
            used_cols += 1;
        }
    }

    if used_cols < width {
        let avail = width.saturating_sub(used_cols);
        let text_trunc = truncate_str(row.text, avail);
        let text_len = display_width(&text_trunc);
        used_cols += text_len;

        queue!(out, SetForegroundColor(fg), Print(&text_trunc),)?;
    }

    if used_cols < width {
        queue!(out, Print(" ".repeat(width - used_cols)))?;
    }

    queue!(out, ResetColor)
}

/// A node line in a hierarchical tree view (e.g. Navigator sessions, windows, panes).
pub struct TreeRow<'a> {
    pub depth: usize,
    pub is_collapsed: Option<bool>, // None = leaf, Some(true) = ▶, Some(false) = ▼
    pub icon: Option<(&'a str, Color)>,
    pub label: &'a str,
    pub badge: Option<(&'a str, Color)>,
    pub is_active: bool,
    pub is_selected: bool,
}

/// Renders a single tree item row with hierarchical branch glyphs.
pub fn render_tree_row<W: Write>(
    out: &mut W,
    row: &TreeRow,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    let (bg, fg) = if row.is_selected {
        (theme.selection_bg, theme.selection_fg)
    } else {
        match row.depth {
            0 => (theme.bg, theme.accent),
            1 => (theme.bg, theme.fg),
            _ => (theme.bg, theme.border),
        }
    };

    let indent_str = match row.depth {
        0 => String::new(),
        1 => "  ├─ ".to_string(),
        _ => format!("  │{}└─ ", "  │".repeat(row.depth.saturating_sub(2))),
    };

    let fold_icon = match row.is_collapsed {
        Some(true) => "▶ ",
        Some(false) => "▼ ",
        None => "",
    };

    let icon_str = match row.icon {
        Some((icon, _)) => icon,
        None => "",
    };

    let active_marker = if row.is_active { " ●" } else { "" };

    let mut line_buf = format!(
        "{}{}{}{}{}",
        indent_str, fold_icon, icon_str, row.label, active_marker
    );

    if let Some((b_text, _)) = row.badge {
        line_buf.push_str(&format!(" {b_text}"));
    }

    let line = truncate_or_pad(&line_buf, width);

    queue!(
        out,
        SetBackgroundColor(bg),
        SetForegroundColor(if row.is_selected {
            theme.selection_fg
        } else {
            fg
        }),
        Print(&line),
        ResetColor,
    )
}

/// Draws a horizontal divider line across `width` columns.
pub fn draw_divider<W: Write>(out: &mut W, width: usize, theme: &Theme) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    queue!(
        out,
        SetForegroundColor(theme.border),
        Print("─".repeat(width)),
        ResetColor,
    )
}

/// Renders a search prompt / header bar.
pub fn render_prompt<W: Write>(
    out: &mut W,
    mode_badge: &str,
    query: &str,
    info: Option<&str>,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    let info_str = info.map(|i| format!(" [{i}]")).unwrap_or_default();
    let text = format!("{mode_badge} / {query}{info_str}");
    let line = truncate_or_pad(&text, width);

    queue!(
        out,
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.accent),
        Print(&line),
        ResetColor,
    )
}

/// Renders a confirmation prompt.
pub fn render_confirm<W: Write>(out: &mut W, prompt: &str, theme: &Theme) -> std::io::Result<()> {
    let msg = format!(" [CONFIRM] {prompt} ");
    let msg = truncate_str(&msg, msg.chars().count());
    queue!(
        out,
        SetBackgroundColor(theme.error),
        SetForegroundColor(theme.bg),
        Print(&msg),
        ResetColor,
    )
}

/// Renders a toast notification message (success or error).
pub fn render_toast<W: Write>(
    out: &mut W,
    message: &str,
    is_error: bool,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    let prefix = if is_error { "  ✖ " } else { "  ✔ " };
    let full = format!("{prefix}{message}");
    let line = truncate_or_pad(&full, width);
    let color = if is_error { theme.error } else { theme.success };

    queue!(
        out,
        SetBackgroundColor(theme.bg),
        SetForegroundColor(color),
        Print(&line),
        ResetColor,
    )
}

/// Renders a bottom keymap hints bar.
pub fn render_hints<W: Write>(
    out: &mut W,
    hints: &str,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    let line = truncate_or_pad(&format!(" {hints}"), width);
    queue!(
        out,
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.info),
        Print(&line),
        ResetColor,
    )
}

/// Renders an empty list placeholder when there are 0 results.
pub fn render_empty_state<W: Write>(
    out: &mut W,
    message: &str,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    let line = truncate_or_pad(&format!("  ({message})"), width);
    queue!(
        out,
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.muted),
        Print(&line),
        ResetColor,
    )
}

/// Configuration parameters for rendering a side preview panel.
pub struct PreviewPanel<'a> {
    pub title: &'a str,
    pub lines: &'a [String],
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// Renders a preview panel side column with title border and content lines.
pub fn render_preview_panel<W: Write>(
    out: &mut W,
    panel: &PreviewPanel,
    theme: &Theme,
) -> std::io::Result<()> {
    if panel.width == 0 || panel.height == 0 {
        return Ok(());
    }

    // Header row
    queue!(out, cursor::MoveTo(panel.x as u16, panel.y as u16))?;
    let header_title = format!("│ {}", panel.title);
    let header_line = truncate_or_pad(&header_title, panel.width);
    queue!(
        out,
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.info),
        Print(&header_line),
        ResetColor,
    )?;

    // Divider row
    if panel.height > 1 {
        queue!(out, cursor::MoveTo(panel.x as u16, (panel.y + 1) as u16))?;
        let div = format!("┼{}", "─".repeat(panel.width.saturating_sub(1)));
        queue!(
            out,
            SetBackgroundColor(theme.bg),
            SetForegroundColor(theme.border),
            Print(&div),
            ResetColor,
        )?;
    }

    // Content rows
    let content_rows = panel.height.saturating_sub(2);
    for r in 0..content_rows {
        let y = panel.y + 2 + r;
        queue!(out, cursor::MoveTo(panel.x as u16, y as u16))?;

        let line_content = if r < panel.lines.len() {
            truncate_or_pad(&panel.lines[r], panel.width.saturating_sub(1))
        } else {
            " ".repeat(panel.width.saturating_sub(1))
        };

        queue!(
            out,
            SetBackgroundColor(theme.bg),
            SetForegroundColor(theme.border),
            Print("│"),
            SetForegroundColor(theme.fg),
            Print(&line_content),
            ResetColor,
        )?;
    }

    Ok(())
}
