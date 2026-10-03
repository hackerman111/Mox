//! Tmux CLI command execution client.

use std::process::Command;

/// Execute a tmux command with optional socket name.
pub fn execute_tmux(socket: Option<&str>, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("tmux");
    if let Some(s) = socket {
        cmd.args(["-L", s]);
    }
    cmd.args(args);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to spawn tmux: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux command failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Applies a list of tmux configuration lines via `source-file -`.
pub fn apply_tmux_commands(socket: Option<&str>, commands: &[String]) -> Result<(), String> {
    let mut cmd = Command::new("tmux");
    if let Some(s) = socket {
        cmd.args(["-L", s]);
    }
    cmd.args(["source-file", "-"]);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn tmux: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        for line in commands {
            if !line.trim().is_empty() {
                let _ = writeln!(stdin, "{line}");
            }
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for tmux: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux source-file failed: {stderr}"));
    }

    Ok(())
}
