//! Tmux CLI command execution client.

use std::process::Command;

/// Execute a tmux command with optional socket name.
pub fn execute_tmux(socket: Option<&str>, args: &[&str]) -> Result<String, String> {
    execute_tmux_raw(socket, args).map(|output| output.trim().to_string())
}

/// Execute a tmux command without stripping coordinate-bearing capture output.
pub fn execute_tmux_raw(socket: Option<&str>, args: &[&str]) -> Result<String, String> {
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

    // Pane captures are coordinate data: leading blank rows and indentation matter.
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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
            if !line.trim().is_empty()
                && let Err(error) = writeln!(stdin, "{line}")
            {
                drop(stdin);
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("tmux configuration write failed: {error}"));
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

/// POSIX quoting for the few tmux interfaces that require a shell command.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Tmux source-file strings use their own quoting, distinct from shell commands.
pub fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

pub fn launch_after_popup(socket: Option<&str>, command: &str) -> Result<(), String> {
    execute_tmux(socket, &["set-option", "key-table", "root"])?;
    let mut tmux = "tmux".to_owned();
    if let Some(socket) = socket {
        tmux.push_str(&format!(" -L {}", shell_quote(socket)));
    }
    execute_tmux(
        socket,
        &[
            "run-shell",
            "-b",
            &format!("{tmux} display-popup -C; {command}"),
        ],
    )
    .map(|_| ())
}

pub fn preview_capture(socket: Option<&str>, pane: &str) -> Result<String, String> {
    let mut command = Command::new("tmux");
    if let Some(socket) = socket {
        command.args(["-L", socket]);
    }
    command.args(["capture-pane", "-p", "-t", pane]);
    let output =
        crate::process::output(&mut command, std::time::Duration::from_millis(500), 65536)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn load_buffer(socket: Option<&str>, text: &str, clipboard: bool) -> Result<(), String> {
    use std::io::Write;
    let mut cmd = Command::new("tmux");
    if let Some(socket) = socket {
        cmd.args(["-L", socket]);
    }
    cmd.arg("load-buffer");
    if clipboard {
        cmd.arg("-w");
    }
    let mut child = cmd
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let write = child
        .stdin
        .take()
        .ok_or("tmux stdin unavailable")?
        .write_all(text.as_bytes());
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    write.map_err(|e| e.to_string())
}

pub fn popup(
    socket: Option<&str>,
    title: &str,
    cwd: Option<&str>,
    command: &str,
) -> Result<(), String> {
    let mut args = vec![
        "display-popup",
        "-E",
        "-w",
        "85%",
        "-h",
        "85%",
        "-b",
        "rounded",
        "-T",
        title,
    ];
    if let Some(cwd) = cwd {
        args.extend(["-d", cwd]);
    }
    args.push(command);
    execute_tmux(socket, &args).map(|_| ())
}

/// Export captures directly to a temporary file; history is never buffered in Rust.
pub fn capture_to_file(
    socket: Option<&str>,
    pane: &str,
    path: &std::path::Path,
    full: bool,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_CAPTURE: AtomicU64 = AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = path.with_extension(format!(
        "capture-{}-{nonce}-{}",
        std::process::id(),
        NEXT_CAPTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&temporary)
        .map_err(|e| format!("{}: {e}", temporary.display()))?;
    let result = (|| {
        let mut cmd = Command::new("tmux");
        if let Some(socket) = socket {
            cmd.args(["-L", socket]);
        }
        cmd.args(["capture-pane", "-p", "-t", pane]);
        if full {
            cmd.args(["-S", "-"]);
        }
        let output = cmd.stdout(file).output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        std::fs::File::open(&temporary)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
