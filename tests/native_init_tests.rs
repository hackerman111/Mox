use mox::keymap::build_default_keymap;
use mox::tmux::{apply_tmux_commands, execute_tmux, generate_init_script};

struct Server(String);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = execute_tmux(Some(&self.0), &["kill-server"]);
    }
}
#[test]
fn repeated_init_keeps_scroll_rules_and_one_status_component() {
    let server = Server(format!("mox-init-{}", std::process::id()));
    let socket = Some(server.0.as_str());
    execute_tmux(
        socket,
        &["-f", "/dev/null", "new-session", "-d", "-s", "test"],
    )
    .expect("tmux must start for integration check");
    execute_tmux(socket, &["set", "-g", "@mox_scroll_line", "^(mytui)$"]).unwrap();
    execute_tmux(socket, &["set", "-g", "@mox_autorestore", "off"]).unwrap();
    for _ in 0..2 {
        let current = execute_tmux(socket, &["show", "-gv", "status-left"]).unwrap();
        let script = generate_init_script(
            &build_default_keymap(),
            "M-m",
            env!("CARGO_BIN_EXE_mox"),
            Some(&current),
        );
        apply_tmux_commands(socket, &script).unwrap();
    }
    assert_eq!(
        execute_tmux(socket, &["show", "-gqv", "@mox_scroll_line"]).unwrap(),
        "^(mytui)$"
    );
    let status = execute_tmux(socket, &["show", "-gv", "status-left"]).unwrap();
    assert_eq!(status.matches("#{E:@mox_indicator}").count(), 1);
    assert_eq!(status.matches("#{E:@mox_autosave}").count(), 1);
    let bindings = execute_tmux(socket, &["list-keys", "-T", "root"]).unwrap();
    assert!(bindings.contains("navigate L"));
    assert!(bindings.contains("@mox_scroll_line"));
    let suspend = execute_tmux(socket, &["list-keys", "-T", "mox_suspend"]).unwrap();
    assert!(suspend.contains("Any"));
    assert!(suspend.contains("M-z"));
    let indicator =
        execute_tmux(socket, &["display-message", "-p", "#{E:@mox_indicator}"]).unwrap();
    assert!(indicator.contains("NORMAL"), "{indicator}");
    execute_tmux(socket, &["set", "-w", "synchronize-panes", "on"]).unwrap();
    let indicator =
        execute_tmux(socket, &["display-message", "-p", "#{E:@mox_indicator}"]).unwrap();
    assert!(indicator.contains("SYNC"), "{indicator}");
}

#[test]
fn init_quotes_binary_path_in_tmux_shell_commands() {
    let server = Server(format!("mox-init-quotes-{}", std::process::id()));
    let socket = Some(server.0.as_str());
    execute_tmux(
        socket,
        &["-f", "/dev/null", "new-session", "-d", "-s", "test"],
    )
    .expect("tmux must start for quoting check");
    execute_tmux(socket, &["set", "-g", "@mox_autorestore", "off"]).unwrap();

    let bin_path = "/tmp/mox's $PATH dir/mox";
    let script = generate_init_script(&build_default_keymap(), "M-m", bin_path, Some("#S"));
    let shell_path = mox::tmux::shell_quote(bin_path);
    for command in [
        format!("{shell_path} which-key"),
        format!("{shell_path} tea"),
        format!("{shell_path} nav"),
        format!("{shell_path} agent-toggle"),
        format!("{shell_path} agent-create"),
        format!("{shell_path} extract --launch-popup"),
    ] {
        let expected = mox::tmux::quote(&command);
        assert!(
            script.iter().any(|line| line.contains(&expected)),
            "missing tmux-quoted command: {command}"
        );
    }
    apply_tmux_commands(socket, &script).expect("quoted paths must source as tmux config");

    let keys = execute_tmux(socket, &["list-keys", "-a"]).expect("list-keys must work");
    assert!(keys.contains("$PATH"), "binary path was expanded: {keys}");
    assert!(
        keys.contains(r"mox'\\''s \$PATH dir/mox"),
        "apostrophe was lost: {keys}"
    );
    assert!(
        keys.contains("which-key"),
        "submenu command missing: {keys}"
    );
    assert!(
        keys.contains("agent-toggle"),
        "agent command missing: {keys}"
    );
    assert!(
        keys.contains("--launch-popup"),
        "flash/extract command missing: {keys}"
    );
}

#[test]
fn init_script_with_catppuccin_mocha_theme() {
    let config = mox::config::Config {
        theme: "catppuccin-mocha".to_string(),
        ..Default::default()
    };

    let script = mox::tmux::generate_init_script_with_config(
        &build_default_keymap(),
        "M-m",
        "/usr/local/bin/mox",
        None,
        &config,
    );

    assert!(
        script
            .iter()
            .any(|l| l == "set -g status-style \"fg=#cdd6f4,bg=#1e1e2e\""),
        "missing status-style for catppuccin-mocha"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g popup-style \"fg=#cdd6f4,bg=#1e1e2e\""),
        "missing popup-style for catppuccin-mocha"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-border-style \"fg=#45475a,bg=default\""),
        "missing pane-border-style for catppuccin-mocha"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-active-border-style \"fg=#89b4fa,bg=default\""),
        "missing pane-active-border-style for catppuccin-mocha"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g message-style \"fg=#1e1e2e,bg=#89b4fa,bold\""),
        "missing message-style for catppuccin-mocha"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g clock-mode-colour \"#89b4fa\""),
        "missing clock-mode-colour for catppuccin-mocha"
    );
}

