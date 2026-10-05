//! Application management, launching, and window tracking.

pub mod model;
pub mod runner;

pub use runner::{check_binary, find_existing_pane, run_app};

/// Open or focus LazyGit in the current tmux session (facade delegating to `run_app`).
pub fn open_lazygit(socket: Option<&str>, cwd: Option<&str>, window: bool) -> Result<(), String> {
    let app = model::AppConfig {
        name: "lazygit".to_string(),
        title: "LazyGit".to_string(),
        command: "lazygit".to_string(),
        key: Some("G".to_string()),
        mode: model::AppLaunchMode::Popup {
            width: "85%".to_string(),
            height: "85%".to_string(),
        },
        focus_existing: true,
        check_binary: true,
    };
    run_app(socket, &app, cwd, window)
}
