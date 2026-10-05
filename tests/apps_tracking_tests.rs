use mox::apps::model::TrackedWindow;
use mox::apps::tracking::{
    list_tracked_windows, parse_tracked_windows, toggle_current_window_tracking,
    toggle_tracked_window, track_window, untrack_window,
};
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
        let socket = format!("mox-tracking-tests-{}-{nonce}", std::process::id());
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
fn test_parse_tracked_windows_filters_and_parses_fields() {
    let raw = "@0\tbash\t$0\t1\t0\n@1\tagent\t$0\t0\t1\n@2\tcodex\t$0\t1\t1\n@3\thtop\t$0\t0\t\n";
    let list = parse_tracked_windows(raw);
    assert_eq!(list.len(), 2);

    assert_eq!(
        list[0],
        TrackedWindow {
            id: "@1".to_string(),
            name: "agent".to_string(),
            session_id: "$0".to_string(),
            active: false,
        }
    );

    assert_eq!(
        list[1],
        TrackedWindow {
            id: "@2".to_string(),
            name: "codex".to_string(),
            session_id: "$0".to_string(),
            active: true,
        }
    );
}

#[test]
fn test_parse_tracked_windows_empty_and_untracked_output() {
    assert_eq!(parse_tracked_windows(""), Vec::<TrackedWindow>::new());
    assert_eq!(
        parse_tracked_windows("   \n\t\n"),
        Vec::<TrackedWindow>::new()
    );

    let untracked_raw = "@0\tbash\t$0\t1\t0\n@1\twork\t$0\t0\t0\n";
    assert_eq!(
        parse_tracked_windows(untracked_raw),
        Vec::<TrackedWindow>::new()
    );

    let malformed_raw = "some malformed text\n@0\tonly_two\n";
    assert_eq!(
        parse_tracked_windows(malformed_raw),
        Vec::<TrackedWindow>::new()
    );
}

#[test]
fn test_track_and_untrack_window_in_isolated_tmux() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping test");
        return;
    };
    let socket = Some(server.socket.as_str());

    // Initially, no windows are tracked
    let tracked = list_tracked_windows(socket).expect("list windows");
    assert!(tracked.is_empty());

    // Track current window
    let target_id = track_window(socket, None).expect("track current window");
    assert!(!target_id.is_empty());

    // List tracked windows should now include this window
    let tracked = list_tracked_windows(socket).expect("list windows");
    assert_eq!(tracked.len(), 1);
    assert_eq!(tracked[0].id, target_id);

    // Untrack the window
    let untracked_id = untrack_window(socket, Some(&target_id)).expect("untrack window");
    assert_eq!(untracked_id, target_id);

    // List tracked windows should be empty again
    let tracked = list_tracked_windows(socket).expect("list windows");
    assert!(tracked.is_empty());
}

#[test]
fn test_toggle_current_window_tracking() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping test");
        return;
    };
    let socket = Some(server.socket.as_str());

    // First toggle turns tracking ON
    let is_tracked = toggle_current_window_tracking(socket).expect("toggle tracking on");
    assert!(is_tracked);
    let tracked = list_tracked_windows(socket).expect("list windows");
    assert_eq!(tracked.len(), 1);

    // Second toggle turns tracking OFF
    let is_tracked = toggle_current_window_tracking(socket).expect("toggle tracking off");
    assert!(!is_tracked);
    let tracked = list_tracked_windows(socket).expect("list windows");
    assert!(tracked.is_empty());
}

#[test]
fn test_toggle_tracked_window_switches_and_restores() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping test");
        return;
    };
    let socket = Some(server.socket.as_str());

    // Window 0 is origin (work window)
    let work_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("get origin window id")
        .trim()
        .to_string();

    // Create a new window for agent
    mox::tmux::execute_tmux(socket, &["new-window", "-n", "agent"]).expect("create agent window");
    let agent_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("get agent window id")
        .trim()
        .to_string();

    // Track the agent window
    track_window(socket, Some(&agent_id)).expect("track agent window");

    // Switch back to work window
    mox::tmux::execute_tmux(socket, &["select-window", "-t", &work_id]).expect("select work");

    let current_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("current window id")
        .trim()
        .to_string();
    assert_eq!(current_id, work_id);

    // Toggle tracked window (from untracked work window -> should jump to agent)
    toggle_tracked_window(socket, None).expect("toggle to agent");
    let current_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("current window id")
        .trim()
        .to_string();
    assert_eq!(current_id, agent_id);

    // Toggle tracked window again (from tracked agent window -> should jump back to work window)
    toggle_tracked_window(socket, None).expect("toggle back to work");
    let current_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("current window id")
        .trim()
        .to_string();
    assert_eq!(current_id, work_id);

    // Toggle specific target window by id
    toggle_tracked_window(socket, Some(&agent_id)).expect("toggle to specific agent");
    let current_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("current window id")
        .trim()
        .to_string();
    assert_eq!(current_id, agent_id);

    toggle_tracked_window(socket, Some(&agent_id)).expect("toggle back from specific agent");
    let current_id = mox::tmux::execute_tmux(socket, &["display-message", "-p", "#{window_id}"])
        .expect("current window id")
        .trim()
        .to_string();
    assert_eq!(current_id, work_id);
}

#[test]
fn test_toggle_tracked_window_no_tracked_windows_returns_err() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping test");
        return;
    };
    let socket = Some(server.socket.as_str());

    let res = toggle_tracked_window(socket, None);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err, "No tracked windows found");
}

#[test]
fn test_apps_module_reexports_tracking() {
    use mox::apps::{
        list_tracked_windows, parse_tracked_windows, toggle_current_window_tracking,
        toggle_tracked_window, track_window, untrack_window,
    };

    let list = parse_tracked_windows("@0\tbash\t$0\t1\t1\n");
    assert_eq!(list.len(), 1);
    let _ = list_tracked_windows;
    let _ = toggle_current_window_tracking;
    let _ = toggle_tracked_window;
    let _ = track_window;
    let _ = untrack_window;
}
