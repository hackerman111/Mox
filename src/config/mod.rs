//! Configuration module for Mox.
//!
//! Provides configuration models, TOML parsing, and default settings
//! for theming, status bar layouts, and key widgets.

pub mod model;
pub mod status;
pub mod theme;

pub use model::{
    ClockConfig, Config, CustomThemeConfig, ModeIcons, ModeIndicatorConfig, ModeLabels,
    SegmentConfig, SegmentKind, SeparatorStyle, StatusConfig, WindowsConfig,
};
pub use status::{
    SeparatorGlyphs, generate_mode_indicator, generate_status_left, generate_status_right,
    generate_window_status, generate_window_status_separator, separator_glyphs,
};
pub use theme::{
    ColorSpec, NamedColor, ParseColorError, catppuccin_mocha, dracula, gruvbox, nord,
    resolve_theme, tokyo_night,
};
