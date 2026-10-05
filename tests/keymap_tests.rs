use mox::keymap::{KeyAction, build_default_keymap};

#[test]
fn test_root_keymap_completeness() {
    let root = build_default_keymap();

    // Check essential navigation
    assert!(root.find_child("h").is_some());
    assert!(root.find_child("j").is_some());
    assert!(root.find_child("k").is_some());
    assert!(root.find_child("l").is_some());
    assert!(root.find_child("H").is_some());
    assert!(root.find_child("L").is_some());

    // Check splits & windows
    assert!(root.find_child("s").is_some());
    assert!(root.find_child("v").is_some());
    assert!(root.find_child("c").is_some());
    assert!(root.find_child("z").is_some());

    // Check submodes
    assert!(root.find_child("r").is_some());
    assert!(root.find_child("w").is_some());
    assert!(root.find_child("S").is_some());
    assert!(root.find_child("g").is_some());

    // Check modal actions
    assert_eq!(
        root.find_child("a").unwrap().action,
        Some(KeyAction::AppsPanel)
    );
    assert_eq!(
        root.find_child("p").unwrap().action,
        Some(KeyAction::FloaxToggle)
    );
    assert_eq!(
        root.find_child("A").unwrap().action,
        Some(KeyAction::AgentCreate)
    );
    assert_eq!(
        root.find_child("B").unwrap().action,
        Some(KeyAction::Feature(mox::actions::FeatureAction::Btop))
    );
    assert_eq!(
        root.find_child("Enter").unwrap().action,
        Some(KeyAction::Navigator)
    );
    assert_eq!(
        root.find_child("t").unwrap().action,
        Some(KeyAction::TrackToggle)
    );
    assert_eq!(
        root.find_child("T").unwrap().action,
        Some(KeyAction::TrackCurrentToggle)
    );
    assert_eq!(
        root.find_child("[").unwrap().action,
        Some(KeyAction::CopyMode)
    );

    assert_eq!(
        root.find_child("?").unwrap().action,
        Some(KeyAction::WhichKey)
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

    let resize_node = root.find_child("r").expect("Resize mode missing");
    assert!(resize_node.sticky, "Resize mode must be sticky");

    // Check resize steps
    assert!(resize_node.find_child("h").is_some());
    assert!(resize_node.find_child("H").is_some());
    assert!(resize_node.find_child("=").is_some());
}

#[test]
fn test_nested_path_resolution() {
    let root = build_default_keymap();

    // g -> w (tree navigator)
    let goto_w = root.resolve_path(&["g", "w"]);
    assert!(goto_w.is_some());
    assert_eq!(goto_w.unwrap().action, Some(KeyAction::Navigator));

    // g -> s (tree navigator)
    let goto_s = root.resolve_path(&["g", "s"]);
    assert!(goto_s.is_some());
    assert_eq!(goto_s.unwrap().action, Some(KeyAction::Navigator));
}

#[test]
fn test_confirmations_for_dangerous_operations() {
    let root = build_default_keymap();

    // kill window confirmation in root mode (D)
    let kill_window_root = root.find_child("D").unwrap();
    match &kill_window_root.action {
        Some(KeyAction::Confirm { prompt, command }) => {
            assert!(prompt.contains("kill-window"));
            assert_eq!(command, "kill-window");
        }
        _ => panic!("Expected Confirm action for kill window in root"),
    }

    // kill window confirmation in window mode (D)
    let window_node = root.find_child("w").unwrap();
    let kill_window = window_node.find_child("D").unwrap();
    match &kill_window.action {
        Some(KeyAction::Confirm { prompt, command }) => {
            assert!(prompt.contains("kill-window"));
            assert_eq!(command, "kill-window");
        }
        _ => panic!("Expected Confirm action for kill window"),
    }

    // kill session confirmation in session mode (D)
    let window_node = root.find_child("w").unwrap();
    let session_node = window_node.find_child("S").unwrap();
    let kill_session = session_node.find_child("D").unwrap();
    match &kill_session.action {
        Some(KeyAction::Confirm { prompt, command }) => {
            assert!(prompt.contains("kill-session"));
            assert_eq!(command, "kill-session");
        }
        _ => panic!("Expected Confirm action for kill session"),
    }
}

#[test]
fn test_guarded_kill_pane_protects_single_pane() {
    let root = build_default_keymap();

    // In root mode, x and d kill pane only when window_panes > 1
    for key in ["x", "d"] {
        let node = root.find_child(key).expect("root kill-pane key missing");
        match &node.action {
            Some(KeyAction::Tmux(cmd)) => {
                assert!(cmd.contains("#{>:#{window_panes},1}"));
                assert!(cmd.contains("kill-pane"));
                assert!(cmd.contains("Cannot kill last pane"));
            }
            _ => panic!("Expected guarded Tmux action for root key {key}"),
        }
    }

    // In window mode, x and d kill pane only when window_panes > 1
    let window_node = root.find_child("w").unwrap();
    for key in ["x", "d"] {
        let node = window_node
            .find_child(key)
            .expect("window kill-pane key missing");
        match &node.action {
            Some(KeyAction::Tmux(cmd)) => {
                assert!(cmd.contains("#{>:#{window_panes},1}"));
                assert!(cmd.contains("kill-pane"));
                assert!(cmd.contains("Cannot kill last pane"));
            }
            _ => panic!("Expected guarded Tmux action for window key {key}"),
        }
    }
}

#[test]
fn test_clean_status_left() {
    use mox::tmux::clean_status_left;

    let original = "#[fg=black,bg=blue,bold] #S #[default]";
    assert_eq!(clean_status_left(original), original);

    let injected_mox = format!(
        "#{{?#{{==:#{{key-table}},mox}},NORMAL,#{{?#{{==:#{{key-table}},mox_r}},RESIZE,}}}}{original}"
    );
    assert_eq!(clean_status_left(&injected_mox), original);

    let injected_moch = format!(
        "#{{?#{{==:#{{key-table}},moch}},NORMAL,#{{?#{{==:#{{key-table}},moch_r}},RESIZE,}}}}{original}"
    );
    assert_eq!(clean_status_left(&injected_moch), original);

    let old_mox = format!("#{{E:@mox_indicator}}{original}");
    assert_eq!(clean_status_left(&old_mox), original);

    let old_moch = format!("#{{E:@moch_indicator}}{original}");
    assert_eq!(clean_status_left(&old_moch), original);
}

#[test]
fn test_navigator_exits_modal_mode() {
    use mox::tmux::generate_init_script;

    let root = build_default_keymap();
    let script = generate_init_script(&root, "M-m", "mox", None);

    let mox_w_s = script
        .iter()
        .find(|l| l.contains("bind-key -T mox_w s"))
        .expect("mox_w s binding missing");
    assert!(
        mox_w_s.contains("set-option key-table root"),
        "navigator must exit to root key-table: {mox_w_s}"
    );
    assert!(
        mox_w_s.contains("nav"),
        "must contain nav command: {mox_w_s}"
    );

    let mox_s_s = script
        .iter()
        .find(|l| l.contains("bind-key -T mox_w_S s"))
        .expect("mox_w_S s binding missing");
    assert!(
        mox_s_s.contains("set-option key-table root"),
        "navigator must exit to root key-table: {mox_s_s}"
    );
    assert!(
        mox_s_s.contains("nav"),
        "must contain nav command: {mox_s_s}"
    );

    let mox_root_a = script
        .iter()
        .find(|l| l.contains("bind-key -T mox a"))
        .expect("mox a binding missing");
    assert!(
        mox_root_a.contains("apps"),
        "must contain apps: {mox_root_a}"
    );

    let mox_root_enter = script
        .iter()
        .find(|l| l.contains("bind-key -T mox Enter"))
        .expect("mox Enter binding missing");
    assert!(
        mox_root_enter.contains("nav"),
        "must contain nav command: {mox_root_enter}"
    );

    let mox_root_t = script
        .iter()
        .find(|l| l.contains("bind-key -T mox t"))
        .expect("mox t binding missing");
    assert!(
        mox_root_t.contains("track toggle"),
        "must contain track toggle: {mox_root_t}"
    );
}

#[test]
fn test_popup_border_style_clean_background() {
    use mox::tmux::generate_init_script;

    let root = build_default_keymap();
    let script = generate_init_script(&root, "M-m", "mox", None);

    let popups: Vec<_> = script
        .iter()
        .filter(|l| l.contains("display-popup"))
        .collect();
    assert!(
        !popups.is_empty(),
        "No display-popup commands found in script"
    );

    for p in popups {
        assert!(
            p.contains("-S \"fg=#81a1c1,bg=default\"") || p.contains("-S 'fg=#81a1c1,bg=default'"),
            "Popup must use transparent border background (-S with bg=default): {p}"
        );
        assert!(
            p.contains("-s \"fg=#eceff4,bg=#2e3440\"") || p.contains("-s 'fg=#eceff4,bg=#2e3440'"),
            "Popup must use solid inner background (-s with bg=#2e3440): {p}"
        );
    }
}

#[test]
fn test_popup_titles_capitalized_mox() {
    use mox::tmux::generate_init_script;

    let root = build_default_keymap();
    let script = generate_init_script(&root, "M-m", "mox", None);

    let popups: Vec<_> = script
        .iter()
        .filter(|l| l.contains("display-popup"))
        .collect();

    for p in popups {
        assert!(
            !p.contains(" mox: "),
            "Popup title must not contain lowercase ' mox: ': {p}"
        );
        assert!(
            !p.contains(" moch: "),
            "Popup title must not contain legacy ' moch: ': {p}"
        );
        if !p.contains("Tea Timer") {
            assert!(
                p.contains(" Mox: "),
                "Popup title must contain capitalized ' Mox: ': {p}"
            );
        }
    }
}

#[test]
fn test_modal_space_o_f_bindings_trigger_features() {
    use mox::actions::FeatureAction;

    let root = build_default_keymap();

    let space_child = root.find_child("Space").expect("Space must be mapped");
    assert_eq!(
        space_child.action,
        Some(KeyAction::Feature(FeatureAction::Palette))
    );

    let o_child = root.find_child("o").expect("o must be mapped");
    assert_eq!(
        o_child.action,
        Some(KeyAction::Feature(FeatureAction::Projects))
    );

    let f_child = root.find_child("f").expect("f must be mapped");
    assert_eq!(
        f_child.action,
        Some(KeyAction::Feature(FeatureAction::Files))
    );

    let tab_child = root.find_child("Tab").expect("Tab must be mapped");
    assert_eq!(
        tab_child.action,
        Some(KeyAction::Tmux("last-window".into()))
    );

    let qmark_child = root.find_child("?").expect("? must be mapped");
    assert_eq!(qmark_child.action, Some(KeyAction::WhichKey));
}

#[test]
fn test_init_script_modal_bindings_and_no_prefix_hijack() {
    use mox::tmux::generate_init_script;

    let root = build_default_keymap();
    let script = generate_init_script(&root, "M-m", "mox", None);

    // Modal table 'mox' must bind Space, o, f to their respective actions
    let mox_space = script
        .iter()
        .find(|l| l.contains("bind-key -T mox Space"))
        .expect("mox Space binding missing");
    assert!(mox_space.contains("action palette"));

    let mox_o = script
        .iter()
        .find(|l| l.contains("bind-key -T mox o"))
        .expect("mox o binding missing");
    assert!(mox_o.contains("action projects"));

    let mox_f = script
        .iter()
        .find(|l| l.contains("bind-key -T mox f"))
        .expect("mox f binding missing");
    assert!(mox_f.contains("action files"));

    // Prefix table must NOT bind Space, o, f, or N
    assert!(
        !script
            .iter()
            .any(|l| l.contains("bind-key -T prefix Space")),
        "prefix Space must not be bound"
    );
    assert!(
        !script.iter().any(|l| l.contains("bind-key -T prefix o")),
        "prefix o must not be bound"
    );
    assert!(
        !script.iter().any(|l| l.contains("bind-key -T prefix f ")),
        "prefix f must not be bound"
    );
    assert!(
        !script.iter().any(|l| l.contains("bind-key -T prefix N ")),
        "prefix N must not be bound"
    );
}

#[test]
fn test_modal_n_and_g_lazygit_bindings_and_display_merge() {
    use mox::actions::FeatureAction;
    use mox::ui::view::build_display_items;

    let root = build_default_keymap();

    let n_child = root.find_child("N").expect("N must be mapped");
    assert_eq!(
        n_child.action,
        Some(KeyAction::Feature(FeatureAction::LazyGit))
    );

    let g_child = root.find_child("G").expect("G must be mapped");
    assert_eq!(
        g_child.action,
        Some(KeyAction::Feature(FeatureAction::LazyGit))
    );

    let display_items = build_display_items(&root);
    assert!(
        display_items
            .iter()
            .any(|(k, l)| k == "N/G" && l == "LazyGit"),
        "display items must contain merged 'N/G' -> 'LazyGit': {display_items:?}"
    );
    assert!(
        !display_items.iter().any(|(k, _)| k == "G"),
        "G must be merged into N/G rather than appearing standalone: {display_items:?}"
    );
}
