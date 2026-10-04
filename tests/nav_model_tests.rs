use mox::nav::model::{TreeItem, flatten_tree, parse_tmux_snapshot, visible_tree};

#[test]
fn test_parse_tmux_snapshot() {
    let raw = "project\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/home/user/code\tmain.rs\t0\n\
               project\t$1\t1\t1\t@1\teditor\t1\t%2\t1\t0\tzsh\t/home/user/code\tzsh\t0\n\
               project\t$1\t1\t2\t@2\tagents\t0\t%3\t0\t1\tclaude\t/home/user/code\tclaude\t1\n\
               other\t$2\t0\t1\t@3\tserver\t1\t%4\t0\t1\tcargo\t/home/user/srv\tbuild\t0\n";

    let sessions = parse_tmux_snapshot(raw);
    assert_eq!(sessions.len(), 2);

    let s0 = &sessions[0];
    assert_eq!(s0.name, "project");
    assert_eq!(s0.id, "$1");
    assert!(s0.active);
    assert_eq!(s0.windows.len(), 2);

    let w0 = &s0.windows[0];
    assert_eq!(w0.name, "editor");
    assert_eq!(w0.id, "@1");
    assert_eq!(w0.index, 1);
    assert!(w0.active);
    assert!(!w0.is_agent);
    assert_eq!(w0.panes.len(), 2);

    let p0 = &w0.panes[0];
    assert_eq!(p0.id, "%1");
    assert_eq!(p0.command, "nvim");
    assert_eq!(p0.cwd, "/home/user/code");
    assert_eq!(p0.title, "main.rs");
    assert!(p0.active);

    let p1 = &w0.panes[1];
    assert_eq!(p1.id, "%2");
    assert_eq!(p1.command, "zsh");
    assert!(!p1.active);

    let w1 = &s0.windows[1];
    assert_eq!(w1.name, "agents");
    assert!(w1.is_agent);
    assert_eq!(w1.panes.len(), 1);
    assert_eq!(w1.panes[0].command, "claude");

    let s1 = &sessions[1];
    assert_eq!(s1.name, "other");
    assert!(!s1.active);
    assert_eq!(s1.windows.len(), 1);
}

#[test]
fn visible_search_tree_borrows_matching_rows_and_keeps_ancestors() {
    let raw = "work\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/code\tCargo.toml\t0\n\
               work\t$1\t1\t1\t@1\teditor\t1\t%2\t1\t0\tzsh\t/home\tzsh\t0\n";
    let mut sessions = parse_tmux_snapshot(raw);
    sessions[0].collapsed = true;
    sessions[0].windows[0].collapsed = true;

    let visible = visible_tree(&sessions, "nvim");
    assert_eq!(visible.len(), 3);
    assert!(matches!(visible[0], TreeItem::Session(_)));
    assert!(matches!(visible[1], TreeItem::Window { .. }));
    assert!(matches!(visible[2], TreeItem::Pane { pane, .. } if pane.id == "%1"));
    assert!(
        matches!(visible[0], TreeItem::Session(session) if std::ptr::eq(session, &sessions[0]))
    );

    let unfiltered = visible_tree(&sessions, "");
    assert_eq!(unfiltered.len(), 1);
    assert!(matches!(unfiltered[0], TreeItem::Session(_)));
}

#[test]
fn test_flatten_tree_expansion_and_collapse() {
    let raw = "project\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/code\tedit\t0\n\
               project\t$1\t1\t1\t@1\teditor\t1\t%2\t1\t0\tzsh\t/code\tzsh\t0\n";
    let mut sessions = parse_tmux_snapshot(raw);

    // Default: both session and window are expanded
    let items = flatten_tree(&sessions);
    // 1 session + 1 window + 2 panes = 4 items
    assert_eq!(items.len(), 4);
    assert!(matches!(items[0], TreeItem::Session(_)));
    assert!(matches!(items[1], TreeItem::Window { .. }));
    assert!(matches!(items[2], TreeItem::Pane { .. }));
    assert!(matches!(items[3], TreeItem::Pane { .. }));

    // Collapse window
    sessions[0].windows[0].collapsed = true;
    let items_collapsed_win = flatten_tree(&sessions);
    // 1 session + 1 window = 2 items
    assert_eq!(items_collapsed_win.len(), 2);

    // Collapse session
    sessions[0].collapsed = true;
    let items_collapsed_sess = flatten_tree(&sessions);
    // 1 session = 1 item
    assert_eq!(items_collapsed_sess.len(), 1);
}
