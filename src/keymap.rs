//! Keymap domain definitions and unified command tree.
//!
//! This module is the single source of truth for all key bindings,
//! descriptions, and actions in `moch`.

/// The action to be executed when a leaf key in the keymap is pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// A raw tmux command string (e.g. `select-pane -L`).
    Tmux(String),
    /// Switch to a designated tmux key-table.
    SwitchTable(String),
    /// Dangerous operation requiring user confirmation before execution.
    Confirm { prompt: String, command: String },
    /// Enter tmux copy/scroll mode with Vim navigation.
    CopyMode,
    /// Launch the which-key help popup.
    WhichKey,
    /// Launch the built-in tea timer.
    TeaTimer,
    /// Exit modal mode back to normal tmux root input.
    ExitModal,
}

/// A node in the hierarchical keymap tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyNode {
    /// The key trigger (e.g. "h", "w", "M-m", "Escape").
    pub key: String,
    /// Short human-readable label shown in which-key menus.
    pub label: String,
    /// The action executed when this node is triggered, or `None` if it is an intermediate branch.
    pub action: Option<KeyAction>,
    /// Child keys accessible under this prefix.
    pub children: Vec<KeyNode>,
    /// Whether this node represents a sticky mode that remains active after actions.
    pub sticky: bool,
}

impl KeyNode {
    /// Create a leaf node with an action.
    pub fn new_action(key: impl Into<String>, label: impl Into<String>, action: KeyAction) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            action: Some(action),
            children: Vec::new(),
            sticky: false,
        }
    }

    /// Create an intermediate branch node with children.
    pub fn new_branch(
        key: impl Into<String>,
        label: impl Into<String>,
        children: Vec<KeyNode>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            action: None,
            children,
            sticky: false,
        }
    }

    /// Create a sticky branch node that remains active after executions.
    pub fn new_sticky(
        key: impl Into<String>,
        label: impl Into<String>,
        children: Vec<KeyNode>,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            action: None,
            children,
            sticky: true,
        }
    }

    /// Look up an immediate child by key string.
    pub fn find_child(&self, key: &str) -> Option<&KeyNode> {
        self.children.iter().find(|c| c.key == key)
    }

    /// Recursively resolve a path of keys (e.g. `["w", "s"]`).
    pub fn resolve_path(&self, path: &[&str]) -> Option<&KeyNode> {
        let mut current = self;
        for &step in path {
            if step.is_empty() {
                continue;
            }
            current = current.find_child(step)?;
        }
        Some(current)
    }

    /// Collect all flattened sequences in this subtree as `(Vec<key>, &KeyNode)`.
    pub fn collect_all(&self) -> Vec<(Vec<String>, &KeyNode)> {
        let mut results = Vec::new();
        let mut current_path = Vec::new();
        self.collect_internal(&mut current_path, &mut results);
        results
    }

    fn collect_internal<'a>(
        &'a self,
        current_path: &mut Vec<String>,
        results: &mut Vec<(Vec<String>, &'a KeyNode)>,
    ) {
        if !self.key.is_empty() {
            current_path.push(self.key.clone());
            results.push((current_path.clone(), self));
        }
        for child in &self.children {
            child.collect_internal(current_path, results);
        }
        if !self.key.is_empty() {
            current_path.pop();
        }
    }
}

