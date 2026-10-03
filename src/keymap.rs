//! Keymap domain definitions and unified command tree.
//!
//! This module is the single source of truth for all key bindings,
//! descriptions, and actions in `mox`.

/// The action to be executed when a leaf key in the keymap is pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// A raw tmux command string (e.g. `select-pane -L`).
    Tmux(String),
    /// Switch to a designated tmux key-table (e.g. "mox_r", "mox_w", "mox").
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
}

/// Builds the canonical default keymap for `moch`.
pub fn build_default_keymap() -> KeyNode {
    // -------------------------------------------------------------
    // 1. Sticky Resize Mode ('r' from root or 'w r')
    // -------------------------------------------------------------
    let resize_children = vec![
        KeyNode::new_action(
            "h",
            "resize left 2",
            KeyAction::Tmux("resize-pane -L 2".into()),
        ),
        KeyNode::new_action(
            "j",
            "resize down 2",
            KeyAction::Tmux("resize-pane -D 2".into()),
        ),
        KeyNode::new_action(
            "k",
            "resize up 2",
            KeyAction::Tmux("resize-pane -U 2".into()),
        ),
        KeyNode::new_action(
            "l",
            "resize right 2",
            KeyAction::Tmux("resize-pane -R 2".into()),
        ),
        KeyNode::new_action(
            "H",
            "resize left 10",
            KeyAction::Tmux("resize-pane -L 10".into()),
        ),
        KeyNode::new_action(
            "J",
            "resize down 10",
            KeyAction::Tmux("resize-pane -D 10".into()),
        ),
        KeyNode::new_action(
            "K",
            "resize up 10",
            KeyAction::Tmux("resize-pane -U 10".into()),
        ),
        KeyNode::new_action(
            "L",
            "resize right 10",
            KeyAction::Tmux("resize-pane -R 10".into()),
        ),
        KeyNode::new_action(
            "=",
            "equalize panes",
            KeyAction::Tmux("select-layout -E".into()),
        ),
        KeyNode::new_action(
            "+",
            "equalize panes",
            KeyAction::Tmux("select-layout -E".into()),
        ),
        KeyNode::new_action("z", "zoom toggle", KeyAction::Tmux("resize-pane -Z".into())),
        KeyNode::new_action("m", "zoom toggle", KeyAction::Tmux("resize-pane -Z".into())),
        KeyNode::new_action("q", "back to normal", KeyAction::SwitchTable("mox".into())),
        KeyNode::new_action(
            "Escape",
            "back to normal",
            KeyAction::SwitchTable("mox".into()),
        ),
        KeyNode::new_action(
            "Enter",
            "back to normal",
            KeyAction::SwitchTable("mox".into()),
        ),
        KeyNode::new_action("i", "exit to insert", KeyAction::ExitModal),
        KeyNode::new_action("M-m", "exit to insert", KeyAction::ExitModal),
        KeyNode::new_action("?", "resize help", KeyAction::WhichKey),
    ];

    // -------------------------------------------------------------
    // 2. Sticky Window Mode ('w')
    // -------------------------------------------------------------
    let mut window_children = vec![
        KeyNode::new_action(
            "h",
            "pane left/prev",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"select-pane -L\" \"select-window -t :-\""
                    .into(),
            ),
        ),
        KeyNode::new_action(
            "l",
            "pane right/next",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"select-pane -R\" \"select-window -t :+\""
                    .into(),
            ),
        ),
        KeyNode::new_action("j", "pane down", KeyAction::Tmux("select-pane -D".into())),
        KeyNode::new_action("k", "pane up", KeyAction::Tmux("select-pane -U".into())),
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
            "s",
            "choose window",
            KeyAction::Tmux("choose-tree -Zw".into()),
        ),
        KeyNode::new_action(
            "v",
            "split vertical",
            KeyAction::Tmux("split-window -h -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action("z", "zoom toggle", KeyAction::Tmux("resize-pane -Z".into())),
        KeyNode::new_action(
            "c",
            "new window",
            KeyAction::Tmux("new-window -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "d",
            "kill pane",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"kill-pane ; refresh-client -S\" \"display-message 'Cannot kill last pane (use D to kill window)'\""
                    .into(),
            ),
        ),
        KeyNode::new_action(
            "x",
            "kill pane",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"kill-pane ; refresh-client -S\" \"display-message 'Cannot kill last pane (use D to kill window)'\""
                    .into(),
            ),
        ),
        KeyNode::new_action(
            "D",
            "kill window",
            KeyAction::Confirm {
                prompt: "kill-window #W? (y/n)".into(),
                command: "kill-window".into(),
            },
        ),
        KeyNode::new_action(
            "X",
            "kill window",
            KeyAction::Confirm {
                prompt: "kill-window #W? (y/n)".into(),
                command: "kill-window".into(),
            },
        ),
        KeyNode::new_action(
            ",",
            "rename window",
            KeyAction::Tmux("command-prompt -I \"#W\" \"rename-window -- '%%'\"".into()),
        ),
        KeyNode::new_action("o", "last window", KeyAction::Tmux("last-window".into())),
        KeyNode::new_action(
            "t",
            "choose window tree",
            KeyAction::Tmux("choose-tree -Zw".into()),
        ),
        KeyNode::new_action("r", "resize mode", KeyAction::SwitchTable("mox_r".into())),
        KeyNode::new_action("q", "back to normal", KeyAction::SwitchTable("mox".into())),
        KeyNode::new_action(
            "Escape",
            "back to normal",
            KeyAction::SwitchTable("mox".into()),
        ),
        KeyNode::new_action("i", "exit to insert", KeyAction::ExitModal),
        KeyNode::new_action("M-m", "exit to insert", KeyAction::ExitModal),
        KeyNode::new_action("?", "window help", KeyAction::WhichKey),
    ];
    for i in 0..=9 {
        window_children.push(KeyNode::new_action(
            i.to_string(),
            format!("window {}", i),
            KeyAction::Tmux(format!("select-window -t :{}", i)),
        ));
    }

    // -------------------------------------------------------------
    // 3. Session Submenu ('S')
    // -------------------------------------------------------------
    let session_children = vec![
        KeyNode::new_action("c", "new session", KeyAction::Tmux("new-session".into())),
        KeyNode::new_action("n", "new session", KeyAction::Tmux("new-session".into())),
        KeyNode::new_action(
            ",",
            "rename session",
            KeyAction::Tmux("command-prompt -I \"#S\" \"rename-session '%%'\"".into()),
        ),
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
            "s",
            "choose session",
            KeyAction::Tmux("choose-tree -Zs".into()),
        ),
        KeyNode::new_action(
            "t",
            "choose session",
            KeyAction::Tmux("choose-tree -Zs".into()),
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
        KeyNode::new_action("q", "back to normal", KeyAction::SwitchTable("mox".into())),
        KeyNode::new_action(
            "Escape",
            "back to normal",
            KeyAction::SwitchTable("mox".into()),
        ),
        KeyNode::new_action("i", "exit to insert", KeyAction::ExitModal),
        KeyNode::new_action("M-m", "exit to insert", KeyAction::ExitModal),
    ];

    // -------------------------------------------------------------
    // 4. Go To Submenu ('g')
    // -------------------------------------------------------------
    let goto_children = vec![
        KeyNode::new_action(
            "w",
            "choose window",
            KeyAction::Tmux("choose-tree -Zw".into()),
        ),
        KeyNode::new_action(
            "s",
            "choose session",
            KeyAction::Tmux("choose-tree -Zs".into()),
        ),
        KeyNode::new_action("o", "last window", KeyAction::Tmux("last-window".into())),
        KeyNode::new_action(
            "h",
            "top-left pane",
            KeyAction::Tmux("select-pane -t :.top-left".into()),
        ),
        KeyNode::new_action(
            "l",
            "bottom-right pane",
            KeyAction::Tmux("select-pane -t :.bottom-right".into()),
        ),
        KeyNode::new_action("q", "back to normal", KeyAction::SwitchTable("mox".into())),
        KeyNode::new_action(
            "Escape",
            "back to normal",
            KeyAction::SwitchTable("mox".into()),
        ),
        KeyNode::new_action("i", "exit to insert", KeyAction::ExitModal),
        KeyNode::new_action("M-m", "exit to insert", KeyAction::ExitModal),
    ];

    // -------------------------------------------------------------
    // 5. Root Modal Keymap ('mox' / NORMAL mode)
    // -------------------------------------------------------------
    let mut root_children = vec![
        // Navigation: smart move - if multiple panes, switch pane; if 1 pane, switch window!
        KeyNode::new_action(
            "h",
            "pane left/prev",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"select-pane -L\" \"select-window -t :-\""
                    .into(),
            ),
        ),
        KeyNode::new_action(
            "l",
            "pane right/next",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"select-pane -R\" \"select-window -t :+\""
                    .into(),
            ),
        ),
        KeyNode::new_action("j", "pane down", KeyAction::Tmux("select-pane -D".into())),
        KeyNode::new_action("k", "pane up", KeyAction::Tmux("select-pane -U".into())),
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
        KeyNode::new_action("o", "last window", KeyAction::Tmux("last-window".into())),
        KeyNode::new_action("Tab", "last window", KeyAction::Tmux("last-window".into())),
        // Direct splits & layout
        KeyNode::new_action(
            "s",
            "split horizontal",
            KeyAction::Tmux("split-window -v -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "v",
            "split vertical",
            KeyAction::Tmux("split-window -h -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action(
            "c",
            "new window",
            KeyAction::Tmux("new-window -c \"#{pane_current_path}\"".into()),
        ),
        KeyNode::new_action("z", "zoom toggle", KeyAction::Tmux("resize-pane -Z".into())),
        KeyNode::new_action(
            "x",
            "kill pane",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"kill-pane ; refresh-client -S\" \"display-message 'Cannot kill last pane (use D to kill window)'\""
                    .into(),
            ),
        ),
        KeyNode::new_action(
            "d",
            "kill pane",
            KeyAction::Tmux(
                "if-shell -F \"#{>:#{window_panes},1}\" \"kill-pane ; refresh-client -S\" \"display-message 'Cannot kill last pane (use D to kill window)'\""
                    .into(),
            ),
        ),
        KeyNode::new_action(
            "D",
            "kill window",
            KeyAction::Confirm {
                prompt: "kill-window #W? (y/n)".into(),
                command: "kill-window".into(),
            },
        ),
        KeyNode::new_action(
            "X",
            "kill window",
            KeyAction::Confirm {
                prompt: "kill-window #W? (y/n)".into(),
                command: "kill-window".into(),
            },
        ),
        // Submodes
        KeyNode::new_sticky("r", "resize mode", resize_children),
        KeyNode::new_sticky("w", "window mode", window_children),
        KeyNode::new_branch("S", "session menu", session_children),
        KeyNode::new_branch("g", "go to...", goto_children),
        // Utilities & Scrolling
        KeyNode::new_action("[", "vim scrollback", KeyAction::CopyMode),
        KeyNode::new_action("T", "tea timer", KeyAction::TeaTimer),
        KeyNode::new_action("t", "tea timer", KeyAction::TeaTimer),
        KeyNode::new_action(
            ":",
            "command prompt",
            KeyAction::Tmux("command-prompt".into()),
        ),
        KeyNode::new_action("y", "paste buffer", KeyAction::Tmux("paste-buffer".into())),
        KeyNode::new_action("?", "which-key help", KeyAction::WhichKey),
        KeyNode::new_action("Space", "which-key help", KeyAction::WhichKey),
        // Exits to Insert Mode
        KeyNode::new_action("i", "insert mode", KeyAction::ExitModal),
        KeyNode::new_action("Escape", "insert mode", KeyAction::ExitModal),
        KeyNode::new_action("q", "insert mode", KeyAction::ExitModal),
        KeyNode::new_action("M-m", "insert mode", KeyAction::ExitModal),
    ];

    // Numbers 0..9 for direct window jumping in normal mode
    for i in 0..=9 {
        root_children.push(KeyNode::new_action(
            i.to_string(),
            format!("window {}", i),
            KeyAction::Tmux(format!("select-window -t :{}", i)),
        ));
    }

    KeyNode {
        key: String::new(),
        label: "NORMAL".into(),
        action: None,
        children: root_children,
        sticky: true,
    }
}
