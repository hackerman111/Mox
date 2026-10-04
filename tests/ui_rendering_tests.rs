//! Unit tests for Theme, Mode, status indicators, and common UI rendering primitives.

use mox::ui::palette::{PaletteItem, render_palette_row};
use mox::ui::render::{
    ListRow, PreviewPanel, TreeRow, Viewport, draw_divider, render_confirm, render_empty_state,
    render_hints, render_list_row, render_preview_panel, render_prompt, render_toast,
    render_tree_row, truncate_or_pad, truncate_str,
};
use mox::ui::status::{Mode, render_status_badge, render_status_bar};
use mox::ui::theme::{Theme, nord};

#[test]
fn test_nord_theme_palette_constants() {
    let theme = Theme::nord();

    assert_eq!(theme.bg, nord::NORD0);
    assert_eq!(theme.selection_bg, nord::NORD2);
    assert_eq!(theme.fg, nord::NORD6);
    assert_eq!(theme.border, nord::NORD3);
    assert_eq!(theme.accent, nord::NORD8);
    assert_eq!(theme.error, nord::NORD11);
    assert_eq!(theme.warning, nord::NORD13);
    assert_eq!(theme.success, nord::NORD14);
}

#[test]
fn test_all_status_modes_and_labels() {
    let theme = Theme::nord();
    let modes = [
        (Mode::Normal, "NORMAL", theme.mode_normal),
        (Mode::Prefix, "PREFIX", theme.mode_prefix),
        (Mode::Copy, "COPY", theme.mode_copy),
        (Mode::Visual, "VISUAL", theme.mode_visual),
        (Mode::Resize, "RESIZE", theme.mode_resize),
        (Mode::Sync, "SYNC", theme.mode_sync),
        (Mode::Suspend, "SUSPEND", theme.mode_suspend),
        (Mode::Agent, "AGENT", theme.mode_agent),
    ];

    for (mode, expected_label, expected_color) in modes {
        assert_eq!(mode.label(), expected_label);
        assert_eq!(mode.color(&theme), expected_color);

        let mut buf = Vec::new();
        assert!(render_status_badge(&mut buf, mode, false, &theme).is_ok());
        let output = String::from_utf8_lossy(&buf);
        assert!(
            output.contains(expected_label),
            "Output should contain mode label {expected_label}"
        );
    }
}

#[test]
fn test_status_badge_sticky_indicator() {
    let theme = Theme::nord();
    let mut buf = Vec::new();
    assert!(render_status_badge(&mut buf, Mode::Resize, true, &theme).is_ok());
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("[STICKY]"));
}

#[test]
fn test_status_bar_rendering() {
    let theme = Theme::nord();
    let mut buf = Vec::new();
    assert!(render_status_bar(&mut buf, "NORMAL", "session: 0", 40, &theme).is_ok());
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("NORMAL"));
    assert!(output.contains("session: 0"));
}

#[test]
fn test_list_row_rendering() {
    let theme = Theme::nord();
    let mut buf = Vec::new();
    let row = ListRow {
        prefix: "> ",
        badge: Some(("[PATH]", theme.badge_path)),
        text: "src/main.rs:42",
        is_selected: true,
    };

    assert!(render_list_row(&mut buf, &row, 40, &theme).is_ok());
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("> "));
    assert!(output.contains("[PATH]"));
    assert!(output.contains("src/main.rs:42"));
}

#[test]
fn test_tree_row_rendering() {
    let theme = Theme::nord();
    let mut buf = Vec::new();
    let session_row = TreeRow {
        depth: 0,
        is_collapsed: Some(false),
        label: "dev",
        badge: None,
        is_active: true,
        is_selected: false,
    };
    assert!(render_tree_row(&mut buf, &session_row, 30, &theme).is_ok());
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("▼ dev *"));

    let mut buf_win = Vec::new();
    let win_row = TreeRow {
        depth: 1,
        is_collapsed: Some(true),
        label: "1: editor",
        badge: Some(("[AI]", theme.mode_agent)),
        is_active: false,
        is_selected: true,
    };
    assert!(render_tree_row(&mut buf_win, &win_row, 30, &theme).is_ok());
    let output_win = String::from_utf8_lossy(&buf_win);
    assert!(output_win.contains("├─ ▶ 1: editor"));
    assert!(output_win.contains("[AI]"));
}

#[test]
fn test_prompt_and_confirm_and_toast() {
    let theme = Theme::nord();

    // Prompt
    let mut buf = Vec::new();
    assert!(
        render_prompt(
            &mut buf,
            "[NORMAL]",
            "cargo test",
            Some("3 matches"),
            40,
            &theme
        )
        .is_ok()
    );
    let prompt_out = String::from_utf8_lossy(&buf);
    assert!(prompt_out.contains("[NORMAL] / cargo test [3 matches]"));

    // Confirm
    let mut buf_c = Vec::new();
    assert!(render_confirm(&mut buf_c, "kill-pane %0? (y/n)", &theme).is_ok());
    let confirm_out = String::from_utf8_lossy(&buf_c);
    assert!(confirm_out.contains("[CONFIRM] kill-pane %0? (y/n)"));

    // Toast
    let mut buf_t = Vec::new();
    assert!(render_toast(&mut buf_t, "Buffer copied", false, 40, &theme).is_ok());
    let toast_out = String::from_utf8_lossy(&buf_t);
    assert!(toast_out.contains("✔ Buffer copied"));

    let mut buf_err = Vec::new();
    assert!(render_toast(&mut buf_err, "Failed to switch", true, 40, &theme).is_ok());
    let toast_err = String::from_utf8_lossy(&buf_err);
    assert!(toast_err.contains("✖ Failed to switch"));
}