/// Builds the canonical default keymap for `moch`.
///
/// Guarantees full functional parity with `tmux-modal` default bindings,
/// with the addition of the built-in tea timer (`T`) and help/which-key (`?`).
pub fn build_default_keymap() -> KeyNode {
    // Window split submenu ('w s' or 's')
    let split_children = vec![
        KeyNode::new_action(
            "j",
            "split horizontal",
            KeyAction::Tmux("split-window -v -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "s",
            "split horizontal",
            KeyAction::Tmux("split-window -v -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "l",
            "split vertical",
            KeyAction::Tmux("split-window -h -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "v",
            "split vertical",
            KeyAction::Tmux("split-window -h -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "f",
            "full split horiz",
            KeyAction::Tmux("split-window -f -v -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "F",
            "full split vert",
            KeyAction::Tmux("split-window -f -h -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch_window".into())),
        KeyNode::new_action(
            "Escape",
            "back",
            KeyAction::SwitchTable("moch_window".into()),
        ),
    ];

    // Window arrange layouts ('w a')
    let arrange_children = vec![
        KeyNode::new_action(
            "1",
            "even horizontal",
            KeyAction::Tmux("select-layout even-horizontal".into()),
        ),
        KeyNode::new_action(
            "2",
            "even vertical",
            KeyAction::Tmux("select-layout even-vertical".into()),
        ),
        KeyNode::new_action(
            "3",
            "main horizontal",
            KeyAction::Tmux("select-layout main-horizontal".into()),
        ),
        KeyNode::new_action(
            "4",
            "main vertical",
            KeyAction::Tmux("select-layout main-vertical".into()),
        ),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch_window".into())),
        KeyNode::new_action(
            "Escape",
            "back",
            KeyAction::SwitchTable("moch_window".into()),
        ),
    ];

    // Window move pane ('w m')
    let move_children = vec![
        KeyNode::new_action(
            "j",
            "swap pane down",
            KeyAction::Tmux("swap-pane -D".into()),
        ),
        KeyNode::new_action("k", "swap pane up", KeyAction::Tmux("swap-pane -U".into())),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch_window".into())),
        KeyNode::new_action(
            "Escape",
            "back",
            KeyAction::SwitchTable("moch_window".into()),
        ),
    ];

    // Sticky Window resize mode ('w r')
    let resize_children = vec![
        KeyNode::new_action(
            "h",
            "resize left 1",
            KeyAction::Tmux("resize-pane -L 1".into()),
        ),
        KeyNode::new_action(
            "j",
            "resize down 1",
            KeyAction::Tmux("resize-pane -D 1".into()),
        ),
        KeyNode::new_action(
            "k",
            "resize up 1",
            KeyAction::Tmux("resize-pane -U 1".into()),
        ),
        KeyNode::new_action(
            "l",
            "resize right 1",
            KeyAction::Tmux("resize-pane -R 1".into()),
        ),
        KeyNode::new_action(
            "H",
            "resize left 5",
            KeyAction::Tmux("resize-pane -L 5".into()),
        ),
        KeyNode::new_action(
            "J",
            "resize down 5",
            KeyAction::Tmux("resize-pane -D 5".into()),
        ),
        KeyNode::new_action(
            "K",
            "resize up 5",
            KeyAction::Tmux("resize-pane -U 5".into()),
        ),
        KeyNode::new_action(
            "L",
            "resize right 5",
            KeyAction::Tmux("resize-pane -R 5".into()),
        ),
        KeyNode::new_action(
            "q",
            "back to window",
            KeyAction::SwitchTable("moch_window".into()),
        ),
        KeyNode::new_action(
            "Escape",
            "back to window",
            KeyAction::SwitchTable("moch_window".into()),
        ),
        KeyNode::new_action("M-m", "exit modal", KeyAction::ExitModal),
    ];

    // Sticky Window / Pane mode ('w')
    let mut window_children = vec![
        KeyNode::new_action("h", "pane left", KeyAction::Tmux("select-pane -L".into())),
        KeyNode::new_action("j", "pane down", KeyAction::Tmux("select-pane -D".into())),
        KeyNode::new_action("k", "pane up", KeyAction::Tmux("select-pane -U".into())),
        KeyNode::new_action("l", "pane right", KeyAction::Tmux("select-pane -R".into())),
        KeyNode::new_action(
            "H",
            "prev window",
            KeyAction::Tmux("select-window -t :-".into()),
        ),
        KeyNode::new_action(
            "L",
            "next window",
            KeyAction::Tmux("select-window -t :+".into()),
        ),
        KeyNode::new_action(
            "c",
            "new window",
            KeyAction::Tmux("new-window -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "d",
            "kill pane",
            KeyAction::Confirm {
                prompt: "kill-pane #P? (y/n)".into(),
                command: "kill-pane".into(),
            },
        ),
        KeyNode::new_action(
            "D",
            "kill window",
            KeyAction::Confirm {
                prompt: "kill-window #W? (y/n)".into(),
                command: "kill-window".into(),
            },
        ),
        KeyNode::new_action("b", "break pane", KeyAction::Tmux("break-pane".into())),
        KeyNode::new_action(
            "n",
            "display panes",
            KeyAction::Tmux("display-panes".into()),
        ),
        KeyNode::new_action("o", "last window", KeyAction::Tmux("last-window".into())),
        KeyNode::new_action("z", "zoom pane", KeyAction::Tmux("resize-pane -Z".into())),
        KeyNode::new_action(
            ",",
            "rename window",
            KeyAction::Tmux("command-prompt -I \"#W\" \"rename-window -- '%%'\"".into()),
        ),
        KeyNode::new_action(
            "i",
            "select index",
            KeyAction::Tmux("command-prompt -p index \"select-window -t ':%%'\"".into()),
        ),
        KeyNode::new_action(
            "t",
            "choose window tree",
            KeyAction::Tmux("choose-tree -Zw".into()),
        ),
        KeyNode::new_branch("s", "split...", split_children),
        KeyNode::new_branch("a", "arrange...", arrange_children),
        KeyNode::new_branch("m", "move pane...", move_children),
        KeyNode::new_sticky("r", "resize mode...", resize_children),
        KeyNode::new_action("q", "back to modal", KeyAction::SwitchTable("moch".into())),
        KeyNode::new_action(
            "Escape",
            "back to modal",
            KeyAction::SwitchTable("moch".into()),
        ),
        KeyNode::new_action("M-m", "exit modal", KeyAction::ExitModal),
    ];

    // Numbers 0..9 for selecting windows in window mode
    for i in 0..=9 {
        window_children.push(KeyNode::new_action(
            i.to_string(),
            format!("window {}", i),
            KeyAction::Tmux(format!("select-window -t :{}", i)),
        ));
    }

    // Goto window children ('g w')
    let mut goto_window_children = vec![
        KeyNode::new_action(
            "h",
            "prev window",
            KeyAction::Tmux("select-window -t :-".into()),
        ),
        KeyNode::new_action(
            "l",
            "next window",
            KeyAction::Tmux("select-window -t :+".into()),
        ),
        KeyNode::new_action(
            "i",
            "select index",
            KeyAction::Tmux("command-prompt -p index \"select-window -t ':%%'\"".into()),
        ),
        KeyNode::new_action("o", "last window", KeyAction::Tmux("last-window".into())),
        KeyNode::new_action(
            "t",
            "choose window",
            KeyAction::Tmux("choose-tree -Zw".into()),
        ),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch_goto".into())),
        KeyNode::new_action("Escape", "back", KeyAction::SwitchTable("moch_goto".into())),
    ];
    for i in 0..=9 {
        goto_window_children.push(KeyNode::new_action(
            i.to_string(),
            format!("window {}", i),
            KeyAction::Tmux(format!("select-window -t :{}", i)),
        ));
    }

    // Goto session children ('g s')
    let goto_session_children = vec![
        KeyNode::new_action(
            "h",
            "prev session",
            KeyAction::Tmux("switch-client -p".into()),
        ),
        KeyNode::new_action(
            "l",
            "next session",
            KeyAction::Tmux("switch-client -n".into()),
        ),
        KeyNode::new_action(
            "t",
            "choose session",
            KeyAction::Tmux("choose-tree -Zs".into()),
        ),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch_goto".into())),
        KeyNode::new_action("Escape", "back", KeyAction::SwitchTable("moch_goto".into())),
    ];

    // Goto submenu ('g')
    let goto_children = vec![
        KeyNode::new_branch("w", "window...", goto_window_children),
        KeyNode::new_branch("s", "session...", goto_session_children),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch".into())),
        KeyNode::new_action("Escape", "back", KeyAction::SwitchTable("moch".into())),
    ];

    // Session submenu ('s')
    let session_children = vec![
        KeyNode::new_action(
            ",",
            "rename session",
            KeyAction::Tmux("command-prompt -I \"#S\" \"rename-session '%%'\"".into()),
        ),
        KeyNode::new_action("c", "new session", KeyAction::Tmux("new-session".into())),
        KeyNode::new_action("n", "new session", KeyAction::Tmux("new-session".into())),
        KeyNode::new_action(
            "D",
            "kill session",
            KeyAction::Confirm {
                prompt: "kill-session #S? (y/n)".into(),
                command: "kill-session".into(),
            },
        ),
        KeyNode::new_action(
            "d",
            "detach client",
            KeyAction::Tmux("detach-client".into()),
        ),
        KeyNode::new_action(
            "h",
            "prev session",
            KeyAction::Tmux("switch-client -p".into()),
        ),
        KeyNode::new_action(
            "l",
            "next session",
            KeyAction::Tmux("switch-client -n".into()),
        ),
        KeyNode::new_action(
            "t",
            "choose session",
            KeyAction::Tmux("choose-tree -Zs".into()),
        ),
        KeyNode::new_action("q", "back", KeyAction::SwitchTable("moch".into())),
        KeyNode::new_action("Escape", "back", KeyAction::SwitchTable("moch".into())),
    ];

    // Root modal keys
    let root_children = vec![
        // Navigation
        KeyNode::new_action("h", "pane left", KeyAction::Tmux("select-pane -L".into())),
        KeyNode::new_action("j", "pane down", KeyAction::Tmux("select-pane -D".into())),
        KeyNode::new_action("k", "pane up", KeyAction::Tmux("select-pane -U".into())),
        KeyNode::new_action("l", "pane right", KeyAction::Tmux("select-pane -R".into())),
        // Top-level modal submenus and sticky modes
        KeyNode::new_sticky("w", "window mode...", window_children),
        KeyNode::new_branch("s", "session...", session_children),
        KeyNode::new_branch("g", "go to...", goto_children),
        // Direct modal actions
        KeyNode::new_action(
            ":",
            "command prompt",
            KeyAction::Tmux("command-prompt".into()),
        ),
        KeyNode::new_action("c", "copy mode (scroll)", KeyAction::CopyMode),
        KeyNode::new_action("y", "paste buffer", KeyAction::Tmux("paste-buffer".into())),
        KeyNode::new_action("T", "tea timer", KeyAction::TeaTimer),
        KeyNode::new_action("?", "which-key help", KeyAction::WhichKey),
        KeyNode::new_action("i", "insert mode (exit)", KeyAction::ExitModal),
        KeyNode::new_action("q", "exit modal", KeyAction::ExitModal),
        KeyNode::new_action("Escape", "exit modal", KeyAction::ExitModal),
        KeyNode::new_action("M-m", "exit modal", KeyAction::ExitModal),
    ];

    KeyNode {
        key: String::new(),
        label: "modal".into(),
        action: None,
        children: root_children,
        sticky: false,
    }
}
