//! Integration and validation tests for test fixtures.

use mox::extract::scanner::scan_lines;
use mox::nav::model::parse_tmux_snapshot;
use mox::ui::status::Mode;
use std::fs;

#[test]
fn test_fixture_tmux_snapshots_parsing() {
    let raw = fs::read_to_string("tests/fixtures/tmux_snapshots.txt")
        .expect("Failed to read tmux_snapshots.txt fixture");

    let sessions = parse_tmux_snapshot(&raw);
    assert_eq!(sessions.len(), 3, "Expected 3 sessions (dev, ops, docs)");

    // Dev session
    let dev = &sessions[0];
    assert_eq!(dev.name, "dev");
    assert!(dev.active);
    assert_eq!(dev.windows.len(), 3, "Expected 3 windows in dev session");

    let editor_win = &dev.windows[0];
    assert_eq!(editor_win.name, "editor");
    assert_eq!(
        editor_win.panes.len(),
        2,
        "Expected 2 panes in editor window"
    );
    assert_eq!(editor_win.panes[0].command, "nvim");

    let agent_win = &dev.windows[1];
    assert_eq!(agent_win.name, "agent");
    assert!(agent_win.is_agent, "Window should be marked as agent");

    // Ops session
    let ops = &sessions[1];
    assert_eq!(ops.name, "ops");
    assert_eq!(ops.windows.len(), 2);

    // Docs session
    let docs = &sessions[2];
    assert_eq!(docs.name, "docs");
    assert_eq!(docs.windows.len(), 1);
}

#[test]
fn test_fixture_entities_json_reading_and_scanner() {
    let content = fs::read_to_string("tests/fixtures/entities.json")
        .expect("Failed to read entities.json fixture");

    // Extract all "input": "..." fields without external JSON dependencies
    let mut inputs = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"input\":") {
            let start = trimmed.find('"').unwrap();
            let after_colon = &trimmed[start + 8..]; // skip "input":
            let first_quote = after_colon.find('"').unwrap();
            let last_quote = after_colon.rfind('"').unwrap();
            let val = &after_colon[first_quote + 1..last_quote];
            inputs.push(val.to_string());
        }
    }
    assert!(
        !inputs.is_empty(),
        "Should extract input strings from entities.json"
    );

    let tokens = scan_lines(&inputs, "%0");
    assert!(
        !tokens.is_empty(),
        "Scanner should extract tokens from fixture inputs"
    );
}

#[test]
fn test_fixture_edge_cases_reading_and_scanning() {
    let content = fs::read_to_string("tests/fixtures/edge_cases.txt")
        .expect("Failed to read edge_cases.txt fixture");

    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let tokens = scan_lines(&lines, "%0");
    // Scanner must run without panic across all edge cases (spaces, quotes, unicode, long line)
    assert!(!tokens.is_empty());
}

#[test]
fn test_fixture_neovim_panes_json_validity() {
    let content = fs::read_to_string("tests/fixtures/neovim_panes.json")
        .expect("Failed to read neovim_panes.json fixture");

    assert!(content.contains("scenario_zero_editors"));
    assert!(content.contains("scenario_single_editor"));
    assert!(content.contains("scenario_multiple_editors_different_cwd"));
    assert!(content.contains("target_file"));
}

#[test]
fn test_fixture_status_modes_match_ui_mode_enum() {
    let content = fs::read_to_string("tests/fixtures/status_modes.json")
        .expect("Failed to read status_modes.json fixture");

    let expected_modes = [
        Mode::Normal,
        Mode::Prefix,
        Mode::Copy,
        Mode::Visual,
        Mode::Resize,
        Mode::Sync,
        Mode::Suspend,
        Mode::Agent,
    ];

    for mode in expected_modes {
        let label_key = format!("\"label\": \"{}\"", mode.label());
        assert!(
            content.contains(&label_key),
            "Fixture status_modes.json should contain {label_key}"
        );
    }
}
