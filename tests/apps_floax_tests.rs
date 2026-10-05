use mox::apps::floax::{
    DEFAULT_FLOAX_HEIGHT, DEFAULT_FLOAX_SESSION_NAME, DEFAULT_FLOAX_TITLE, DEFAULT_FLOAX_WIDTH,
    FloaxConfig, ensure_floax_session, has_floax_session, toggle_floax,
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
        let socket = format!("mox-floax-tests-{}-{nonce}", std::process::id());
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
fn test_default_floax_config_values() {
    let config = FloaxConfig::default();
    assert_eq!(config.session_name, "mox-scratch");
    assert_eq!(config.title, " FloaX Scratchpad ");
    assert_eq!(config.width, "85%");
    assert_eq!(config.height, "85%");
    assert!(!config.change_path);

    assert_eq!(DEFAULT_FLOAX_SESSION_NAME, "mox-scratch");
    assert_eq!(DEFAULT_FLOAX_TITLE, " FloaX Scratchpad ");
    assert_eq!(DEFAULT_FLOAX_WIDTH, "85%");
    assert_eq!(DEFAULT_FLOAX_HEIGHT, "85%");
}

#[test]
fn test_floax_config_custom_builder_options() {
    let custom = FloaxConfig::default()
        .with_session_name("custom-scratch")
        .with_title(" My Scratchpad ")
        .with_width("90%")
        .with_height("80%")
        .with_change_path(true);

    assert_eq!(custom.session_name, "custom-scratch");
    assert_eq!(custom.title, " My Scratchpad ");
    assert_eq!(custom.width, "90%");
    assert_eq!(custom.height, "80%");
    assert!(custom.change_path);
}

#[test]
fn test_command_generation_for_popup_attach() {
    let config = FloaxConfig::default();

    // Without explicit socket
    let attach_no_sock = config.build_attach_command(None);
    assert_eq!(attach_no_sock, "tmux attach-session -t mox-scratch");

    let popup_args_no_sock = config.build_popup_args(None);
    assert_eq!(
        popup_args_no_sock,
        vec![
            "display-popup",
            "-E",
            "-w",
            "85%",
            "-h",
            "85%",
            "-b",
            "rounded",
            "-T",
            " FloaX Scratchpad ",
            "tmux attach-session -t mox-scratch",
        ]
    );

    // With explicit socket
    let attach_with_sock = config.build_attach_command(Some("test-socket"));
    assert_eq!(
        attach_with_sock,
        "tmux -L 'test-socket' attach-session -t mox-scratch"
    );

    let popup_args_with_sock = config.build_popup_args(Some("test-socket"));
    assert_eq!(
        popup_args_with_sock,
        vec![
            "display-popup",
            "-E",
            "-w",
            "85%",
            "-h",
            "85%",
            "-b",
            "rounded",
            "-T",
            " FloaX Scratchpad ",
            "tmux -L 'test-socket' attach-session -t mox-scratch",
        ]
    );
}

#[test]
fn test_floax_session_lifecycle_and_options() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping test_floax_session_lifecycle_and_options");
        return;
    };
    let socket = Some(server.socket.as_str());
    let config = FloaxConfig::default().with_session_name("test-scratch-lifecycle");

    // Initially session should not exist
    assert!(!has_floax_session(socket, &config.session_name));

    // Ensure session creates it in background
    let temp_dir = std::env::temp_dir();
    let temp_str = temp_dir.to_str().unwrap();
    ensure_floax_session(socket, &config, Some(temp_str)).expect("ensure session succeeded");

    // Now session should exist
    assert!(has_floax_session(socket, &config.session_name));

    // Calling ensure again is idempotent
    ensure_floax_session(socket, &config, Some(temp_str))
        .expect("idempotent ensure session succeeded");
    assert!(has_floax_session(socket, &config.session_name));

    // Verify tmux options status is off and detach-on-destroy is on
    let status_opt = mox::tmux::execute_tmux(
        socket,
        &["show-option", "-t", &config.session_name, "-qv", "status"],
    )
    .expect("get status option");
    assert_eq!(status_opt, "off");

    let detach_opt = mox::tmux::execute_tmux(
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
    assert_eq!(detach_opt, "on");
}

#[test]
fn test_apps_module_reexports() {
    use mox::apps::{FloaxConfig, ensure_floax_session, has_floax_session, toggle_floax};
    let config = FloaxConfig::default();
    assert_eq!(config.session_name, "mox-scratch");
    assert!(!has_floax_session(
        Some("nonexistent-socket-12345"),
        &config.session_name
    ));
    let _ = ensure_floax_session;
    let _ = toggle_floax;
}

#[test]
fn test_toggle_floax_inside_session_detaches_client() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!(
            "tmux server unavailable; skipping test_toggle_floax_inside_session_detaches_client"
        );
        return;
    };
    let socket = Some(server.socket.as_str());

    // The isolated server default session is named "origin".
    // If FloaxConfig.session_name matches "origin", toggle_floax sees current_session == session_name
    // and calls detach-client. In headless tmux without a client, detach-client returns Err("no current client"),
    // proving the detach-client branch was reached!
    let config = FloaxConfig::default().with_session_name("origin");
    let result = toggle_floax(socket, &config, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("no current client"),
        "expected detach-client failure due to headless client, got: {err}"
    );
}

#[test]
fn test_toggle_floax_change_path_behavior() {
    let Some(server) = IsolatedTmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping test_toggle_floax_change_path_behavior");
        return;
    };
    let socket = Some(server.socket.as_str());
    let config = FloaxConfig::default()
        .with_session_name("test-change-path")
        .with_change_path(true);

    // Initial session in temp dir
    let temp_dir = std::env::temp_dir();
    let temp_str = temp_dir.to_str().unwrap();
    ensure_floax_session(socket, &config, Some(temp_str)).expect("session created");

    // Toggle with target directory /
    let result = toggle_floax(socket, &config, Some("/"));
    // Even if display-popup fails due to no client, the session should still be active
    assert!(has_floax_session(socket, &config.session_name));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("no current client"),
        "expected display-popup failure due to headless client, got: {err}"
    );
}
