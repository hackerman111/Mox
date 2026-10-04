use mox::projects::{discover_git_repositories, parse_session_snapshot, unique_session_name};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("mox-projects-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn discovery_finds_git_dirs_once_and_stops_at_depth_limit() {
    let temp = TempDir::new();
    let root = temp.0.join("root");
    let repo = root.join("group").join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let mut too_deep = root.clone();
    for segment in ["one", "two", "three", "four", "five"] {
        too_deep.push(segment);
    }
    fs::create_dir_all(too_deep.join(".git")).unwrap();

    let found = discover_git_repositories(&[root.clone(), root]);
    assert_eq!(found, vec![fs::canonicalize(repo).unwrap()]);
}

#[test]
fn names_are_safe_and_collision_free() {
    let existing = HashSet::from(["my_project".to_owned(), "my_project-2".to_owned()]);
    assert_eq!(
        unique_session_name(&PathBuf::from("/tmp/my project"), &existing),
        "my_project-3"
    );
    assert_eq!(
        unique_session_name(&PathBuf::from("/tmp/!!!"), &HashSet::new()),
        "project"
    );
}

#[test]
fn snapshot_keeps_exact_session_id_and_uses_active_pane_cwd() {
    let raw = "work\t$1\t1\t0\t0\t/tmp/old\nwork\t$1\t1\t1\t1\t/tmp/active\nother\t$2\t0\t1\t1\t/tmp/other\nthird\t$3\t0\t1\t1\t/tmp/other\n";
    let sessions = parse_session_snapshot(raw);
    assert_eq!(sessions.len(), 3);
    assert_eq!(
        (sessions[0].id.as_str(), sessions[0].cwd.as_str()),
        ("$1", "/tmp/active")
    );
    assert_eq!(
        (sessions[1].id.as_str(), sessions[1].cwd.as_str()),
        ("$2", "/tmp/other")
    );
    assert_eq!(
        (sessions[2].id.as_str(), sessions[2].cwd.as_str()),
        ("$3", "/tmp/other")
    );
}
