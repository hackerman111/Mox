//! Keymap domain types and node structure.

/// The action to be executed when a leaf key in the keymap is pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    Feature(crate::actions::FeatureAction),
    /// A raw tmux command string (e.g. `select-pane -L`).
    Tmux(String),
    /// Switch to a designated tmux key-table (e.g. "mox_r", "mox_w", "mox").
    SwitchTable(String),
    /// Dangerous operation requiring user confirmation before execution.
    Confirm {
        prompt: String,
        command: String,
    },
    /// Enter tmux copy/scroll mode with Vim navigation.
    CopyMode,
    /// Interactive Flash overlay, also dispatched from which-key menus.
    Flash(crate::flash::FlashConfig),
    /// Categorized token picker.
    Extract,
    /// Launch the which-key help popup.
    WhichKey,
    /// Launch the built-in tea timer.
    TeaTimer,
    /// Launch the unified Vim-like fuzzy tree navigator.
    Navigator,
    /// Toggle between work window and AI agent window.
    AgentToggle,
    /// Create a new AI agent window.
    AgentCreate,
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

/// Trait for types that can match a key string (e.g. `&str`, `char`, `String`).
pub trait KeyMatcher {
    /// Returns true if this matcher matches `key`.
    fn matches_key(&self, key: &str) -> bool;
}

impl KeyMatcher for &str {
    #[inline]
    fn matches_key(&self, key: &str) -> bool {
        *self == key
    }
}

impl KeyMatcher for &String {
    #[inline]
    fn matches_key(&self, key: &str) -> bool {
        self.as_str() == key
    }
}

impl KeyMatcher for String {
    #[inline]
    fn matches_key(&self, key: &str) -> bool {
        self.as_str() == key
    }
}

impl KeyMatcher for char {
    #[inline]
    fn matches_key(&self, key: &str) -> bool {
        key.len() == self.len_utf8() && key.starts_with(*self)
    }
}

impl KeyMatcher for &char {
    #[inline]
    fn matches_key(&self, key: &str) -> bool {
        key.len() == self.len_utf8() && key.starts_with(**self)
    }
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

    /// Look up an immediate child by key string or character.
    pub fn find_child<K: KeyMatcher>(&self, key: K) -> Option<&KeyNode> {
        self.children.iter().find(|c| key.matches_key(&c.key))
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
