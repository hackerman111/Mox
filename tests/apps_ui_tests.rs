use mox::apps::model::{AppConfig, AppLaunchMode, TrackedWindow};
use mox::apps::ui::{build_picker_items, execute_selected_item};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct IsolatedTmuxServer {
    socket: String,
}

impl IsolatedTmuxServer {
    fn new() -> Option<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let socket = format!("mox-ui-tests-{}-{nonce}", std::process::id());
        let status = Command::new("tmux")
            .args([
                "-L",
                &socket,
                "-f",
                "/dev/null",
                "new-session",
                "-d",
                "-s",
                "main",
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
fn test_build_picker_items_empty_and_idle() {
    let items = build_picker_items(&[], false, &[], false);

    // Floax item exists and indicates IDLE
    let floax_item = items
        .iter()
        .find(|i| i.id == "floax")
        .expect("FloaX item should be present");
    assert!(
        floax_item.label.to_lowercase().contains("idle")
            || floax_item.detail.to_lowercase().contains("idle")
    );

    // Track current window item exists when current_window_tracked is false
    let track_item = items
        .iter()
        .find(|i| i.id == "action:track_current")
        .expect("track_current item should be present");
    assert_eq!(track_item.label, "[+] Track current window");

    // Untrack current should NOT be present
    assert!(!items.iter().any(|i| i.id == "action:untrack_current"));

    // Toggle track action exists
    assert!(items.iter().any(|i| i.id == "action:toggle_track"));
}

#[test]
fn test_build_picker_items_with_apps_and_floax_running() {
    let apps = vec![
        AppConfig {
            name: "lazygit".to_string(),
            title: "LazyGit".to_string(),
            command: "lazygit".to_string(),
            key: Some("G".to_string()),
            mode: AppLaunchMode::Popup {
                width: "85%".to_string(),
                height: "85%".to_string(),
            },
            focus_existing: true,
            check_binary: true,
        },
        AppConfig {
            name: "htop".to_string(),
            title: "Htop".to_string(),
            command: "htop".to_string(),
            key: None,
            mode: AppLaunchMode::Window,
            focus_existing: false,
            check_binary: false,
        },
    ];

    let items = build_picker_items(&apps, true, &[], true);

    // App items exist with correct IDs
    let lazygit_item = items
        .iter()
        .find(|i| i.id == "app:lazygit")
        .expect("lazygit item should exist");
    assert!(lazygit_item.label.contains("LazyGit"));
    assert!(lazygit_item.label.contains("G") || lazygit_item.detail.contains("G"));
    assert!(lazygit_item.detail.contains("lazygit"));
    assert!(lazygit_item.detail.contains("popup"));

    let htop_item = items
        .iter()
        .find(|i| i.id == "app:htop")
        .expect("htop item should exist");
    assert!(htop_item.label.contains("Htop"));
    assert!(htop_item.detail.contains("htop"));
    assert!(htop_item.detail.contains("window"));

    // Floax item reflects RUNNING
    let floax_item = items
        .iter()
        .find(|i| i.id == "floax")
        .expect("FloaX item should be present");
    assert!(
        floax_item.label.to_lowercase().contains("running")
            || floax_item.detail.to_lowercase().contains("running")
    );

    // Untrack current item exists when current_window_tracked is true
    let untrack_item = items
        .iter()
        .find(|i| i.id == "action:untrack_current")
        .expect("untrack_current item should be present");
    assert_eq!(untrack_item.label, "[-] Untrack current window");

    // Track current should NOT be present
    assert!(!items.iter().any(|i| i.id == "action:track_current"));

    // Toggle track action exists
    assert!(items.iter().any(|i| i.id == "action:toggle_track"));
}

#[test]
fn test_build_picker_items_with_tracked_windows() {
    let tracked = vec![
        TrackedWindow {
            id: "@1".to_string(),
            name: "editor".to_string(),
            session_id: "$0".to_string(),
            active: false,
        },
        TrackedWindow {
            id: "@2".to_string(),
            name: "logs".to_string(),
            session_id: "$0".to_string(),
            active: true,
        },
    ];

    let items = build_picker_items(&[], false, &tracked, false);

    let win1 = items
        .iter()
        .find(|i| i.id == "window:@1")
        .expect("window:@1 should exist");
    assert!(win1.label.contains("editor"));
    assert!(win1.label.contains("@1") || win1.detail.contains("@1"));
    assert!(win1.detail.contains("$0"));

    let win2 = items
        .iter()
        .find(|i| i.id == "window:@2")
        .expect("window:@2 should exist");
    assert!(win2.label.contains("logs"));
    assert!(win2.label.contains("@2") || win2.detail.contains("@2"));
}

#[test]
fn test_execute_selected_item_errors_on_unknown() {
    let apps = vec![];
    let result = execute_selected_item(None, "unknown_item_id", &apps);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown_item_id"));

    let result = execute_selected_item(None, "app:missing_app", &apps);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("missing_app"));
}

#[test]
fn test_execute_selected_item_dispatch_with_tmux() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux not available; skipping integration dispatch test");
        return;
    };
    let socket = Some(server.socket.as_str());

    // 1. Test track_current dispatch
    assert!(execute_selected_item(socket, "action:track_current", &[]).is_ok());

    // 2. Test untrack_current dispatch
    assert!(execute_selected_item(socket, "action:untrack_current", &[]).is_ok());

    // Create a second window so window switching can operate
    let _ = Command::new("tmux")
        .args(["-L", &server.socket, "new-window", "-n", "second"])
        .status();

    // 3. Test toggle_track dispatch
    let _ = execute_selected_item(socket, "action:track_current", &[]);
    assert!(execute_selected_item(socket, "action:toggle_track", &[]).is_ok());

    // 4. Test window:<id> dispatch
    assert!(execute_selected_item(socket, "window:@0", &[]).is_ok());

    // 5. Test app execution dispatch
    let apps = vec![AppConfig {
        name: "sh_app".to_string(),
        title: "Sh App".to_string(),
        command: "sh".to_string(),
        key: None,
        mode: AppLaunchMode::Window,
        focus_existing: false,
        check_binary: true,
    }];
    assert!(execute_selected_item(socket, "app:sh_app", &apps).is_ok());

    // 6. Test floax dispatch
    assert!(!mox::apps::has_floax_session(socket, "mox-scratch"));
    // Calling "floax" creates the session; in headless tmux display-popup may error with "no current client"
    let _ = execute_selected_item(socket, "floax", &apps);
    assert!(mox::apps::has_floax_session(socket, "mox-scratch"));
}

#[test]
fn test_apps_module_reexports_ui() {
    use mox::apps::{build_picker_items, execute_selected_item, run_apps_panel};
    let _ = build_picker_items;
    let _ = execute_selected_item;
    let _ = run_apps_panel;
}
