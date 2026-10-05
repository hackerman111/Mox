use mox::keymap::build_default_keymap;
use mox::nav::model::{TreeItem, fetch_tmux_snapshot};
use mox::nav::ui::{execute_nav_tmux, switch_to_item};
use mox::tmux::{apply_tmux_commands, execute_tmux, generate_init_script};
use std::process::Command;

struct TmuxTestServer {
    socket: String,
}

impl TmuxTestServer {
    fn new(socket_name: &str) -> Option<Self> {
        let socket = socket_name.to_string();
        // Clean up any existing instance first
        let _ = Command::new("tmux")
            .args(["-L", &socket, "kill-server"])
            .output();

        let status = Command::new("tmux")
            .args([
                "-L",
                &socket,
                "-f",
                "/dev/null",
                "new-session",
                "-d",
                "-s",
                "test",
            ])
            .status();

        match status {
            Ok(s) if s.success() => Some(Self { socket }),
            _ => None,
        }
    }
}

impl Drop for TmuxTestServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.socket, "kill-server"])
            .output();
    }
}

#[test]
fn test_tmux_isolated_server_integration() {
    let Some(server) = TmuxTestServer::new("mox-integration-test") else {
        eprintln!("tmux binary not functional, skipping integration test");
        return;
    };

    let keymap = build_default_keymap();
    let script = generate_init_script(&keymap, "M-m", env!("CARGO_BIN_EXE_mox"), Some("#S"));

    let result = apply_tmux_commands(Some(&server.socket), &script);
    assert!(
        result.is_ok(),
        "apply_tmux_commands failed: {:?}",
        result.err()
    );

    // 1. Verify mox root table bindings
    let all_keys =
        execute_tmux(Some(&server.socket), &["list-keys"]).expect("Failed to list tmux keys");

    assert!(all_keys.contains("bind-key    -T mox") || all_keys.contains("bind-key -T mox"));
    assert!(all_keys.contains("select-pane -L"));
    assert!(all_keys.contains("select-pane -D"));
    assert!(all_keys.contains("select-pane -U"));
    assert!(all_keys.contains("select-pane -R"));
    assert!(all_keys.contains("set-option key-table root")); // q/Esc/M-m exit
    assert!(all_keys.contains("which-key")); // which-key available

    // 2. Verify sticky window mode bindings
    assert!(all_keys.contains("bind-key    -T mox_w") || all_keys.contains("bind-key -T mox_w"));
    assert!(all_keys.contains("new-window"));
    assert!(all_keys.contains("kill-pane"));
    assert!(all_keys.contains("kill-window"));

    // 4. Verify Navigator, Apps, and Agent bindings
    assert!(all_keys.contains("apps"));
    assert!(all_keys.contains("agent-create"));
    assert!(all_keys.contains("nav"));
}

#[test]
fn test_tmux_agent_creation_and_toggle_integration() {
    let Some(server) = TmuxTestServer::new("mox-agent-test") else {
        eprintln!("tmux binary not functional, skipping agent test");
        return;
    };

    // Initially window 1 is active (e.g. name "test" or "0")
    let initial_win = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{window_id}"],
    )
    .expect("display-message failed");
    let initial_win = initial_win.trim().to_string();

    // 1. Create agent window
    mox::agent::create_agent_window(Some(&server.socket)).expect("create_agent_window failed");

    // Check that active window is now agent
    let agent_win_id = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{window_id}"],
    )
    .expect("display-message failed");
    let agent_win_id = agent_win_id.trim().to_string();
    assert_ne!(initial_win, agent_win_id);

    let is_agent = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{@mox_is_agent}"],
    )
    .expect("check @mox_is_agent");
    assert_eq!(is_agent.trim(), "1");

    // 2. Toggle agent window (should return to initial window)
    mox::agent::toggle_agent_window(Some(&server.socket)).expect("toggle_agent_window failed");

    let current_win_id = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{window_id}"],
    )
    .expect("display-message failed");
    assert_eq!(current_win_id.trim(), initial_win);

    // 3. Toggle again (should return to agent window)
    mox::agent::toggle_agent_window(Some(&server.socket)).expect("toggle_agent_window 2 failed");

    let current_win_id = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{window_id}"],
    )
    .expect("display-message failed");
    assert_eq!(current_win_id.trim(), agent_win_id);
}

