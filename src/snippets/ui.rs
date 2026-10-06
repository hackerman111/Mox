//! UI layout and rendering for the Snippets & Cheatsheet panel.

use super::model::Snippet;
use super::template::extract_parameters;
use crate::ui::render::{
    PreviewPanel, draw_divider, render_confirm, render_empty_state, render_hints,
    render_preview_panel, render_prompt, truncate_or_pad,
};
use crate::ui::theme::Theme;
use crossterm::{
    cursor, queue,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::Write;

pub fn build_snippet_preview(snippet: &Snippet) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("Title: {}", snippet.title));
    if !snippet.tags.is_empty() {
        let tags_str = snippet
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("Tags: {tags_str}"));
    }
    if !snippet.description.is_empty() {
        lines.push(format!("Description: {}", snippet.description));
    }
    lines.push(String::new());
    lines.push("Command:".to_string());
    lines.push(format!("  {}", snippet.command));

    let params = extract_parameters(&snippet.command);
    if !params.is_empty() {
        lines.push(String::new());
        lines.push("Parameters:".to_string());
        for p in params {
            let def_info = match p.default {
                Some(def) => format!(" (default: {def})"),
                None => " (required)".to_string(),
            };
            lines.push(format!("  • <{}>{def_info}", p.name));
        }
    }
    lines
}

pub struct SnippetsViewState<'a> {
    pub snippets: &'a [Snippet],
    pub query: &'a str,
    pub selected: usize,
    pub offset: usize,
    pub confirm_delete: Option<&'a str>,
    pub prompt_input: Option<(&'a str, &'a str)>,
    pub width: usize,
    pub height: usize,
}

pub fn render_snippets_view<W: Write>(
    out: &mut W,
    view: &SnippetsViewState<'_>,
    theme: &Theme,
) -> Result<(), String> {
    let _ = queue!(
        out,
        cursor::MoveTo(0, 0),
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.fg)
    );

    let list_width = (view.width * 55 / 100)
        .max(35)
        .min(view.width.saturating_sub(25));
    let preview_width = view.width.saturating_sub(list_width);
    let content_height = view.height.saturating_sub(4);

    // 1. Search Bar
    let _ = queue!(out, cursor::MoveTo(0, 0));
    let count_badge = format!("{}/{}", view.snippets.len(), view.snippets.len());
    let _ = render_prompt(
        out,
        "[SNIPPETS]",
        view.query,
        Some(&count_badge),
        list_width,
        theme,
    );

    // Divider
    let _ = queue!(out, cursor::MoveTo(0, 1));
    let _ = draw_divider(out, list_width, theme);

    // List rows
    for row in 0..content_height {
        let y = (row + 2) as u16;
        let item_idx = view.offset + row;
        let _ = queue!(out, cursor::MoveTo(0, y));

        if item_idx < view.snippets.len() {
            let s = &view.snippets[item_idx];
            let is_sel = item_idx == view.selected;

            let (bg, fg) = if is_sel {
                (theme.selection_bg, theme.selection_fg)
            } else {
                (theme.bg, theme.fg)
            };

            let tags_str = if !s.tags.is_empty() {
                format!(
                    " [{}]",
                    s.tags
                        .iter()
                        .map(|t| format!("#{t}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            } else {
                String::new()
            };

            let prefix = if is_sel { ">  " } else { "   " };
            let title_part = format!("{prefix}{}{tags_str}", s.title);
            let line = truncate_or_pad(&title_part, list_width);

            let _ = queue!(
                out,
                SetBackgroundColor(bg),
                SetForegroundColor(if is_sel { theme.selection_fg } else { fg }),
                Print(&line),
                ResetColor
            );
        } else if view.snippets.is_empty() && row == 0 {
            let _ = render_empty_state(out, "No matching commands", list_width, theme);
        } else {
            let _ = queue!(out, Print(" ".repeat(list_width)));
        }
    }

    // 2. Right Preview Panel
    if preview_width > 0 {
        let preview_lines = if let Some(selected_snippet) = view.snippets.get(view.selected) {
            build_snippet_preview(selected_snippet)
        } else {
            vec!["No command selected".to_string()]
        };

        let _ = render_preview_panel(
            out,
            &PreviewPanel {
                title: "Command Details",
                lines: &preview_lines,
                x: list_width,
                y: 0,
                width: preview_width,
                height: view.height.saturating_sub(2),
            },
            theme,
        );
    }

    // 3. Footer
    let footer_y = (view.height.saturating_sub(2)) as u16;
    let _ = queue!(out, cursor::MoveTo(0, footer_y));
    let _ = draw_divider(out, view.width, theme);

    let status_y = (view.height.saturating_sub(1)) as u16;
    let _ = queue!(out, cursor::MoveTo(0, status_y));

    if let Some(target) = view.confirm_delete {
        let prompt_text = format!("Delete snippet '{target}'? (y/n)");
        let _ = render_confirm(out, &prompt_text, theme);
        let _ = queue!(out, Clear(ClearType::UntilNewLine));
    } else if let Some((p_title, p_buf)) = view.prompt_input {
        let prompt_str = format!(" {p_title}: {p_buf}_ ");
        let _ = queue!(
            out,
            SetBackgroundColor(theme.warning),
            SetForegroundColor(theme.bg),
            Print(truncate_or_pad(&prompt_str, view.width)),
            ResetColor,
            Clear(ClearType::UntilNewLine)
        );
    } else {
        let hints = "Enter:Run │ y:Yank │ a:Add │ d:Delete │ /:Search │ Esc:Exit";
        let _ = render_hints(out, hints, view.width, theme);
        let _ = queue!(out, Clear(ClearType::UntilNewLine));
    }

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}
