//! Canonical file-location parsing and transport to registered Neovim instances.
pub mod files;
use crate::tmux::{execute_tmux, shell_quote};
use crate::ui::{
    picker::{PickerItem, run_picker},
    preview::Preview,
};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub const LUA: &str = include_str!("../../lua/mox.lua");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
}
impl Location {
    pub fn parse(value: &str) -> Result<Self, String> {
        let mut path = value;
        let mut numbers = Vec::new();
        for _ in 0..2 {
            let Some((head, tail)) = path.rsplit_once(':') else {
                break;
            };
            if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) {
                break;
            }
            let number: usize = tail.parse().map_err(|_| "Location number overflow")?;
            if number == 0 {
                return Err("Line and column must be positive".into());
            }
            numbers.push(number);
            path = head;
        }
        if path.is_empty() || path.contains('\0') {
            return Err("Empty or invalid file path".into());
        }
        let (line, column) = match numbers.as_slice() {
            [line] => (Some(*line), None),
            [column, line] => (Some(*line), Some(*column)),
            _ => (None, None),
        };
        Ok(Self {
            path: PathBuf::from(path),
            line,
            column,
        })
    }
}
pub fn register(socket: Option<&str>, pane: &str, server: &str, pid: u32) -> Result<(), String> {
    if !valid_server_name(server) || pid == 0 {
        return Err("Invalid Neovim endpoint".into());
    }
    let root = execute_tmux(
        socket,
        &["display-message", "-p", "-t", pane, "#{pane_pid}"],
    )?;
    if !descendant(pid, root.parse().map_err(|_| "Invalid pane PID")?) {
        return Err("Neovim does not belong to this pane".into());
    }
    execute_tmux(
        socket,
        &["set-option", "-p", "-t", pane, "@mox_nvim_server", server],
    )?;
    execute_tmux(
        socket,
        &[
            "set-option",
            "-p",
            "-t",
            pane,
            "@mox_nvim_pid",
            &pid.to_string(),
        ],
    )?;
    Ok(())
}
fn descendant(mut pid: u32, root: u32) -> bool {
    for _ in 0..64 {
        if pid == root {
            return true;
        }
        if pid <= 1 {
            return false;
        }
        let mut command = Command::new("ps");
        command.args(["-o", "ppid=", "-p", &pid.to_string()]);
        let Ok(out) = crate::process::output(&mut command, Duration::from_millis(200), 4096) else {
            return false;
        };
        let Ok(parent) = String::from_utf8_lossy(&out.stdout).trim().parse() else {
            return false;
        };
        pid = parent;
    }
    false
}
#[derive(Debug)]
struct Editor {
    pane: String,
    pane_pid: u32,
    server: String,
    pid: u32,
    label: String,
}
fn editors(socket: Option<&str>) -> Result<Vec<Editor>, String> {
    let raw = execute_tmux(
        socket,
        &[
            "list-panes",
            "-a",
            "-F",
            "#{pane_id}\t#{pane_current_command}\t#{pane_pid}\t#{@mox_nvim_server}\t#{@mox_nvim_pid}\t#{pane_current_path}",
        ],
    )?;
    let mut found = Vec::new();
    for line in raw.lines() {
        let fields = line.splitn(6, '\t').collect::<Vec<_>>();
        if fields.len() != 6 || fields[1] != "nvim" {
            continue;
        }
        if fields[3].is_empty() {
            return Err(format!(
                "Neovim in {} has no Mox bridge; load `mox editor setup` in its init.lua",
                fields[0]
            ));
        }
        let pane_pid = fields[2].parse().map_err(|_| "Invalid tmux pane PID")?;
        let pid = fields[4]
            .parse()
            .map_err(|_| "Invalid registered Neovim PID")?;
        found.push(Editor {
            pane: fields[0].into(),
            pane_pid,
            server: fields[3].into(),
            pid,
            label: format!("{} · {}", fields[0], fields[5]),
        });
    }
    Ok(found)
}
fn remote(server: &str, expression: &str) -> Result<String, String> {
    let mut command = Command::new("nvim");
    command.args(["--server", server, "--remote-expr", expression]);
    let out = crate::process::output(&mut command, Duration::from_secs(2), 65536)?;
    if !out.status.success() {
        return Err(format!(
            "Neovim RPC: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().into())
}
pub fn open(
    socket: Option<&str>,
    source_pane: Option<&str>,
    mut location: Location,
) -> Result<(), String> {
    let cwd = match source_pane {
        Some(pane) => execute_tmux(
            socket,
            &["display-message", "-p", "-t", pane, "#{pane_current_path}"],
        )?,
        None => execute_tmux(socket, &["display-message", "-p", "#{pane_current_path}"])?,
    };
    if !location.path.is_absolute() {
        location.path = PathBuf::from(&cwd).join(location.path);
    }
    let candidates = editors(socket)?;
    let selected = match candidates.len() {
        0 => None,
        1 => Some(0),
        _ => {
            let items = candidates
                .iter()
                .map(|e| PickerItem {
                    id: e.pane.clone(),
                    label: e.label.clone(),
                    detail: e.server.clone(),
                    preview: Preview::Pane(e.pane.clone()),
                })
                .collect::<Vec<_>>();
            let Some(index) = run_picker("Choose Neovim", &items, socket)? else {
                return Ok(());
            };
            Some(index)
        }
    };
    if let Some(index) = selected {
        let editor = &candidates[index];
        let current = execute_tmux(
            socket,
            &[
                "display-message",
                "-p",
                "-t",
                &editor.pane,
                "#{pane_current_command}\t#{pane_pid}\t#{@mox_nvim_server}\t#{@mox_nvim_pid}",
            ],
        )?;
        if !endpoint_matches(&current, editor) || !descendant(editor.pid, editor.pane_pid) {
            return Err("Stale Neovim endpoint".into());
        }
        if remote(&editor.server, "getpid()")? != editor.pid.to_string() {
            return Err("Stale Neovim endpoint".into());
        }
        let args = serde_json::to_string(&(
            location.path.to_str().ok_or("Non-UTF8 filename")?,
            location.line.unwrap_or(1),
            location.column.unwrap_or(1),
        ))
        .map_err(|e| e.to_string())?;
        // JSON is a Vim single-quoted string, never an Ex command or filename interpolation.
        let expression = format!(
            "luaeval('require(\"mox\").open(_A)', json_decode('{}'))",
            args.replace('\'', "''")
        );
        // The embedded bridge provides a global so no runtime Lua package path is required.
        let expression = expression.replace("require(\"mox\").open", "_G.Mox.open");
        remote(&editor.server, &expression)?;
        execute_tmux(socket, &["select-pane", "-t", &editor.pane])?;
        execute_tmux(socket, &["select-window", "-t", &editor.pane])?;
        execute_tmux(socket, &["switch-client", "-t", &editor.pane])?;
    } else {
        let bootstrap = format!("lua {LUA}");
        let cmd = format!(
            "nvim --cmd {} +{} -- {}",
            shell_quote(&bootstrap),
            shell_quote(&format!(
                "call cursor({},{})",
                location.line.unwrap_or(1),
                location.column.unwrap_or(1)
            )),
            shell_quote(location.path.to_str().ok_or("Non-UTF8 filename")?)
        );
        execute_tmux(socket, &["new-window", "-n", "editor", "-c", &cwd, &cmd])?;
    }
    Ok(())
}

fn valid_server_name(server: &str) -> bool {
    !server.is_empty() && !server.chars().any(char::is_control)
}

fn endpoint_matches(value: &str, editor: &Editor) -> bool {
    let mut fields = value.split('\t');
    fields.next() == Some("nvim")
        && fields.next().and_then(|pid| pid.parse::<u32>().ok()) == Some(editor.pane_pid)
        && fields.next() == Some(editor.server.as_str())
        && fields.next().and_then(|pid| pid.parse::<u32>().ok()) == Some(editor.pid)
        && fields.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        Editor {
            pane: "%4".into(),
            pane_pid: 120,
            server: "/tmp/nvim-120.sock".into(),
            pid: 121,
            label: String::new(),
        }
    }

    #[test]
    fn registered_endpoint_must_match_current_pane_and_process() {
        let editor = editor();
        assert!(endpoint_matches(
            "nvim\t120\t/tmp/nvim-120.sock\t121",
            &editor
        ));
        assert!(!endpoint_matches(
            "nvim\t120\t/tmp/other.sock\t121",
            &editor
        ));
        assert!(!endpoint_matches(
            "nvim\t999\t/tmp/nvim-120.sock\t121",
            &editor
        ));
        assert!(!endpoint_matches(
            "nvim\t120\t/tmp/nvim-120.sock\t999",
            &editor
        ));
        assert!(!endpoint_matches(
            "vim\t120\t/tmp/nvim-120.sock\t121",
            &editor
        ));
    }

    #[test]
    fn server_names_reject_tmux_record_delimiters() {
        assert!(valid_server_name("/tmp/nvim.sock"));
        assert!(!valid_server_name(""));
        assert!(!valid_server_name("/tmp/nvim.sock\nother"));
        assert!(!valid_server_name("/tmp/nvim\tsock"));
    }
}
