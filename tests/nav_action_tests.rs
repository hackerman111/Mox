use mox::nav::model::{PaneInfo, SessionInfo, TreeItem, WindowInfo};
use mox::nav::ui::{format_kill_command, format_rename_command, format_switch_command};

#[test]
fn test_switch_command_generation() {
    let session = SessionInfo {
        id: "$1".into(),
        name: "main".into(),
        active: true,
        windows: Vec::new(),
        collapsed: false,
    };
    let window = WindowInfo {
        id: "@1".into(),
        index: 1,
        name: "code".into(),
        active: true,
        is_agent: false,
        panes: Vec::new(),
        collapsed: false,
    };
    let pane = PaneInfo {
        id: "%1".into(),
        index: 0,
        active: true,
        command: "nvim".into(),
        cwd: "/code".into(),
        title: "title".into(),
    };

    let item_sess = TreeItem::Session(&session);
    let item_win = TreeItem::Window {
        session: &session,
        window: &window,
    };
    let item_pane = TreeItem::Pane {
        session: &session,
        window: &window,
        pane: &pane,
    };

    assert_eq!(format_switch_command(&item_sess), "switch-client -t $1");
    assert_eq!(format_switch_command(&item_win), "select-window -t @1");
    assert_eq!(
        format_switch_command(&item_pane),
        "select-pane -t %1 ; select-window -t @1"
    );
}

#[test]
fn test_kill_and_rename_command_generation() {
    let session = SessionInfo {
        id: "$1".into(),
        name: "main".into(),
        active: true,
        windows: Vec::new(),
        collapsed: false,
    };
    let window = WindowInfo {
        id: "@2".into(),
        index: 2,
        name: "term".into(),
        active: false,
        is_agent: false,
        panes: Vec::new(),
        collapsed: false,
    };
    let pane = PaneInfo {
        id: "%3".into(),
        index: 0,
        active: true,
        command: "zsh".into(),
        cwd: "/home".into(),
        title: "".into(),
    };

    let item_sess = TreeItem::Session(&session);
    let item_win = TreeItem::Window {
        session: &session,
        window: &window,
    };
    let item_pane = TreeItem::Pane {
        session: &session,
        window: &window,
        pane: &pane,
    };

    assert_eq!(format_kill_command(&item_sess), "kill-session -t $1");
    assert_eq!(format_kill_command(&item_win), "kill-window -t @2");
    assert_eq!(format_kill_command(&item_pane), "kill-pane -t %3");

    assert_eq!(
        format_rename_command(&item_sess, "dev").as_deref(),
        Some("rename-session -t $1 'dev'")
    );
    assert_eq!(
        format_rename_command(&item_win, "work").as_deref(),
        Some("rename-window -t @2 'work'")
    );
    assert_eq!(
        format_rename_command(&item_pane, "name").as_deref(),
        Some("select-pane -t %3 -T 'name'")
    );
    assert_eq!(
        format_rename_command(&item_pane, "it's mine").as_deref(),
        Some("select-pane -t %3 -T 'it'\\''s mine'")
    );
}
