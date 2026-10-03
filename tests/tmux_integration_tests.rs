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
    let script = generate_init_script(&keymap, "M-m", "mox", Some("#S"));

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

    // 4. Verify Navigator and Agent bindings
    assert!(all_keys.contains("agent-toggle"));
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
