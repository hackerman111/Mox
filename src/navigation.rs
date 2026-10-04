//! Smart pane navigation and process-sensitive scroll configuration.
use crate::tmux::{execute_tmux, shell_quote};

pub fn navigate(
    socket: Option<&str>,
    pane: Option<&str>,
    direction: &str,
    from_editor: bool,
) -> Result<(), String> {
    let (option, key) = match direction {
        "L" | "h" => ("-L", "C-h"),
        "D" | "j" => ("-D", "C-j"),
        "U" | "k" => ("-U", "C-k"),
        "R" | "l" => ("-R", "C-l"),
        _ => return Err("Direction must be L/D/U/R or h/j/k/l".into()),
    };
    let target = match pane {
        Some(pane) => pane.to_owned(),
        None => execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])?,
    };
    let command = execute_tmux(
        socket,
        &[
            "display-message",
            "-p",
            "-t",
            &target,
            "#{pane_current_command}",
        ],
    )?;
    if command == "nvim" && !from_editor {
        execute_tmux(socket, &["send-keys", "-t", &target, key])?;
    } else {
        execute_tmux(socket, &["select-pane", "-t", &target, option])?;
    }
    Ok(())
}
pub fn suspend(socket: Option<&str>, client: Option<&str>, resume: bool) -> Result<(), String> {
    let mut args = vec!["switch-client"];
    if let Some(client) = client {
        args.extend(["-c", client]);
    }
    args.extend(["-T", if resume { "root" } else { "mox_suspend" }]);
    execute_tmux(socket, &args).map(|_| ())
}
pub fn bindings(binary: &str) -> Vec<String> {
    let mut lines = vec![
        "bind-key -n M-z switch-client -T mox_suspend".into(),
        "bind-key -T mox_suspend M-z switch-client -T root".into(),
        "bind-key -T mox_suspend Any { send-keys ; switch-client -T mox_suspend }".into(),
        "bind-key -T mox_suspend WheelUpPane { send-keys -M ; switch-client -T mox_suspend }"
            .into(),
        "bind-key -T mox_suspend WheelDownPane { send-keys -M ; switch-client -T mox_suspend }"
            .into(),
    ];
    for (key, direction) in [("C-h", "L"), ("C-j", "D"), ("C-k", "U"), ("C-l", "R")] {
        let command = format!(
            "{} navigate {direction} --pane '#{{pane_id}}'",
            shell_quote(binary)
        );
        lines.push(format!(
            "bind-key -n {key} run-shell -b {}",
            shell_quote(&command)
        ));
    }
    lines
}
pub fn scroll_bindings() -> Vec<String> {
    let mut lines = vec![
        "if-shell -F '#{==:#{@mox_scroll_line},}' { set -g @mox_scroll_line '^(fzf|man|less|pager)$' }".into(),
        "if-shell -F '#{==:#{@mox_scroll_page},}' { set -g @mox_scroll_page '^(irssi)$' }".into(),
        "if-shell -F '#{==:#{@mox_scroll_mouse},}' { set -g @mox_scroll_mouse '^(nvim|vim)$' }".into(),
    ];
    for (wheel, key, page) in [
        ("WheelUpPane", "Up", "PageUp"),
        ("WheelDownPane", "Down", "PageDown"),
    ] {
        let history = if wheel == "WheelUpPane" {
            "copy-mode -e -t '{mouse}' ; send-keys -X -t '{mouse}' -N 3 scroll-up"
        } else {
            "send-keys -M"
        };
        // Entire wheel dispatch stays inside tmux: no process launch per mouse event.
        let body = format!(
            "if-shell -F -t '{{mouse}}' '#{{||:#{{pane_in_mode}},#{{||:#{{mouse_any_flag}},#{{m/r:#{{@mox_scroll_mouse}},#{{pane_current_command}}}}}}}}' {{ send-keys -M }} {{ if-shell -F -t '{{mouse}}' '#{{m/r:#{{@mox_scroll_line}},#{{pane_current_command}}}}' {{ send-keys -t '{{mouse}}' {key} }} {{ if-shell -F -t '{{mouse}}' '#{{m/r:#{{@mox_scroll_page}},#{{pane_current_command}}}}' {{ send-keys -t '{{mouse}}' {page} }} {{ {history} }} }} }}"
        );
        lines.push(format!("bind-key -n {wheel} {body}"));
    }
    lines
}
