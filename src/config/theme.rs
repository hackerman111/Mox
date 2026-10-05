//! Color specification parsing, theme presets, and theme resolution.
//!
//! Provides `ColorSpec` for parsing flexible color descriptions (hex, ANSI names,
//! 256 colors, default) and mapping them to crossterm and tmux representations,
//! as well as predefined theme palettes and user override resolution.

use crossterm::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Visitor};
use std::fmt;
use std::str::FromStr;

use crate::config::CustomThemeConfig;
use crate::ui::theme::Theme;

/// Error returned when parsing an invalid color string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseColorError {
    InvalidHex(String),
    InvalidAnsi(String),
    Unknown(String),
    Empty,
}

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex(s) => write!(f, "Invalid hex color format: '{s}'"),
            Self::InvalidAnsi(s) => write!(f, "Invalid ANSI 256 color number: '{s}'"),
            Self::Unknown(s) => write!(f, "Unknown color specification: '{s}'"),
            Self::Empty => write!(f, "Empty color string"),
        }
    }
}

impl std::error::Error for ParseColorError {}

/// Standard named ANSI colors (0..=7) and bright variants (8..=15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl NamedColor {
    /// Converts named color to crossterm representation.
    pub fn to_crossterm(self) -> Color {
        match self {
            Self::Black => Color::Black,
            Self::Red => Color::Red,
            Self::Green => Color::Green,
            Self::Yellow => Color::Yellow,
            Self::Blue => Color::Blue,
            Self::Magenta => Color::Magenta,
            Self::Cyan => Color::Cyan,
            Self::White => Color::White,
            Self::BrightBlack => Color::DarkGrey,
            Self::BrightRed => Color::AnsiValue(9),
            Self::BrightGreen => Color::AnsiValue(10),
            Self::BrightYellow => Color::AnsiValue(11),
            Self::BrightBlue => Color::AnsiValue(12),
            Self::BrightMagenta => Color::AnsiValue(13),
            Self::BrightCyan => Color::AnsiValue(14),
            Self::BrightWhite => Color::AnsiValue(15),
        }
    }

    /// Converts named color to tmux color string.
    pub fn to_tmux(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::BrightBlack => "brightblack",
            Self::BrightRed => "brightred",
            Self::BrightGreen => "brightgreen",
            Self::BrightYellow => "brightyellow",
            Self::BrightBlue => "brightblue",
            Self::BrightMagenta => "brightmagenta",
            Self::BrightCyan => "brightcyan",
            Self::BrightWhite => "brightwhite",
        }
    }
}

/// Represents a parsed color specification supporting Hex, ANSI, 256-color, or Default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpec {
    Default,
    Rgb { r: u8, g: u8, b: u8 },
    Ansi(u8),
    Named(NamedColor),
}

impl ColorSpec {
    /// Converts to `crossterm::style::Color`.
    pub fn to_crossterm(&self) -> Color {
        match *self {
            Self::Default => Color::Reset,
            Self::Rgb { r, g, b } => Color::Rgb { r, g, b },
            Self::Ansi(val) => Color::AnsiValue(val),
            Self::Named(named) => named.to_crossterm(),
        }
    }

    /// Converts to tmux color string representation.
    pub fn to_tmux(&self) -> String {
        match *self {
            Self::Default => "default".to_string(),
            Self::Rgb { r, g, b } => format!("#{r:02x}{g:02x}{b:02x}"),
            Self::Ansi(val) => format!("colour{val}"),
            Self::Named(named) => named.to_tmux().to_string(),
        }
    }
}

impl FromStr for ColorSpec {
    type Err = ParseColorError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let s = src.trim();
        if s.is_empty() {
            return Err(ParseColorError::Empty);
        }

