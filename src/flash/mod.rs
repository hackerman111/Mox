//! Flash jump and motion labeling engine.
//!
//! Provides sub-millisecond pattern matching and home-row prioritized
//! label assignment for bidirectional cursor navigation and remote text operations.

pub mod labeler;
pub mod matcher;
pub mod overlay;
pub mod text_object;
pub mod ui;

pub use labeler::{ALPHABET, LabeledTarget, MatchTarget, assign_labels, generate_labels};
pub use matcher::{find_char_motion_matches, find_matches, find_token_matches};
pub use overlay::{FlashConfig, FlashMode, build_flash_command, launch_flash_popup};
pub use text_object::{TextObject, resolve_text_object, resolve_text_object_span};
pub use ui::run_flash_overlay;
