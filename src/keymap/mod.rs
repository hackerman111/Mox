//! Keymap domain definitions and unified command tree.
//!
//! This module is the single source of truth for all key bindings,
//! descriptions, and actions in `mox`.

pub mod defaults;
pub mod types;

pub use defaults::build_default_keymap;
pub use types::{KeyAction, KeyNode};
