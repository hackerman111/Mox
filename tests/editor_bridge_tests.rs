use mox::editor::LUA;
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("mox-editor-{}-{stamp}", std::process::id()))
}

fn run_nvim(bootstrap: &str, tail: &str, state: &std::path::Path) -> std::process::Output {
    let mut command = Command::new("nvim");
    command
        .args([
            "--headless",
            "-u",
            "NONE",
            "--cmd",
            bootstrap,
            "-c",
            tail,
            "-c",
            "qa!",
        ])
        .env("XDG_STATE_HOME", state.join("state"))
        .env("XDG_CACHE_HOME", state.join("cache"))
        .env("XDG_CONFIG_HOME", state.join("config"))
        .env("NVIM_LOG_FILE", state.join("nvim.log"));
    command
        .output()
        .expect("nvim must be installed for bridge tests")
}

#[test]
fn embedded_bootstrap_runs_past_lua_line_comment() {
    let dir = temp_dir();
    fs::create_dir_all(&dir).unwrap();
    let bootstrap = format!("lua {LUA}");
    let output = run_nvim(
        &bootstrap,
        "lua print('MOX_BRIDGE=' .. tostring(type(_G.Mox.open)))",
        &dir,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "status {:?}; stdout {stdout:?}; stderr {stderr:?}",
        output.status.code()
    );
    assert!(
        format!("{stdout}{stderr}").contains("MOX_BRIDGE=function"),
        "stdout: {stdout:?}; stderr: {stderr:?}"
    );
}

#[test]
fn bridge_clamps_cursor_to_existing_lines_and_line_bytes() {
    let dir = temp_dir();
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sample.txt");
    fs::write(&file, "abcde\nx").unwrap();
    let encoded_path = serde_json::to_string(file.to_str().unwrap()).unwrap();
    let bootstrap = format!("lua {LUA}");
    let tail = format!(
        "lua _G.Mox.open({{{encoded_path}, 999, 999}}); local c = vim.api.nvim_win_get_cursor(0); print('CURSOR=' .. c[1] .. ',' .. c[2])"
    );
    let output = run_nvim(&bootstrap, &tail, &dir);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "status {:?}; stdout {stdout:?}; stderr {stderr:?}",
        output.status.code()
    );
    assert!(
        format!("{stdout}{stderr}").contains("CURSOR=2,0"),
        "stdout: {stdout:?}; stderr: {stderr:?}"
    );
}
