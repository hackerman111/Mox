use crossterm::style::Color;
use mox::config::theme::{
    ColorSpec, catppuccin_mocha, dracula, gruvbox, nord, resolve_theme, tokyo_night,
};
use mox::config::{Config, CustomThemeConfig};
use mox::ui::theme::Theme;
use std::str::FromStr;

#[test]
fn test_colorspec_hex_parsing() {
    // 6-digit hex
    let spec = ColorSpec::from_str("#2e3440").expect("Valid 6-digit hex");
    assert_eq!(
        spec.to_crossterm(),
        Color::Rgb {
            r: 0x2e,
            g: 0x34,
            b: 0x40
        }
    );
    assert_eq!(spec.to_tmux(), "#2e3440");

    let spec = ColorSpec::from_str("#88c0d0").expect("Valid 6-digit hex");
    assert_eq!(
        spec.to_crossterm(),
        Color::Rgb {
            r: 0x88,
            g: 0xc0,
            b: 0xd0
        }
    );
    assert_eq!(spec.to_tmux(), "#88c0d0");

    // Case-insensitive 6-digit hex
    let spec = ColorSpec::from_str("#88C0D0").expect("Case-insensitive hex");
    assert_eq!(
        spec.to_crossterm(),
        Color::Rgb {
            r: 0x88,
            g: 0xc0,
            b: 0xd0
        }
    );
    assert_eq!(spec.to_tmux(), "#88c0d0");

    // 3-digit short hex
    let spec = ColorSpec::from_str("#fff").expect("Valid 3-digit hex");
    assert_eq!(
        spec.to_crossterm(),
        Color::Rgb {
            r: 255,
            g: 255,
            b: 255
        }
    );
    assert_eq!(spec.to_tmux(), "#ffffff");

    let spec = ColorSpec::from_str("#123").expect("Valid 3-digit hex");
    assert_eq!(
        spec.to_crossterm(),
        Color::Rgb {
            r: 0x11,
            g: 0x22,
            b: 0x33
        }
    );
    assert_eq!(spec.to_tmux(), "#112233");
}

#[test]
fn test_colorspec_named_ansi_parsing() {
    let cases = [
        ("black", Color::Black, "black"),
        ("red", Color::Red, "red"),
        ("green", Color::Green, "green"),
        ("yellow", Color::Yellow, "yellow"),
        ("blue", Color::Blue, "blue"),
        ("magenta", Color::Magenta, "magenta"),
        ("cyan", Color::Cyan, "cyan"),
        ("white", Color::White, "white"),
        ("brightblack", Color::DarkGrey, "brightblack"),
        ("brightred", Color::AnsiValue(9), "brightred"),
        ("brightgreen", Color::AnsiValue(10), "brightgreen"),
        ("brightyellow", Color::AnsiValue(11), "brightyellow"),
        ("brightblue", Color::AnsiValue(12), "brightblue"),
        ("brightmagenta", Color::AnsiValue(13), "brightmagenta"),
        ("brightcyan", Color::AnsiValue(14), "brightcyan"),
        ("brightwhite", Color::AnsiValue(15), "brightwhite"),
    ];

    for (input, expected_ct, expected_tmux) in cases {
        let spec =
            ColorSpec::from_str(input).unwrap_or_else(|e| panic!("Failed to parse {input}: {e}"));
        assert_eq!(
            spec.to_crossterm(),
            expected_ct,
            "crossterm mismatch for {input}"
        );
        assert_eq!(spec.to_tmux(), expected_tmux, "tmux mismatch for {input}");
    }

    // Variations in casing and delimiters
    let spec = ColorSpec::from_str("bright_black").expect("bright_black");
    assert_eq!(spec.to_crossterm(), Color::DarkGrey);
    assert_eq!(spec.to_tmux(), "brightblack");

    let spec = ColorSpec::from_str("BRIGHT-CYAN").expect("BRIGHT-CYAN");
    assert_eq!(spec.to_crossterm(), Color::AnsiValue(14));
    assert_eq!(spec.to_tmux(), "brightcyan");
}

#[test]
fn test_colorspec_256_colors() {
    let cases = [
        ("colour236", 236),
        ("color236", 236),
        ("236", 236),
        ("colour12", 12),
        ("color12", 12),
        ("12", 12),
        ("colour0", 0),
        ("0", 0),
        ("colour255", 255),
        ("255", 255),
    ];

    for (input, index) in cases {
        let spec =
            ColorSpec::from_str(input).unwrap_or_else(|e| panic!("Failed to parse {input}: {e}"));
        assert_eq!(spec.to_crossterm(), Color::AnsiValue(index));
        assert_eq!(spec.to_tmux(), format!("colour{index}"));
    }
}

#[test]
fn test_colorspec_default() {
    let spec = ColorSpec::from_str("default").expect("default");
    assert_eq!(spec.to_crossterm(), Color::Reset);
    assert_eq!(spec.to_tmux(), "default");

    let spec = ColorSpec::from_str("none").expect("none");
    assert_eq!(spec.to_crossterm(), Color::Reset);
    assert_eq!(spec.to_tmux(), "default");
}

