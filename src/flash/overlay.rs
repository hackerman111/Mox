//! Flash popup overlay launcher and configuration.
//!
//! Handles terminal geometry queries, popup arguments preparation, and
//! launching borderless tmux `display-popup` windows.

use crate::extract::model::EntityKind;
use crate::tmux::execute_tmux;

/// Operating mode for the Flash navigation overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashMode {
    /// Incremental search and home-row label jump into `copy-mode-vi`.
    Jump,
    /// Direct home-row label assignment on visible extracted entities for clipboard copy.
    QuickYank,
    /// Direct home-row label assignment on visible entities for editor or browser opening.
    QuickOpen,
    /// Remote target selection and semantic text object extraction without moving shell cursor.
    RemoteYank,
    /// Single-character forward or backward motion scanning.
    CharMotion,
}

/// Configuration settings for the Flash overlay session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashConfig {
    /// Active navigation mode.
    pub mode: FlashMode,
    /// Whether to search across all window panes instead of only the active pane.
    pub multi_pane: bool,
    /// Optional semantic entity filter (e.g. paths only, URLs only).
    pub entity_filter: Option<EntityKind>,
    /// Target tmux pane identifier (e.g. `"%0"`).
    pub target_pane: Option<String>,
    /// Target character for single-character motion (`f`/`F`).
    pub motion_char: Option<char>,
    /// Direction of character motion (`true` for forward, `false` for backward).
    pub motion_forward: bool,
}

impl Default for FlashConfig {
    fn default() -> Self {
        Self::new_jump(false)
    }
}

impl FlashConfig {
    /// Constructs a new configuration for Flash Jump mode.
    pub fn new_jump(multi_pane: bool) -> Self {
        Self {
            mode: FlashMode::Jump,
            multi_pane,
            entity_filter: None,
            target_pane: None,
            motion_char: None,
            motion_forward: true,
        }
    }

    /// Constructs a new configuration for Remote Yank mode.
    pub fn new_remote_yank(multi_pane: bool) -> Self {
        Self {
            mode: FlashMode::RemoteYank,
            multi_pane,
            entity_filter: None,
            target_pane: None,
            motion_char: None,
            motion_forward: true,
        }
    }

    /// Constructs a new configuration for Quick Yank mode with an optional entity filter.
    pub fn new_quick_yank(entity_filter: Option<EntityKind>, multi_pane: bool) -> Self {
        Self {
            mode: FlashMode::QuickYank,
            multi_pane,
            entity_filter,
            target_pane: None,
            motion_char: None,
            motion_forward: true,
        }
    }

    /// Constructs a new configuration for Quick Open mode with an optional entity filter.
    pub fn new_quick_open(entity_filter: Option<EntityKind>, multi_pane: bool) -> Self {
        Self {
            mode: FlashMode::QuickOpen,
            multi_pane,
            entity_filter,
            target_pane: None,
            motion_char: None,
            motion_forward: true,
        }
    }

    /// Constructs a new configuration for single-character motion (`f`/`F`).
    pub fn new_char_motion(motion_char: char, motion_forward: bool, multi_pane: bool) -> Self {
        Self {
            mode: FlashMode::CharMotion,
            multi_pane,
            entity_filter: None,
            target_pane: None,
            motion_char: Some(motion_char),
            motion_forward,
        }
    }
}

/// Builds the `mox flash ...` command string to execute inside the tmux popup.
pub fn build_flash_command(bin_path: &str, cfg: &FlashConfig, socket: Option<&str>) -> String {
    let mode_str = match cfg.mode {
        FlashMode::Jump => "jump",
        FlashMode::QuickYank => "quick-yank",
        FlashMode::QuickOpen => "quick-open",
        FlashMode::RemoteYank => "remote-yank",
        FlashMode::CharMotion => "char-motion",
    };

    let mut cmd = format!(
        "{} flash --mode {mode_str}",
        crate::tmux::shell_quote(bin_path)
    );

    if cfg.multi_pane {
        cmd.push_str(" --multi-pane");
    }

    if let Some(filter) = cfg.entity_filter {
        let filter_name = match filter {
            EntityKind::Path => "path",
            EntityKind::Url => "url",
            EntityKind::Hash => "hash",
            EntityKind::Ip => "ip",
            EntityKind::Uuid => "uuid",
            EntityKind::Command => "command",
            EntityKind::Quoted => "quoted",
            EntityKind::Number => "number",
            EntityKind::Word => "word",
            EntityKind::DockerImage => "docker",
            EntityKind::KubernetesResource => "k8s",
            EntityKind::HexColor => "color",
            EntityKind::IpfsCid => "ipfs",
        };
        cmd.push_str(&format!(" --filter {filter_name}"));
    }

    if let Some(ch) = cfg.motion_char {
        cmd.push_str(&format!(
            " --char {}",
            crate::tmux::shell_quote(&ch.to_string())
        ));
    }

    if !cfg.motion_forward {
        cmd.push_str(" --backward");
    }

    if let Some(target) = &cfg.target_pane {
        cmd.push_str(&format!(
            " --target-pane {}",
            crate::tmux::shell_quote(target)
        ));
    }

    if let Some(sock) = socket {
        cmd.push_str(&format!(" --socket {}", crate::tmux::shell_quote(sock)));
    }

    cmd
}

/// Queries tmux geometry and launches a borderless `display-popup` running `mox flash`.
pub fn launch_flash_popup(socket: Option<&str>, cfg: &FlashConfig) -> std::io::Result<()> {
    let (x, y, w, h) = if cfg.multi_pane {
        // Multi-pane spans entire tmux window
        (
            "0".to_string(),
            "0".to_string(),
            "100%".to_string(),
            "100%".to_string(),
        )
    } else {
        // Single-pane queries active or target pane dimensions
        let mut query_args = vec!["display-message", "-p"];
        if let Some(target) = &cfg.target_pane {
            query_args.push("-t");
            query_args.push(target);
        }
        query_args.push("#{pane_left},#{pane_top},#{pane_width},#{pane_height}");

        match execute_tmux(socket, &query_args) {
            Ok(dims) => {
                let parts: Vec<&str> = dims.trim().split(',').collect();
                if parts.len() == 4 {
                    (
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                        parts[3].to_string(),
                    )
                } else {
                    (
                        "0".to_string(),
                        "0".to_string(),
                        "100%".to_string(),
                        "100%".to_string(),
                    )
                }
            }
            Err(_) => (
                "0".to_string(),
                "0".to_string(),
                "100%".to_string(),
                "100%".to_string(),
            ),
        }
    };

    let bin_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "mox".to_string());

    let flash_cmd = build_flash_command(&bin_path, cfg, socket);

    let popup_args = [
        "display-popup",
        "-E",
        "-B",
        "-x",
        &x,
        "-y",
        &y,
        "-w",
        &w,
        "-h",
        &h,
        &flash_cmd,
    ];

    execute_tmux(socket, &popup_args).map_err(std::io::Error::other)?;

    Ok(())
}
