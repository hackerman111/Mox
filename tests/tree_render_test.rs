use crossterm::style::Color;
use mox::ui::render::{TreeRow, render_tree_row};
use mox::ui::theme::Theme;

#[test]
fn test_tree_row_renders_icon_and_clean_branch() {
    let theme = Theme::load(None);
    let mut out = Vec::new();
    let row = TreeRow {
        depth: 1,
        is_collapsed: Some(false),
        icon: Some(("󰖲 ", Color::Yellow)),
        label: "window 1: editor",
        badge: Some(("[AI]", Color::Cyan)),
        is_active: true,
        is_selected: false,
    };

    render_tree_row(&mut out, &row, 50, &theme).expect("render succeeds");
    let output = String::from_utf8_lossy(&out);
    assert!(output.contains("├─"));
    assert!(output.contains("󰖲"));
    assert!(output.contains("window 1: editor"));
    assert!(output.contains("●"));
}
