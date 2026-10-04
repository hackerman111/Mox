//! Project discovery and tmux session switching.

use crate::process::output;
use crate::tmux::execute_tmux;
use crate::ui::picker::{PickerItem, run_picker};
use crate::ui::preview::Preview;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const MAX_SCAN_DEPTH: usize = 4;
const MAX_SCAN_ENTRIES: usize = 5_000;
const MAX_RECENT: usize = 100;

#[derive(Debug, Clone)]
pub(crate) struct Project {
    path: PathBuf,
    session: Option<ExistingSession>,
}

#[derive(Debug, Clone)]
struct ExistingSession {
    id: String,
    name: String,
}

/// Discover projects, show them in the picker, then switch to or create a session.
pub(crate) fn discover(socket: Option<&str>, roots: &[PathBuf]) -> Result<Vec<Project>, String> {
    let sessions = existing_sessions(socket)?;
    let recent_path = recent_file()?;
    let recent = read_recent(&recent_path);
    let mut projects = Vec::new();
    let mut seen = HashSet::new();

    for session in &sessions {
        add_project(
            &mut projects,
            &mut seen,
            PathBuf::from(&session.cwd),
            Some(ExistingSession {
                id: session.id.clone(),
                name: session.name.clone(),
            }),
        );
    }
    for path in recent {
        add_project(&mut projects, &mut seen, path, None);
    }

    let roots = configured_roots(roots);
    for root in &roots {
        add_project(&mut projects, &mut seen, root.clone(), None);
    }
    for path in discover_git_repositories(&roots) {
        add_project(&mut projects, &mut seen, path, None);
    }

    for path in zoxide_directories() {
        add_project(&mut projects, &mut seen, path, None);
    }

    if let Ok(cwd) = std::env::current_dir() {
        add_project(&mut projects, &mut seen, cwd, None);
    }

    Ok(projects)
}

pub fn run(socket: Option<&str>, roots: &[PathBuf]) -> Result<(), String> {
    let projects = discover(socket, roots)?;
    let items: Vec<_> = projects.iter().map(item).collect();
    if let Some(index) = run_picker("Projects", &items, socket)? {
        activate(socket, &projects[index])?;
    }
    Ok(())
}

pub(crate) fn item(project: &Project) -> PickerItem {
    let label = match &project.session {
        Some(session) => format!("{} · {}", session.name, project.path.display()),
        None => project.path.display().to_string(),
    };
    PickerItem {
        id: project.path.to_string_lossy().into_owned(),
        label,
        detail: project.path.display().to_string(),
        preview: Preview::Directory(project.path.clone()),
    }
}

pub(crate) fn activate(socket: Option<&str>, project: &Project) -> Result<(), String> {
    if let Some(session) = &project.session {
        execute_tmux(socket, &["switch-client", "-t", &session.id])?;
    } else {
        open_project(socket, project)?;
    }
    remember(&recent_file()?, &project.path)
}

#[derive(Debug)]
pub struct SessionCwd {
    pub id: String,
    pub name: String,
    pub cwd: String,
    rank: usize,
}

fn existing_sessions(socket: Option<&str>) -> Result<Vec<SessionCwd>, String> {
    let format = "#{session_name}\t#{session_id}\t#{session_attached}\t#{window_active}\t#{pane_active}\t#{pane_current_path}";
    let raw = execute_tmux(socket, &["list-panes", "-a", "-F", format])?;
    Ok(parse_session_snapshot(&raw))
}

/// Parses the tmux pane snapshot, preferring each session's active pane cwd.
pub fn parse_session_snapshot(raw: &str) -> Vec<SessionCwd> {
    let mut sessions: Vec<SessionCwd> = Vec::new();
    let mut seen = HashSet::new();
    for line in raw.lines() {
        let fields: Vec<_> = line.splitn(6, '\t').collect();
        if fields.len() != 6 {
            continue;
        }
        let rank = usize::from(fields[3] == "1") * 2 + usize::from(fields[4] == "1");
        if seen.insert(fields[1].to_owned()) {
            sessions.push(SessionCwd {
                name: fields[0].to_owned(),
                id: fields[1].to_owned(),
                cwd: fields[5].to_owned(),
                rank,
            });
        } else if let Some(session) = sessions.iter_mut().find(|session| session.id == fields[1])
            && rank > session.rank
        {
            session.cwd = fields[5].to_owned();
            session.rank = rank;
        }
    }
    sessions
}

fn existing_session_names(socket: Option<&str>) -> HashSet<String> {
    let Ok(raw) = execute_tmux(socket, &["list-sessions", "-F", "#{session_name}"]) else {
        return HashSet::new();
    };
    raw.lines().map(str::to_owned).collect()
}

