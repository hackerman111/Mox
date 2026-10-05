//! Centralized Theme engine and Nord color palette definition.
//!
//! Provides semantic color roles for the Mox TUI so all screens (Navigator,
//! Extract, Which-key, Flash, and future Palette/Projects) share a consistent
//! visual style without hardcoded colors scattered across feature modules.

use crossterm::style::Color;

/// Canonical Nord color constants.
pub mod nord {
    use crossterm::style::Color;

    // Polar Night
    pub const NORD0: Color = Color::Rgb {
        r: 46,
        g: 52,
        b: 64,
    };
    pub const NORD1: Color = Color::Rgb {
        r: 59,
        g: 66,
        b: 82,
    };
    pub const NORD2: Color = Color::Rgb {
        r: 67,
        g: 76,
        b: 94,
    };
    pub const NORD3: Color = Color::Rgb {
        r: 76,
        g: 86,
        b: 106,
    };

    // Snow Storm
    pub const NORD4: Color = Color::Rgb {
        r: 216,
        g: 222,
        b: 233,
    };
    pub const NORD5: Color = Color::Rgb {
        r: 229,
        g: 233,
        b: 240,
    };
    pub const NORD6: Color = Color::Rgb {
        r: 236,
        g: 239,
        b: 244,
    };

    // Frost
    pub const NORD7: Color = Color::Rgb {
        r: 143,
        g: 188,
        b: 187,
    };
    pub const NORD8: Color = Color::Rgb {
        r: 136,
        g: 192,
        b: 208,
    };
    pub const NORD9: Color = Color::Rgb {
        r: 129,
        g: 161,
        b: 193,
    };
    pub const NORD10: Color = Color::Rgb {
        r: 94,
        g: 129,
        b: 172,
    };

    // Aurora
    pub const NORD11: Color = Color::Rgb {
        r: 191,
        g: 97,
        b: 106,
    };
    pub const NORD12: Color = Color::Rgb {
        r: 208,
        g: 135,
        b: 112,
    };
    pub const NORD13: Color = Color::Rgb {
        r: 235,
        g: 203,
        b: 139,
    };
    pub const NORD14: Color = Color::Rgb {
        r: 163,
        g: 190,
        b: 140,
    };
    pub const NORD15: Color = Color::Rgb {
        r: 180,
        g: 142,
        b: 173,
    };
}

/// Semantic color roles for the Mox UI layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    // Base layout
    pub bg: Color,
    pub fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub border: Color,
    pub muted: Color,

    // Feedback & state
    pub accent: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,

    // Entity Badges
    pub badge_path: Color,
    pub badge_url: Color,
    pub badge_hash: Color,
    pub badge_ip: Color,
    pub badge_command: Color,
    pub badge_word: Color,
    pub badge_docker: Color,
    pub badge_k8s: Color,
    pub badge_color: Color,
    pub badge_ipfs: Color,

    // Mode colors for status indicator
    pub mode_normal: Color,
    pub mode_prefix: Color,
    pub mode_copy: Color,
    pub mode_visual: Color,
    pub mode_resize: Color,
    pub mode_sync: Color,
    pub mode_suspend: Color,
    pub mode_agent: Color,
}

impl Theme {
    /// Constructs the canonical Nord theme.
    pub fn nord() -> Self {
        Self {
            bg: nord::NORD0,
            fg: nord::NORD6,
            selection_bg: nord::NORD2,
            selection_fg: nord::NORD6,
            border: nord::NORD3,
            muted: nord::NORD3,

            accent: nord::NORD8,
            error: nord::NORD11,
            warning: nord::NORD13,
            success: nord::NORD14,
            info: nord::NORD9,

            badge_path: nord::NORD14,
            badge_url: nord::NORD13,
            badge_hash: nord::NORD7,
            badge_ip: nord::NORD12,
            badge_command: nord::NORD8,
            badge_word: nord::NORD4,
            badge_docker: nord::NORD9,
            badge_k8s: nord::NORD10,
            badge_color: nord::NORD15,
            badge_ipfs: nord::NORD7,

            mode_normal: nord::NORD14,
            mode_prefix: nord::NORD13,
            mode_copy: nord::NORD10,
            mode_visual: nord::NORD12,
            mode_resize: nord::NORD15,
            mode_sync: nord::NORD11,
            mode_suspend: nord::NORD3,
            mode_agent: nord::NORD15,
        }
    }

    /// Resolves theme settings from configuration.
    pub fn from_config(config: &crate::config::Config) -> Self {
        crate::config::resolve_theme(&config.theme, config.custom_theme.as_ref())
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::nord()
    }
}
