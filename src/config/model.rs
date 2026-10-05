//! Configuration data models for Mox.
//!
//! Provides strongly-typed models representing the TOML configuration schema,
//! with sensible defaults for themes, status bar widgets, clock, mode indicators,
//! and window styling.

use crate::apps::model::default_apps;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level configuration representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Active theme name ("nord", "catppuccin-mocha", "tokyo-night", "gruvbox", "dracula", "custom").
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Status bar configuration.
    #[serde(default)]
    pub status: StatusConfig,

    /// Clock widget and date display configuration.
    #[serde(default)]
    pub clock: ClockConfig,

    /// Mode indicator widget configuration.
    #[serde(default)]
    pub mode_indicator: ModeIndicatorConfig,

    /// Window list widget configuration.
    #[serde(default)]
    pub windows: WindowsConfig,

    /// Optional custom theme color overrides.
    #[serde(default, alias = "theme_custom", alias = "theme.custom")]
    pub custom_theme: Option<CustomThemeConfig>,

    /// Configured applications.
    #[serde(default = "default_apps")]
    pub apps: Vec<crate::apps::model::AppConfig>,
}

impl Config {
    /// Parses configuration from a TOML string with default fallbacks.
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            status: StatusConfig::default(),
            clock: ClockConfig::default(),
            mode_indicator: ModeIndicatorConfig::default(),
            windows: WindowsConfig::default(),
            custom_theme: None,
            apps: default_apps(),
        }
    }
}

/// Status bar separator visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SeparatorStyle {
    /// Powerline triangular arrows ( / ).
    #[default]
    Powerline,
    /// Rounded bubble caps ( / ).
    Rounded,
    /// Slanted forward/backward slashes ( / ).
    Slanted,
    /// Flat space/bar delimiter.
    Flat,
    /// No separator symbol between segments.
    None,
}

/// Status bar layout and segment configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusConfig {
    /// Whether the status bar is rendered.
    #[serde(default = "default_true")]
    pub enable: bool,

    /// Vertical placement ("bottom" or "top").
    #[serde(default = "default_position")]
    pub position: String,

    /// Style of segment boundary separators.
    #[serde(default)]
    pub separator_style: SeparatorStyle,

    /// Left-aligned status segments.
    #[serde(default = "default_status_left")]
    pub left: Vec<SegmentConfig>,

    /// Right-aligned status segments.
    #[serde(default = "default_status_right")]
    pub right: Vec<SegmentConfig>,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            enable: true,
            position: default_position(),
            separator_style: SeparatorStyle::default(),
            left: default_status_left(),
            right: default_status_right(),
        }
    }
}

/// Configuration for an individual segment in the status bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentConfig {
    /// Type/role of the status segment.
    pub kind: SegmentKind,

    /// Optional leading icon or glyph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Optional custom strftime/formatting template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Optional shell command to execute for custom segments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Optional condition name to gate segment rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

impl SegmentConfig {
    /// Creates a minimal segment with only the kind specified.
    pub fn new(kind: SegmentKind) -> Self {
        Self {
            kind,
            icon: None,
            format: None,
            command: None,
            when: None,
        }
    }
}

/// Supported kinds of status bar segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    /// Active modal/keybinding mode indicator.
    #[default]
    Mode,
    /// Active session name.
    Session,
    /// Window list pills/tabs.
    Windows,
    /// Current calendar date.
    Date,
    /// Current time clock.
    Clock,
    /// System hostname or target machine.
    Host,
    /// Arbitrary command-backed custom segment.
    Custom,
}

/// Clock and calendar widget configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockConfig {
    /// Strftime format for the time component.
    #[serde(default = "default_clock_format")]
    pub format: String,

    /// Strftime format for the date component.
    #[serde(default = "default_date_format")]
    pub date_format: String,

    /// Whether to force 12-hour clock format (AM/PM).
    #[serde(default)]
    pub style_12h: bool,

    /// Whether to display date alongside time.
    #[serde(default = "default_true")]
    pub show_date: bool,

    /// Whether to render Nerd Font icons.
    #[serde(default = "default_true")]
    pub nerd_icons: bool,

    /// Whether to render date and time as separate pill capsules.
    #[serde(default)]
    pub pills: bool,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: default_clock_format(),
            date_format: default_date_format(),
            style_12h: false,
            show_date: true,
            nerd_icons: true,
            pills: false,
        }
    }
}

/// Mode indicator widget configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModeIndicatorConfig {
    /// Text labels for modes.
    #[serde(default)]
    pub labels: ModeLabels,

    /// Glyph icons for modes.
    #[serde(default)]
    pub icons: ModeIcons,
}

/// Text labels for Mox/tmux operating modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeLabels {
    #[serde(default = "default_label_normal")]
    pub normal: String,
    #[serde(default = "default_label_prefix")]
    pub prefix: String,
    #[serde(default = "default_label_copy")]
    pub copy: String,
    #[serde(default = "default_label_visual")]
    pub visual: String,
    #[serde(default = "default_label_resize")]
    pub resize: String,
    #[serde(default = "default_label_sync")]
    pub sync: String,
    #[serde(default = "default_label_suspend")]
    pub suspend: String,
    #[serde(default = "default_label_agent")]
    pub agent: String,
}

