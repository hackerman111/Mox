//! Vim-like fuzzy tree navigator for tmux sessions, windows, and panes.

pub mod fuzzy;
pub mod model;
pub mod ui;

pub use ui::run_navigator;


