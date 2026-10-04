//! Versioned, atomic session snapshots and conservative ID-remapped restoration.
mod restore;
use crate::tmux::execute_tmux;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u32,
    pub active_session: Option<String>,
    pub sessions: Vec<Session>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub name: String,
    pub windows: Vec<Window>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub index: u32,
    pub name: String,
    pub layout: String,
    pub active: bool,
    pub zoom: bool,
    pub panes: Vec<Pane>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    pub id: String,
    pub index: u32,
    pub cwd: PathBuf,
    pub active: bool,
    pub command: String,
    pub argv: Vec<String>,
}

pub fn default_path(socket: Option<&str>) -> Result<PathBuf, String> {
    let implicit = std::env::var("TMUX")
        .ok()
        .and_then(|value| value.split(',').next().map(str::to_owned))
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
        });
    let name = socket.or(implicit.as_deref()).unwrap_or("default");
    // Hex encoding keeps distinct socket names distinct and never produces path separators.
    let suffix = name
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(crate::state::directory()?.join(format!("snapshot-{suffix}.json")))
}
pub fn capture(socket: Option<&str>) -> Result<Snapshot, String> {
    let format = "#{session_name}\t#{window_index}\t#{window_name}\t#{window_layout}\t#{window_active}\t#{window_zoomed_flag}\t#{pane_id}\t#{pane_index}\t#{pane_current_path}\t#{pane_active}\t#{pane_current_command}\t#{@mox_nvim_pid}";
    let raw = crate::tmux::execute_tmux_raw(socket, &["list-panes", "-a", "-F", format])?;
    let active_session = execute_tmux(socket, &["display-message", "-p", "#{session_name}"]).ok();
    let mut grouped: BTreeMap<String, BTreeMap<u32, Window>> = BTreeMap::new();
    for line in raw.lines() {
        let f = line.split('\t').collect::<Vec<_>>();
        if f.len() != 12 {
            return Err("Snapshot contains an unsupported tab/newline in metadata".into());
        }
        let index = f[1].parse().map_err(|_| "Invalid window index")?;
        let window = grouped
            .entry(f[0].into())
            .or_default()
            .entry(index)
            .or_insert_with(|| Window {
                index,
                name: f[2].into(),
                layout: f[3].into(),
                active: f[4] == "1",
                zoom: f[5] == "1",
                panes: Vec::new(),
            });
        let argv = if f[10] == "nvim" {
            f[11].parse().ok().and_then(read_argv).unwrap_or_default()
        } else if f[10] == "lazygit" {
            vec!["lazygit".into()]
        } else {
            Vec::new()
        };
        window.panes.push(Pane {
            id: f[6].into(),
            index: f[7].parse().map_err(|_| "Invalid pane index")?,
            cwd: f[8].into(),
            active: f[9] == "1",
            command: f[10].into(),
            argv,
        });
    }
    let snapshot = Snapshot {
        version: 1,
        active_session,
        sessions: grouped
            .into_iter()
            .map(|(name, windows)| Session {
                name,
                windows: windows.into_values().collect(),
            })
            .collect(),
    };
    validate(&snapshot)?;
    Ok(snapshot)
}
fn read_argv(pid: u32) -> Option<Vec<String>> {
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if raw.len() > 65536 {
        return None;
    }
    let argv = raw
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if argv
        .first()
        .and_then(|s| Path::new(s).file_name())
        .is_some_and(|s| s == "nvim")
    {
        Some(argv)
    } else {
        None
    }
}
pub fn validate(snapshot: &Snapshot) -> Result<(), String> {
    if snapshot.version != 1 || snapshot.sessions.is_empty() || snapshot.sessions.len() > 1000 {
        return Err("Unsupported or empty snapshot".into());
    }
    let mut names = std::collections::HashSet::new();
    let mut pane_ids = std::collections::HashMap::new();
    let mut shared: std::collections::HashMap<&str, &Window> = std::collections::HashMap::new();
    for session in &snapshot.sessions {
        if session.name.is_empty()
            || session.name.contains([':', '\0', '\n', '\t'])
            || !names.insert(&session.name)
            || session.windows.is_empty()
            || session.windows.len() > 1000
        {
            return Err("Invalid/duplicate session".into());
        }
        let mut windows = std::collections::HashSet::new();
        for window in &session.windows {
            if !windows.insert(window.index)
                || window.panes.is_empty()
                || window.panes.len() > 1000
                || window.layout.len() > 65536
            {
                return Err("Invalid/duplicate window".into());
            }
            let first = window.panes[0].id.as_str();
            if let Some(other) = shared.insert(first, window)
                && (other.panes != window.panes
                    || other.layout != window.layout
                    || other.name != window.name
                    || other.zoom != window.zoom)
            {
                return Err("Inconsistent linked window metadata".into());
            }
            let mut local = std::collections::HashSet::new();
            let mut local_ids = std::collections::HashSet::new();
            for pane in &window.panes {
                if !pane
                    .id
                    .strip_prefix('%')
                    .is_some_and(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
                    || !local.insert(pane.index)
                    || !local_ids.insert(&pane.id)
                    || pane_ids
                        .insert(&pane.id, first)
                        .is_some_and(|owner| owner != first)
                    || !pane.cwd.is_absolute()
                    || pane.argv.iter().any(|v| v.contains('\0'))
                {
                    return Err("Invalid pane metadata".into());
                }
            }
        }
    }
    Ok(())
}
pub fn save(socket: Option<&str>, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let _lock = crate::state::lock(&path.with_extension("lock"))?;
    let snapshot = capture(socket)?;
    let data = serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?;
    crate::state::atomic_write(path, &data)
}
pub fn read(path: &Path) -> Result<Snapshot, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Snapshot too large".into());
    }
    let snapshot = serde_json::from_slice(&bytes).map_err(|e| format!("Invalid snapshot: {e}"))?;
    validate(&snapshot)?;
    Ok(snapshot)
}
pub fn restore(socket: Option<&str>, path: &Path, commands: bool) -> Result<(), String> {
    let _lock = crate::state::lock(&path.with_extension("lock"))?;
    restore::apply(socket, &read(path)?, commands)
}
pub fn autosave(socket: Option<&str>, path: &Path, interval: u64) -> Result<(), String> {
    if path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_secs() < interval.max(10))
    {
        return Ok(());
    }
    save(socket, path)
}
pub fn autorestore(socket: Option<&str>, path: &Path) -> Result<(), String> {
    let _lock = crate::state::lock(&path.with_extension("lock"))?;
    let done = execute_tmux(socket, &["show-option", "-gqv", "@mox_restore_done"])?;
    if done == "1" {
        return Ok(());
    }
    if path.exists() {
        restore::apply(socket, &read(path)?, false)?;
    }
    execute_tmux(socket, &["set-option", "-g", "@mox_restore_done", "1"])?;
    Ok(())
}
