//! Tmux CLI interaction and configuration generation.

pub mod client;
pub mod config;
pub mod scroll;

pub use client::load_buffer;
pub use client::preview_capture;
pub use client::{apply_tmux_commands, execute_tmux, execute_tmux_raw};
pub use client::{capture_to_file, popup, shell_quote};
pub use client::{launch_after_popup, quote};
pub use config::{clean_status_left, generate_init_script};
pub use scroll::generate_scroll_config;