        // 1. Hex color
        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex);
        }

        let lower = s.to_ascii_lowercase();

        // 2. Default / None
        if lower == "default" || lower == "none" || lower == "reset" {
            return Ok(ColorSpec::Default);
        }

        // 3. 256 colors: colourN or colorN
        if let Some(num_str) = lower
            .strip_prefix("colour")
            .or_else(|| lower.strip_prefix("color"))
        {
            let val = num_str
                .parse::<u8>()
                .map_err(|_| ParseColorError::InvalidAnsi(s.to_string()))?;
            return Ok(ColorSpec::Ansi(val));
        }

        // 4. Raw number (0..=255)
        if s.chars().all(|c| c.is_ascii_digit()) {
            let val = s
                .parse::<u8>()
                .map_err(|_| ParseColorError::InvalidAnsi(s.to_string()))?;
            return Ok(ColorSpec::Ansi(val));
        }

        // 5. Named ANSI colors & bright variants
        // Normalize by removing '-' and '_'
        let normalized: String = lower.chars().filter(|&c| c != '-' && c != '_').collect();
        let named = match normalized.as_str() {
            "black" => NamedColor::Black,
            "red" => NamedColor::Red,
            "green" => NamedColor::Green,
            "yellow" => NamedColor::Yellow,
            "blue" => NamedColor::Blue,
            "magenta" => NamedColor::Magenta,
            "cyan" => NamedColor::Cyan,
            "white" => NamedColor::White,

            "brightblack" | "darkgrey" | "darkgray" => NamedColor::BrightBlack,
            "brightred" => NamedColor::BrightRed,
            "brightgreen" => NamedColor::BrightGreen,
            "brightyellow" => NamedColor::BrightYellow,
            "brightblue" => NamedColor::BrightBlue,
            "brightmagenta" => NamedColor::BrightMagenta,
            "brightcyan" => NamedColor::BrightCyan,
            "brightwhite" => NamedColor::BrightWhite,

            "gray" | "grey" => NamedColor::White,

            _ => return Err(ParseColorError::Unknown(s.to_string())),
        };

        Ok(ColorSpec::Named(named))
    }
}

fn parse_hex(hex: &str) -> Result<ColorSpec, ParseColorError> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ParseColorError::InvalidHex(format!("#{hex}")));
    }

    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16)
                .map_err(|_| ParseColorError::InvalidHex(format!("#{hex}")))?;
            let g = u8::from_str_radix(&hex[1..2], 16)
                .map_err(|_| ParseColorError::InvalidHex(format!("#{hex}")))?;
            let b = u8::from_str_radix(&hex[2..3], 16)
                .map_err(|_| ParseColorError::InvalidHex(format!("#{hex}")))?;
            Ok(ColorSpec::Rgb {
                r: (r << 4) | r,
                g: (g << 4) | g,
                b: (b << 4) | b,
            })
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| ParseColorError::InvalidHex(format!("#{hex}")))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| ParseColorError::InvalidHex(format!("#{hex}")))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| ParseColorError::InvalidHex(format!("#{hex}")))?;
            Ok(ColorSpec::Rgb { r, g, b })
        }
        _ => Err(ParseColorError::InvalidHex(format!("#{hex}"))),
    }
}

impl TryFrom<&str> for ColorSpec {
    type Error = ParseColorError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl Serialize for ColorSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_tmux())
    }
}

impl<'de> Deserialize<'de> for ColorSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorSpecVisitor;

        impl<'de> Visitor<'de> for ColorSpecVisitor {
            type Value = ColorSpec;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a valid color string (e.g. '#2e3440', 'red', 'colour236') or integer 0..=255",
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                ColorSpec::from_str(v).map_err(serde::de::Error::custom)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v <= 255 {
                    Ok(ColorSpec::Ansi(v as u8))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "ANSI color index {v} out of range 0..=255"
                    )))
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if (0..=255).contains(&v) {
                    Ok(ColorSpec::Ansi(v as u8))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "ANSI color index {v} out of range 0..=255"
                    )))
                }
            }
        }

        deserializer.deserialize_any(ColorSpecVisitor)
    }
}

/// Canonical Nord color scheme.
pub fn nord() -> Theme {
    Theme::nord()
}

