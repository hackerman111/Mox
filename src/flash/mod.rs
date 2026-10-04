//! Flash jump and motion labeling engine.
//!
//! Provides sub-millisecond pattern matching and home-row prioritized
//! label assignment for bidirectional cursor navigation and remote text operations.

pub mod labeler;
pub mod matcher;
pub mod text_object;

pub use labeler::{ALPHABET, LabeledTarget, MatchTarget, assign_labels, generate_labels};
pub use matcher::{find_char_motion_matches, find_matches, find_token_matches};
pub use text_object::{TextObject, resolve_text_object, resolve_text_object_span};
