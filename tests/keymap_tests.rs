use moch::keymap::{KeyAction, build_default_keymap};

#[test]
fn test_root_keymap_completeness() {
    let root = build_default_keymap();

    // Check essential navigation
    assert!(root.find_child("h").is_some());
    assert!(root.find_child("j").is_some());
    assert!(root.find_child("k").is_some());
    assert!(root.find_child("l").is_some());

    // Check submenus
    assert!(root.find_child("w").is_some());
    assert!(root.find_child("s").is_some());
    assert!(root.find_child("g").is_some());

    // Check modal actions
    assert_eq!(
        root.find_child("c").unwrap().action,
        Some(KeyAction::CopyMode)
    );
    assert_eq!(
        root.find_child("T").unwrap().action,
        Some(KeyAction::TeaTimer)
    );
    assert_eq!(
        root.find_child("q").unwrap().action,
        Some(KeyAction::ExitModal)
    );
    assert_eq!(
        root.find_child("Escape").unwrap().action,
        Some(KeyAction::ExitModal)
    );
    assert_eq!(
        root.find_child("M-m").unwrap().action,
        Some(KeyAction::ExitModal)
    );
    assert_eq!(
        root.find_child("i").unwrap().action,
        Some(KeyAction::ExitModal)
    );
}

#[test]
fn test_sticky_window_and_resize_modes() {
    let root = build_default_keymap();

    let window_node = root.find_child("w").expect("Window mode missing");
    assert!(window_node.sticky, "Window mode must be sticky");

    let resize_node = window_node.find_child("r").expect("Resize mode missing");
    assert!(resize_node.sticky, "Resize mode must be sticky");

    // Check resize steps
    assert!(resize_node.find_child("h").is_some());
    assert!(resize_node.find_child("H").is_some());
}

#[test]
fn test_nested_path_resolution() {
    let root = build_default_keymap();

    // w -> s -> j (split horizontal)
    let split_j = root.resolve_path(&["w", "s", "j"]);
    assert!(split_j.is_some());
    if let Some(KeyAction::Tmux(cmd)) = &split_j.unwrap().action {
        assert!(cmd.contains("split-window"));
    } else {
        panic!("Expected Tmux action for split");
    }

    // g -> w -> 1 (goto window 1)
    let goto_w1 = root.resolve_path(&["g", "w", "1"]);
    assert!(goto_w1.is_some());
    if let Some(KeyAction::Tmux(cmd)) = &goto_w1.unwrap().action {
        assert!(cmd.contains("select-window -t :1"));
    } else {
        panic!("Expected Tmux action for goto window");
    }

    // g -> s -> t (choose session tree)
    let goto_st = root.resolve_path(&["g", "s", "t"]);
    assert!(goto_st.is_some());
    if let Some(KeyAction::Tmux(cmd)) = &goto_st.unwrap().action {
        assert!(cmd.contains("choose-tree -Zs"));
    } else {
        panic!("Expected Tmux action for choose session");
    }
}

#[test]
fn test_confirmations_for_dangerous_operations() {
    let root = build_default_keymap();
    let window_node = root.find_child("w").unwrap();

    // kill pane confirmation
    let kill_pane = window_node.find_child("d").unwrap();
    match &kill_pane.action {
        Some(KeyAction::Confirm { prompt, command }) => {
            assert!(prompt.contains("kill-pane"));
            assert_eq!(command, "kill-pane");
        }
        _ => panic!("Expected Confirm action for kill pane"),
    }

    // kill window confirmation
    let kill_window = window_node.find_child("D").unwrap();
    match &kill_window.action {
        Some(KeyAction::Confirm { prompt, command }) => {
            assert!(prompt.contains("kill-window"));
            assert_eq!(command, "kill-window");
        }
        _ => panic!("Expected Confirm action for kill window"),
    }

    // kill session confirmation
    let session_node = root.find_child("s").unwrap();
    let kill_session = session_node.find_child("D").unwrap();
    match &kill_session.action {
        Some(KeyAction::Confirm { prompt, command }) => {
            assert!(prompt.contains("kill-session"));
            assert_eq!(command, "kill-session");
        }
        _ => panic!("Expected Confirm action for kill session"),
    }
}