/// Catppuccin Mocha color scheme.
pub fn catppuccin_mocha() -> Theme {
    Theme {
        bg: Color::Rgb {
            r: 0x1e,
            g: 0x1e,
            b: 0x2e,
        },
        fg: Color::Rgb {
            r: 0xcd,
            g: 0xd6,
            b: 0xf4,
        },
        selection_bg: Color::Rgb {
            r: 0x31,
            g: 0x32,
            b: 0x44,
        },
        selection_fg: Color::Rgb {
            r: 0xcd,
            g: 0xd6,
            b: 0xf4,
        },
        border: Color::Rgb {
            r: 0x45,
            g: 0x47,
            b: 0x5a,
        },
        muted: Color::Rgb {
            r: 0x6c,
            g: 0x70,
            b: 0x86,
        },

        accent: Color::Rgb {
            r: 0x89,
            g: 0xb4,
            b: 0xfa,
        },
        error: Color::Rgb {
            r: 0xf3,
            g: 0x8b,
            b: 0xa8,
        },
        warning: Color::Rgb {
            r: 0xf9,
            g: 0xe2,
            b: 0xaf,
        },
        success: Color::Rgb {
            r: 0xa6,
            g: 0xe3,
            b: 0xa1,
        },
        info: Color::Rgb {
            r: 0x89,
            g: 0xdc,
            b: 0xeb,
        },

        badge_path: Color::Rgb {
            r: 0xa6,
            g: 0xe3,
            b: 0xa1,
        },
        badge_url: Color::Rgb {
            r: 0xf9,
            g: 0xe2,
            b: 0xaf,
        },
        badge_hash: Color::Rgb {
            r: 0x94,
            g: 0xe2,
            b: 0xd5,
        },
        badge_ip: Color::Rgb {
            r: 0xfa,
            g: 0xb3,
            b: 0x87,
        },
        badge_command: Color::Rgb {
            r: 0x89,
            g: 0xdc,
            b: 0xeb,
        },
        badge_word: Color::Rgb {
            r: 0xa6,
            g: 0xad,
            b: 0xc8,
        },
        badge_docker: Color::Rgb {
            r: 0x74,
            g: 0xc7,
            b: 0xec,
        },
        badge_k8s: Color::Rgb {
            r: 0x89,
            g: 0xb4,
            b: 0xfa,
        },
        badge_color: Color::Rgb {
            r: 0xcb,
            g: 0xa6,
            b: 0xf7,
        },
        badge_ipfs: Color::Rgb {
            r: 0x94,
            g: 0xe2,
            b: 0xd5,
        },

        mode_normal: Color::Rgb {
            r: 0xa6,
            g: 0xe3,
            b: 0xa1,
        },
        mode_prefix: Color::Rgb {
            r: 0xf9,
            g: 0xe2,
            b: 0xaf,
        },
        mode_copy: Color::Rgb {
            r: 0x89,
            g: 0xb4,
            b: 0xfa,
        },
        mode_visual: Color::Rgb {
            r: 0xfa,
            g: 0xb3,
            b: 0x87,
        },
        mode_resize: Color::Rgb {
            r: 0xcb,
            g: 0xa6,
            b: 0xf7,
        },
        mode_sync: Color::Rgb {
            r: 0xf3,
            g: 0x8b,
            b: 0xa8,
        },
        mode_suspend: Color::Rgb {
            r: 0x6c,
            g: 0x70,
            b: 0x86,
        },
        mode_agent: Color::Rgb {
            r: 0xcb,
            g: 0xa6,
            b: 0xf7,
        },
    }
}

