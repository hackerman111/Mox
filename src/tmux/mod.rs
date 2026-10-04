//! Tmux CLI interaction and configuration generation.

pub mod client;
pub mod config;
pub mod scroll;

pub use client::{apply_tmux_commands, execute_tmux, execute_tmux_raw};
pub use config::{clean_status_left, generate_init_script};
pub use scroll::generate_scroll_config;
