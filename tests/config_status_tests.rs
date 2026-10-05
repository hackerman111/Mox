use mox::config::status::{
    generate_mode_indicator, generate_status_left, generate_status_right, generate_window_status,
    generate_window_status_separator,
};
use mox::config::{Config, ModeIndicatorConfig, SegmentConfig, SegmentKind, SeparatorStyle};
use mox::ui::status::{replace_mode_indicator_placeholder, tmux_indicator_with_config};
use mox::ui::theme::Theme;

#[test]
fn test_generate_status_left_default_nord_powerline() {
    let config = Config::default();
    let theme = Theme::nord();

    let left = generate_status_left(&config, &theme);

    // Mode indicator badge must be present
    assert!(
        left.contains("NORMAL"),
        "Left status should include Mode indicator"
    );
    assert!(
        left.contains('●'),
        "Left status should include normal mode icon"
    );

    // Session segment should be present with #S
    assert!(
        left.contains("#S"),
        "Left status should include session segment"
    );

    // Powerline hard separator from Session (#88c0d0) to StatusBg (#2e3440)
    assert!(
        left.contains("#[fg=#88c0d0,bg=#2e3440]#[default]"),
        "Powerline transition from session accent to status background missing in: {left}"
    );

    // Session formatting: fg=#2e3440, bg=#88c0d0, bold
    assert!(
        left.contains("#[fg=#2e3440,bg=#88c0d0,bold] #S "),
        "Session segment formatting incorrect in: {left}"
    );
}

#[test]
fn test_generate_status_right_default_nord() {
    let config = Config::default();
    let theme = Theme::nord();

    let right = generate_status_right(&config, &theme);

    // Status background (#2e3440) to Date (#434c5e) transition
    assert!(
        right.contains("#[fg=#434c5e,bg=#2e3440]"),
        "Transition to date missing in: {right}"
    );

    // Date formatting with icon
    assert!(
        right.contains("󰃭 %Y-%m-%d"),
        "Date format with nerd icon missing in: {right}"
    );

    // Soft separator between Date and Clock (pills=false by default)
    assert!(
        right.contains(''),
        "Soft separator between date and clock missing in: {right}"
    );

    // Clock formatting with icon
    assert!(
        right.contains(" %H:%M"),
        "Clock format with nerd icon missing in: {right}"
    );

    // Clock (#434c5e) to Host (#88c0d0) transition
    assert!(
        right.contains("#[fg=#88c0d0,bg=#434c5e]"),
        "Transition from clock to host missing in: {right}"
    );

    // Host formatting with icon and bold accent
    assert!(
        right.contains("#[fg=#2e3440,bg=#88c0d0,bold] 󰒋 #H "),
        "Host segment formatting missing in: {right}"
    );
}

#[test]
fn test_separator_styles_rounded() {
    let mut config = Config::default();
    config.status.separator_style = SeparatorStyle::Rounded;
    let theme = Theme::nord();

    let left = generate_status_left(&config, &theme);
    // Rounded hard separator for status-left is 
    assert!(
        left.contains("#[fg=#88c0d0,bg=#2e3440]#[default]"),
        "Rounded status-left separator missing in: {left}"
    );

    let right = generate_status_right(&config, &theme);
    // Rounded hard separator for status-right is 
    assert!(
        right.contains("#[fg=#434c5e,bg=#2e3440]"),
        "Rounded status-right transition missing in: {right}"
    );
    // Rounded soft separator is (
    assert!(
        right.contains('('),
        "Rounded soft separator missing in: {right}"
    );
}

#[test]
fn test_separator_styles_flat() {
    let mut config = Config::default();
    config.status.separator_style = SeparatorStyle::Flat;
    let theme = Theme::nord();

    let left = generate_status_left(&config, &theme);
    assert!(left.contains('|'), "Flat separator missing in left: {left}");

    let right = generate_status_right(&config, &theme);
    assert!(
        right.contains('|'),
        "Flat separator missing in right: {right}"
    );
}

#[test]
fn test_clock_formats_12h_vs_24h() {
    let mut config_12h = Config::default();
    config_12h.clock.style_12h = true;
    let theme = Theme::nord();

    let right_12h = generate_status_right(&config_12h, &theme);
    assert!(
        right_12h.contains("%I:%M %p"),
        "12-hour clock format not used in: {right_12h}"
    );

    let mut config_custom = Config::default();
    config_custom.clock.style_12h = false;
    config_custom.clock.format = "%H:%M:%S".to_string();

    let right_custom = generate_status_right(&config_custom, &theme);
    assert!(
        right_custom.contains("%H:%M:%S"),
        "Custom 24h clock format not used in: {right_custom}"
    );
}

#[test]
fn test_clock_nerd_icons_enabled_vs_disabled() {
    let mut config = Config::default();
    config.clock.nerd_icons = false;
    let theme = Theme::nord();

    let right = generate_status_right(&config, &theme);
    // Should NOT contain default nerd icons
    assert!(!right.contains("󰃭"), "Date nerd icon should be absent");
    assert!(!right.contains(""), "Clock nerd icon should be absent");
    assert!(!right.contains("󰒋"), "Host nerd icon should be absent");

    // But should still contain formats
    assert!(right.contains("%Y-%m-%d"));
    assert!(right.contains("%H:%M"));
    assert!(right.contains("#H"));
}