/// Tokyo Night color scheme.
pub fn tokyo_night() -> Theme {
    Theme {
        bg: Color::Rgb {
            r: 0x1a,
            g: 0x1b,
            b: 0x26,
        },
        fg: Color::Rgb {
            r: 0xc0,
            g: 0xca,
            b: 0xf5,
        },
        selection_bg: Color::Rgb {
            r: 0x28,
            g: 0x34,
            b: 0x57,
        },
        selection_fg: Color::Rgb {
            r: 0xc0,
            g: 0xca,
            b: 0xf5,
        },
        border: Color::Rgb {
            r: 0x29,
            g: 0xa4,
            b: 0xbd,
        },
        muted: Color::Rgb {
            r: 0x56,
            g: 0x5f,
            b: 0x89,
        },

        accent: Color::Rgb {
            r: 0x7a,
            g: 0xa2,
            b: 0xf7,
        },
        error: Color::Rgb {
            r: 0xf7,
            g: 0x76,
            b: 0x8e,
        },
        warning: Color::Rgb {
            r: 0xe0,
            g: 0xaf,
            b: 0x68,
        },
        success: Color::Rgb {
            r: 0x9e,
            g: 0xce,
            b: 0x6a,
        },
        info: Color::Rgb {
            r: 0x7d,
            g: 0xcf,
            b: 0xff,
        },

        badge_path: Color::Rgb {
            r: 0x9e,
            g: 0xce,
            b: 0x6a,
        },
        badge_url: Color::Rgb {
            r: 0xe0,
            g: 0xaf,
            b: 0x68,
        },
        badge_hash: Color::Rgb {
            r: 0x7d,
            g: 0xcf,
            b: 0xff,
        },
        badge_ip: Color::Rgb {
            r: 0xff,
            g: 0x9e,
            b: 0x64,
        },
        badge_command: Color::Rgb {
            r: 0x7a,
            g: 0xa2,
            b: 0xf7,
        },
        badge_word: Color::Rgb {
            r: 0xc0,
            g: 0xca,
            b: 0xf5,
        },
        badge_docker: Color::Rgb {
            r: 0x2a,
            g: 0xc3,
            b: 0xde,
        },
        badge_k8s: Color::Rgb {
            r: 0x7a,
            g: 0xa2,
            b: 0xf7,
        },
        badge_color: Color::Rgb {
            r: 0xbb,
            g: 0x9a,
            b: 0xf7,
        },
        badge_ipfs: Color::Rgb {
            r: 0x7d,
            g: 0xcf,
            b: 0xff,
        },

        mode_normal: Color::Rgb {
            r: 0x9e,
            g: 0xce,
            b: 0x6a,
        },
        mode_prefix: Color::Rgb {
            r: 0xe0,
            g: 0xaf,
            b: 0x68,
        },
        mode_copy: Color::Rgb {
            r: 0x7a,
            g: 0xa2,
            b: 0xf7,
        },
        mode_visual: Color::Rgb {
            r: 0xff,
            g: 0x9e,
            b: 0x64,
        },
        mode_resize: Color::Rgb {
            r: 0xbb,
            g: 0x9a,
            b: 0xf7,
        },
        mode_sync: Color::Rgb {
            r: 0xf7,
            g: 0x76,
            b: 0x8e,
        },
        mode_suspend: Color::Rgb {
            r: 0x56,
            g: 0x5f,
            b: 0x89,
        },
        mode_agent: Color::Rgb {
            r: 0xbb,
            g: 0x9a,
            b: 0xf7,
        },
    }
}

/// Gruvbox color scheme.
pub fn gruvbox() -> Theme {
    Theme {
        bg: Color::Rgb {
            r: 0x28,
            g: 0x28,
            b: 0x28,
        },
        fg: Color::Rgb {
            r: 0xeb,
            g: 0xdb,
            b: 0xb2,
        },
        selection_bg: Color::Rgb {
            r: 0x3c,
            g: 0x38,
            b: 0x36,
        },
        selection_fg: Color::Rgb {
            r: 0xeb,
            g: 0xdb,
            b: 0xb2,
        },
        border: Color::Rgb {
            r: 0x50,
            g: 0x49,
            b: 0x45,
        },
        muted: Color::Rgb {
            r: 0x7c,
            g: 0x6f,
            b: 0x64,
        },

        accent: Color::Rgb {
            r: 0xfe,
            g: 0x80,
            b: 0x19,
        },
        error: Color::Rgb {
            r: 0xfb,
            g: 0x49,
            b: 0x34,
        },
        warning: Color::Rgb {
            r: 0xfa,
            g: 0xbd,
            b: 0x2f,
        },
        success: Color::Rgb {
            r: 0xb8,
            g: 0xbb,
            b: 0x26,
        },
        info: Color::Rgb {
            r: 0x83,
            g: 0xa5,
            b: 0x98,
        },

        badge_path: Color::Rgb {
            r: 0xb8,
            g: 0xbb,
            b: 0x26,
        },
        badge_url: Color::Rgb {
            r: 0xfa,
            g: 0xbd,
            b: 0x2f,
        },
        badge_hash: Color::Rgb {
            r: 0x8e,
            g: 0xc0,
            b: 0x7c,
        },
        badge_ip: Color::Rgb {
            r: 0xfe,
            g: 0x80,
            b: 0x19,
        },
        badge_command: Color::Rgb {
            r: 0x83,
            g: 0xa5,
            b: 0x98,
        },
        badge_word: Color::Rgb {
            r: 0xeb,
            g: 0xdb,
            b: 0xb2,
        },
        badge_docker: Color::Rgb {
            r: 0x83,
            g: 0xa5,
            b: 0x98,
        },
        badge_k8s: Color::Rgb {
            r: 0x45,
            g: 0x85,
            b: 0x88,
        },
        badge_color: Color::Rgb {
            r: 0xd3,
            g: 0x86,
            b: 0x9b,
        },
        badge_ipfs: Color::Rgb {
            r: 0x8e,
            g: 0xc0,
            b: 0x7c,
        },

        mode_normal: Color::Rgb {
            r: 0xb8,
            g: 0xbb,
            b: 0x26,
        },
        mode_prefix: Color::Rgb {
            r: 0xfa,
            g: 0xbd,
            b: 0x2f,
        },
        mode_copy: Color::Rgb {
            r: 0x83,
            g: 0xa5,
            b: 0x98,
        },
        mode_visual: Color::Rgb {
            r: 0xfe,
            g: 0x80,
            b: 0x19,
        },
        mode_resize: Color::Rgb {
            r: 0xd3,
            g: 0x86,
            b: 0x9b,
        },
        mode_sync: Color::Rgb {
            r: 0xfb,
            g: 0x49,
            b: 0x34,
        },
        mode_suspend: Color::Rgb {
            r: 0x7c,
            g: 0x6f,
            b: 0x64,
        },
        mode_agent: Color::Rgb {
            r: 0xd3,
            g: 0x86,
            b: 0x9b,
        },
    }
}

