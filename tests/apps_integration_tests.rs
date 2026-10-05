use mox::apps::floax::{FloaxConfig, ensure_floax_session, has_floax_session};
use mox::apps::model::{AppConfig, AppLaunchMode};
use mox::apps::runner::{find_existing_pane, run_app};
use mox::apps::tracking::{
    list_tracked_windows, toggle_tracked_window, track_window, untrack_window,
};
use mox::apps::ui::{build_picker_items, execute_selected_item};
use mox::apps::{check_binary, open_lazygit};
use mox::tmux::execute_tmux;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct IsolatedTmuxServer {
    socket: String,
}

impl IsolatedTmuxServer {
    fn new(name: &str) -> Option<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let socket = format!("mox-apps-int-{name}-{}-{nonce}", std::process::id());
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
                "origin",
            ])
            .status()
            .ok()?;

        status.success().then_some(Self { socket })
    }
}

impl Drop for IsolatedTmuxServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.socket, "kill-server"])
            .output();
    }
}

#[test]
fn test_floax_persistent_scratchpad_integration() {
    let Some(server) = IsolatedTmuxServer::new("floax") else {
        eprintln!("tmux unavailable; skipping floax scratchpad integration test");
        return;
    };
    let socket = Some(server.socket.as_str());
    let config = FloaxConfig::default();

    // 1. Initial state: scratchpad session does not exist
    assert!(!has_floax_session(socket, &config.session_name));

    // 2. Call ensure_floax_session and verify session is created
    ensure_floax_session(socket, &config, None).expect("ensure_floax_session should succeed");
    assert!(
        has_floax_session(socket, &config.session_name),
        "has_floax_session must be true after ensure"
    );

    // 3. Verify mox-scratch session options: status == off and detach-on-destroy == on
    let status_opt = execute_tmux(
        socket,
        &["show-option", "-t", &config.session_name, "-qv", "status"],
    )
    .expect("get status option");
    assert_eq!(status_opt.trim(), "off", "status option must be off");

    let detach_opt = execute_tmux(
        socket,
        &[
            "show-option",
            "-t",
            &config.session_name,
            "-qv",
            "detach-on-destroy",
        ],
    )
    .expect("get detach-on-destroy option");
    assert_eq!(
        detach_opt.trim(),
        "on",
        "detach-on-destroy option must be on"
    );

    // 4. Send command to mox-scratch pane (simulating codex / agent interaction)
    let marker = "CODEX_SCRATCHPAD_ACTIVE_INTEGRATION_TEST_42";
    execute_tmux(
        socket,
        &[
            "send-keys",
            "-t",
            &format!("{}:0", config.session_name),
            &format!("echo '{marker}'"),
            "Enter",
        ],
    )
    .expect("send-keys to scratchpad pane");

    // Wait and verify output appears in mox-scratch pane
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut captured = false;
    while Instant::now() < deadline {
        if let Ok(content) = execute_tmux(
            socket,
            &[
                "capture-pane",
                "-t",
                &format!("{}:0", config.session_name),
                "-p",
            ],
        ) && content.contains(marker)
        {
            captured = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(captured, "Scratchpad pane must capture sent command output");

    // 5. Verify session stays alive across multiple calls
    ensure_floax_session(socket, &config, None).expect("idempotent ensure should succeed");
    assert!(has_floax_session(socket, &config.session_name));

    // 6. Checking the session proves persistent state is retained
    let content = execute_tmux(
        socket,
        &[
            "capture-pane",
            "-t",
            &format!("{}:0", config.session_name),
            "-p",
        ],
    )
    .expect("capture pane after second ensure");
    assert!(
        content.contains(marker),
        "Persistent state in scratchpad must be retained across multiple calls"
    );
}

#[test]
fn test_tmux_window_tracking_and_quick_toggling_integration() {
    let Some(server) = IsolatedTmuxServer::new("tracking") else {
        eprintln!("tmux unavailable; skipping window tracking integration test");
        return;
    };
    let socket = Some(server.socket.as_str());

    // 1. Setup multiple windows: "code-work" and "agent-codex"
    execute_tmux(socket, &["rename-window", "-t", "origin:0", "code-work"])
        .expect("rename window to code-work");
    let code_work_id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("get code-work window id")
        .trim()
        .to_string();

    execute_tmux(socket, &["new-window", "-n", "agent-codex"]).expect("create agent-codex window");
    let agent_codex_id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("get agent-codex window id")
        .trim()
        .to_string();

    assert_ne!(code_work_id, agent_codex_id);

    // 2. Mark "agent-codex" window as tracked via track_window
    let tracked_target =
        track_window(socket, Some(&agent_codex_id)).expect("track agent-codex window");
    assert_eq!(tracked_target, agent_codex_id);

    // 3. Verify list_tracked_windows includes "agent-codex"
    let tracked = list_tracked_windows(socket).expect("list tracked windows");
    assert_eq!(tracked.len(), 1);
    assert_eq!(tracked[0].id, agent_codex_id);
    assert_eq!(tracked[0].name, "agent-codex");

    // 4. Switch to "code-work" window
    execute_tmux(socket, &["select-window", "-t", &code_work_id]).expect("select code-work");
    let active_id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(active_id, code_work_id);

    // 5. From "code-work", call toggle_tracked_window:
    //    Verifies active window switches to "agent-codex" and @mox_prev_window is set to "code-work".
    toggle_tracked_window(socket, None).expect("toggle to tracked window");
    let active_id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(
        active_id, agent_codex_id,
        "Active window must switch to agent-codex"
    );

    let prev_window = execute_tmux(socket, &["show-option", "-qv", "@mox_prev_window"])
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(
        prev_window, code_work_id,
        "@mox_prev_window must be recorded as code-work"
    );

    // 6. Call toggle_tracked_window again from "agent-codex":
    //    Verifies active window switches back to "code-work".
    toggle_tracked_window(socket, None).expect("toggle back to previous window");
    let active_id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(
        active_id, code_work_id,
        "Active window must switch back to code-work"
    );

    // 7. Call untrack_window: verifies window is removed from tracked list.
    let untracked_target =
        untrack_window(socket, Some(&agent_codex_id)).expect("untrack agent-codex window");
    assert_eq!(untracked_target, agent_codex_id);

    let tracked = list_tracked_windows(socket).expect("list tracked windows");
    assert!(
        tracked.is_empty(),
        "Tracked windows list must be empty after untrack"
    );
}

#[test]
fn test_apps_runner_and_lazygit_integration() {
    let Some(server) = IsolatedTmuxServer::new("runner") else {
        eprintln!("tmux unavailable; skipping apps runner integration test");
        return;
    };
    let socket = Some(server.socket.as_str());
    let temp_dir = std::env::temp_dir();
    let temp_str = temp_dir.to_str().unwrap();

    // 1. Verify open_lazygit backward compatibility facade error handling on invalid cwd
    let nonexistent = "/path/to/nonexistent/working/directory/mox_test_xyz_987";
    let bad_cwd_res = open_lazygit(socket, Some(nonexistent), true);
    assert_eq!(
        bad_cwd_res,
        Err(format!("working directory does not exist: {nonexistent}"))
    );

    // 2. Verify open_lazygit execution with valid cwd
    let lazygit_installed = check_binary("lazygit").is_ok();
    let lazygit_res = open_lazygit(socket, Some(temp_str), true);
    if lazygit_installed {
        assert!(
            lazygit_res.is_ok(),
            "open_lazygit must succeed when lazygit is installed: {lazygit_res:?}"
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut lazygit_ready = false;
        while Instant::now() < deadline {
            let panes = execute_tmux(
                socket,
                &["list-panes", "-a", "-F", "#{pane_current_command}"],
            )
            .unwrap_or_default();
            if panes.lines().any(|cmd| cmd.contains("lazygit")) {
                lazygit_ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(lazygit_ready, "LazyGit pane must become ready");
    } else {
        assert!(
            lazygit_res.unwrap_err().contains("lazygit is unavailable:"),
            "open_lazygit must report unavailable binary when lazygit is not installed"
        );
    }

    // 3. Verify run_app pane focusing when an instance is already open
    let session_id = execute_tmux(socket, &["display-message", "-p", "#{session_id}"])
        .expect("get session id")
        .trim()
        .to_string();

    // Create an application window running sleep in temp_str
    execute_tmux(
        socket,
        &[
            "new-window",
            "-d",
            "-c",
            temp_str,
            "-n",
            "existing-app",
            "sleep 300",
        ],
    )
    .expect("create existing app window");

    // Poll until tmux sees the process running so pane_current_command is "sleep"
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut target_pane = None;
    while Instant::now() < deadline {
        if let Ok(Some(pane)) = find_existing_pane(socket, &session_id, "sleep", temp_str) {
            target_pane = Some(pane);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let existing_pane = target_pane.expect("find_existing_pane must discover running sleep pane");

    // Make sure we are currently on origin:0, NOT on the existing app pane
    execute_tmux(socket, &["select-window", "-t", "origin:0"]).expect("select origin:0");
    let initial_pane = execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])
        .expect("get active pane")
        .trim()
        .to_string();
    assert_ne!(
        initial_pane, existing_pane,
        "Active pane must initially differ from existing app pane"
    );

    let win_count_before = execute_tmux(socket, &["list-windows", "-F", "#{window_id}"])
        .expect("list window ids")
        .lines()
        .count();

    // Define AppConfig with focus_existing: true
    let app = AppConfig {
        name: "existing-sleep-app".to_string(),
        title: "Existing Sleep App".to_string(),
        command: "sleep 300".to_string(),
        key: None,
        mode: AppLaunchMode::Window,
        focus_existing: true,
        check_binary: false,
    };

    // run_app should focus the existing pane rather than creating a new window
    run_app(socket, &app, Some(temp_str), false).expect("run_app should succeed");

    let win_count_after = execute_tmux(socket, &["list-windows", "-F", "#{window_id}"])
        .expect("list window ids")
        .lines()
        .count();
    assert_eq!(
        win_count_before, win_count_after,
        "No new window should be created when focus_existing finds matching pane"
    );

    let active_pane = execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])
        .expect("get active pane")
        .trim()
        .to_string();
    assert_eq!(
        active_pane, existing_pane,
        "Existing pane must be focused when run_app is called"
    );

    // 4. Verify btop runner configuration execution
    let btop_app = AppConfig {
        name: "btop".to_string(),
        title: "Btop System Monitor".to_string(),
        command: "btop".to_string(),
        key: Some("B".to_string()),
        mode: AppLaunchMode::Window,
        focus_existing: true,
        check_binary: true,
    };
    let btop_installed = check_binary("btop").is_ok();
    let btop_res = run_app(socket, &btop_app, Some(temp_str), false);
    if btop_installed {
        assert!(
            btop_res.is_ok(),
            "run_app for btop must succeed when btop is installed: {btop_res:?}"
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut btop_ready = false;
        while Instant::now() < deadline {
            let panes = execute_tmux(
                socket,
                &["list-panes", "-a", "-F", "#{pane_current_command}"],
            )
            .unwrap_or_default();
            if panes.lines().any(|cmd| cmd.contains("btop")) {
                btop_ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(btop_ready, "Btop pane must become ready");
    } else {
        assert!(
            btop_res.unwrap_err().contains("btop is unavailable:"),
            "run_app must report unavailable binary when btop is not installed"
        );
    }
}

#[test]
fn test_apps_panel_ui_picker_items_and_actions_integration() {
    let Some(server) = IsolatedTmuxServer::new("ui-actions") else {
        eprintln!("tmux unavailable; skipping ui actions integration test");
        return;
    };
    let socket = Some(server.socket.as_str());

    // Create a window to track
    execute_tmux(socket, &["new-window", "-n", "ui-test-win"]).expect("create ui test window");
    let test_win_id = execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("get window id")
        .trim()
        .to_string();

    // Track it via execute_selected_item("action:track_current")
    execute_tmux(socket, &["select-window", "-t", &test_win_id]).expect("select test win");
    execute_selected_item(socket, "action:track_current", &[]).expect("track current action");

    let tracked = list_tracked_windows(socket).expect("list tracked");
    assert_eq!(tracked.len(), 1);
    assert_eq!(tracked[0].id, test_win_id);

    // Build picker items and verify items structure
    let items = build_picker_items(&[], false, &tracked, true);
    assert!(
        items
            .iter()
            .any(|i| i.id == format!("window:{test_win_id}"))
    );
    assert!(items.iter().any(|i| i.id == "action:untrack_current"));
    assert!(items.iter().any(|i| i.id == "action:toggle_track"));
    assert!(items.iter().any(|i| i.id == "floax"));

    // Untrack via execute_selected_item("action:untrack_current")
    execute_selected_item(socket, "action:untrack_current", &[]).expect("untrack current action");
    let tracked = list_tracked_windows(socket).expect("list tracked");
    assert!(tracked.is_empty());
}