#[test]
fn test_clock_pills_enabled_vs_disabled() {
    let theme = Theme::nord();

    // Default pills=false: shared background, soft separator
    let config_no_pills = Config::default();
    let right_no_pills = generate_status_right(&config_no_pills, &theme);
    assert!(
        right_no_pills.contains(''),
        "Soft separator expected when pills=false"
    );

    // pills=true: Date and Clock get distinct backgrounds and hard separator
    let mut config_pills = Config::default();
    config_pills.clock.pills = true;
    let right_pills = generate_status_right(&config_pills, &theme);

    // Hard right arrow between Date (#81a1c1 info) and Clock (#434c5e selection_bg)
    assert!(
        right_pills.contains("#[fg=#434c5e,bg=#81a1c1]"),
        "Hard separator between pills missing in: {right_pills}"
    );
}

#[test]
fn test_custom_mode_indicator_labels_and_icons() {
    let mut mode_config = ModeIndicatorConfig::default();
    mode_config.labels.normal = "DEF".to_string();
    mode_config.icons.normal = "N".to_string();
    mode_config.labels.prefix = "CMD".to_string();
    mode_config.icons.prefix = "P".to_string();

    let theme = Theme::nord();
    let indicator = tmux_indicator_with_config(&mode_config, &theme);

    assert!(
        indicator.contains("N DEF"),
        "Custom normal label/icon missing in: {indicator}"
    );
    assert!(
        indicator.contains("P CMD"),
        "Custom prefix label/icon missing in: {indicator}"
    );
}

#[test]
fn test_replace_mode_indicator_placeholder() {
    let custom_status = "left #[bold]#{tmux_mode_indicator}#[default] right";
    let badge = "#[fg=white,bg=blue] MODE #[default]";

    let replaced = replace_mode_indicator_placeholder(custom_status, badge);
    assert_eq!(
        replaced,
        "left #[bold]#[fg=white,bg=blue] MODE #[default]#[default] right"
    );

    let untouched = replace_mode_indicator_placeholder("regular string", badge);
    assert_eq!(untouched, "regular string");
}

#[test]
fn test_window_status_generation_powerline() {
    let config = Config::default();
    let theme = Theme::nord();

    // Inactive window
    let inactive = generate_window_status(&config, &theme, false);
    assert_eq!(
        inactive,
        "#[fg=#2e3440,bg=#434c5e] #[fg=#eceff4,bg=#434c5e]#I  #W#F #[fg=#434c5e,bg=#2e3440]"
    );

    // Current window
    let current = generate_window_status(&config, &theme, true);
    assert_eq!(
        current,
        "#[fg=#2e3440,bg=#88c0d0] #[fg=#2e3440,bg=#88c0d0,bold]#I  #W#F #[fg=#88c0d0,bg=#2e3440]"
    );

    // Separator between window status tabs
    assert_eq!(generate_window_status_separator(&config), "");
}

#[test]
fn test_window_status_generation_rounded() {
    let mut config = Config::default();
    config.windows.style = "rounded".to_string();
    let theme = Theme::nord();

    let inactive = generate_window_status(&config, &theme, false);
    assert_eq!(
        inactive,
        "#[fg=#434c5e,bg=#2e3440]#[fg=#eceff4,bg=#434c5e]#I #W#F#[fg=#434c5e,bg=#2e3440]"
    );

    let current = generate_window_status(&config, &theme, true);
    assert_eq!(
        current,
        "#[fg=#88c0d0,bg=#2e3440]#[fg=#2e3440,bg=#88c0d0,bold]#I #W#F#[fg=#88c0d0,bg=#2e3440]"
    );
}

#[test]
fn test_window_status_generation_flat() {
    let mut config = Config::default();
    config.windows.style = "flat".to_string();
    let theme = Theme::nord();

    let inactive = generate_window_status(&config, &theme, false);
    assert_eq!(inactive, " #[fg=#eceff4,bg=#434c5e]#I #W#F ");

    let current = generate_window_status(&config, &theme, true);
    assert_eq!(current, " #[fg=#2e3440,bg=#88c0d0,bold]#I #W#F ");
}

#[test]
fn test_window_status_show_flags_false() {
    let mut config = Config::default();
    config.windows.show_flags = false;
    let theme = Theme::nord();

    let inactive = generate_window_status(&config, &theme, false);
    assert_eq!(
        inactive,
        "#[fg=#2e3440,bg=#434c5e] #[fg=#eceff4,bg=#434c5e]#I  #W #[fg=#434c5e,bg=#2e3440]"
    );
}

#[test]
fn test_generate_mode_indicator() {
    let config = Config::default();
    let theme = Theme::nord();

    let indicator = generate_mode_indicator(&config, &theme);
    assert!(indicator.contains("NORMAL"));
    assert!(indicator.contains("PREFIX"));
}

#[test]
fn test_custom_segment_and_session_icon() {
    let mut config = Config::default();
    config.status.left = vec![
        SegmentConfig {
            kind: SegmentKind::Session,
            icon: Some("⚡".to_string()),
            format: None,
            command: None,
            when: None,
        },
        SegmentConfig {
            kind: SegmentKind::Custom,
            icon: None,
            format: None,
            command: Some("whoami".to_string()),
            when: None,
        },
    ];
    let theme = Theme::nord();

    let left = generate_status_left(&config, &theme);
    assert!(
        left.contains("⚡ #S"),
        "Custom session icon missing in: {left}"
    );
    assert!(
        left.contains("#(whoami)"),
        "Custom command segment missing in: {left}"
    );
}
