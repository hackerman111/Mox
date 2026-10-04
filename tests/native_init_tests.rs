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
