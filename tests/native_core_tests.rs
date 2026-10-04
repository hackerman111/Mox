use mox::{
    editor::Location,
    persist, state,
    ui::picker::{Input, PickerState},
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("mox-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn location_parses_line_column_and_unicode_paths() {
    let location = Location::parse("/tmp/日本語.rs:12:4").unwrap();
    assert_eq!(location.path, Path::new("/tmp/日本語.rs"));
    assert_eq!(location.line, Some(12));
    assert_eq!(location.column, Some(4));

    let location = Location::parse("relative/file.rs:8").unwrap();
    assert_eq!(location.path, Path::new("relative/file.rs"));
    assert_eq!(location.line, Some(8));
    assert_eq!(location.column, None);
    assert!(Location::parse("file.rs:0").is_err());
    assert!(Location::parse("file.rs:999999999999999999999999999").is_err());
}

#[test]
fn picker_preserves_selection_when_refiltering_and_handles_empty_unicode() {
    let mut state = PickerState::default();
    let values = [(0, "alpha"), (1, "beta"), (2, "alphabet")];
    state.filter(values);
    state.selected = state.indices.iter().position(|&index| index == 1).unwrap();
    state.query = "be".into();
    state.filter(values);
    assert_eq!(state.indices, [1, 2]);
    assert_eq!(state.indices[state.selected], 1);

    state.query.clear();
    state.filter([(0, "日本語 file"), (1, "other")]);
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('語'),
        crossterm::event::KeyModifiers::NONE,
    );
    assert_eq!(state.input(key, 2, true), Input::Changed);
    state.filter([(0, "日本語 file"), (1, "other")]);
    assert_eq!(state.indices, [0]);
    let backspace = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Backspace,
        crossterm::event::KeyModifiers::NONE,
    );
    assert_eq!(state.input(backspace, 1, true), Input::Changed);
    assert_eq!(state.query, "");

    state.query = "missing".into();
    state.filter([(0, "日本語 file"), (1, "other")]);
    assert!(state.indices.is_empty());
    state.viewport(0, 5);
    let down = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::NONE,
    );
    state.input(down, 0, true);
    assert_eq!(state.selected, 0);
}

#[test]
fn state_lock_excludes_concurrent_writer_and_atomic_write_replaces_contents() {
    let dir = temp_dir("state-test");
    let lock_path = dir.join("state.lock");
    let guard = state::lock(&lock_path).unwrap();
    assert!(state::lock(&lock_path).is_err());
    drop(guard);
    assert!(state::lock(&lock_path).is_ok());

    let path = dir.join("state.json");
    state::atomic_write(&path, b"old").unwrap();
    state::atomic_write(&path, b"complete new value").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"complete new value");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn corrupt_or_unsupported_snapshots_are_rejected() {
    let dir = temp_dir("snapshot-test");
    let path = dir.join("snapshot.json");
    fs::write(
        &path,
        br#"{"version":1,"sessions":[{"name":"bad:name","windows":[]}] }"#,
    )
    .unwrap();
    assert!(persist::read(&path).is_err());
    let mut snapshot = serde_json::json!({
        "version": 1, "active_session": null, "sessions": [{
            "name": "valid", "windows": [{
                "index": 0, "name": "main", "layout": "b25d,80x24,0,0,0",
                "active": true, "zoom": false, "panes": [{
                    "id": "%0", "index": 0, "cwd": dir,
                    "active": true, "command": "sh", "argv": []
                }]
            }]
        }]
    });
    fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    assert!(persist::read(&path).is_ok());
    let panes = snapshot["sessions"][0]["windows"][0]["panes"]
        .as_array_mut()
        .unwrap();
    let mut duplicate = panes[0].clone();
    duplicate["index"] = 1.into();
    panes.push(duplicate);
    fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    assert!(
        persist::read(&path)
            .unwrap_err()
            .contains("Invalid pane metadata")
    );
    fs::write(
        &path,
        br#"{"version":99,"active_session":null,"sessions":[]}"#,
    )
    .unwrap();
    assert!(persist::read(&path).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn bounded_process_output_captures_both_streams_and_enforces_combined_limit() {
    use std::process::Command;
    let mut command = Command::new("sh");
    command.args(["-c", "printf out; printf err >&2; exit 7"]);
    let output = mox::process::output(&mut command, Duration::from_secs(2), 16).unwrap();
    assert_eq!(output.stdout, b"out");
    assert_eq!(output.stderr, b"err");
    assert_eq!(output.status.code(), Some(7));

    let mut command = Command::new("sh");
    command.args(["-c", "printf 12345; printf 67890 >&2"]);
    assert!(
        mox::process::output(&mut command, Duration::from_secs(2), 8)
            .unwrap_err()
            .contains("output limit")
    );
}

#[cfg(target_os = "linux")]
#[test]
fn bounded_process_timeout_kills_descendants() {
    use std::process::Command;
    let dir = temp_dir("process-test");
    let pid_file = dir.join("child.pid");
    let mut command = Command::new("sh");
    command
        .args(["-c", "sleep 30 & echo $! > \"$1\"; wait", "sh"])
        .arg(&pid_file);
    assert!(
        mox::process::output(&mut command, Duration::from_millis(150), 1024)
            .unwrap_err()
            .contains("timed out")
    );
    let pid: u32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // A killed orphan can remain briefly as a zombie until init reaps it; either
    // absence or the zombie state proves it can no longer execute.
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"));
    assert!(
        stat.as_ref().is_err() || stat.as_ref().unwrap().split_whitespace().nth(2) == Some("Z")
    );
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn bounded_process_timeout_returns_an_error() {
    use std::process::Command;
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 5"]);
    assert!(
        mox::process::output(&mut command, Duration::from_millis(80), 1024)
            .unwrap_err()
            .contains("timed out")
    );
}
