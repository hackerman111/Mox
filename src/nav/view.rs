//! Dual-pane tree navigator rendering and formatting.

use super::model::TreeItem;
use super::state::NavigatorApp;
use crate::ui::render::{
    PreviewPanel, TreeRow, draw_divider, render_confirm, render_hints, render_preview_panel,
    render_prompt, render_tree_row, truncate_or_pad,
};
use crate::ui::theme::Theme;
use crossterm::{
    cursor, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::Write;

pub fn render_navigator<W: Write>(
    app: &NavigatorApp,
    items: &[TreeItem],
    width: usize,
    height: usize,
    out: &mut W,
) -> Result<(), String> {
    render_navigator_with_theme(app, items, width, height, out, &Theme::load(None))
}

pub fn render_navigator_with_theme<W: Write>(
    app: &NavigatorApp,
    items: &[TreeItem],
    width: usize,
    height: usize,
    out: &mut W,
    theme: &Theme,
) -> Result<(), String> {
    let _ = queue!(
        out,
        cursor::MoveTo(0, 0),
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.fg)
    );

    let tree_width = if app.show_preview {
        (width * 55 / 100).max(30).min(width.saturating_sub(20))
    } else {
        width
    };
    let preview_width = width.saturating_sub(tree_width);
    let content_height = height.saturating_sub(4);

    // 1. Search Bar
    let _ = queue!(out, cursor::MoveTo(0, 0));
    let mode_badge = if app.in_search_mode {
        "[SEARCH]"
    } else {
        "[NORMAL]"
    };
    let _ = render_prompt(out, mode_badge, &app.picker.query, None, tree_width, theme);

    // Divider line
    let _ = queue!(out, cursor::MoveTo(0, 1));
    let _ = draw_divider(out, tree_width, theme);
    // Tree rows
    for row in 0..content_height {
        let y = (row + 2) as u16;
        let item_idx = app.picker.offset + row;

        let _ = queue!(out, cursor::MoveTo(0, y));

        if item_idx < items.len() {
            let item = &items[item_idx];
            let (depth, collapsed, label, active) =
                tree_row_parts(item, app.picker.query.is_empty());
            let row = TreeRow {
                depth,
                is_collapsed: collapsed,
                icon: None,
                label: &label,
                badge: None,
                is_active: active,
                is_selected: item_idx == app.picker.selected,
            };
            let _ = render_tree_row(out, &row, tree_width, theme);
        } else {
            let _ = queue!(out, Print(" ".repeat(tree_width)));
        }
    }

    if app.show_preview && preview_width > 0 {
        let _ = render_preview_panel(
            out,
            &PreviewPanel {
                title: "Preview",
                lines: &app.cached_preview_lines,
                x: tree_width,
                y: 0,
                width: preview_width,
                height: height.saturating_sub(2),
            },
            theme,
        );
    }

    // 3. Footer / Help Bar
    let footer_y = (height.saturating_sub(2)) as u16;
    let _ = queue!(out, cursor::MoveTo(0, footer_y));
    let _ = draw_divider(out, width, theme);

    let status_y = (height.saturating_sub(1)) as u16;
    let _ = queue!(out, cursor::MoveTo(0, status_y));

    if let Some((prompt, _)) = &app.confirm_action {
        let _ = render_confirm(out, prompt, theme);
        let _ = queue!(out, Clear(ClearType::UntilNewLine));
    } else if let Some((target_type, _)) = &app.rename_target {
        let prompt_str = format!(" Rename {target_type}: {}_ ", app.rename_buffer);
        let _ = queue!(
            out,
            SetBackgroundColor(theme.warning),
            SetForegroundColor(theme.bg),
            Print(truncate_or_pad(&prompt_str, width)),
            ResetColor
        );
    } else {
        let help_text =
            "Enter:Switch │ dd:Kill │ r:Rename │ Tab:Fold │ ?:Preview │ /:Search │ Esc:Exit";
        let _ = render_hints(out, help_text, width, theme);
        let _ = queue!(out, Clear(ClearType::UntilNewLine));
    }

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn tree_row_parts(
    item: &TreeItem<'_>,
    preserve_collapse: bool,
) -> (usize, Option<bool>, String, bool) {
    match item {
        TreeItem::Session(session) => (
            0,
            Some(preserve_collapse && session.collapsed),
            session.name.clone(),
            session.active,
        ),
        TreeItem::Window { window, .. } => (
            1,
            Some(preserve_collapse && window.collapsed),
            format!(
                "{}: {}{}",
                window.index,
                window.name,
                if window.is_agent { " [AI]" } else { "" }
            ),
            window.active,
        ),
        TreeItem::Pane { pane, .. } => {
            let title_info = if !pane.title.is_empty() && pane.title != pane.command {
                format!(" [{}]", pane.title)
            } else {
                String::new()
            };
            (
                2,
                None,
                format!("{} {}  {}{title_info}", pane.id, pane.command, pane.cwd),
                pane.active,
            )
        }
    }
}

pub fn format_tree_line(item: &TreeItem, max_w: usize) -> String {
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

pub fn item_color(item: &TreeItem, theme: &Theme) -> Color {
    match item {
        TreeItem::Session(_) => theme.accent,
        TreeItem::Window { window, .. } => {
            if window.is_agent {
                theme.mode_agent
            } else {
                theme.warning
            }
        }
        TreeItem::Pane { .. } => theme.fg,
    }
}
