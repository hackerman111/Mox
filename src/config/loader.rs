//! Hierarchical configuration loader and backward compatibility overrides.
//!
//! Handles configuration file discovery ($XDG_CONFIG_HOME or ~/.config/mox/config.toml),
//! parsing, fallback to defaults on missing/malformed files, and applying overrides
//! from native Mox tmux options as well as legacy Nord tmux and mode-indicator options.

use crate::config::model::{Config, SeparatorStyle};
use std::path::{Path, PathBuf};

/// Resolves the canonical default path for Mox configuration file.
///
/// Priority:
/// 1. `$XDG_CONFIG_HOME/mox/config.toml` (if `$XDG_CONFIG_HOME` is set and non-empty).
/// 2. `$HOME/.config/mox/config.toml` (if `$HOME` is set and non-empty).
///
/// Returns `None` if neither environment variable is configured.
pub fn default_config_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.trim().is_empty()
    {
        return Some(PathBuf::from(xdg.trim()).join("mox").join("config.toml"));
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Some(
            PathBuf::from(home.trim())
                .join(".config")
                .join("mox")
                .join("config.toml"),
        );
    }
    None
}

/// Parses a TOML string into a [`Config`] model.
pub fn parse_config_str(content: &str) -> Result<Config, toml::de::Error> {
    Config::from_toml(content)
}

/// Loads Mox configuration from an explicit path or the default location.
///
/// If `explicit_path` is provided:
/// - Attempts to read and parse the file.
/// - If reading or parsing fails, logs a warning to `stderr` and returns [`Config::default()`].
///
/// If `explicit_path` is `None`:
/// - Checks [`default_config_path()`].
/// - If the file exists, attempts to parse it; logs a warning on invalid syntax and returns [`Config::default()`].
/// - If the file does not exist, silently returns [`Config::default()`].
pub fn load_config(explicit_path: Option<&Path>) -> Config {
    if let Some(path) = explicit_path {
        match std::fs::read_to_string(path) {
            Ok(content) => match parse_config_str(&content) {
                Ok(cfg) => cfg,
                Err(err) => {
                    eprintln!(
                        "Warning: failed to parse config file at '{}': {err}",
                        path.display()
                    );
                    Config::default()
                }
            },
            Err(err) => {
                eprintln!(
                    "Warning: failed to read config file at '{}': {err}",
                    path.display()
                );
                Config::default()
            }
        }
    } else if let Some(path) = default_config_path() {
        if path.is_file() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match parse_config_str(&content) {
                    Ok(cfg) => cfg,
                    Err(err) => {
                        eprintln!(
                            "Warning: failed to parse default config file at '{}': {err}",
                            path.display()
                        );
                        Config::default()
                    }
                },
                Err(err) => {
                    eprintln!(
                        "Warning: failed to read default config file at '{}': {err}",
                        path.display()
                    );
                    Config::default()
                }
            }
        } else {
            Config::default()
        }
    } else {
        Config::default()
    }
}

/// Applies configuration overrides from tmux server options.
///
/// Accepts an option provider closure `get_option` returning `Option<String>` for a given key.
/// Overrides are evaluated in hierarchical precedence:
/// 1. Legacy Nord (`insp/tmux`) and mode indicator (`insp/tmux-mode-indicator`) compatibility options.
/// 2. Native Mox options (`@mox_*`), which take highest precedence over legacy values.
pub fn apply_tmux_overrides<F>(config: &mut Config, mut get_option: F)
where
    F: FnMut(&str) -> Option<String>,
{
    // --- Legacy Nord tmux compatibility options ---
    if let Some(val) = get_option("@nord_tmux_show_status_content")
        && val.trim() == "0"
    {
        config.status.enable = false;
    }

    if let Some(val) = get_option("@nord_tmux_no_patched_font")
        && val.trim() == "1"
    {
        config.status.separator_style = SeparatorStyle::Flat;
    }

    if let Some(val) = get_option("@nord_tmux_date_format") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.clock.date_format = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("clock-mode-style")
        && val.trim() == "12"
    {
        config.clock.style_12h = true;
    }

    // --- Legacy Mode Indicator compatibility options ---
    if let Some(val) = get_option("@mode_indicator_prefix_prompt") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.mode_indicator.labels.prefix = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("@mode_indicator_copy_prompt") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.mode_indicator.labels.copy = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("@mode_indicator_sync_prompt") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.mode_indicator.labels.sync = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("@mode_indicator_empty_prompt") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.mode_indicator.labels.normal = trimmed.to_string();
        }
    }

    // --- Native Mox options (highest precedence) ---
    if let Some(val) = get_option("@mox_theme") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.theme = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("@mox_separator_style") {
        match val.trim().to_lowercase().as_str() {
            "powerline" => config.status.separator_style = SeparatorStyle::Powerline,
            "rounded" => config.status.separator_style = SeparatorStyle::Rounded,
            "slanted" => config.status.separator_style = SeparatorStyle::Slanted,
            "flat" => config.status.separator_style = SeparatorStyle::Flat,
            "none" => config.status.separator_style = SeparatorStyle::None,
            _ => {}
        }
    }

    if let Some(val) = get_option("@mox_clock_format") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.clock.format = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("@mox_date_format") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            config.clock.date_format = trimmed.to_string();
        }
    }

    if let Some(val) = get_option("@mox_clock_12h") {
        let trimmed = val.trim();
        if trimmed == "1" || trimmed.eq_ignore_ascii_case("true") {
            config.clock.style_12h = true;
        } else if trimmed == "0" || trimmed.eq_ignore_ascii_case("false") {
            config.clock.style_12h = false;
        }
    }

    if let Some(val) = get_option("@mox_status_position") {
        let trimmed = val.trim().to_lowercase();
        if trimmed == "top" || trimmed == "bottom" {
            config.status.position = trimmed;
        }
    } else if let Some(val) = get_option("status-position") {
        let trimmed = val.trim().to_lowercase();
        if trimmed == "top" || trimmed == "bottom" {
            config.status.position = trimmed;
        }
    }
}

