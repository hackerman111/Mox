//! Dual-pane tree navigator rendering and formatting.

use super::model::TreeItem;
use super::state::NavigatorApp;
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::Write;

pub fn render_navigator(
    app: &NavigatorApp,
    items: &[TreeItem],
    width: usize,
    height: usize,
    out: &mut std::io::Stdout,
) -> Result<(), String> {
    let _ = execute!(
        out,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All),
        SetBackgroundColor(Color::Rgb {
            r: 46,
            g: 52,
            b: 64
        }),
        SetForegroundColor(Color::Rgb {
            r: 236,
            g: 239,
            b: 244
        })
    );

    let tree_width = if app.show_preview {
        (width * 55 / 100).max(30).min(width.saturating_sub(20))
    } else {
        width
    };
    let preview_width = width.saturating_sub(tree_width).saturating_sub(1);
    let content_height = height.saturating_sub(4);

    // 1. Search Bar
    let _ = execute!(out, cursor::MoveTo(0, 0));
    let mode_badge = if app.in_search_mode {
        "[SEARCH]"
    } else {
        "[NORMAL]"
    };
    let query_display = format!(" / {}", app.search_query);
    let bar_line = truncate_or_pad(&format!("{mode_badge}{query_display}"), tree_width);

    let _ = execute!(
        out,
        SetForegroundColor(Color::Rgb {
            r: 136,
            g: 192,
            b: 208
        }),
        Print(&bar_line),
        ResetColor,
        SetBackgroundColor(Color::Rgb {
            r: 46,
            g: 52,
            b: 64
        })
    );

    if app.show_preview && preview_width > 0 {
        let _ = execute!(
            out,
            cursor::MoveTo(tree_width as u16, 0),
            SetForegroundColor(Color::Rgb {
                r: 129,
                g: 161,
                b: 193
            }),
            Print("│ Preview"),
            ResetColor,
            SetBackgroundColor(Color::Rgb {
                r: 46,
                g: 52,
                b: 64
            })
        );
    }

    // Divider line
    let _ = execute!(out, cursor::MoveTo(0, 1));
    let div_line = "─".repeat(tree_width);
    let _ = execute!(
        out,
        SetForegroundColor(Color::Rgb {
            r: 76,
            g: 86,
            b: 106
        }),
        Print(&div_line)
    );
    if app.show_preview && preview_width > 0 {
        let _ = execute!(
            out,
            cursor::MoveTo(tree_width as u16, 1),
            Print("┼"),
            Print("─".repeat(preview_width))
        );
    }

    // Scroll calculation for tree
    let mut scroll_offset = app.scroll_offset;
    if app.selected_index < scroll_offset {
        scroll_offset = app.selected_index;
    } else if app.selected_index >= scroll_offset + content_height {
        scroll_offset = app.selected_index.saturating_sub(content_height) + 1;
    }

    // 2. Tree Items
    for row in 0..content_height {
        let y = (row + 2) as u16;
        let item_idx = scroll_offset + row;

        let _ = execute!(out, cursor::MoveTo(0, y));

        if item_idx < items.len() {
            let item = &items[item_idx];
            let is_selected = item_idx == app.selected_index;

            let line_text = format_tree_line(item, tree_width);

            if is_selected {
                let _ = execute!(
                    out,
                    SetBackgroundColor(Color::Rgb {
                        r: 67,
                        g: 76,
                        b: 94
                    }),
                    SetForegroundColor(Color::Rgb {
                        r: 236,
                        g: 239,
                        b: 244
                    }),
                    Print(&line_text),
                    ResetColor,
                    SetBackgroundColor(Color::Rgb {
                        r: 46,
                        g: 52,
                        b: 64
                    })
                );
            } else {
                let _ = execute!(
                    out,
                    SetForegroundColor(item_color(item)),
                    Print(&line_text),
                    ResetColor,
                    SetBackgroundColor(Color::Rgb {
                        r: 46,
                        g: 52,
                        b: 64
                    })
                );
            }
        } else {
            let _ = execute!(out, Print(" ".repeat(tree_width)));
        }

        // Preview Divider and Content
        if app.show_preview && preview_width > 0 {
            let _ = execute!(
                out,
                cursor::MoveTo(tree_width as u16, y),
                SetForegroundColor(Color::Rgb {
                    r: 76,
                    g: 86,
                    b: 106
                }),
                Print("│"),
                ResetColor,
                SetBackgroundColor(Color::Rgb {
                    r: 46,
                    g: 52,
                    b: 64
                })
            );

            let preview_line = if row < app.cached_preview_lines.len() {
                truncate_or_pad(&app.cached_preview_lines[row], preview_width)
            } else {
                " ".repeat(preview_width)
            };

            let _ = execute!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: 216,
                    g: 222,
                    b: 233
                }),
                Print(&preview_line),
                ResetColor,
                SetBackgroundColor(Color::Rgb {
                    r: 46,
                    g: 52,
                    b: 64
                })
            );
        }
    }

    // 3. Footer / Help Bar
    let footer_y = (height.saturating_sub(2)) as u16;
    let _ = execute!(
        out,
        cursor::MoveTo(0, footer_y),
        SetForegroundColor(Color::Rgb {
            r: 76,
            g: 86,
            b: 106
        }),
        Print("─".repeat(width))
    );

    let status_y = (height.saturating_sub(1)) as u16;
    let _ = execute!(out, cursor::MoveTo(0, status_y));

    if let Some((prompt, _)) = &app.confirm_action {
        let prompt_str = format!(" [CONFIRM] {prompt} ");
        let _ = execute!(
            out,
            SetBackgroundColor(Color::Rgb {
                r: 191,
                g: 97,
                b: 106
            }),
            SetForegroundColor(Color::Black),
            Print(&prompt_str),
            ResetColor
        );
    } else if let Some((target_type, _)) = &app.rename_target {
        let prompt_str = format!(" Rename {target_type}: {}_ ", app.rename_buffer);
        let _ = execute!(
            out,
            SetBackgroundColor(Color::Rgb {
                r: 235,
                g: 203,
                b: 139
            }),
            SetForegroundColor(Color::Black),
            Print(&prompt_str),
            ResetColor
        );
    } else {
        let help_text =
            " Enter:Switch │ dd:Kill │ r:Rename │ Tab:Fold │ ?:Preview │ /:Search │ Esc:Exit";
        let _ = execute!(
            out,
            SetForegroundColor(Color::Rgb {
                r: 143,
                g: 188,
                b: 187
            }),
            Print(help_text),
            ResetColor
        );
    }

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
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

pub fn truncate_or_pad(s: &str, target_width: usize) -> String {
    let char_count = s.chars().count();
    if char_count < target_width {
        format!("{s}{}", " ".repeat(target_width - char_count))
    } else {
        s.chars().take(target_width).collect()
    }
}

pub fn item_color(item: &TreeItem) -> Color {
    match item {
        TreeItem::Session(_) => Color::Rgb {
            r: 136,
            g: 192,
            b: 208,
        },
        TreeItem::Window { window, .. } => {
            if window.is_agent {
                Color::Rgb {
                    r: 180,
                    g: 142,
                    b: 173,
                }
            } else {
                Color::Rgb {
                    r: 235,
                    g: 203,
                    b: 139,
                }
            }
        }
        TreeItem::Pane { .. } => Color::Rgb {
            r: 216,
            g: 222,
            b: 233,
        },
    }
}