impl Default for ModeLabels {
    fn default() -> Self {
        Self {
            normal: default_label_normal(),
            prefix: default_label_prefix(),
            copy: default_label_copy(),
            visual: default_label_visual(),
            resize: default_label_resize(),
            sync: default_label_sync(),
            suspend: default_label_suspend(),
            agent: default_label_agent(),
        }
    }
}

/// Glyph icons for Mox/tmux operating modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeIcons {
    #[serde(default = "default_icon_normal")]
    pub normal: String,
    #[serde(default = "default_icon_prefix")]
    pub prefix: String,
    #[serde(default = "default_icon_copy")]
    pub copy: String,
    #[serde(default = "default_icon_visual")]
    pub visual: String,
    #[serde(default = "default_icon_resize")]
    pub resize: String,
    #[serde(default = "default_icon_sync")]
    pub sync: String,
    #[serde(default = "default_icon_suspend")]
    pub suspend: String,
    #[serde(default = "default_icon_agent")]
    pub agent: String,
}

impl Default for ModeIcons {
    fn default() -> Self {
        Self {
            normal: default_icon_normal(),
            prefix: default_icon_prefix(),
            copy: default_icon_copy(),
            visual: default_icon_visual(),
            resize: default_icon_resize(),
            sync: default_icon_sync(),
            suspend: default_icon_suspend(),
            agent: default_icon_agent(),
        }
    }
}

/// Window status formatting options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsConfig {
    /// Visual style ("powerline", "rounded", "flat", "classic").
    #[serde(default = "default_windows_style")]
    pub style: String,

    /// Whether to render window flag symbols (`#F`).
    #[serde(default = "default_true")]
    pub show_flags: bool,
}

impl Default for WindowsConfig {
    fn default() -> Self {
        Self {
            style: default_windows_style(),
            show_flags: true,
        }
    }
}

/// User overrides for theme colors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomThemeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_fg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    /// Additional custom or semantic color keys.
    #[serde(flatten)]
    pub colors: BTreeMap<String, String>,
}

impl CustomThemeConfig {
    /// Looks up a color value by key name across explicit fields and the extra colors map.
    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "bg" => self
                .bg
                .as_deref()
                .or_else(|| self.colors.get("bg").map(String::as_str)),
            "fg" => self
                .fg
                .as_deref()
                .or_else(|| self.colors.get("fg").map(String::as_str)),
            "accent" => self
                .accent
                .as_deref()
                .or_else(|| self.colors.get("accent").map(String::as_str)),
            "selection_bg" => self
                .selection_bg
                .as_deref()
                .or_else(|| self.colors.get("selection_bg").map(String::as_str)),
            "selection_fg" => self
                .selection_fg
                .as_deref()
                .or_else(|| self.colors.get("selection_fg").map(String::as_str)),
            "border" => self
                .border
                .as_deref()
                .or_else(|| self.colors.get("border").map(String::as_str)),
            "muted" => self
                .muted
                .as_deref()
                .or_else(|| self.colors.get("muted").map(String::as_str)),
            "error" => self
                .error
                .as_deref()
                .or_else(|| self.colors.get("error").map(String::as_str)),
            "warning" => self
                .warning
                .as_deref()
                .or_else(|| self.colors.get("warning").map(String::as_str)),
            "success" => self
                .success
                .as_deref()
                .or_else(|| self.colors.get("success").map(String::as_str)),
            "info" => self
                .info
                .as_deref()
                .or_else(|| self.colors.get("info").map(String::as_str)),
            other => self.colors.get(other).map(String::as_str),
        }
    }
}

// Serde default helper functions

fn default_theme() -> String {
    "nord".to_string()
}

fn default_true() -> bool {
    true
}

fn default_position() -> String {
    "bottom".to_string()
}

fn default_status_left() -> Vec<SegmentConfig> {
    vec![
        SegmentConfig::new(SegmentKind::Mode),
        SegmentConfig::new(SegmentKind::Session),
    ]
}

fn default_status_right() -> Vec<SegmentConfig> {
    vec![
        SegmentConfig::new(SegmentKind::Date),
        SegmentConfig::new(SegmentKind::Clock),
        SegmentConfig::new(SegmentKind::Host),
    ]
}

fn default_clock_format() -> String {
    "%H:%M".to_string()
}

fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

fn default_label_normal() -> String {
    "NORMAL".to_string()
}

fn default_label_prefix() -> String {
    "PREFIX".to_string()
}

fn default_label_copy() -> String {
    "COPY".to_string()
}

fn default_label_visual() -> String {
    "VISUAL".to_string()
}

fn default_label_resize() -> String {
    "RESIZE".to_string()
}

fn default_label_sync() -> String {
    "SYNC".to_string()
}

fn default_label_suspend() -> String {
    "SUSPEND".to_string()
}

fn default_label_agent() -> String {
    "AGENT".to_string()
}

fn default_icon_normal() -> String {
    "●".to_string()
}

fn default_icon_prefix() -> String {
    "◆".to_string()
}

fn default_icon_copy() -> String {
    "⎘".to_string()
}

fn default_icon_visual() -> String {
    "◈".to_string()
}

fn default_icon_resize() -> String {
    "↕".to_string()
}

fn default_icon_sync() -> String {
    "⇄".to_string()
}

fn default_icon_suspend() -> String {
    "⏸".to_string()
}

fn default_icon_agent() -> String {
    "▲".to_string()
}

fn default_windows_style() -> String {
    "powerline".to_string()
}
