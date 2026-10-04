use mox::tmux::{execute_tmux, load_buffer};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TmuxServer {
    socket: String,
}

impl TmuxServer {
    fn new() -> Option<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let socket = format!("mox-clipboard-tests-{}-{nonce}", std::process::id());
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
            .status()
            .ok()?;
        status.success().then_some(Self { socket })
    }
}

impl Drop for TmuxServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.socket, "kill-server"])
            .output();
    }
}

#[test]
fn streamed_text_is_available_in_tmux_buffer() {
    let Some(server) = TmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping clipboard integration test");
        return;
    };
    let text = "large-ish clipboard payload\nsecond line";
    // Keep this test from touching the host system clipboard.
    load_buffer(Some(&server.socket), text, false).unwrap();
    assert_eq!(
        execute_tmux(Some(&server.socket), &["show-buffer"]).unwrap(),
        text
    );

    load_buffer(Some(&server.socket), "replacement", false).unwrap();
    assert_eq!(
        execute_tmux(Some(&server.socket), &["show-buffer"]).unwrap(),
        "replacement"
    );
}

#[test]
fn open_url_rejects_unsupported_schemes_before_spawning() {
    assert!(mox::clipboard::open_url("javascript:alert(1)").is_err());
    assert!(mox::clipboard::open_url("file:///etc/passwd").is_err());
}
