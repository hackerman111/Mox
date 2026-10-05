use mox::config::{
    ClockConfig, Config, CustomThemeConfig, ModeIcons, ModeIndicatorConfig, ModeLabels,
    SegmentConfig, SegmentKind, SeparatorStyle, StatusConfig, WindowsConfig,
};

#[test]
fn test_default_config_from_empty_toml() {
    let toml_str = "";
    let config: Config =
        toml::from_str(toml_str).expect("Empty TOML should deserialize with defaults");

    assert_eq!(config, Config::default());
    assert_eq!(config.status, StatusConfig::default());
    assert_eq!(config.clock, ClockConfig::default());
    assert_eq!(config.mode_indicator, ModeIndicatorConfig::default());
    assert_eq!(config.mode_indicator.labels, ModeLabels::default());
    assert_eq!(config.mode_indicator.icons, ModeIcons::default());
    assert_eq!(config.windows, WindowsConfig::default());
    assert_eq!(config.theme, "nord");

    // Status defaults
    assert!(config.status.enable);
    assert_eq!(config.status.position, "bottom");
    assert_eq!(config.status.separator_style, SeparatorStyle::Powerline);
    assert_eq!(config.status.left[0], SegmentConfig::new(SegmentKind::Mode));
    assert_eq!(
        config.status.left[1],
        SegmentConfig::new(SegmentKind::Session)
    );
    assert_eq!(
        config.status.right[0],
        SegmentConfig::new(SegmentKind::Date)
    );
    assert_eq!(
        config.status.right[1],
        SegmentConfig::new(SegmentKind::Clock)
    );
    assert_eq!(
        config.status.right[2],
        SegmentConfig::new(SegmentKind::Host)
    );

    // Clock defaults
    assert_eq!(config.clock.format, "%H:%M");
    assert_eq!(config.clock.date_format, "%Y-%m-%d");
    assert!(!config.clock.style_12h);
    assert!(config.clock.show_date);
    assert!(config.clock.nerd_icons);
    assert!(!config.clock.pills);

    // Mode indicator defaults
    assert_eq!(config.mode_indicator.labels.normal, "NORMAL");
    assert_eq!(config.mode_indicator.labels.prefix, "PREFIX");
    assert_eq!(config.mode_indicator.labels.copy, "COPY");
    assert_eq!(config.mode_indicator.labels.visual, "VISUAL");
    assert_eq!(config.mode_indicator.labels.resize, "RESIZE");
    assert_eq!(config.mode_indicator.labels.sync, "SYNC");
    assert_eq!(config.mode_indicator.labels.suspend, "SUSPEND");
    assert_eq!(config.mode_indicator.labels.agent, "AGENT");

    assert_eq!(config.mode_indicator.icons.normal, "●");
    assert_eq!(config.mode_indicator.icons.prefix, "◆");
    assert_eq!(config.mode_indicator.icons.copy, "⎘");
    assert_eq!(config.mode_indicator.icons.visual, "◈");
    assert_eq!(config.mode_indicator.icons.resize, "↕");
    assert_eq!(config.mode_indicator.icons.sync, "⇄");
    assert_eq!(config.mode_indicator.icons.suspend, "⏸");
    assert_eq!(config.mode_indicator.icons.agent, "▲");

    // Windows defaults
    assert_eq!(config.windows.style, "powerline");
    assert!(config.windows.show_flags);

    // Custom theme default is None
    assert!(config.custom_theme.is_none());
    assert_eq!(CustomThemeConfig::default().colors.len(), 0);
}

