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
            let parts = tree_row_parts(item, app.picker.query.is_empty(), theme);
            let row = TreeRow {
                depth: parts.depth,
                is_collapsed: parts.collapsed,
                icon: parts.icon,
                label: &parts.label,
                badge: parts.badge,
                is_active: parts.active,
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

pub struct NavTreeRowParts {
    pub depth: usize,
    pub collapsed: Option<bool>,
    pub icon: Option<(&'static str, Color)>,
    pub label: String,
    pub badge: Option<(&'static str, Color)>,
    pub active: bool,
}

pub fn tree_row_parts(
    item: &TreeItem<'_>,
    preserve_collapse: bool,
    theme: &Theme,
) -> NavTreeRowParts {
    match item {
        TreeItem::Session(session) => NavTreeRowParts {
            depth: 0,
            collapsed: Some(preserve_collapse && session.collapsed),
            icon: Some(("󰍹 ", theme.accent)),
            label: session.name.clone(),
            badge: None,
            active: session.active,
        },
        TreeItem::Window { window, .. } => {
            let (icon, badge) = if window.is_agent {
                (("󰚩 ", theme.mode_agent), Some(("[AI]", theme.mode_agent)))
            } else {
                (("󰖲 ", theme.warning), None)
            };
            NavTreeRowParts {
                depth: 1,
                collapsed: Some(preserve_collapse && window.collapsed),
                icon: Some(icon),
                label: format!("{}: {}", window.index, window.name),
                badge,
                active: window.active,
            }
        }
        TreeItem::Pane { pane, .. } => {
            let title_info = if !pane.title.is_empty() && pane.title != pane.command {
                format!(" [{}]", pane.title)
            } else {
                String::new()
            };
            NavTreeRowParts {
                depth: 2,
                collapsed: None,
                icon: Some((" ", theme.fg)),
                label: format!("{} {}  {}{title_info}", pane.id, pane.command, pane.cwd),
                badge: None,
                active: pane.active,
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::model::{SessionInfo, TreeItem};

    #[test]
    fn test_tree_row_parts_populates_icons() {
        let session = SessionInfo {
            id: "$0".into(),
            name: "main".into(),
            active: true,
            windows: vec![],
            collapsed: false,
        };
        let item = TreeItem::Session(&session);
        let theme = Theme::load(None);
        let row_data = tree_row_parts(&item, true, &theme);
        assert_eq!(row_data.icon.unwrap().0, "󰍹 ");
    }
}
