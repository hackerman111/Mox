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

#[test]
fn test_local_directory_tree_formats_hierarchy() {
    let temp_dir = std::env::temp_dir().join("mox_tree_test_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("sub")).unwrap();
    std::fs::write(temp_dir.join("file1.txt"), "hello").unwrap();
    std::fs::write(temp_dir.join("sub").join("file2.rs"), "fn main() {}").unwrap();

    let lines = mox::ui::preview::local_directory(&temp_dir).expect("loads directory tree");
    assert!(!lines.is_empty());
    assert!(lines.iter().any(|l| l.contains("sub") && l.contains("")));
    assert!(lines.iter().any(|l| l.contains("file1.txt") && l.contains("")));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