#[test]
fn test_preview_panel_rendering() {
    let theme = Theme::nord();
    let mut buf = Vec::new();
    let lines = vec![
        "fn main() {".to_string(),
        "    println!(\"hello\");".to_string(),
        "}".to_string(),
    ];

    let panel = PreviewPanel {
        title: "Preview",
        lines: &lines,
        x: 0,
        y: 0,
        width: 30,
        height: 10,
    };
    assert!(render_preview_panel(&mut buf, &panel, &theme).is_ok());
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("Preview"));
    assert!(output.contains("fn main()"));
}

#[test]
fn test_palette_row_rendering() {
    let theme = Theme::nord();
    let mut buf = Vec::new();
    let item = PaletteItem {
        title: "Split Window".to_string(),
        key_hint: Some("w s j".to_string()),
        category: Some("Window".to_string()),
        description: Some("Split horizontally".to_string()),
    };

    assert!(render_palette_row(&mut buf, &item, true, 40, &theme).is_ok());
    let output = String::from_utf8_lossy(&buf);
    assert!(output.contains("[Window]"));
    assert!(output.contains("Split Window"));
    assert!(output.contains("(w s j)"));
}

#[test]
fn test_narrow_terminals_no_panic() {
    let theme = Theme::nord();
    let widths = [0, 1, 5, 10, 15, 20];

    for &w in &widths {
        let mut buf = Vec::new();
        let list_row = ListRow {
            prefix: "> ",
            badge: Some(("[P]", theme.badge_path)),
            text: "long_filename_that_definitely_exceeds_narrow_width.rs:123:45",
            is_selected: false,
        };
        assert!(render_list_row(&mut buf, &list_row, w, &theme).is_ok());

        let mut buf2 = Vec::new();
        let tree_row = TreeRow {
            depth: 2,
            is_collapsed: None,
            label: "deeply_nested_pane_with_lots_of_text",
            badge: None,
            is_active: false,
            is_selected: true,
        };
        assert!(render_tree_row(&mut buf2, &tree_row, w, &theme).is_ok());

        let mut buf3 = Vec::new();
        assert!(render_prompt(&mut buf3, "[SEARCH]", "query", None, w, &theme).is_ok());

        let mut buf4 = Vec::new();
        assert!(render_toast(&mut buf4, "message", false, w, &theme).is_ok());

        let mut buf5 = Vec::new();
        assert!(render_hints(&mut buf5, "hints", w, &theme).is_ok());

        let mut buf6 = Vec::new();
        assert!(render_empty_state(&mut buf6, "empty", w, &theme).is_ok());

        let mut buf7 = Vec::new();
        assert!(draw_divider(&mut buf7, w, &theme).is_ok());
    }
}

#[test]
fn test_viewport_scroll_calculation() {
    // Selection within first page
    assert_eq!(Viewport::adjust_scroll(0, 0, 10), 0);
    assert_eq!(Viewport::adjust_scroll(5, 0, 10), 0);
    assert_eq!(Viewport::adjust_scroll(9, 0, 10), 0);

    // Selection moving past bottom of viewport
    assert_eq!(Viewport::adjust_scroll(10, 0, 10), 1);
    assert_eq!(Viewport::adjust_scroll(15, 0, 10), 6);

    // Selection moving above current offset
    assert_eq!(Viewport::adjust_scroll(3, 6, 10), 3);

    // Degenerate zero visible height
    assert_eq!(Viewport::adjust_scroll(5, 0, 0), 5);
}

#[test]
fn test_truncate_or_pad_unicode() {
    let s = "こんにちは世界";
    let padded = truncate_or_pad(s, 10);
    assert_eq!(mox::ui::render::display_width(&padded), 10);

    let truncated = truncate_or_pad(s, 4);
    assert_eq!(truncated, "こん");
    assert_eq!(mox::ui::render::display_width(&truncated), 4);

    let ascii = truncate_str("hello world", 5);
    assert_eq!(ascii, "hello");
}

#[test]
fn padding_and_truncation_use_terminal_cell_width() {
    use mox::ui::render::{display_width, truncate_or_pad, truncate_str};
    assert_eq!(truncate_str("界abc", 3), "界a");
    assert_eq!(truncate_or_pad("界", 4), "界  ");
    assert_eq!(display_width(&truncate_or_pad("e\u{301}", 4)), 4);
    assert_eq!(truncate_str("界", 1), "");
}