/// Dracula color scheme.
pub fn dracula() -> Theme {
    Theme {
        bg: Color::Rgb {
            r: 0x28,
            g: 0x2a,
            b: 0x36,
        },
        fg: Color::Rgb {
            r: 0xf8,
            g: 0xf8,
            b: 0xf2,
        },
        selection_bg: Color::Rgb {
            r: 0x44,
            g: 0x47,
            b: 0x5a,
        },
        selection_fg: Color::Rgb {
            r: 0xf8,
            g: 0xf8,
            b: 0xf2,
        },
        border: Color::Rgb {
            r: 0x62,
            g: 0x72,
            b: 0xa4,
        },
        muted: Color::Rgb {
            r: 0x62,
            g: 0x72,
            b: 0xa4,
        },

        accent: Color::Rgb {
            r: 0xbd,
            g: 0x93,
            b: 0xf9,
        },
        error: Color::Rgb {
            r: 0xff,
            g: 0x55,
            b: 0x55,
        },
        warning: Color::Rgb {
            r: 0xff,
            g: 0xb8,
            b: 0x6c,
        },
        success: Color::Rgb {
            r: 0x50,
            g: 0xfa,
            b: 0x7b,
        },
        info: Color::Rgb {
            r: 0x8b,
            g: 0xe9,
            b: 0xfd,
        },

        badge_path: Color::Rgb {
            r: 0x50,
            g: 0xfa,
            b: 0x7b,
        },
        badge_url: Color::Rgb {
            r: 0xff,
            g: 0xb8,
            b: 0x6c,
        },
        badge_hash: Color::Rgb {
            r: 0x8b,
            g: 0xe9,
            b: 0xfd,
        },
        badge_ip: Color::Rgb {
            r: 0xff,
            g: 0xb8,
            b: 0x6c,
        },
        badge_command: Color::Rgb {
            r: 0xbd,
            g: 0x93,
            b: 0xf9,
        },
        badge_word: Color::Rgb {
            r: 0xf8,
            g: 0xf8,
            b: 0xf2,
        },
        badge_docker: Color::Rgb {
            r: 0x8b,
            g: 0xe9,
            b: 0xfd,
        },
        badge_k8s: Color::Rgb {
            r: 0x62,
            g: 0x72,
            b: 0xa4,
        },
        badge_color: Color::Rgb {
            r: 0xff,
            g: 0x79,
            b: 0xc6,
        },
        badge_ipfs: Color::Rgb {
            r: 0x8b,
            g: 0xe9,
            b: 0xfd,
        },

        mode_normal: Color::Rgb {
            r: 0x50,
            g: 0xfa,
            b: 0x7b,
        },
        mode_prefix: Color::Rgb {
            r: 0xff,
            g: 0xb8,
            b: 0x6c,
        },
        mode_copy: Color::Rgb {
            r: 0x8b,
            g: 0xe9,
            b: 0xfd,
        },
        mode_visual: Color::Rgb {
            r: 0xff,
            g: 0x79,
            b: 0xc6,
        },
        mode_resize: Color::Rgb {
            r: 0xbd,
            g: 0x93,
            b: 0xf9,
        },
        mode_sync: Color::Rgb {
            r: 0xff,
            g: 0x55,
            b: 0x55,
        },
        mode_suspend: Color::Rgb {
            r: 0x62,
            g: 0x72,
            b: 0xa4,
        },
        mode_agent: Color::Rgb {
            r: 0xff,
            g: 0x79,
            b: 0xc6,
        },
    }
}

