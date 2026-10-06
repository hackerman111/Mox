//! Favorite terminal commands cheatsheet and parameter template engine.

pub mod model;
pub mod template;

pub use model::Snippet;
pub use template::{TemplateParam, extract_parameters, substitute_parameters};
