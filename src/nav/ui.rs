//! Interactive dual-pane TUI tree navigator and live preview for tmux.
//!
//! Submodule re-exports for backward compatibility.

pub use super::action::{
    execute_nav_tmux, format_kill_command, format_rename_command, format_switch_command,
    switch_to_item,
};
pub use super::runner::run_navigator;
pub use super::state::NavigatorApp;
pub use super::view::render_navigator;