/// Queries tmux global options via `show-option -gqv` and applies overrides to [`Config`].
pub fn apply_tmux_server_overrides(config: &mut Config, socket: Option<&str>) {
    apply_tmux_overrides(config, |key| {
        crate::tmux::execute_tmux(socket, &["show-option", "-gqv", key])
            .ok()
            .and_then(|val| {
                let trimmed = val.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
    });
}

/// Generates a documented default TOML configuration template string.
///
/// Guaranteed to deserialize cleanly into [`Config::default()`].
pub fn default_toml() -> &'static str {
    r##"# ==============================================================================
# Mox Configuration Template
# ==============================================================================
# This configuration file controls theme selection, status bar layout,
# clock widgets, mode indicators, and window list formatting.

# ------------------------------------------------------------------------------
# Theme Selection
# ------------------------------------------------------------------------------
# Built-in themes: "nord", "catppuccin-mocha", "tokyo-night", "gruvbox", "dracula", "custom"
theme = "nord"

# ------------------------------------------------------------------------------
# Status Bar Configuration
# ------------------------------------------------------------------------------
[status]
# Enable or disable status bar rendering
enable = true

# Position of the status bar: "bottom" or "top"
position = "bottom"

# Separator visual style: "powerline", "rounded", "slanted", "flat", "none"
separator_style = "powerline"

# Left-aligned status segments
# Kinds: "mode", "session", "windows", "date", "clock", "host", "custom"
[[status.left]]
kind = "mode"

[[status.left]]
kind = "session"

# Right-aligned status segments
[[status.right]]
kind = "date"

[[status.right]]
kind = "clock"

[[status.right]]
kind = "host"

# ------------------------------------------------------------------------------
# Clock Widget Configuration
# ------------------------------------------------------------------------------
[clock]
# Strftime time format (e.g. "%H:%M", "%I:%M %p")
format = "%H:%M"

# Strftime date format (e.g. "%Y-%m-%d", "%d/%m/%Y")
date_format = "%Y-%m-%d"

# Use 12-hour clock format instead of 24-hour
style_12h = false

# Show date segment alongside time
show_date = true

# Use Nerd Font icons for date and clock
nerd_icons = true

# Render date and time as separate pill capsules
pills = false

# ------------------------------------------------------------------------------
# Mode Indicator Configuration
# ------------------------------------------------------------------------------
[mode_indicator.labels]
normal = "NORMAL"
prefix = "PREFIX"
copy = "COPY"
visual = "VISUAL"
resize = "RESIZE"
sync = "SYNC"
suspend = "SUSPEND"
agent = "AGENT"

[mode_indicator.icons]
normal = "●"
prefix = "◆"
copy = "⎘"
visual = "◈"
resize = "↕"
sync = "⇄"
suspend = "⏸"
agent = "▲"

# ------------------------------------------------------------------------------
# Window List Configuration
# ------------------------------------------------------------------------------
[windows]
# Window tab visual style: "powerline", "rounded", "flat", "classic"
style = "powerline"

# Render window flags (e.g. #F for current/zoomed/activity markers)
show_flags = true

# ------------------------------------------------------------------------------
# Custom Theme Color Overrides (used when theme = "custom" or to override theme)
# ------------------------------------------------------------------------------
# [theme.custom]
# bg = "#2e3440"
# fg = "#eceff4"
# accent = "#88c0d0"
# border = "#4c566a"
# muted = "#4c566a"
# selection_bg = "#434c5e"
# selection_fg = "#eceff4"
# error = "#bf616a"
# warning = "#ebcb8b"
# success = "#a3be8c"
# info = "#81a1c1"
"##
}
