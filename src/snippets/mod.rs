//! Favorite terminal commands cheatsheet and parameter template engine.

pub mod model;
pub mod runner;
pub mod storage;
pub mod template;
pub mod ui;

pub use model::Snippet;
pub use runner::run_snippets_panel;
pub use storage::{default_snippets, default_snippets_path, load_snippets, save_snippets};
pub use template::{TemplateParam, extract_parameters, substitute_parameters};
