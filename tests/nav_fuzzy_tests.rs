use mox::nav::fuzzy::{filter_sessions, fuzzy_match};
use mox::nav::model::{TreeItem, flatten_tree, parse_tmux_snapshot, visible_tree};

#[test]
fn test_fuzzy_match_basic() {
    assert!(fuzzy_match("nvim", "nv").is_some());
    assert!(fuzzy_match("claude-code", "cc").is_some());
    assert!(fuzzy_match("project", "proj").is_some());
    assert!(fuzzy_match("bash", "zsh").is_none());

    // Higher score for exact or prefix matches
    let prefix_score = fuzzy_match("nvim", "nv").unwrap();
    let sub_score = fuzzy_match("main_nvim", "nv").unwrap();
    assert!(prefix_score > sub_score);
}

#[test]
fn borrowed_tree_projection_matches_legacy_filter_without_model_clones() {
    let raw = "work\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/code/mox\tCargo.toml\t0\n\
               work\t$1\t1\t2\t@2\tshell\t0\t%2\t0\t1\tzsh\t/home\tzsh\t0\n\
               other\t$2\t0\t1\t@3\tserver\t1\t%3\t0\t1\tcargo\t/srv\trun\t0\n";
    let sessions = parse_tmux_snapshot(raw);
    let filtered = filter_sessions(&sessions, "nvim");
    let legacy = flatten_tree(&filtered);
    let borrowed = visible_tree(&sessions, "nvim");

    let key = |item: &TreeItem<'_>| match item {
        TreeItem::Session(session) => session.id.clone(),
        TreeItem::Window { window, .. } => window.id.clone(),
        TreeItem::Pane { pane, .. } => pane.id.clone(),
    };
    assert_eq!(
        legacy.iter().map(key).collect::<Vec<_>>(),
        borrowed.iter().map(key).collect::<Vec<_>>()
    );
    assert!(matches!(borrowed[0], TreeItem::Session(_)));
}

#[test]
fn navigator_fuzzy_uses_shared_ui_scorer() {
    assert_eq!(
        fuzzy_match("project/main.rs", "pmr"),
        mox::ui::fuzzy::fuzzy_match("project/main.rs", "pmr")
    );
}

#[test]
fn test_filter_sessions_preserves_hierarchy() {
    let raw = "work\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/code/mox\tCargo.toml\t0\n\
               work\t$1\t1\t2\t@2\tshell\t0\t%2\t0\t1\tzsh\t/home\tzsh\t0\n\
               other\t$2\t0\t1\t@3\tserver\t1\t%3\t0\t1\tcargo\t/srv\trun\t0\n";
    let sessions = parse_tmux_snapshot(raw);

    // Filter by pane command "nvim"
    let filtered_nvim = filter_sessions(&sessions, "nvim");
    assert_eq!(filtered_nvim.len(), 1);
    assert_eq!(filtered_nvim[0].name, "work");
    assert!(!filtered_nvim[0].collapsed);
    assert_eq!(filtered_nvim[0].windows.len(), 1);
    assert_eq!(filtered_nvim[0].windows[0].name, "editor");
    assert!(!filtered_nvim[0].windows[0].collapsed);
    assert_eq!(filtered_nvim[0].windows[0].panes.len(), 1);
    assert_eq!(filtered_nvim[0].windows[0].panes[0].command, "nvim");

    // Filter by pane cwd "/srv"
    let filtered_cwd = filter_sessions(&sessions, "srv");
    assert_eq!(filtered_cwd.len(), 1);
    assert_eq!(filtered_cwd[0].name, "other");
    assert_eq!(filtered_cwd[0].windows[0].name, "server");

    // Filter by session name "other"
    let filtered_sess = filter_sessions(&sessions, "other");
    assert_eq!(filtered_sess.len(), 1);
    assert_eq!(filtered_sess[0].name, "other");

    // Empty query returns everything
    let all = filter_sessions(&sessions, "");
    assert_eq!(all.len(), 2);
}
