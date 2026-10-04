use mox::editor::files::{parse_ripgrep_json, split_nul_paths};
use std::path::PathBuf;

#[test]
fn nul_separated_paths_preserve_embedded_newlines() {
    let paths = split_nul_paths(b"src/first\npart.rs\0src/second.rs\0");
    assert_eq!(
        paths,
        [
            PathBuf::from("src/first\npart.rs"),
            PathBuf::from("src/second.rs")
        ]
    );
}

#[test]
fn ripgrep_json_preserves_newline_paths_and_match_location() {
    let event = br#"{"type":"match","data":{"path":{"text":"src/first\npart.rs"},"line_number":12,"submatches":[{"start":4,"end":7,"match":{"text":"foo"}}]}}"#;
    let matches = parse_ripgrep_json(event);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].path, PathBuf::from("src/first\npart.rs"));
    assert_eq!(matches[0].line, Some(12));
    assert_eq!(matches[0].column, Some(5));
}

#[test]
fn ripgrep_json_ignores_non_match_events_and_bad_lines() {
    let events = b"not-json\n{\"type\":\"begin\",\"data\":{}}\n";
    assert!(parse_ripgrep_json(events).is_empty());
}