#[test]
fn test_tmux_navigator_switch_and_actions_integration() {
    let Some(server) = TmuxTestServer::new("mox-nav-action-test") else {
        eprintln!("tmux binary not functional, skipping nav actions test");
        return;
    };

    // 1. Create a second pane in window 1
    execute_tmux(Some(&server.socket), &["split-window", "-v"]).expect("split-window failed");

    // Create a second window
    execute_tmux(Some(&server.socket), &["new-window", "-n", "second"]).expect("new-window failed");

    // Fetch snapshot
    let snapshot = fetch_tmux_snapshot(Some(&server.socket)).expect("fetch_tmux_snapshot failed");
    assert_eq!(snapshot.len(), 1);
    let session = &snapshot[0];
    assert_eq!(session.windows.len(), 2);

    let first_win = &session.windows[0];
    assert_eq!(first_win.panes.len(), 2);
    let first_pane = &first_win.panes[0];

    // Currently we are in window 2. Switch to first pane of first window.
    let pane_item = TreeItem::Pane {
        session,
        window: first_win,
        pane: first_pane,
    };
    switch_to_item(Some(&server.socket), &pane_item).expect("switch_to_item failed");

    // Verify window and pane selected
    let active_win = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{window_id}"],
    )
    .expect("display-message failed");
    assert_eq!(active_win.trim(), first_win.id);

    let active_pane = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{pane_id}"],
    )
    .expect("display-message failed");
    assert_eq!(active_pane.trim(), first_pane.id);

    // 2. Test renaming window via execute_nav_tmux
    execute_nav_tmux(
        Some(&server.socket),
        &format!("rename-window -t {} 'renamed_win'", first_win.id),
    )
    .expect("execute_nav_tmux rename failed");

    let win_name = execute_tmux(
        Some(&server.socket),
        &["display-message", "-p", "#{window_name}"],
    )
    .expect("display-message failed");
    assert_eq!(win_name.trim(), "renamed_win");

    let title = "it's my pane";
    let rename = mox::nav::format_rename_command(&pane_item, title).unwrap();
    execute_nav_tmux(Some(&server.socket), &rename).expect("rename pane title failed");
    assert_eq!(
        execute_tmux(
            Some(&server.socket),
            &[
                "display-message",
                "-p",
                "-t",
                &first_pane.id,
                "#{pane_title}"
            ]
        )
        .unwrap(),
        title
    );

    // 3. Test killing pane via execute_nav_tmux
    let second_pane = &first_win.panes[1];
    execute_nav_tmux(
        Some(&server.socket),
        &format!("kill-pane -t {}", second_pane.id),
    )
    .expect("execute_nav_tmux kill-pane failed");

    let fresh = fetch_tmux_snapshot(Some(&server.socket)).expect("fresh snapshot failed");
    let fresh_first_win = &fresh[0].windows[0];
    assert_eq!(fresh_first_win.panes.len(), 1);
    assert_eq!(fresh_first_win.panes[0].id, first_pane.id);
}

