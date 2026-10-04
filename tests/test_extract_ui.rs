use mox::extract::model::{EntityKind, ExtractedToken};
use mox::extract::ui::ExtractState;

#[test]
fn test_extract_state_category_cycling() {
    let mut state = ExtractState::new(vec![]);
    assert_eq!(state.current_category, None);
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Path));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Url));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Hash));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Ip));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Command));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Uuid));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Quoted));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Number));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::DockerImage));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::KubernetesResource));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::HexColor));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::IpfsCid));
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Word));
    state.cycle_category_forward();
    assert_eq!(state.current_category, None);
}

#[test]
fn test_extract_state_category_cycling_backward() {
    let mut state = ExtractState::new(vec![]);
    assert_eq!(state.current_category, None);
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Word));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::IpfsCid));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::HexColor));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::KubernetesResource));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::DockerImage));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Number));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Quoted));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Uuid));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Command));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Ip));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Hash));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Url));
    state.cycle_category_backward();
    assert_eq!(state.current_category, Some(EntityKind::Path));
    state.cycle_category_backward();
    assert_eq!(state.current_category, None);
}

#[test]
fn test_extract_state_empty_category_and_refilter_preserves_original_selection() {
    let mut state = ExtractState::new(vec![
        token(EntityKind::Path, "alpha.rs"),
        token(EntityKind::Url, "https://example.test"),
        token(EntityKind::Path, "beta.rs"),
    ]);
    assert_eq!(state.filtered_indices, vec![0, 1, 2]);
    state.selected_index = 2;
    state.query = "beta".into();
    state.apply_filter();
    assert_eq!(state.filtered_indices, vec![2]);
    assert_eq!(state.selected_index, 0);
    state.query.clear();
    state.apply_filter();
    assert_eq!(state.selected_token().unwrap().clean_text, "beta.rs");

    state.current_category = Some(EntityKind::DockerImage);
    state.apply_filter();
    assert!(state.filtered_indices.is_empty());
    assert_eq!(state.selected_index, 0);
    assert!(state.selected_token().is_none());
}

fn token(kind: EntityKind, text: &str) -> ExtractedToken {
    ExtractedToken {
        kind,
        raw_text: text.into(),
        clean_text: text.into(),
        line_number: None,
        col_number: None,
        pane_id: "%0".into(),
        screen_row: 0,
        col_start: 0,
        col_end: text.len(),
    }
}

#[test]
fn test_extract_state_fuzzy_filtering() {
    let tokens = vec![
        ExtractedToken {
            kind: EntityKind::Path,
            raw_text: "src/main.rs:10".into(),
            clean_text: "src/main.rs".into(),
            line_number: Some(10),
            col_number: None,
            pane_id: "%0".into(),
            screen_row: 0,
            col_start: 0,
            col_end: 14,
        },
        ExtractedToken {
            kind: EntityKind::Url,
            raw_text: "https://github.com".into(),
            clean_text: "https://github.com".into(),
            line_number: None,
            col_number: None,
            pane_id: "%0".into(),
            screen_row: 1,
            col_start: 0,
            col_end: 18,
        },
    ];

    let mut state = ExtractState::new(tokens);
    assert_eq!(state.filtered_indices.len(), 2);

    state.query = "main".into();
    state.apply_filter();
    assert_eq!(state.filtered_indices.len(), 1);
    assert_eq!(state.filtered_indices[0], 0);

    // Filter by URL category
    state.query.clear();
    state.current_category = Some(EntityKind::Url);
    state.apply_filter();
    assert_eq!(state.filtered_indices.len(), 1);
    assert_eq!(state.filtered_indices[0], 1);
}

#[test]
fn test_extract_state_selection_navigation() {
    let tokens = vec![
        ExtractedToken {
            kind: EntityKind::Path,
            raw_text: "file1".into(),
            clean_text: "file1".into(),
            line_number: None,
            col_number: None,
            pane_id: "%0".into(),
            screen_row: 0,
            col_start: 0,
            col_end: 5,
        },
        ExtractedToken {
            kind: EntityKind::Path,
            raw_text: "file2".into(),
            clean_text: "file2".into(),
            line_number: None,
            col_number: None,
            pane_id: "%0".into(),
            screen_row: 1,
            col_start: 0,
            col_end: 5,
        },
    ];
    let mut state = ExtractState::new(tokens);
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.selected_token().unwrap().clean_text, "file1");

    state.move_selection_down();
    assert_eq!(state.selected_index, 1);
    assert_eq!(state.selected_token().unwrap().clean_text, "file2");

    // Don't go past the end
    state.move_selection_down();
    assert_eq!(state.selected_index, 1);

    state.move_selection_up();
    assert_eq!(state.selected_index, 0);

    // Don't go past the top
    state.move_selection_up();
    assert_eq!(state.selected_index, 0);
}

#[test]
fn test_extract_state_picker_mode() {
    use mox::extract::ui::PickerMode;
    let mut state = ExtractState::new(vec![]);
    assert_eq!(state.mode, PickerMode::Normal);
    state.mode = PickerMode::Search;
    assert_eq!(state.mode, PickerMode::Search);
}