#[test]
fn init_script_with_tokyo_night_theme() {
    let config = mox::config::Config {
        theme: "tokyo-night".to_string(),
        ..Default::default()
    };

    let script = mox::tmux::generate_init_script_with_config(
        &build_default_keymap(),
        "M-m",
        "/usr/local/bin/mox",
        None,
        &config,
    );

    assert!(
        script
            .iter()
            .any(|l| l == "set -g status-style \"fg=#c0caf5,bg=#1a1b26\""),
        "missing status-style for tokyo-night"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g popup-style \"fg=#c0caf5,bg=#1a1b26\""),
        "missing popup-style for tokyo-night"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-border-style \"fg=#29a4bd,bg=default\""),
        "missing pane-border-style for tokyo-night"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-active-border-style \"fg=#7aa2f7,bg=default\""),
        "missing pane-active-border-style for tokyo-night"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g message-style \"fg=#1a1b26,bg=#7aa2f7,bold\""),
        "missing message-style for tokyo-night"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g clock-mode-colour \"#7aa2f7\""),
        "missing clock-mode-colour for tokyo-night"
    );
}

#[test]
fn init_script_with_dracula_theme() {
    let config = mox::config::Config {
        theme: "dracula".to_string(),
        ..Default::default()
    };

    let script = mox::tmux::generate_init_script_with_config(
        &build_default_keymap(),
        "M-m",
        "/usr/local/bin/mox",
        None,
        &config,
    );

    assert!(
        script
            .iter()
            .any(|l| l == "set -g status-style \"fg=#f8f8f2,bg=#282a36\""),
        "missing status-style for dracula"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g popup-style \"fg=#f8f8f2,bg=#282a36\""),
        "missing popup-style for dracula"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-border-style \"fg=#6272a4,bg=default\""),
        "missing pane-border-style for dracula"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-active-border-style \"fg=#bd93f9,bg=default\""),
        "missing pane-active-border-style for dracula"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g message-style \"fg=#282a36,bg=#bd93f9,bold\""),
        "missing message-style for dracula"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g clock-mode-colour \"#bd93f9\""),
        "missing clock-mode-colour for dracula"
    );
}

#[test]
fn init_script_with_custom_theme() {
    let config = mox::config::Config {
        theme: "custom".to_string(),
        custom_theme: Some(mox::config::CustomThemeConfig {
            bg: Some("#112233".to_string()),
            fg: Some("#445566".to_string()),
            accent: Some("#778899".to_string()),
            border: Some("#aabbcc".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let script = mox::tmux::generate_init_script_with_config(
        &build_default_keymap(),
        "M-m",
        "/usr/local/bin/mox",
        None,
        &config,
    );

    assert!(
        script
            .iter()
            .any(|l| l == "set -g status-style \"fg=#445566,bg=#112233\""),
        "missing status-style for custom theme"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g popup-style \"fg=#445566,bg=#112233\""),
        "missing popup-style for custom theme"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-border-style \"fg=#aabbcc,bg=default\""),
        "missing pane-border-style for custom theme"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g pane-active-border-style \"fg=#778899,bg=default\""),
        "missing pane-active-border-style for custom theme"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g message-style \"fg=#112233,bg=#778899,bold\""),
        "missing message-style for custom theme"
    );
    assert!(
        script
            .iter()
            .any(|l| l == "set -g clock-mode-colour \"#778899\""),
        "missing clock-mode-colour for custom theme"
    );
}

#[test]
fn init_applies_theme_to_live_tmux_server() {
    let server = Server(format!("mox-init-theme-live-{}", std::process::id()));
    let socket = Some(server.0.as_str());
    execute_tmux(
        socket,
        &["-f", "/dev/null", "new-session", "-d", "-s", "test"],
    )
    .expect("tmux must start for live theme application check");

    let config = mox::config::Config {
        theme: "catppuccin-mocha".to_string(),
        ..Default::default()
    };

    let script = mox::tmux::generate_init_script_with_config(
        &build_default_keymap(),
        "M-m",
        env!("CARGO_BIN_EXE_mox"),
        None,
        &config,
    );
    apply_tmux_commands(socket, &script).expect("applying theme script must succeed");

    let status_style = execute_tmux(socket, &["show", "-gv", "status-style"]).unwrap();
    assert!(
        status_style.contains("fg=#cdd6f4") && status_style.contains("bg=#1e1e2e"),
        "unexpected status-style: {status_style}"
    );

    let clock = execute_tmux(socket, &["show", "-gv", "clock-mode-colour"]).unwrap();
    assert_eq!(clock.trim(), "#89b4fa");

    let pane_border = execute_tmux(socket, &["show", "-gv", "pane-border-style"]).unwrap();
    assert!(
        pane_border.contains("fg=#45475a"),
        "unexpected pane-border: {pane_border}"
    );

    let pane_active = execute_tmux(socket, &["show", "-gv", "pane-active-border-style"]).unwrap();
    assert!(
        pane_active.contains("fg=#89b4fa"),
        "unexpected pane-active-border: {pane_active}"
    );

    let status_left = execute_tmux(socket, &["show", "-gv", "status-left"]).unwrap();
    assert_eq!(status_left.matches("#{E:@mox_indicator}").count(), 1);
}
