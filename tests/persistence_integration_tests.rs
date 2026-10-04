use mox::persist::{self, Snapshot};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("mox-persist-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn child(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(&path).expect("create working directory");
        path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct TmuxServer {
    socket: String,
}

impl TmuxServer {
    fn new(cwd: &Path) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before Unix epoch")
            .as_nanos();
        let socket = format!("mox-persist-{}-{nonce}", std::process::id());
        let cwd = cwd.to_str().expect("UTF-8 temp directory");
        tmux(
            &socket,
            &[
                "-f",
                "/dev/null",
                "new-session",
                "-d",
                "-s",
                "base",
                "-c",
                cwd,
            ],
        );
        Self { socket }
    }

    fn run(&self, args: &[&str]) -> String {
        tmux(&self.socket, args)
    }

    fn stop(&self) {
        let path = self.run(&["display-message", "-p", "#{socket_path}"]);
        self.run(&["kill-server"]);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::os::unix::net::UnixStream::connect(&path).is_ok() {
            assert!(std::time::Instant::now() < deadline, "tmux did not stop");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

impl Drop for TmuxServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.socket, "kill-server"])
            .output();
    }
}

fn tmux(socket: &str, args: &[&str]) -> String {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket)
        .args(args)
        .output()
        .expect("run tmux; integration coverage requires tmux");
    assert!(
        output.status.success(),
        "tmux {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

fn run_persist(socket: &str, path: &Path, subcommand: &str) -> Output {
    let path = path.to_str().expect("UTF-8 snapshot path");
    Command::new(env!("CARGO_BIN_EXE_mox"))
        .env("HOME", Path::new(path).parent().unwrap())
        .args(["persist", subcommand, "--socket", socket, "--path", path])
        .output()
        .expect("run isolated mox persistence binary")
}

fn snapshot_view(snapshot: &Snapshot) -> Vec<SessionView> {
    let mut sessions = snapshot
        .sessions
        .iter()
        .map(|session| {
            let mut windows = session
                .windows
                .iter()
                .map(|window| {
                    let mut panes = window
                        .panes
                        .iter()
                        .map(|pane| (pane.index, pane.cwd.clone(), pane.active))
                        .collect::<Vec<_>>();
                    panes.sort_by_key(|pane| pane.0);
                    WindowView(
                        window.index,
                        window.name.clone(),
                        normalize_layout(&window.layout),
                        window.active,
                        window.zoom,
                        panes,
                    )
                })
                .collect::<Vec<_>>();
            windows.sort_by_key(|window| window.0);
            SessionView(session.name.clone(), windows)
        })
        .collect::<Vec<_>>();
    sessions.sort_by(|a, b| a.0.cmp(&b.0));
    sessions
}

type PaneView = (u32, PathBuf, bool);
#[derive(Debug, PartialEq, Eq)]
struct WindowView(u32, String, String, bool, bool, Vec<PaneView>);
#[derive(Debug, PartialEq, Eq)]
struct SessionView(String, Vec<WindowView>);

fn normalize_layout(layout: &str) -> String {
    fn number(input: &str, offset: &mut usize) -> String {
        let start = *offset;
        while input
            .as_bytes()
            .get(*offset)
            .is_some_and(u8::is_ascii_digit)
        {
            *offset += 1;
        }
        input[start..*offset].to_owned()
    }

    fn punctuation(input: &str, offset: &mut usize, expected: u8) {
        assert_eq!(input.as_bytes().get(*offset), Some(&expected));
        *offset += 1;
    }

    fn cell(input: &str, offset: &mut usize, output: &mut String) {
        output.push_str(&number(input, offset));
        punctuation(input, offset, b'x');
        output.push('x');
        output.push_str(&number(input, offset));
        punctuation(input, offset, b',');
        output.push(',');
        output.push_str(&number(input, offset));
        punctuation(input, offset, b',');
        output.push(',');
        output.push_str(&number(input, offset));

        match input.as_bytes().get(*offset).copied() {
            Some(open @ (b'[' | b'{')) => {
                *offset += 1;
                output.push(open as char);
                cell(input, offset, output);
                while input.as_bytes().get(*offset) == Some(&b',') {
                    *offset += 1;
                    output.push(',');
                    cell(input, offset, output);
                }
                let close = if open == b'[' { b']' } else { b'}' };
                punctuation(input, offset, close);
                output.push(close as char);
            }
            Some(b',') => {
                *offset += 1;
                let _pane_id = number(input, offset);
                output.push_str(",pane");
            }
            other => panic!("unexpected tmux layout token: {other:?}"),
        }
    }

    let (_, body) = layout.split_once(',').expect("layout checksum");
    let mut output = String::new();
    let mut offset = 0;
    cell(body, &mut offset, &mut output);
    assert_eq!(offset, body.len());
    output
}

fn add_session_topology(server: &TmuxServer, dirs: &[PathBuf]) {
    let base = dirs[0].to_str().unwrap();
    let a1 = dirs[1].to_str().unwrap();
    let a2 = dirs[2].to_str().unwrap();
    let a3 = dirs[3].to_str().unwrap();
    let b1 = dirs[4].to_str().unwrap();
    let b2 = dirs[5].to_str().unwrap();

    server.run(&["new-session", "-d", "-s", "alpha", "-c", a1]);
    server.run(&["rename-window", "-t", "alpha:0", "alpha-main"]);
    server.run(&["split-window", "-d", "-t", "alpha:0", "-c", a2]);
    server.run(&["select-layout", "-t", "alpha:0", "even-horizontal"]);
    let pane = server.run(&["list-panes", "-t", "alpha:0", "-F", "#{pane_id}"]);
    let active_pane = pane.lines().last().expect("second pane");
    server.run(&["select-pane", "-t", active_pane]);
    server.run(&["resize-pane", "-Z", "-t", active_pane]);

    server.run(&[
        "new-window",
        "-d",
        "-t",
        "alpha:",
        "-n",
        "alpha-detail",
        "-c",
        a3,
    ]);
    server.run(&["split-window", "-d", "-t", "alpha:1", "-c", base]);
    server.run(&["select-layout", "-t", "alpha:1", "main-vertical"]);
    server.run(&["select-window", "-t", "alpha:1"]);

    server.run(&["new-session", "-d", "-s", "beta", "-c", b1]);
    server.run(&["rename-window", "-t", "beta:0", "beta-main"]);
    server.run(&["split-window", "-d", "-t", "beta:0", "-c", b2]);
    server.run(&["select-layout", "-t", "beta:0", "even-vertical"]);
    let pane = server.run(&["list-panes", "-t", "beta:0", "-F", "#{pane_id}"]);
    server.run(&[
        "select-pane",
        "-t",
        pane.lines().last().expect("second pane"),
    ]);
}

#[test]
fn cli_save_restore_preserves_layout_and_repeated_restore_is_idempotent() {
    let dirs = TestDir::new();
    let working_dirs = [
        "base",
        "alpha-one",
        "alpha-two",
        "alpha-detail",
        "beta-one",
        "beta-two",
    ]
    .map(|name| dirs.child(name));
    let server = TmuxServer::new(&working_dirs[0]);
    add_session_topology(&server, &working_dirs);

    let snapshot_path = dirs.0.join("sessions.json");
    let saved = run_persist(&server.socket, &snapshot_path, "save");
    assert!(
        saved.status.success(),
        "save failed: {}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let before = persist::read(&snapshot_path).expect("read saved snapshot");
    let expected = snapshot_view(&before);
    assert!(expected.iter().any(|session| session.0 == "alpha"));
    assert!(expected.iter().any(|session| session.0 == "beta"));
    assert!(
        expected
            .iter()
            .any(|session| session.1.iter().any(|window| window.4))
    );

    server.run(&["kill-session", "-t", "alpha"]);
    server.run(&["kill-session", "-t", "beta"]);
    assert_eq!(
        server.run(&["list-sessions", "-F", "#{session_name}"]),
        "base"
    );

    let restored = run_persist(&server.socket, &snapshot_path, "restore");
    assert!(
        restored.status.success(),
        "restore failed: {}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let once = persist::capture(Some(&server.socket)).expect("capture restored state");
    assert_eq!(snapshot_view(&once), expected);

    let restored_again = run_persist(&server.socket, &snapshot_path, "restore");
    assert!(
        restored_again.status.success(),
        "repeat restore failed: {}",
        String::from_utf8_lossy(&restored_again.stderr)
    );
    let twice = persist::capture(Some(&server.socket)).expect("capture repeated restore");
    assert_eq!(snapshot_view(&twice), expected);
    assert_eq!(
        server
            .run(&["list-sessions", "-F", "#{session_name}"])
            .lines()
            .count(),
        expected.len(),
        "repeat restore created duplicate sessions"
    );
}

#[test]
fn cli_restore_of_corrupt_snapshot_does_not_change_tmux_state() {
    let dirs = TestDir::new();
    let cwd = dirs.child("cwd");
    let server = TmuxServer::new(&cwd);
    server.run(&["new-window", "-d", "-t", "base", "-n", "untouched"]);
    let sessions_before = server.run(&["list-sessions", "-F", "#{session_name}"]);
    let panes_before = server.run(&[
        "list-panes",
        "-a",
        "-F",
        "#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_current_path}",
    ]);
    let snapshot_path = dirs.0.join("corrupt.json");
    fs::write(
        &snapshot_path,
        br#"{"version":1,"sessions":["not a session"]}"#,
    )
    .expect("write corrupt snapshot");

    let result = run_persist(&server.socket, &snapshot_path, "restore");
    assert!(!result.status.success(), "corrupt snapshot was accepted");
    assert_eq!(
        server.run(&["list-sessions", "-F", "#{session_name}"]),
        sessions_before
    );
    assert_eq!(
        server.run(&[
            "list-panes",
            "-a",
            "-F",
            "#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_current_path}",
        ]),
        panes_before
    );
}

#[test]
fn cold_restore_preserves_linked_windows() {
    let directory = TestDir::new();
    let server = TmuxServer::new(&directory.0);
    server.run(&[
        "new-session",
        "-d",
        "-s",
        "other",
        "-c",
        directory.0.to_str().unwrap(),
    ]);
    server.run(&["link-window", "-s", "base:0", "-t", "other:4"]);
    server.run(&["select-window", "-t", "other:4"]);
    let path = directory.0.join("snapshot.json");
    let saved = run_persist(&server.socket, &path, "save");
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    server.stop();
    let restored = run_persist(&server.socket, &path, "restore");
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    let base = server.run(&[
        "display-message",
        "-p",
        "-t",
        "base:0",
        "#{window_id},#{pane_id}",
    ]);
    let other = server.run(&[
        "display-message",
        "-p",
        "-t",
        "other:4",
        "#{window_id},#{pane_id}",
    ]);
    assert_eq!(base, other);
    assert_eq!(
        server.run(&["display-message", "-p", "-t", "other:4", "#{window_active}"]),
        "1"
    );
}

#[test]
fn dotted_session_name_survives_save_and_restore() {
    let directory = TestDir::new();
    let server = TmuxServer::new(&directory.0);
    server.run(&["rename-session", "-t", "base", "project.v1"]);
    let path = directory.0.join("snapshot.json");
    let saved = run_persist(&server.socket, &path, "save");
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    server.run(&["new-session", "-d", "-s", "keep-server"]);
    let id = server
        .run(&["list-sessions", "-F", "#{session_name}\t#{session_id}"])
        .lines()
        .find_map(|line| line.strip_prefix("project.v1\t").map(str::to_owned))
        .unwrap();
    server.run(&["kill-session", "-t", &id]);
    let restored = run_persist(&server.socket, &path, "restore");
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    assert!(
        server
            .run(&["list-sessions", "-F", "#{session_name}"])
            .lines()
            .any(|name| name == "project.v1")
    );
}
