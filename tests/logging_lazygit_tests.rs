use mox::tmux::execute_tmux;
use std::path::PathBuf;
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
        let socket = format!("mox-plugin-tests-{}-{nonce}", std::process::id());
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

fn temp_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mox-{name}-{}-{nonce}", std::process::id()))
}

fn run_mox(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mox"))
        .args(args)
        .output()
        .expect("run mox integration binary")
}

#[test]
fn logging_capture_clear_and_external_pipe_is_preserved() {
    let Some(server) = TmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping logging integration test");
        return;
    };
    let socket = Some(server.socket.as_str());
    let pane = execute_tmux(socket, &["display-message", "-p", "#{pane_id}"]).unwrap();
    let visible = temp_path("visible.log");
    let full = temp_path("full.log");

    mox::logging::save(socket, &pane, &visible, false).unwrap();
    mox::logging::save(socket, &pane, &full, true).unwrap();
    assert!(visible.exists());
    assert!(full.exists());

    execute_tmux(socket, &["pipe-pane", "-t", &pane, "cat >/dev/null"]).unwrap();
    assert!(mox::logging::start(socket, &pane, &visible).is_err());
    assert_eq!(
        execute_tmux(
            socket,
            &["display-message", "-p", "-t", &pane, "#{pane_pipe}"]
        )
        .unwrap(),
        "1"
    );
    assert!(mox::logging::stop(socket, &pane).is_err());
    assert_eq!(
        execute_tmux(
            socket,
            &["display-message", "-p", "-t", &pane, "#{pane_pipe}"]
        )
        .unwrap(),
        "1"
    );
    execute_tmux(socket, &["pipe-pane", "-t", &pane]).unwrap();

    execute_tmux(
        socket,
        &["send-keys", "-t", &pane, "clear-test-line", "Enter"],
    )
    .unwrap();
    mox::logging::clear(socket, &pane).unwrap();
    assert!(
        execute_tmux(
            socket,
            &["display-message", "-p", "-t", &pane, "#{history_size}"]
        )
        .unwrap()
        .parse::<u64>()
        .unwrap()
            <= 1
    );

    let _ = std::fs::remove_file(visible);
    let _ = std::fs::remove_file(full);
}

#[test]
fn lazygit_open_uses_popup_or_reports_missing_binary_cleanly() {
    let Some(server) = TmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping LazyGit integration test");
        return;
    };
    let socket = Some(server.socket.as_str());
    let cwd = execute_tmux(socket, &["display-message", "-p", "#{pane_current_path}"]).unwrap();
    let available = Command::new("lazygit")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    let result = mox::lazygit::open(socket, Some(&cwd), true);
    if !available {
        assert!(result.unwrap_err().contains("lazygit"));
        return;
    }

    result.unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let command = execute_tmux(
            socket,
            &["display-message", "-p", "#{pane_current_command}"],
        )
        .unwrap();
        if command == "lazygit" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "LazyGit pane did not become ready; current command is {command:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn logging_cli_sink_writes_pane_output_and_owns_start_stop_lifecycle() {
    let Some(server) = TmuxServer::new() else {
        eprintln!("tmux server unavailable; skipping logging sink integration test");
        return;
    };
    let socket = server.socket.as_str();
    let pane = execute_tmux(Some(socket), &["display-message", "-p", "#{pane_id}"]).unwrap();
    let path = temp_path("sink.log");
    let path_arg = path.to_str().unwrap();

    let start = run_mox(&[
        "log", "start", path_arg, "--socket", socket, "--pane", &pane,
    ]);
    assert!(
        start.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&start.stderr)
    );
    let repeated_start = run_mox(&[
        "log", "start", path_arg, "--socket", socket, "--pane", &pane,
    ]);
    assert!(
        !repeated_start.status.success(),
        "repeated start replaced the active pipe"
    );

    let marker = format!("mox-log-marker-{}", std::process::id());
    execute_tmux(
        Some(socket),
        &[
            "send-keys",
            "-t",
            &pane,
            &format!("printf '{marker}\\n'"),
            "Enter",
        ],
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        if content.contains(&marker) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "logging sink did not capture pane output; current file is {content:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let stop = run_mox(&["log", "stop", "--socket", socket, "--pane", &pane]);
    assert!(
        stop.status.success(),
        "stop failed: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    let repeated_stop = run_mox(&["log", "stop", "--socket", socket, "--pane", &pane]);
    assert!(
        !repeated_stop.status.success(),
        "repeated stop claimed an inactive pipe"
    );

    let before = std::fs::read(&path).unwrap();
    execute_tmux(
        Some(socket),
        &[
            "send-keys",
            "-t",
            &pane,
            "printf 'after-stop-marker\\n'",
            "Enter",
        ],
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "sink kept logging after stop"
    );
    let _ = std::fs::remove_file(path);
}
