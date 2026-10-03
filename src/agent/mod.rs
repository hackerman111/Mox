//! AI Agent window tracking and fast-toggle management.

pub mod manager;
pub mod model;

pub use manager::{create_agent_window, toggle_agent_window};
pub use model::{WindowSummary, calculate_next_agent_name, parse_window_list_for_agents};