#[test]
fn test_custom_config_deserialization() {
    let toml_str = r##"
theme = "catppuccin-mocha"

[status]
enable = false
position = "top"
separator_style = "rounded"
left = [
    { kind = "mode" },
    { kind = "custom", command = "neoamp tmux", when = "has_neoamp" }
]
right = [
    { kind = "clock", format = "%I:%M %p", icon = "clock_icon" }
]

[clock]
format = "%I:%M %p"
date_format = "%d.%m.%Y"
style_12h = true
show_date = false
nerd_icons = false
pills = true

[mode_indicator.labels]
normal = "NORM"
prefix = "PREF"

[mode_indicator.icons]
normal = "N"
prefix = "P"

[windows]
style = "flat"
show_flags = false

[custom_theme]
bg = "#112233"
fg = "#aabbcc"
accent = "#ff007f"
border = "#334455"
"##;

    let config: Config =
        toml::from_str(toml_str).expect("Custom TOML should deserialize correctly");

    assert_eq!(config.theme, "catppuccin-mocha");

    assert!(!config.status.enable);
    assert_eq!(config.status.position, "top");
    assert_eq!(config.status.separator_style, SeparatorStyle::Rounded);
    assert_eq!(config.status.left.len(), 2);
    assert_eq!(config.status.left[0].kind, SegmentKind::Mode);
    assert_eq!(config.status.left[1].kind, SegmentKind::Custom);
    assert_eq!(
        config.status.left[1].command.as_deref(),
        Some("neoamp tmux")
    );
    assert_eq!(config.status.left[1].when.as_deref(), Some("has_neoamp"));

    assert_eq!(config.status.right.len(), 1);
    assert_eq!(config.status.right[0].kind, SegmentKind::Clock);
    assert_eq!(config.status.right[0].format.as_deref(), Some("%I:%M %p"));
    assert_eq!(config.status.right[0].icon.as_deref(), Some("clock_icon"));

    assert_eq!(config.clock.format, "%I:%M %p");
    assert_eq!(config.clock.date_format, "%d.%m.%Y");
    assert!(config.clock.style_12h);
    assert!(!config.clock.show_date);
    assert!(!config.clock.nerd_icons);
    assert!(config.clock.pills);

    // Overridden mode indicators
    assert_eq!(config.mode_indicator.labels.normal, "NORM");
    assert_eq!(config.mode_indicator.labels.prefix, "PREF");
    // Non-overridden mode indicators retain defaults
    assert_eq!(config.mode_indicator.labels.copy, "COPY");
    assert_eq!(config.mode_indicator.labels.agent, "AGENT");

    assert_eq!(config.mode_indicator.icons.normal, "N");
    assert_eq!(config.mode_indicator.icons.prefix, "P");
    assert_eq!(config.mode_indicator.icons.copy, "⎘");
    assert_eq!(config.mode_indicator.icons.agent, "▲");

    assert_eq!(config.windows.style, "flat");
    assert!(!config.windows.show_flags);

    let custom = config.custom_theme.expect("Custom theme should be present");
    assert_eq!(custom.bg.as_deref(), Some("#112233"));
    assert_eq!(custom.fg.as_deref(), Some("#aabbcc"));
    assert_eq!(custom.accent.as_deref(), Some("#ff007f"));
    assert_eq!(custom.get("border"), Some("#334455"));
}

#[test]
fn test_custom_theme_aliases() {
    let toml_str = r##"
[theme_custom]
bg = "#000000"
fg = "#ffffff"
"##;
    let config: Config = toml::from_str(toml_str).expect("theme_custom alias should be accepted");
    let custom = config
        .custom_theme
        .expect("Custom theme should be present via alias");
    assert_eq!(custom.bg.as_deref(), Some("#000000"));
    assert_eq!(custom.fg.as_deref(), Some("#ffffff"));
}

#[test]
fn test_separator_styles() {
    let styles = [
        ("powerline", SeparatorStyle::Powerline),
        ("rounded", SeparatorStyle::Rounded),
        ("slanted", SeparatorStyle::Slanted),
        ("flat", SeparatorStyle::Flat),
        ("none", SeparatorStyle::None),
    ];

    for (name, expected) in styles {
        let toml_str = format!("separator_style = \"{}\"", name);
        #[derive(serde::Deserialize)]
        struct S {
            separator_style: SeparatorStyle,
        }
        let parsed: S = toml::from_str(&toml_str).expect("Should parse separator style");
        assert_eq!(parsed.separator_style, expected);
    }
}

#[test]
fn test_segment_kinds() {
    let kinds = [
        ("mode", SegmentKind::Mode),
        ("session", SegmentKind::Session),
        ("windows", SegmentKind::Windows),
        ("date", SegmentKind::Date),
        ("clock", SegmentKind::Clock),
        ("host", SegmentKind::Host),
        ("custom", SegmentKind::Custom),
    ];

    for (name, expected) in kinds {
        let toml_str = format!("kind = \"{}\"", name);
        #[derive(serde::Deserialize)]
        struct S {
            kind: SegmentKind,
        }
        let parsed: S = toml::from_str(&toml_str).expect("Should parse segment kind");
        assert_eq!(parsed.kind, expected);
    }
}

#[test]
fn test_config_roundtrip_serialization() {
    let original = Config::default();
    let serialized = toml::to_string(&original).expect("Serialization should succeed");
    let deserialized: Config = toml::from_str(&serialized).expect("Deserialization should succeed");
    assert_eq!(original, deserialized);
}