fn open_project(socket: Option<&str>, project: &Project) -> Result<(), String> {
    if let Some(session) = existing_sessions(socket)?
        .into_iter()
        .find(|session| canonical_path(&session.cwd).as_deref() == Some(project.path.as_path()))
    {
        return execute_tmux(socket, &["switch-client", "-t", &session.id]).map(|_| ());
    }

    let path = project.path.to_string_lossy();
    for _ in 0..8 {
        let names = existing_session_names(socket);
        let name = unique_session_name(&project.path, &names);
        match execute_tmux(
            socket,
            &[
                "new-session",
                "-d",
                "-P",
                "-F",
                "#{session_id}",
                "-c",
                &path,
                "-s",
                &name,
            ],
        ) {
            Ok(session_id) => {
                return execute_tmux(socket, &["switch-client", "-t", session_id.trim()])
                    .map(|_| ());
            }
            Err(error) => {
                if let Some(session) = existing_sessions(socket)?.into_iter().find(|session| {
                    canonical_path(&session.cwd).as_deref() == Some(project.path.as_path())
                }) {
                    return execute_tmux(socket, &["switch-client", "-t", &session.id]).map(|_| ());
                }
                let refreshed_names = existing_session_names(socket);
                if !refreshed_names.contains(&name) || names.contains(&name) {
                    return Err(error);
                }
            }
        }
    }
    Err("Could not allocate a unique tmux session name after 8 attempts".to_owned())
}

fn canonical_path(path: &str) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn configured_roots(cli_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = cli_roots.to_vec();
    if let Some(env_roots) = std::env::var_os("MOX_PROJECT_ROOTS") {
        roots.extend(std::env::split_paths(&env_roots));
    }
    roots
}

/// Returns Git working directories found under the supplied roots. Directory
/// symlinks are skipped, so traversal cannot loop back into an ancestor.
pub fn discover_git_repositories(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut visited = HashSet::new();
    let mut entries = 0usize;
    for root in roots {
        let mut stack = vec![(root.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            let Ok(canonical) = fs::canonicalize(&dir) else {
                continue;
            };
            if !visited.insert(canonical.clone()) {
                continue;
            }
            if canonical.join(".git").exists() {
                found.push(canonical);
                continue;
            }
            if depth >= MAX_SCAN_DEPTH {
                continue;
            }
            let Ok(children) = fs::read_dir(&canonical) else {
                continue;
            };
            for child in children.flatten() {
                if entries >= MAX_SCAN_ENTRIES {
                    return found;
                }
                entries += 1;
                let Ok(kind) = child.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    stack.push((child.path(), depth + 1));
                }
            }
        }
    }
    found
}

fn zoxide_directories() -> Vec<PathBuf> {
    let mut command = Command::new("zoxide");
    command.args(["query", "-l"]);
    let Ok(output) = output(&mut command, Duration::from_millis(300), 1024 * 1024) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect()
}

fn add_project(
    projects: &mut Vec<Project>,
    seen: &mut HashSet<PathBuf>,
    path: PathBuf,
    session: Option<ExistingSession>,
) {
    if !path.is_dir() {
        return;
    }
    let canonical = fs::canonicalize(&path).unwrap_or(path);
    let new_path = seen.insert(canonical.clone());
    if !new_path && session.is_none() {
        return;
    }
    projects.push(Project {
        path: canonical,
        session,
    });
}

fn project_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_str().unwrap_or("project"))
        .to_owned()
}

/// Produces a tmux-safe session name with a deterministic collision suffix.
pub fn unique_session_name(path: &Path, existing: &HashSet<String>) -> String {
    let base = project_label(path)
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .take(64)
        .collect::<String>();
    let base = base.trim_matches('_');
    let base = if base.is_empty() { "project" } else { base };
    if !existing.contains(base) {
        return base.to_owned();
    }
    for suffix in 2usize.. {
        let candidate = format!("{base}-{suffix}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn recent_file() -> Result<PathBuf, String> {
    Ok(crate::state::directory()?.join("projects-recent"))
}

fn read_recent(file: &Path) -> Vec<PathBuf> {
    fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .take(MAX_RECENT)
        .collect()
}

fn remember(file: &Path, path: &Path) -> Result<(), String> {
    let mut recent = vec![path.to_path_buf()];
    for old in read_recent(file) {
        if old != path && recent.len() < MAX_RECENT {
            recent.push(old);
        }
    }
    let Some(parent) = file.parent() else {
        return Err("Recent-projects file has no parent directory".to_owned());
    };
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let contents = recent
        .iter()
        .map(|entry| entry.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    crate::state::atomic_write(file, contents.as_bytes())
}