/// Resolves a theme name and optional custom overrides into an active `Theme`.
///
/// Matches `theme_name` case-insensitively and against kebab-case/snake-case variants.
/// If unrecognized, falls back to the canonical `Nord` theme.
pub fn resolve_theme(theme_name: &str, custom: Option<&CustomThemeConfig>) -> Theme {
    let normalized = theme_name.trim().to_ascii_lowercase().replace('_', "-");
    let mut theme = match normalized.as_str() {
        "nord" => nord(),
        "catppuccin" | "catppuccin-mocha" => catppuccin_mocha(),
        "tokyo-night" | "tokyonight" => tokyo_night(),
        "gruvbox" => gruvbox(),
        "dracula" => dracula(),
        _ => nord(),
    };

    if let Some(custom) = custom {
        apply_custom_overrides(&mut theme, custom);
    }

    theme
}

fn apply_override(target: &mut Color, custom: &CustomThemeConfig, key: &str) {
    if let Some(spec) = custom.get(key).and_then(|s| s.parse::<ColorSpec>().ok()) {
        *target = spec.to_crossterm();
    }
}

fn apply_custom_overrides(theme: &mut Theme, custom: &CustomThemeConfig) {
    apply_override(&mut theme.bg, custom, "bg");
    apply_override(&mut theme.fg, custom, "fg");
    apply_override(&mut theme.selection_bg, custom, "selection_bg");
    apply_override(&mut theme.selection_fg, custom, "selection_fg");
    apply_override(&mut theme.border, custom, "border");
    apply_override(&mut theme.muted, custom, "muted");

    apply_override(&mut theme.accent, custom, "accent");
    apply_override(&mut theme.error, custom, "error");
    apply_override(&mut theme.warning, custom, "warning");
    apply_override(&mut theme.success, custom, "success");
    apply_override(&mut theme.info, custom, "info");

    apply_override(&mut theme.badge_path, custom, "badge_path");
    apply_override(&mut theme.badge_url, custom, "badge_url");
    apply_override(&mut theme.badge_hash, custom, "badge_hash");
    apply_override(&mut theme.badge_ip, custom, "badge_ip");
    apply_override(&mut theme.badge_command, custom, "badge_command");
    apply_override(&mut theme.badge_word, custom, "badge_word");
    apply_override(&mut theme.badge_docker, custom, "badge_docker");
    apply_override(&mut theme.badge_k8s, custom, "badge_k8s");
    apply_override(&mut theme.badge_color, custom, "badge_color");
    apply_override(&mut theme.badge_ipfs, custom, "badge_ipfs");

    apply_override(&mut theme.mode_normal, custom, "mode_normal");
    apply_override(&mut theme.mode_prefix, custom, "mode_prefix");
    apply_override(&mut theme.mode_copy, custom, "mode_copy");
    apply_override(&mut theme.mode_visual, custom, "mode_visual");
    apply_override(&mut theme.mode_resize, custom, "mode_resize");
    apply_override(&mut theme.mode_sync, custom, "mode_sync");
    apply_override(&mut theme.mode_suspend, custom, "mode_suspend");
    apply_override(&mut theme.mode_agent, custom, "mode_agent");
}
