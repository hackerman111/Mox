//! Configuration module for Mox.
//!
//! Provides configuration models, TOML parsing, and default settings
//! for theming, status bar layouts, and key widgets.

pub mod model;
pub mod theme;

pub use model::{
    ClockConfig, Config, CustomThemeConfig, ModeIcons, ModeIndicatorConfig, ModeLabels,
    SegmentConfig, SegmentKind, SeparatorStyle, StatusConfig, WindowsConfig,
};
pub use theme::{
    ColorSpec, NamedColor, ParseColorError, catppuccin_mocha, dracula, gruvbox, nord,
    resolve_theme, tokyo_night,
};
