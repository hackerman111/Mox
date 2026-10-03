use moch::keymap::build_default_keymap;
use moch::tmux::{apply_tmux_commands, execute_tmux, generate_init_script};
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
    let Some(server) = TmuxTestServer::new("moch-integration-test") else {
        eprintln!("tmux binary not functional, skipping integration test");
        return;
    };

    let keymap = build_default_keymap();
    let script = generate_init_script(&keymap, "M-m", "moch");

    let result = apply_tmux_commands(Some(&server.socket), &script);
    assert!(
        result.is_ok(),
        "apply_tmux_commands failed: {:?}",
        result.err()
    );

    // 1. Verify moch root table bindings
    let all_keys =
        execute_tmux(Some(&server.socket), &["list-keys"]).expect("Failed to list tmux keys");

    assert!(all_keys.contains("bind-key    -T moch") || all_keys.contains("bind-key -T moch"));
    assert!(all_keys.contains("select-pane -L"));
    assert!(all_keys.contains("select-pane -D"));
    assert!(all_keys.contains("select-pane -U"));
    assert!(all_keys.contains("select-pane -R"));
    assert!(all_keys.contains("set-option key-table root")); // q/Esc/M-m exit
    assert!(all_keys.contains("which-key --prefix 'w'")); // popup on prefix

    // 2. Verify sticky window mode bindings
    assert!(all_keys.contains("bind-key    -T moch_w") || all_keys.contains("bind-key -T moch_w"));
    assert!(all_keys.contains("new-window"));
    assert!(all_keys.contains("kill-pane"));
    assert!(all_keys.contains("kill-window"));

    // 3. Verify Vim scroll copy-mode-vi bindings
    assert!(
        all_keys.contains("bind-key    -T copy-mode-vi")
            || all_keys.contains("bind-key -T copy-mode-vi")
    );
    assert!(all_keys.contains("cursor-down"));
    assert!(all_keys.contains("cursor-up"));
    assert!(all_keys.contains("halfpage-down"));
    assert!(all_keys.contains("halfpage-up"));
    assert!(all_keys.contains("page-down"));
    assert!(all_keys.contains("page-up"));
    assert!(all_keys.contains("history-bottom"));
    assert!(all_keys.contains("copy-mode-vi-g"));
}
