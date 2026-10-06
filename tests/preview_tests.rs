use mox::ui::preview::{Preview, PreviewWorker, load};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn temp_path(extension: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "mox-preview-{}-{nonce}.{extension}",
        std::process::id()
    ))
}

fn preview_cli(command_name: &str, path: &std::path::Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mox"));
    command.args([command_name, "--path"]).arg(path);
    mox::process::output(&mut command, Duration::from_secs(2), 1_048_576).unwrap()
}

#[test]
fn file_preview_is_bounded_and_rejects_directories() {
    let path = temp_path("txt");
    fs::write(&path, vec![b'x'; 100_000]).unwrap();
    let lines = load(&Preview::File(path.clone()), None).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 65_536);
    fs::remove_file(&path).unwrap();

    let error = load(&Preview::File(std::env::temp_dir()), None).unwrap_err();
    assert!(error.contains("regular files"));
}

#[test]
fn hidden_preview_cli_returns_bounded_json_for_files_and_directories() {
    let file = temp_path("txt");
    fs::write(&file, "first\nsecond\n").unwrap();
    let output = preview_cli("preview-file", &file);
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Vec<String>>(&output.stdout).unwrap(),
        vec!["first", "second"]
    );
    fs::remove_file(&file).unwrap();

    let directory = temp_path("dir");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("b"), "").unwrap();
    fs::write(directory.join("a"), "").unwrap();
    let output = preview_cli("preview-directory", &directory);
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Vec<String>>(&output.stdout).unwrap(),
        vec!["├──  a", "└──  b"]
    );
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn fifo_is_opened_nonblocking_then_rejected_as_nonregular() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let path = temp_path("fifo");
    let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: c_path is NUL-terminated and points to valid memory for the duration of the call.
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    let started = Instant::now();
    let result = preview_cli("preview-file", &path);
    let elapsed = started.elapsed();
    fs::remove_file(path).unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("regular files"));
    assert!(
        elapsed < Duration::from_secs(1),
        "FIFO open took {elapsed:?}"
    );
}

#[test]
fn preview_worker_discards_old_generations_and_joins_on_drop() {
    let started = Instant::now();
    let worker = PreviewWorker::new(None);
    assert!(worker.request(1, &Preview::Text("stale".into())));

    let deadline = Instant::now() + Duration::from_secs(1);
    let accepted = loop {
        if worker.request(2, &Preview::Text("current".into())) {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        let _ = worker.result(2);
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(accepted, "worker did not accept the next bounded request");

    let result = loop {
        if let Some(lines) = worker.result(2) {
            break Some(lines);
        }
        if Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(result, Some(vec!["current".to_string()]));
    let queue_deadline = Instant::now() + Duration::from_secs(1);
    while !worker.request(3, &Preview::Text("queued at shutdown".into())) {
        assert!(Instant::now() < queue_deadline, "worker queue stayed full");
        std::thread::sleep(Duration::from_millis(2));
    }
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn rendered_text_drops_terminal_controls() {
    assert_eq!(
        mox::ui::render::truncate_str("ok\x1b[2J\tend\u{009b}31m", 64),
        "ok[2J end31m"
    );
    assert_eq!(mox::ui::render::truncate_or_pad("a\x1b[2Jb", 4), "a[2J");
}
