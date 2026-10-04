//! Text extraction and tokenization pipeline for terminal navigation.

pub mod model;
pub mod scanner;

pub use model::{EntityKind, ExtractedToken};
pub use scanner::{scan_line, scan_lines};
