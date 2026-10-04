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
        let path =
            std::env::temp_dir().join(format!("mox-persist-hooks-{}-{nonce}", std::process::id()));
        fs::create_dir_all(path.join("home")).expect("create isolated test home");
        Self(path)
    }

    fn child(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(&path).expect("create test directory");
        path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct TmuxServer(String);

impl TmuxServer {
    fn new(cwd: &Path, label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before Unix epoch")
            .as_nanos();
        let socket = format!("mox-hooks-{label}-{}-{nonce}", std::process::id());
        let cwd = cwd.to_str().expect("UTF-8 test cwd");
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
        Self(socket)
    }

    fn run(&self, args: &[&str]) -> String {
        tmux(&self.0, args)
    }
}

impl Drop for TmuxServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.0, "kill-server"])
            .output();
    }
}

fn tmux(socket: &str, args: &[&str]) -> String {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket)
        .args(args)
        .output()
        .expect("run tmux; persistence hooks integration requires tmux");
    assert!(
        output.status.success(),
        "tmux {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned()
}

fn run_persist(socket: &str, home: &Path, path: &Path, command: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mox"))
        .env("HOME", home)
        .args([
            "persist",
            command,
            "--socket",
            socket,
            "--path",
            path.to_str().expect("UTF-8 snapshot path"),
        ])
        .output()
        .expect("run mox persistence CLI")
}

fn assert_success(output: Output, command: &str) {
    assert!(
        output.status.success(),
        "persist {command} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn autosave_respects_snapshot_mtime_and_autorestore_is_idempotent() {
    let dirs = TestDir::new();
    let home = dirs.0.join("home");
    let source_cwd = dirs.child("source-cwd");
    let target_cwd = dirs.child("target-cwd");
    let source = TmuxServer::new(&source_cwd, "source");
    let target = TmuxServer::new(&target_cwd, "target");
    let path = dirs.0.join("explicit-snapshot.json");

    // The explicit path is honored and the first autosave creates the snapshot.
    assert_success(run_persist(&source.0, &home, &path, "autosave"), "autosave");
    let first_mtime = fs::metadata(&path)
        .expect("autosave creates requested snapshot")
        .modified()
        .expect("snapshot mtime");

    // A changed server is not written again while the snapshot is younger than
    // the minimum interval, so repeated status-hook invocations are bounded.
    source.run(&["new-window", "-d", "-t", "base", "-n", "later"]);
    assert_success(
        run_persist(&source.0, &home, &path, "autosave"),
        "autosave repeat",
    );
    assert_eq!(
        fs::metadata(&path)
            .expect("snapshot remains present")
            .modified()
            .expect("snapshot mtime"),
        first_mtime,
        "young snapshots should not be rewritten by repeated autosave"
    );

    // Save the source topology explicitly, restore it to another isolated
    // server, and invoke autorestore again. The server option makes the second
    // call a no-op instead of duplicating or rebuilding restored sessions.
    source.run(&[
        "new-session",
        "-d",
        "-s",
        "recovered",
        "-c",
        source_cwd.to_str().expect("UTF-8 source cwd"),
    ]);
    let restore_path = dirs.0.join("restore-snapshot.json");
    assert_success(run_persist(&source.0, &home, &restore_path, "save"), "save");
    assert_success(
        run_persist(&target.0, &home, &restore_path, "autorestore"),
        "autorestore",
    );
    let once = target.run(&["list-sessions", "-F", "#{session_name}"]);
    assert!(once.lines().any(|name| name == "base"));
    assert!(once.lines().any(|name| name == "recovered"));
    assert_success(
        run_persist(&target.0, &home, &restore_path, "autorestore"),
        "autorestore repeat",
    );
    assert_eq!(
        target.run(&["list-sessions", "-F", "#{session_name}"]),
        once,
        "repeated autorestore should not duplicate or alter restored sessions"
    );
    assert_eq!(
        target.run(&["show-option", "-gqv", "@mox_restore_done"]),
        "1"
    );
}
