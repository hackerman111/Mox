use mox::apps::model::{AppConfig, AppLaunchMode, TrackedWindow};
use mox::config::model::Config;

#[test]
fn test_default_config_includes_default_apps() {
    let config = Config::default();
    assert_eq!(config.apps.len(), 2);
    assert_eq!(config.apps[0].name, "lazygit");
    assert_eq!(config.apps[0].title, "LazyGit");
    assert_eq!(config.apps[0].command, "lazygit");
    assert_eq!(config.apps[0].key, Some("G".to_string()));
    assert_eq!(
        config.apps[0].mode,
        AppLaunchMode::Popup {
            width: "85%".to_string(),
            height: "85%".to_string(),
        }
    );
    assert!(config.apps[0].focus_existing);
    assert!(config.apps[0].check_binary);

    assert_eq!(config.apps[1].name, "btop");
    assert_eq!(config.apps[1].title, "btop");
    assert_eq!(config.apps[1].command, "btop");
    assert_eq!(config.apps[1].key, Some("B".to_string()));
    assert_eq!(
        config.apps[1].mode,
        AppLaunchMode::Popup {
            width: "85%".to_string(),
            height: "85%".to_string(),
        }
    );
    assert!(config.apps[1].focus_existing);
    assert!(config.apps[1].check_binary);
}

#[test]
fn test_parse_custom_app_config() {
    let toml = r#"
        [[apps]]
        name = "htop"
        title = "Htop Monitor"
        command = "htop"
        key = "h"
        mode = { type = "popup", width = "90%", height = "80%" }
        focus_existing = true
        check_binary = true
    "#;
    let config: Config = toml::from_str(toml).expect("valid toml");
    assert_eq!(config.apps.len(), 1);
    let app: &AppConfig = &config.apps[0];
    assert_eq!(app.name, "htop");
    assert_eq!(app.title, "Htop Monitor");
    assert_eq!(app.command, "htop");
    assert_eq!(app.key, Some("h".to_string()));
    assert_eq!(
        app.mode,
        AppLaunchMode::Popup {
            width: "90%".into(),
            height: "80%".into(),
        }
    );
    assert!(app.focus_existing);
    assert!(app.check_binary);
}

#[test]
fn test_app_config_direct_construction() {
    let app = AppConfig {
        name: "test".to_string(),
        title: "Test App".to_string(),
        command: "test-cmd".to_string(),
        key: None,
        mode: AppLaunchMode::Window,
        focus_existing: false,
        check_binary: false,
    };
    assert_eq!(app.name, "test");
    assert_eq!(app.mode, AppLaunchMode::Window);
    assert!(!app.focus_existing);
    assert!(!app.check_binary);
}

#[test]
fn test_tracked_window_model() {
    let window = TrackedWindow {
        id: "@1".to_string(),
        name: "editor".to_string(),
        session_id: "$0".to_string(),
        active: true,
    };
    assert_eq!(window.id, "@1");
    assert_eq!(window.name, "editor");
    assert_eq!(window.session_id, "$0");
    assert!(window.active);
}