#[test]
fn test_tmux_flash_and_extract_integration() {
    let Some(server) = TmuxTestServer::new("mox-flash-extract-test") else {
        eprintln!("tmux binary not functional, skipping integration test");
        return;
    };

    let keymap = build_default_keymap();
    let script = generate_init_script(&keymap, "M-m", env!("CARGO_BIN_EXE_mox"), Some("#S"));
    apply_tmux_commands(Some(&server.socket), &script).expect("failed to apply mox config");

    // Verify mox table has flash and extract bindings
    let root_keys = execute_tmux(Some(&server.socket), &["list-keys", "-T", "mox"])
        .expect("list-keys mox failed");
    assert!(
        root_keys.contains("flash --mode jump --launch-popup"),
        "mox must contain flash jump"
    );
    assert!(
        root_keys.contains("flash --mode jump --multi-pane --launch-popup"),
        "mox must contain multi-pane flash"
    );
    assert!(
        root_keys.contains("extract --launch-popup"),
        "mox must contain extract picker"
    );

    // Verify mox_y has quick yank and remote yank
    let y_keys = execute_tmux(Some(&server.socket), &["list-keys", "-T", "mox_y"])
        .expect("list-keys mox_y failed");
    assert!(
        y_keys.contains("quick-yank"),
        "mox_y must contain quick-yank"
    );
    assert!(
        y_keys.contains("filter hash"),
        "mox_y must contain hash filter"
    );
    assert!(
        y_keys.contains("remote-yank"),
        "mox_y must contain remote-yank"
    );
    assert!(
        y_keys.contains("extract --launch-popup"),
        "mox_y must contain extract"
    );

    // Verify copy-mode-vi has flash and extract bindings
    let vi_keys = execute_tmux(Some(&server.socket), &["list-keys", "-T", "copy-mode-vi"])
        .expect("list-keys copy-mode-vi failed");
    assert!(
        vi_keys.contains("flash --launch-popup"),
        "copy-mode-vi must contain flash"
    );
    assert!(
        vi_keys.contains("extract --launch-popup"),
        "copy-mode-vi must contain extract"
    );

    // Send sample text to pane and verify extraction via scanner
    execute_tmux(
        Some(&server.socket),
        &[
            "send-keys",
            "-l",
            "-t",
            "test",
            "echo 'https://github.com/foo/bar src/main.rs:10:2 9f8e7d6c5b'",
        ],
    )
    .expect("send-keys failed");
    execute_tmux(Some(&server.socket), &["send-keys", "-t", "test", "Enter"])
        .expect("send Enter failed");

    // Allow shell to render echo
    std::thread::sleep(std::time::Duration::from_millis(100));

    let captured = execute_tmux(Some(&server.socket), &["capture-pane", "-p", "-t", "test"])
        .expect("capture-pane failed");

    let lines: Vec<String> = captured.lines().map(|s| s.to_string()).collect();
    let tokens = mox::extract::scanner::scan_lines(&lines, "%0");

    assert!(
        tokens
            .iter()
            .any(|t| t.kind == mox::extract::model::EntityKind::Url
                && t.clean_text.contains("github.com/foo/bar"))
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == mox::extract::model::EntityKind::Path
                && t.clean_text == "src/main.rs"
                && t.line_number == Some(10))
    );
    assert!(
        tokens.iter().any(
            |t| t.kind == mox::extract::model::EntityKind::Hash && t.clean_text == "9f8e7d6c5b"
        )
    );
}

#[test]
fn test_capture_preserves_rows_and_columns_and_jump_keeps_selection() {
    use mox::flash::ui::jump_to_target;
    use mox::tmux::execute_tmux_raw;
    let Some(server) = TmuxTestServer::new("mox-coordinate-test") else {
        eprintln!("tmux unavailable, skipping coordinate integration test");
        return;
    };
    let socket = Some(server.socket.as_str());
    // A literal format gives deterministic blank rows without relying on shell output.
    let raw =
        execute_tmux_raw(socket, &["display-message", "-p", "-l", "\n    token\n\n"]).unwrap();
    assert_eq!(raw, "\n    token\n\n\n");
    jump_to_target(socket, "%0", 2, 0).unwrap();
    execute_tmux(socket, &["send-keys", "-X", "begin-selection"]).unwrap();
    jump_to_target(socket, "%0", 4, 0).unwrap();
    let cursor = execute_tmux(
        socket,
        &[
            "display-message",
            "-p",
            "#{copy_cursor_y},#{selection_present},#{selection_start_y}",
        ],
    )
    .unwrap();
    assert_eq!(cursor, "4,1,2");
}
