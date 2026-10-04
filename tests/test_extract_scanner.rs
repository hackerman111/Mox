use mox::extract::model::EntityKind;
use mox::extract::scanner::{scan_line, scan_lines};

#[test]
fn test_scan_entities_in_line() {
    let line = "error at src/lib.rs:25:3 see https://example.com commit d8a37b2f4a";
    let tokens = scan_line(line, 0, "%0");

    assert!(tokens.iter().any(|t| t.kind == EntityKind::Path
        && t.clean_text == "src/lib.rs"
        && t.line_number == Some(25)
        && t.col_number == Some(3)));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Url && t.clean_text == "https://example.com")
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Hash && t.clean_text == "d8a37b2f4a")
    );
}

#[test]
fn test_scan_ip_and_quotes() {
    let line = "connected to 192.168.1.1:8080 message \"hello world\"";
    let tokens = scan_line(line, 1, "%0");

    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Ip && t.clean_text.starts_with("192.168.1.1"))
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Quoted && t.clean_text == "hello world")
    );
}

#[test]
fn test_scan_lines_multiple() {
    let lines = vec![
        "Line 1: https://github.com/rust-lang/rust".to_string(),
        "Line 2: cat /etc/hosts".to_string(),
    ];
    let tokens = scan_lines(&lines, "%1");
    assert!(tokens.iter().any(|t| t.kind == EntityKind::Url));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Path && t.clean_text == "/etc/hosts")
    );
}

#[test]
fn test_scan_trailing_punctuation() {
    let line = "Check (https://example.com/api?v=1), or /var/log/syslog:42; also `cargo test`.";
    let tokens = scan_line(line, 0, "%0");

    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Url && t.clean_text == "https://example.com/api?v=1")
    );
    assert!(tokens.iter().any(|t| t.kind == EntityKind::Path
        && t.clean_text == "/var/log/syslog"
        && t.line_number == Some(42)));
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Quoted && t.clean_text == "cargo test")
    );
}
