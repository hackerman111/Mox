use mox::config::{
    Config, SeparatorStyle, apply_tmux_overrides, apply_tmux_server_overrides, default_config_path,
    default_toml, load_config, parse_config_str,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("mox-loader-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("failed to create temp test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_parse_config_str_valid() {
    let toml = r#"
theme = "tokyo-night"

[status]
enable = true
position = "top"
separator_style = "slanted"

[clock]
format = "%I:%M %p"
date_format = "%d/%m/%Y"
style_12h = true
show_date = false
nerd_icons = false
pills = true
"#;

    let config = parse_config_str(toml).expect("valid TOML should parse");
    assert_eq!(config.theme, "tokyo-night");
    assert!(config.status.enable);
    assert_eq!(config.status.position, "top");
    assert_eq!(config.status.separator_style, SeparatorStyle::Slanted);
    assert_eq!(config.clock.format, "%I:%M %p");
    assert_eq!(config.clock.date_format, "%d/%m/%Y");
    assert!(config.clock.style_12h);
    assert!(!config.clock.show_date);
    assert!(!config.clock.nerd_icons);
    assert!(config.clock.pills);
}

#[test]
fn test_parse_config_str_invalid() {
    let toml = "theme = [unclosed string";
    assert!(parse_config_str(toml).is_err());
}

#[test]
fn test_load_config_from_valid_file() {
    let temp = TempDir::new("valid");
    let file_path = temp.path().join("config.toml");
    let toml = r#"
theme = "gruvbox"

[status]
separator_style = "rounded"
"#;
    fs::write(&file_path, toml).expect("failed to write test config");

    let config = load_config(Some(&file_path));
    assert_eq!(config.theme, "gruvbox");
    assert_eq!(config.status.separator_style, SeparatorStyle::Rounded);
}

#[test]
fn test_load_config_nonexistent_explicit_path() {
    let temp = TempDir::new("nonexistent");
    let missing_path = temp.path().join("does_not_exist.toml");

    let config = load_config(Some(&missing_path));
    assert_eq!(config, Config::default());
}

#[test]
fn test_load_config_invalid_syntax_explicit_path() {
    let temp = TempDir::new("invalid");
    let file_path = temp.path().join("config.toml");
    fs::write(&file_path, "invalid = [[[syntax").expect("failed to write test config");

    let config = load_config(Some(&file_path));
    assert_eq!(config, Config::default());
}

#[test]
fn test_load_config_none_explicit_path() {
    // Calling load_config(None) should return a Config instance (default if file is not found)
    let config = load_config(None);
    // Config should have a valid theme string
    assert!(!config.theme.is_empty());
}

#[test]
fn test_default_config_path_resolution() {
    let path = default_config_path();
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.trim().is_empty()
    {
        let expected = PathBuf::from(xdg).join("mox").join("config.toml");
        assert_eq!(path, Some(expected));
    } else if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        let expected = PathBuf::from(home)
            .join(".config")
            .join("mox")
            .join("config.toml");
        assert_eq!(path, Some(expected));
    }
}

#[test]
fn test_apply_tmux_overrides_native_options() {
    let mut config = Config::default();

    let mut options = HashMap::new();
    options.insert("@mox_theme", "dracula".to_string());
    options.insert("@mox_separator_style", "slanted".to_string());
    options.insert("@mox_clock_format", "%H:%M:%S".to_string());
    options.insert("@mox_date_format", "%A, %B %d".to_string());
    options.insert("@mox_clock_12h", "1".to_string());

    apply_tmux_overrides(&mut config, |key| options.get(key).cloned());

    assert_eq!(config.theme, "dracula");
    assert_eq!(config.status.separator_style, SeparatorStyle::Slanted);
    assert_eq!(config.clock.format, "%H:%M:%S");
    assert_eq!(config.clock.date_format, "%A, %B %d");
    assert!(config.clock.style_12h);

    // Test remaining separator styles and bool variations
    let styles = [
        ("powerline", SeparatorStyle::Powerline),
        ("rounded", SeparatorStyle::Rounded),
        ("slanted", SeparatorStyle::Slanted),
        ("flat", SeparatorStyle::Flat),
        ("none", SeparatorStyle::None),
    ];
    for (name, expected) in styles {
        let mut cfg = Config::default();
        apply_tmux_overrides(&mut cfg, |key| {
            if key == "@mox_separator_style" {
                Some(name.to_string())
            } else {
                None
            }
        });
        assert_eq!(cfg.status.separator_style, expected);
    }

    // Test @mox_clock_12h = "true"
    let mut cfg = Config::default();
    apply_tmux_overrides(&mut cfg, |key| {
        if key == "@mox_clock_12h" {
            Some("true".to_string())
        } else {
            None
        }
    });
    assert!(cfg.clock.style_12h);
}

#[test]
fn test_apply_tmux_overrides_nord_compat() {
    let mut config = Config::default();

    let mut options = HashMap::new();
    options.insert("@nord_tmux_show_status_content", "0".to_string());
    options.insert("@nord_tmux_no_patched_font", "1".to_string());
    options.insert("@nord_tmux_date_format", "%d.%m.%Y".to_string());
    options.insert("clock-mode-style", "12".to_string());

    apply_tmux_overrides(&mut config, |key| options.get(key).cloned());

    assert!(!config.status.enable);
    assert_eq!(config.status.separator_style, SeparatorStyle::Flat);
    assert_eq!(config.clock.date_format, "%d.%m.%Y");
    assert!(config.clock.style_12h);
}

#[test]
fn test_apply_tmux_overrides_mode_indicator_compat() {
    let mut config = Config::default();

    let mut options = HashMap::new();
    options.insert("@mode_indicator_prefix_prompt", " WAIT ".to_string());
    options.insert("@mode_indicator_copy_prompt", " COPY ".to_string());
    options.insert("@mode_indicator_sync_prompt", " SYNC ".to_string());
    options.insert("@mode_indicator_empty_prompt", " TMUX ".to_string());

    apply_tmux_overrides(&mut config, |key| options.get(key).cloned());

    assert_eq!(config.mode_indicator.labels.prefix, "WAIT");
    assert_eq!(config.mode_indicator.labels.copy, "COPY");
    assert_eq!(config.mode_indicator.labels.sync, "SYNC");
    assert_eq!(config.mode_indicator.labels.normal, "TMUX");
}

#[test]
fn test_apply_tmux_overrides_precedence() {
    let mut config = Config::default();

    let mut options = HashMap::new();
    // Legacy Nord options
    options.insert("@nord_tmux_no_patched_font", "1".to_string());
    options.insert("@nord_tmux_date_format", "%d.%m.%Y".to_string());
    options.insert("clock-mode-style", "12".to_string());

    // Native Mox options (higher precedence)
    options.insert("@mox_separator_style", "rounded".to_string());
    options.insert("@mox_date_format", "%Y/%m/%d".to_string());
    options.insert("@mox_clock_12h", "0".to_string());

    apply_tmux_overrides(&mut config, |key| options.get(key).cloned());

    // Native Mox overrides should win
    assert_eq!(config.status.separator_style, SeparatorStyle::Rounded);
    assert_eq!(config.clock.date_format, "%Y/%m/%d");
    assert!(!config.clock.style_12h);
}

#[test]
fn test_default_toml_parses_cleanly_and_matches_default() {
    let toml = default_toml();
    assert!(!toml.is_empty(), "default_toml should not be empty");

    let parsed: Config =
        parse_config_str(toml).expect("default_toml must parse cleanly into Config");
    assert_eq!(
        parsed,
        Config::default(),
        "default_toml must match Config::default()"
    );
}

#[test]
fn test_apply_tmux_server_overrides_graceful() {
    let mut config = Config::default();
    // Nonexistent socket should not panic or corrupt config
    apply_tmux_server_overrides(&mut config, Some("nonexistent_mox_test_socket_9999"));
    assert_eq!(config, Config::default());
}

#[test]
fn test_apply_tmux_overrides_status_position() {
    let mut config = Config::default();
    assert_eq!(config.status.position, "bottom");

    let mut options = HashMap::new();
    options.insert("status-position", "top".to_string());
    apply_tmux_overrides(&mut config, |key| options.get(key).cloned());
    assert_eq!(config.status.position, "top");

    // @mox_status_position has precedence over status-position
    options.insert("@mox_status_position", "bottom".to_string());
    apply_tmux_overrides(&mut config, |key| options.get(key).cloned());
    assert_eq!(config.status.position, "bottom");
}
