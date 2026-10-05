//! Domain models for application management, launching, and window tracking.

use serde::{Deserialize, Serialize};

/// Mode for launching an application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppLaunchMode {
    Popup {
        #[serde(default = "default_width")]
        width: String,
        #[serde(default = "default_height")]
        height: String,
    },
    Window,
}

impl Default for AppLaunchMode {
    fn default() -> Self {
        Self::Popup {
            width: default_width(),
            height: default_height(),
        }
    }
}

fn default_width() -> String {
    "85%".to_string()
}

fn default_height() -> String {
    "85%".to_string()
}

fn default_true() -> bool {
    true
}

/// Configuration entry for an application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub title: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default)]
    pub mode: AppLaunchMode,
    #[serde(default = "default_true")]
    pub focus_existing: bool,
    #[serde(default = "default_true")]
    pub check_binary: bool,
}

/// Model representing a tracked tmux window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedWindow {
    pub id: String,
    pub name: String,
    pub session_id: String,
    pub active: bool,
}

/// Returns the built-in default applications (LazyGit and btop).
pub fn default_apps() -> Vec<AppConfig> {
    vec![
        AppConfig {
            name: "lazygit".to_string(),
            title: "LazyGit".to_string(),
            command: "lazygit".to_string(),
            key: Some("G".to_string()),
            mode: AppLaunchMode::Popup {
                width: "85%".to_string(),
                height: "85%".to_string(),
            },
            focus_existing: true,
            check_binary: true,
        },
        AppConfig {
            name: "btop".to_string(),
            title: "btop".to_string(),
            command: "btop".to_string(),
            key: Some("B".to_string()),
            mode: AppLaunchMode::Popup {
                width: "85%".to_string(),
                height: "85%".to_string(),
            },
            focus_existing: true,
            check_binary: true,
        },
    ]
}
