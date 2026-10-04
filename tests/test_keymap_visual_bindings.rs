use mox::keymap::build_default_keymap;
use mox::tmux::scroll::generate_scroll_config;

#[test]
fn test_visual_and_flash_keybindings_exist() {
    let keymap = build_default_keymap();
    assert!(
        keymap.find_child('s').is_some(),
        "Modal mode must contain 's' for Flash jump"
    );
    assert!(
        keymap.find_child('S').is_some(),
        "Modal mode must contain 'S' for multi-pane Flash jump"
    );
    assert!(
        keymap.find_child('e').is_some(),
        "Modal mode must contain 'e' for Extract picker"
    );

    // Under 'y' submenu
    let y_node = keymap.find_child('y').expect("Must have 'y' node");
    assert!(
        y_node.find_child('f').is_some(),
        "y f for quick entity yank"
    );
    assert!(y_node.find_child('h').is_some(), "y h for quick hash yank");
    assert!(
        y_node.find_child('s').is_some(),
        "y s for remote flash yank"
    );
    assert!(y_node.find_child('e').is_some(), "y e for extract picker");

    // Under 'g' submenu
    let g_node = keymap.find_child('g').expect("Must have 'g' node");
    assert!(g_node.find_child('f').is_some(), "g f for quick file open");
    assert!(g_node.find_child('x').is_some(), "g x for quick URL open");
}

#[test]
fn test_scroll_config_contains_flash_and_mouse() {
    let config = generate_scroll_config();
    assert!(
        config.iter().any(|c| c.contains("WheelUpPane")),
        "Must configure mouse wheel"
    );
    assert!(
        config
            .iter()
            .any(|c| c.contains("copy-mode-vi s") || c.contains("copy-mode-vi")),
        "Must configure copy-mode-vi motions"
    );
}

#[test]
fn interactive_bindings_release_the_tmux_queue_and_show_visual_status() {
    use mox::tmux::{clean_status_left, generate_init_script};
    let script = generate_init_script(&build_default_keymap(), "M-m", "/tmp/mox", Some("#S"));
    for key in ["f", "F", "R"] {
        let binding = script
            .iter()
            .find(|s| s.starts_with(&format!("bind-key -T copy-mode-vi {key} ")))
            .unwrap();
        assert!(binding.contains("run-shell -b"));
        assert!(!binding.contains("command-prompt"));
    }
    let status = script
        .iter()
        .find(|s| s.starts_with("set -g status-left "))
        .unwrap();
    assert!(status.contains("selection_present,VISUAL,COPY"));
    assert!(status.contains("◈"));
    let base = status
        .strip_prefix("set -g status-left \"")
        .unwrap()
        .strip_suffix('"')
        .unwrap();
    assert_eq!(clean_status_left(base), "#S");
}
