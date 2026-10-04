//! Text extraction and tokenization pipeline for terminal navigation.

pub mod model;
pub mod scanner;
pub mod ui;

pub use model::{EntityKind, ExtractedToken};
pub use scanner::{scan_line, scan_lines};
pub use ui::{ExtractState, PickerMode, launch_extract_popup, run_extract_picker};
