//! Unified Vim-like fuzzy tree navigator for tmux.

pub mod action;
pub mod fuzzy;
pub mod model;
pub mod runner;
pub mod state;
pub mod ui;
pub mod view;

pub use action::{
    format_kill_command, format_rename_command, format_switch_command, switch_to_item,
};
pub use runner::run_navigator;
