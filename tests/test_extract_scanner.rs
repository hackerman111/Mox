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

#[test]
fn test_new_entity_kinds_and_precedence() {
    let cid = "bafkreiaha4dqobyha4dqobyha4dqobyha4dqobyha4dqobyha4dqobyha4";
    let cid_v0 = "QmNwvNPrBc6g9frTfjxtekH3frhukQiDk56z1mDaohvgyN";
    let line = format!(
        "url https://ghcr.io/org/image:v1 image ghcr.io/org/image:v1 resource pod/web-api color #abc #A1b2C3 path src/main.rs:42:7 hash d8a37b2f ip 192.168.1.1 cid {cid} oldcid {cid_v0} word standalone"
    );
    let tokens = scan_line(&line, 4, "%9");

    for (kind, text) in [
        (EntityKind::Url, "https://ghcr.io/org/image:v1"),
        (EntityKind::DockerImage, "ghcr.io/org/image:v1"),
        (EntityKind::KubernetesResource, "pod/web-api"),
        (EntityKind::HexColor, "#abc"),
        (EntityKind::HexColor, "#A1b2C3"),
        (EntityKind::Path, "src/main.rs:42:7"),
        (EntityKind::Hash, "d8a37b2f"),
        (EntityKind::Ip, "192.168.1.1"),
        (EntityKind::IpfsCid, cid),
        (EntityKind::IpfsCid, cid_v0),
        (EntityKind::Word, "standalone"),
    ] {
        assert!(
            tokens.iter().any(|t| t.kind == kind && t.raw_text == text),
            "missing {kind:?} {text:?}: {tokens:#?}"
        );
    }
    assert!(
        !tokens
            .iter()
            .any(|t| t.kind == EntityKind::DockerImage
                && t.raw_text == "https://ghcr.io/org/image:v1")
    );
    let path = tokens.iter().find(|t| t.kind == EntityKind::Path).unwrap();
    assert_eq!((path.line_number, path.col_number), (Some(42), Some(7)));
    assert_eq!((path.screen_row, path.pane_id.as_str()), (4, "%9"));
}

#[test]
fn test_ipfs_validation_docker_file_line_and_word_quote_boundaries() {
    let invalid_cid = "bafy-not-a-valid-cid";
    let tokens = scan_line(
        &format!(
            "src/file.rs:12:3 ghcr.io/team/app@sha256:{} {invalid_cid} 'quotedword' usefulword",
            "a".repeat(64)
        ),
        0,
        "%0",
    );
    assert!(tokens.iter().any(|t| t.kind == EntityKind::Path
        && t.clean_text == "src/file.rs"
        && t.line_number == Some(12)
        && t.col_number == Some(3)));
    assert!(
        tokens.iter().any(|t| t.kind == EntityKind::DockerImage
            && t.raw_text.starts_with("ghcr.io/team/app@sha256:"))
    );
    assert!(
        !tokens
            .iter()
            .any(|t| t.kind == EntityKind::IpfsCid && t.raw_text == invalid_cid)
    );
    assert!(
        !tokens
            .iter()
            .any(|t| t.kind == EntityKind::Word && t.raw_text == "quotedword")
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Word && t.raw_text == "usefulword")
    );
    assert!(
        !tokens
            .iter()
            .any(|t| t.kind == EntityKind::DockerImage && t.raw_text == "src/file.rs:12:3")
    );
}

#[test]
fn test_unicode_offsets_and_rich_entities_are_not_words() {
    let tokens = scan_line("界 https://example.org standalone слово", 2, "%3");
    let url = tokens.iter().find(|t| t.kind == EntityKind::Url).unwrap();
    assert_eq!((url.col_start, url.col_end), (2, 21));
    let word = tokens.iter().find(|t| t.kind == EntityKind::Word).unwrap();
    assert_eq!(word.raw_text, "standalone");
    assert_eq!(word.col_start, 22);
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == EntityKind::Word && t.raw_text == "слово")
    );
}

#[test]
fn test_command_only_emitted_after_shell_prompt() {
    let tokens = scan_line(" user@host:~/repo$ cargo test --workspace", 1, "%2");
    let command = tokens
        .iter()
        .find(|t| t.kind == EntityKind::Command)
        .unwrap();
    assert_eq!(command.clean_text, "cargo test --workspace");
    assert_eq!((command.col_start, command.col_end), (19, 41));
    assert!(!tokens.iter().any(|t| t.kind == EntityKind::Word));

    let output = scan_line("cargo test --workspace", 0, "%2");
    assert!(!output.iter().any(|t| t.kind == EntityKind::Command));
}

#[test]
fn punctuation_only_terminal_words_do_not_panic() {
    for line in [
        ": ; () [] {} <> '' `` ||",
        ">",
        "(hello)",
        "[] /tmp/file.rs:1:2",
    ] {
        let tokens = mox::extract::scanner::scan_line(line, 0, "%0");
        assert!(tokens.iter().all(|token| token.col_start <= token.col_end));
    }
}
