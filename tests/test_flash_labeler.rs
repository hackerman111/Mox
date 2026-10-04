use mox::extract::model::{EntityKind, ExtractedToken};
use mox::flash::labeler::{MatchTarget, assign_labels};
use mox::flash::matcher::{find_char_motion_matches, find_matches, find_token_matches};

#[test]
fn test_label_assignment_ergonomics() {
    let targets = vec![
        MatchTarget {
            row: 1,
            col: 5,
            pane_id: "%0".into(),
            matched_text: "foo".into(),
        },
        MatchTarget {
            row: 2,
            col: 10,
            pane_id: "%0".into(),
            matched_text: "bar".into(),
        },
    ];
    let labeled = assign_labels(&targets, 1, 0); // cursor at row 1, col 0
    assert_eq!(labeled.len(), 2);
    assert_eq!(labeled[0].label, "a");
    assert_eq!(labeled[1].label, "s");
}

#[test]
fn test_label_assignment_expansion_many_targets() {
    let targets: Vec<MatchTarget> = (0..30)
        .map(|i| MatchTarget {
            row: i,
            col: 0,
            pane_id: "%0".into(),
            matched_text: format!("target_{i}"),
        })
        .collect();

    let labeled = assign_labels(&targets, 0, 0);
    assert_eq!(labeled.len(), 30);
    // All labels must be unique
    let mut labels: Vec<String> = labeled.iter().map(|l| l.label.clone()).collect();
    labels.sort();
    labels.dedup();
    assert_eq!(labels.len(), 30);
}

#[test]
fn test_pattern_matching_smart_case() {
    let lines = vec![
        "Error: failed to connect".to_string(),
        "another error occurred".to_string(),
        "NoErrorHere".to_string(),
    ];
    let matches = find_matches(&lines, "error", "%0");
    assert_eq!(matches.len(), 3); // "Error", "error", "Error"

    let matches_case = find_matches(&lines, "Error", "%0");
    assert_eq!(matches_case.len(), 2); // Exact case "Error"
}

#[test]
fn test_char_motion_matching() {
    let lines = vec!["abc def abc ghi".to_string()];
    // Find 'a' forward from col 1
    let matches = find_char_motion_matches(&lines, 'a', true, 0, 1, "%0");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].col, 8);
}

#[test]
fn test_char_motion_matching_backward() {
    let lines = vec!["abc def abc ghi".to_string()];
    // Find 'a' backward from col 8
    let matches = find_char_motion_matches(&lines, 'a', false, 0, 8, "%0");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].col, 0);
}

#[test]
fn test_token_matching() {
    let tokens = vec![
        ExtractedToken {
            kind: EntityKind::Path,
            raw_text: "src/main.rs:42:10".to_string(),
            clean_text: "src/main.rs".to_string(),
            line_number: Some(42),
            col_number: Some(10),
            pane_id: "%0".to_string(),
            screen_row: 5,
            col_start: 12,
            col_end: 29,
        },
        ExtractedToken {
            kind: EntityKind::Url,
            raw_text: "https://example.com".to_string(),
            clean_text: "https://example.com".to_string(),
            line_number: None,
            col_number: None,
            pane_id: "%0".to_string(),
            screen_row: 6,
            col_start: 0,
            col_end: 19,
        },
    ];

    let path_matches = find_token_matches(&tokens, Some(EntityKind::Path));
    assert_eq!(path_matches.len(), 1);
    assert_eq!(path_matches[0].row, 5);
    assert_eq!(path_matches[0].col, 12);
    assert_eq!(path_matches[0].matched_text, "src/main.rs");

    let all_matches = find_token_matches(&tokens, None);
    assert_eq!(all_matches.len(), 2);
}
