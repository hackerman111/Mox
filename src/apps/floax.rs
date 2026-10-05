//! Native Floax persistent scratchpad terminal session management and popup toggle.

use crate::tmux::execute_tmux;

pub const DEFAULT_FLOAX_SESSION_NAME: &str = "mox-scratch";
pub const DEFAULT_FLOAX_TITLE: &str = " FloaX Scratchpad ";
pub const DEFAULT_FLOAX_WIDTH: &str = "85%";
pub const DEFAULT_FLOAX_HEIGHT: &str = "85%";

pub const DEFAULT_SESSION_NAME: &str = DEFAULT_FLOAX_SESSION_NAME;
pub const DEFAULT_TITLE: &str = DEFAULT_FLOAX_TITLE;
pub const DEFAULT_WIDTH: &str = DEFAULT_FLOAX_WIDTH;
pub const DEFAULT_HEIGHT: &str = DEFAULT_FLOAX_HEIGHT;

/// Configuration for the Floax persistent scratchpad terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloaxConfig {
    pub session_name: String,
    pub title: String,
    pub width: String,
    pub height: String,
    pub change_path: bool,
}

impl Default for FloaxConfig {
    fn default() -> Self {
        Self {
            session_name: DEFAULT_FLOAX_SESSION_NAME.to_string(),
            title: DEFAULT_FLOAX_TITLE.to_string(),
            width: DEFAULT_FLOAX_WIDTH.to_string(),
            height: DEFAULT_FLOAX_HEIGHT.to_string(),
            change_path: false,
        }
    }
}

pub type FloaxSession = FloaxConfig;

impl FloaxConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session_name(mut self, name: impl Into<String>) -> Self {
        self.session_name = name.into();
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_width(mut self, width: impl Into<String>) -> Self {
        self.width = width.into();
        self
    }

    pub fn with_height(mut self, height: impl Into<String>) -> Self {
        self.height = height.into();
        self
    }

    pub fn with_change_path(mut self, change_path: bool) -> Self {
        self.change_path = change_path;
        self
    }

    /// Build the command string executed inside the popup to attach to this floax session.
    pub fn build_attach_command(&self, socket: Option<&str>) -> String {
        if let Some(socket) = socket {
            format!(
                "tmux -L {} attach-session -t {}",
                crate::tmux::shell_quote(socket),
                self.session_name
            )
        } else {
            format!("tmux attach-session -t {}", self.session_name)
        }
    }

    /// Build the full arguments slice for tmux `display-popup`.
    pub fn build_popup_args(&self, socket: Option<&str>) -> Vec<String> {
        vec![
            "display-popup".to_string(),
            "-E".to_string(),
            "-w".to_string(),
            self.width.clone(),
            "-h".to_string(),
            self.height.clone(),
            "-b".to_string(),
            "rounded".to_string(),
            "-T".to_string(),
            self.title.clone(),
            self.build_attach_command(socket),
        ]
    }
}

/// Check if the floax tmux session currently exists.
pub fn has_floax_session(socket: Option<&str>, session_name: &str) -> bool {
    execute_tmux(socket, &["has-session", "-t", session_name]).is_ok()
}

/// Ensure that the floax tmux session exists, spawning it in the background if necessary.
pub fn ensure_floax_session(
    socket: Option<&str>,
    config: &FloaxConfig,
    cwd: Option<&str>,
) -> Result<(), String> {
    if has_floax_session(socket, &config.session_name) {
        return Ok(());
    }

    let target_cwd = match cwd {
        Some(path) => path.to_string(),
        None => execute_tmux(socket, &["display-message", "-p", "#{pane_current_path}"])?,
    };

    execute_tmux(
        socket,
        &[
            "new-session",
            "-d",
            "-s",
            &config.session_name,
            "-c",
            &target_cwd,
        ],
    )?;
    execute_tmux(
        socket,
        &["set-option", "-t", &config.session_name, "status", "off"],
    )?;
    execute_tmux(
        socket,
        &[
            "set-option",
            "-t",
            &config.session_name,
            "detach-on-destroy",
            "on",
        ],
    )?;

    Ok(())
}

/// Check if any tmux client is currently attached to the floax session.
pub fn is_floax_attached(socket: Option<&str>, session_name: &str) -> bool {
    let clients = execute_tmux(
        socket,
        &["list-clients", "-t", session_name, "-F", "#{client_name}"],
    )
    .unwrap_or_default();
    !clients.trim().is_empty()
}

/// Check if the currently active tmux session is the floax session.
pub fn is_inside_floax_session(socket: Option<&str>, session_name: &str) -> bool {
    execute_tmux(socket, &["display-message", "-p", "#{session_name}"])
        .map(|s| s.trim() == session_name)
        .unwrap_or(false)
}

/// Toggle the floax scratchpad terminal.
///
/// 1. If currently inside the floax session OR if any client is attached to the floax session,
///    detaches the client and dismisses the popup so the background session remains alive.
/// 2. Ensures the floax session exists.
/// 3. If `change_path` is enabled, synchronizes the floax session cwd with target cwd.
/// 4. Opens the scratchpad in a rounded display popup attached to the session.
pub fn toggle_floax(
    socket: Option<&str>,
    config: &FloaxConfig,
    cwd: Option<&str>,
) -> Result<(), String> {
    // 1. Check if we are currently inside the floax session or if the popup is open:
    if is_inside_floax_session(socket, &config.session_name)
        || is_floax_attached(socket, &config.session_name)
    {
        let _ = execute_tmux(socket, &["set-option", "key-table", "root"]);
        let _ = execute_tmux(socket, &["refresh-client", "-S"]);
        let _ = execute_tmux(socket, &["detach-client", "-s", &config.session_name]);
        let _ = execute_tmux(socket, &["display-popup", "-C"]);
        return Ok(());
    }

    // 2. Ensure floax session exists:
    ensure_floax_session(socket, config, cwd)?;

    // 3. If config.change_path:
    if config.change_path {
        let target_cwd = match cwd {
            Some(path) => path.to_string(),
            None => execute_tmux(socket, &["display-message", "-p", "#{pane_current_path}"])?,
        };
        let scratch_path = execute_tmux(
            socket,
            &[
                "display-message",
                "-t",
                &config.session_name,
                "-p",
                "#{pane_current_path}",
            ],
        )?;
        if target_cwd != scratch_path {
            let escaped = target_cwd.replace('"', "\\\"");
            let cd_cmd = format!(" cd \"{escaped}\"\n");
            execute_tmux(socket, &["send-keys", "-t", &config.session_name, &cd_cmd])?;
        }
    }

    // 4. Build and run display-popup:
    let attach_cmd = config.build_attach_command(socket);
    execute_tmux(
        socket,
        &[
            "display-popup",
            "-E",
            "-w",
            &config.width,
            "-h",
            &config.height,
            "-b",
            "rounded",
            "-T",
            &config.title,
            &attach_cmd,
        ],
    )?;

    Ok(())
}
