use mox::extract::model::{EntityKind, ExtractedToken};

#[test]
fn test_extracted_token_creation() {
    let token = ExtractedToken {
        kind: EntityKind::Path,
        raw_text: "src/main.rs:42:10".to_string(),
        clean_text: "src/main.rs".to_string(),
        line_number: Some(42),
        col_number: Some(10),
        pane_id: "%0".to_string(),
        screen_row: 5,
        col_start: 12,
        col_end: 29,
    };

    assert_eq!(token.kind, EntityKind::Path);
    assert_eq!(token.clean_text, "src/main.rs");
    assert_eq!(token.line_number, Some(42));
    assert_eq!(token.col_number, Some(10));
}

#[test]
fn test_entity_kind_labels_parse_all_variants() {
    for kind in EntityKind::ALL {
        assert_eq!(EntityKind::parse(kind.label()), Some(kind));
    }
    assert_eq!(
        EntityKind::parse("docker-image"),
        Some(EntityKind::DockerImage)
    );
    assert_eq!(EntityKind::parse("not-a-kind"), None);
}