#[test]
fn test_colorspec_invalid_inputs() {
    assert!(ColorSpec::from_str("#12").is_err());
    assert!(ColorSpec::from_str("#12345").is_err());
    assert!(ColorSpec::from_str("#gggggg").is_err());
    assert!(ColorSpec::from_str("256").is_err());
    assert!(ColorSpec::from_str("colour256").is_err());
    assert!(ColorSpec::from_str("invalid_color_xyz").is_err());
}

#[test]
fn test_colorspec_serde_roundtrip() {
    let json = serde_json::to_string(&ColorSpec::from_str("#2e3440").unwrap()).unwrap();
    assert_eq!(json, "\"#2e3440\"");
    let decoded: ColorSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.to_tmux(), "#2e3440");

    let json = serde_json::to_string(&ColorSpec::from_str("colour12").unwrap()).unwrap();
    assert_eq!(json, "\"colour12\"");
    let decoded: ColorSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.to_tmux(), "colour12");

    let json = serde_json::to_string(&ColorSpec::from_str("black").unwrap()).unwrap();
    assert_eq!(json, "\"black\"");
    let decoded: ColorSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.to_tmux(), "black");
}

#[test]
fn test_preset_palettes() {
    // Nord
    let nord_theme = nord();
    assert_eq!(nord_theme, Theme::nord());
    assert_eq!(resolve_theme("nord", None), Theme::nord());

    // Catppuccin Mocha
    let cat_theme = catppuccin_mocha();
    assert_eq!(resolve_theme("catppuccin-mocha", None), cat_theme);
    assert_eq!(resolve_theme("catppuccin_mocha", None), cat_theme);
    assert_eq!(
        cat_theme.bg,
        Color::Rgb {
            r: 0x1e,
            g: 0x1e,
            b: 0x2e
        }
    );
    assert_eq!(
        cat_theme.fg,
        Color::Rgb {
            r: 0xcd,
            g: 0xd6,
            b: 0xf4
        }
    );
    assert_eq!(
        cat_theme.selection_bg,
        Color::Rgb {
            r: 0x31,
            g: 0x32,
            b: 0x44
        }
    );
    assert_eq!(
        cat_theme.border,
        Color::Rgb {
            r: 0x45,
            g: 0x47,
            b: 0x5a
        }
    );
    assert_eq!(
        cat_theme.accent,
        Color::Rgb {
            r: 0x89,
            g: 0xb4,
            b: 0xfa
        }
    );
    assert_eq!(
        cat_theme.error,
        Color::Rgb {
            r: 0xf3,
            g: 0x8b,
            b: 0xa8
        }
    );
    assert_eq!(
        cat_theme.warning,
        Color::Rgb {
            r: 0xf9,
            g: 0xe2,
            b: 0xaf
        }
    );
    assert_eq!(
        cat_theme.success,
        Color::Rgb {
            r: 0xa6,
            g: 0xe3,
            b: 0xa1
        }
    );

    // Tokyo Night
    let tokyo_theme = tokyo_night();
    assert_eq!(resolve_theme("tokyo-night", None), tokyo_theme);
    assert_eq!(resolve_theme("tokyo_night", None), tokyo_theme);
    assert_eq!(
        tokyo_theme.bg,
        Color::Rgb {
            r: 0x1a,
            g: 0x1b,
            b: 0x26
        }
    );
    assert_eq!(
        tokyo_theme.fg,
        Color::Rgb {
            r: 0xc0,
            g: 0xca,
            b: 0xf5
        }
    );
    assert_eq!(
        tokyo_theme.selection_bg,
        Color::Rgb {
            r: 0x28,
            g: 0x34,
            b: 0x57
        }
    );
    assert_eq!(
        tokyo_theme.border,
        Color::Rgb {
            r: 0x29,
            g: 0xa4,
            b: 0xbd
        }
    );
    assert_eq!(
        tokyo_theme.accent,
        Color::Rgb {
            r: 0x7a,
            g: 0xa2,
            b: 0xf7
        }
    );
    assert_eq!(
        tokyo_theme.error,
        Color::Rgb {
            r: 0xf7,
            g: 0x76,
            b: 0x8e
        }
    );
    assert_eq!(
        tokyo_theme.warning,
        Color::Rgb {
            r: 0xe0,
            g: 0xaf,
            b: 0x68
        }
    );
    assert_eq!(
        tokyo_theme.success,
        Color::Rgb {
            r: 0x9e,
            g: 0xce,
            b: 0x6a
        }
    );
    assert_eq!(
        tokyo_theme.info,
        Color::Rgb {
            r: 0x7d,
            g: 0xcf,
            b: 0xff
        }
    );

    // Gruvbox
    let gruv_theme = gruvbox();
    assert_eq!(resolve_theme("gruvbox", None), gruv_theme);
    assert_eq!(
        gruv_theme.bg,
        Color::Rgb {
            r: 0x28,
            g: 0x28,
            b: 0x28
        }
    );
    assert_eq!(
        gruv_theme.fg,
        Color::Rgb {
            r: 0xeb,
            g: 0xdb,
            b: 0xb2
        }
    );
    assert_eq!(
        gruv_theme.selection_bg,
        Color::Rgb {
            r: 0x3c,
            g: 0x38,
            b: 0x36
        }
    );
    assert_eq!(
        gruv_theme.border,
        Color::Rgb {
            r: 0x50,
            g: 0x49,
            b: 0x45
        }
    );
    assert_eq!(
        gruv_theme.accent,
        Color::Rgb {
            r: 0xfe,
            g: 0x80,
            b: 0x19
        }
    );
    assert_eq!(
        gruv_theme.error,
        Color::Rgb {
            r: 0xfb,
            g: 0x49,
            b: 0x34
        }
    );
    assert_eq!(
        gruv_theme.warning,
        Color::Rgb {
            r: 0xfa,
            g: 0xbd,
            b: 0x2f
        }
    );
    assert_eq!(
        gruv_theme.success,
        Color::Rgb {
            r: 0xb8,
            g: 0xbb,
            b: 0x26
        }
    );
    assert_eq!(
        gruv_theme.info,
        Color::Rgb {
            r: 0x83,
            g: 0xa5,
            b: 0x98
        }
    );

    // Dracula
    let drac_theme = dracula();
    assert_eq!(resolve_theme("dracula", None), drac_theme);
    assert_eq!(resolve_theme("DRACULA", None), drac_theme);
    assert_eq!(
        drac_theme.bg,
        Color::Rgb {
            r: 0x28,
            g: 0x2a,
            b: 0x36
        }
    );
    assert_eq!(
        drac_theme.fg,
        Color::Rgb {
            r: 0xf8,
            g: 0xf8,
            b: 0xf2
        }
    );
    assert_eq!(
        drac_theme.selection_bg,
        Color::Rgb {
            r: 0x44,
            g: 0x47,
            b: 0x5a
        }
    );
    assert_eq!(
        drac_theme.border,
        Color::Rgb {
            r: 0x62,
            g: 0x72,
            b: 0xa4
        }
    );
    assert_eq!(
        drac_theme.accent,
        Color::Rgb {
            r: 0xbd,
            g: 0x93,
            b: 0xf9
        }
    );
    assert_eq!(
        drac_theme.error,
        Color::Rgb {
            r: 0xff,
            g: 0x55,
            b: 0x55
        }
    );
    assert_eq!(
        drac_theme.warning,
        Color::Rgb {
            r: 0xff,
            g: 0xb8,
            b: 0x6c
        }
    );
    assert_eq!(
        drac_theme.success,
        Color::Rgb {
            r: 0x50,
            g: 0xfa,
            b: 0x7b
        }
    );
    assert_eq!(
        drac_theme.info,
        Color::Rgb {
            r: 0x8b,
            g: 0xe9,
            b: 0xfd
        }
    );

    // Unknown defaults to Nord
    assert_eq!(resolve_theme("nonexistent-theme", None), Theme::nord());
}

