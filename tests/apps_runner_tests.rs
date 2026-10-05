use mox::apps::model::{AppConfig, AppLaunchMode};
use mox::apps::{check_binary, run_app};

#[test]
fn test_check_binary_available_and_missing() {
    assert!(check_binary("sh").is_ok());

    let err = check_binary("nonexistent_binary_xyz_123").unwrap_err();
    assert!(
        err.contains("nonexistent_binary_xyz_123 is unavailable:"),
        "expected error to mention binary unavailable, got: {err}"
    );
}

#[test]
fn test_run_app_nonexistent_working_directory() {
    let app = AppConfig {
        name: "test_cwd".to_string(),
        title: "Test Cwd".to_string(),
        command: "sh".to_string(),
        key: None,
        mode: AppLaunchMode::Window,
        focus_existing: false,
        check_binary: false,
    };
    let nonexistent = "/path/to/nonexistent/working/directory/xyz_987";
    let result = run_app(None, &app, Some(nonexistent), false);
    assert_eq!(
        result,
        Err(format!("working directory does not exist: {nonexistent}"))
    );
}

#[test]
fn test_run_app_missing_binary() {
    let app = AppConfig {
        name: "test_missing".to_string(),
        title: "Test Missing".to_string(),
        command: "nonexistent_binary_xyz_123".to_string(),
        key: None,
        mode: AppLaunchMode::Window,
        focus_existing: false,
        check_binary: true,
    };
    let temp_dir = std::env::temp_dir();
    let result = run_app(None, &app, Some(temp_dir.to_str().unwrap()), false);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("nonexistent_binary_xyz_123 is unavailable:"),
        "expected error to mention binary unavailable, got: {err}"
    );
}
