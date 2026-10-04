//! Which-key popup UI layout, grid formatting, and terminal rendering.

use crate::keymap::{KeyAction, KeyNode};
use crate::ui::render::{render_confirm, render_hints, render_toast};
use crate::ui::status::{Mode, render_status_badge};
use crate::ui::theme::Theme;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Attribute, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};

pub fn build_display_items(node: &KeyNode) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let keys: std::collections::HashSet<&str> =
        node.children.iter().map(|c| c.key.as_str()).collect();

    let has_digits = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
        .iter()
        .all(|d| keys.contains(d));

    let mut skip_keys = std::collections::HashSet::new();
    skip_keys.insert("Backspace");
    skip_keys.insert("q");
    skip_keys.insert("Escape");
    skip_keys.insert("i");
    skip_keys.insert("M-m");

    if !node
        .children
        .iter()
        .any(|c| c.key == "Enter" && c.action != Some(KeyAction::ExitModal))
    {
        skip_keys.insert("Enter");
    }

    if has_digits {
        for d in ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
            skip_keys.insert(d);
        }
    }

    if keys.contains("d") && keys.contains("x") {
        skip_keys.insert("x");
    }
    if keys.contains("D") && keys.contains("X") {
        skip_keys.insert("X");
    }
    if keys.contains("t") && keys.contains("T") {
        skip_keys.insert("T");
    }
    if keys.contains("o") && keys.contains("Tab") {
        skip_keys.insert("Tab");
    }
    if keys.contains("?") && keys.contains("Space") {
        skip_keys.insert("Space");
    }

    for child in &node.children {
        if skip_keys.contains(child.key.as_str()) {
            continue;
        }

        let key_disp = if child.key == "d" && keys.contains("x") {
            "x/d".to_string()
        } else if child.key == "D" && keys.contains("X") {
            "X/D".to_string()
        } else if child.key == "t" && keys.contains("T") {
            "t/T".to_string()
        } else if child.key == "o" && keys.contains("Tab") {
            "o/Tab".to_string()
        } else if child.key == "?" && keys.contains("Space") {
            "?/␣".to_string()
        } else if child.key == "Space" {
            "␣".to_string()
        } else {
            child.key.clone()
        };

        let label = if !child.children.is_empty() {
            format!("{}…", child.label)
        } else {
            child.label.clone()
        };

        items.push((key_disp, label));
    }

    if has_digits {
        items.push(("0-9".to_string(), "select win".to_string()));
    }

    items
}

pub fn render_menu(
    node: &KeyNode,
    _history: &[&KeyNode],
    _prefix_str: &str,
    status_msg: Option<&str>,
) -> Result<(), String> {
    let theme = Theme::nord();
    let mut out = stdout();
    execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0))
        .map_err(|e| format!("Render error: {e}"))?;

    let (cols, _rows) = terminal::size().unwrap_or((82, 15));
    let cols = cols as usize;

    let mode_name = if node.label.is_empty() {
        "NORMAL"
    } else {
        &node.label
    };

    let mode = match mode_name.to_uppercase().as_str() {
        "NORMAL" => Mode::Normal,
        "PREFIX" => Mode::Prefix,
        "COPY" => Mode::Copy,
        "VISUAL" => Mode::Visual,
        "RESIZE" => Mode::Resize,
        "WINDOW" => Mode::Normal,
        "SYNC" => Mode::Sync,
        "SUSPEND" => Mode::Suspend,
        "AGENT" => Mode::Agent,
        _ => Mode::Normal,
    };

    execute!(out, Print("  ")).map_err(|e| e.to_string())?;
    render_status_badge(&mut out, mode, node.sticky, &theme).map_err(|e| e.to_string())?;
    execute!(
        out,
        Print(if node.sticky { "" } else { "  which-key" }),
        Print("\r\n\r\n")
    )
    .map_err(|e| e.to_string())?;

    let display_items = build_display_items(node);
    let num_columns = if cols >= 68 && display_items.len() > 6 {
        3
    } else if cols >= 44 && display_items.len() > 3 {
        2
    } else {
        1
    };

    let col_w = (cols / num_columns).max(22);

    for chunk in display_items.chunks(num_columns) {
        for (key, label) in chunk {
            let max_label_len = col_w.saturating_sub(9).max(8);
            execute!(
                out,
                Print("  "),
                SetForegroundColor(theme.warning),
                SetAttribute(Attribute::Bold),
                Print(super::render::truncate_or_pad(key, 5)),
                SetAttribute(Attribute::Reset),
                SetForegroundColor(theme.fg),
                Print("  "),
                Print(super::render::truncate_or_pad(label, max_label_len)),
                ResetColor
            )
            .map_err(|e| e.to_string())?;
        }
        execute!(out, Print("\r\n")).map_err(|e| e.to_string())?;
    }

    if let Some(msg) = status_msg {
        execute!(out, Print("\r\n")).map_err(|e| e.to_string())?;
        render_toast(&mut out, msg, false, cols, &theme).map_err(|e| e.to_string())?;
        execute!(out, Print("\r\n")).map_err(|e| e.to_string())?;
    } else {
        execute!(out, Print("\r\n")).map_err(|e| e.to_string())?;
        render_hints(
            &mut out,
            "[key] run    [q/Esc] back    [i] insert",
            cols,
            &theme,
        )
        .map_err(|e| e.to_string())?;
        execute!(out, Print("\r\n")).map_err(|e| e.to_string())?;
    }

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn prompt_confirm(prompt: &str) -> Result<bool, String> {
    let theme = Theme::nord();
    let mut out = stdout();
    execute!(out, cursor::MoveToNextLine(1)).map_err(|e| e.to_string())?;
    render_confirm(&mut out, prompt, &theme).map_err(|e| e.to_string())?;
    execute!(out, Print(" (y/n) ")).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;

    loop {
        if let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                _ => {}
            }
        }
    }
}