#[test]
fn test_custom_theme_overrides() {
    let mut custom = CustomThemeConfig {
        accent: Some("#ff79c6".to_string()),
        border: Some("colour12".to_string()),
        ..Default::default()
    };
    custom
        .colors
        .insert("mode_normal".to_string(), "green".to_string());

    let resolved = resolve_theme("dracula", Some(&custom));
    // Overridden fields
    assert_eq!(
        resolved.accent,
        Color::Rgb {
            r: 0xff,
            g: 0x79,
            b: 0xc6
        }
    );
    assert_eq!(resolved.border, Color::AnsiValue(12));
    assert_eq!(resolved.mode_normal, Color::Green);

    // Non-overridden fields keep Dracula base values
    assert_eq!(
        resolved.bg,
        Color::Rgb {
            r: 0x28,
            g: 0x2a,
            b: 0x36
        }
    );
    assert_eq!(
        resolved.fg,
        Color::Rgb {
            r: 0xf8,
            g: 0xf8,
            b: 0xf2
        }
    );
    assert_eq!(
        resolved.error,
        Color::Rgb {
            r: 0xff,
            g: 0x55,
            b: 0x55
        }
    );
}

#[test]
fn test_theme_from_config() {
    let default_config = Config::default();
    let theme = Theme::from_config(&default_config);
    assert_eq!(theme, Theme::nord());

    let toml_str = r##"
theme = "tokyo-night"

[custom_theme]
accent = "#ff007f"
"##;
    let config: Config = toml::from_str(toml_str).expect("Valid config TOML");
    let theme = Theme::from_config(&config);
    assert_eq!(
        theme.bg,
        Color::Rgb {
            r: 0x1a,
            g: 0x1b,
            b: 0x26
        }
    );
    assert_eq!(
        theme.accent,
        Color::Rgb {
            r: 0xff,
            g: 0x00,
            b: 0x7f
        }
    );
}
