//! Bounded file, Git change, and ripgrep pickers for registered Neovim instances.

use super::Location;
use crate::process::output;
use crate::tmux::execute_tmux;
use crate::ui::picker::{PickerItem, run_picker};
use crate::ui::preview::Preview;
use std::collections::HashSet;
#[cfg(unix)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ITEMS: usize = 10_000;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Files,
    Git,
    Ripgrep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchLocation {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

/// Opens the selected file or search match in a registered Neovim instance.
pub fn pick(
    socket: Option<&str>,
    source: Source,
    query: Option<&str>,
    root: Option<&Path>,
) -> Result<(), String> {
    let root = match root {
        Some(root) => root.to_path_buf(),
        None => PathBuf::from(execute_tmux(
            socket,
            &["display-message", "-p", "#{pane_current_path}"],
        )?),
    };
    let root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }

    let locations = match source {
        Source::Files => list_files(&root)?,
        Source::Git => git_files(&root)?,
        Source::Ripgrep => {
            let Some(query) = query.filter(|query| !query.is_empty()) else {
                return Ok(());
            };
            ripgrep_matches(&root, query)?
        }
    };
    if locations.is_empty() {
        return Ok(());
    }

    let items = locations
        .iter()
        .enumerate()
        .map(|(index, location)| {
            let absolute = absolute_path(&root, &location.path);
            let relative = absolute.strip_prefix(&root).unwrap_or(&absolute);
            let label = location_label(relative, location.line, location.column);
            PickerItem {
                id: index.to_string(),
                label,
                detail: absolute.display().to_string(),
                preview: Preview::File(absolute),
            }
        })
        .collect::<Vec<_>>();

    let title = match source {
        Source::Files => "Files",
        Source::Git => "Git changes",
        Source::Ripgrep => "Ripgrep",
    };
    let Some(index) = run_picker(title, &items, socket)? else {
        return Ok(());
    };
    let selected = &locations[index];
    let location = Location {
        path: absolute_path(&root, &selected.path),
        line: selected.line,
        column: selected.column,
    };
    super::open(socket, None, location)
}

fn list_files(root: &Path) -> Result<Vec<MatchLocation>, String> {
    let mut command = Command::new("rg");
    command.args(["--files", "-0"]).current_dir(root);
    let output = run_bounded(&mut command, "rg --files")?;
    Ok(split_nul_paths(&output.stdout)
        .into_iter()
        .take(MAX_ITEMS)
        .filter(|path| root.join(path).is_file())
        .map(|path| MatchLocation {
            path,
            line: None,
            column: None,
        })
        .collect())
}

fn git_files(root: &Path) -> Result<Vec<MatchLocation>, String> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let mut examined = 0usize;
    for args in [
        &["diff", "--name-only", "-z", "--"][..],
        &["diff", "--cached", "--name-only", "-z", "--"][..],
        &["ls-files", "--others", "--exclude-standard", "-z"][..],
    ] {
        let mut command = Command::new("git");
        command.args(args).current_dir(root);
        let output = run_bounded(&mut command, "git file listing")?;
        for path in split_nul_paths(&output.stdout) {
            if examined >= MAX_ITEMS {
                return Ok(files);
            }
            examined += 1;
            if seen.insert(path.clone()) && root.join(&path).is_file() {
                files.push(MatchLocation {
                    path,
                    line: None,
                    column: None,
                });
            }
        }
    }
    Ok(files)
}

fn ripgrep_matches(root: &Path, query: &str) -> Result<Vec<MatchLocation>, String> {
    let mut command = Command::new("rg");
    command
        .args(["--json", "--line-number", "-e", query, "--", "."])
        .current_dir(root);
    match output(&mut command, COMMAND_TIMEOUT, MAX_OUTPUT_BYTES) {
        Ok(output) if output.status.success() || output.status.code() == Some(1) => {
            Ok(parse_ripgrep_json(&output.stdout)
                .into_iter()
                .take(MAX_ITEMS)
                .filter(|location| root.join(&location.path).is_file())
                .collect())
        }
        Ok(output) => Err(format!(
            "rg search failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) => Err(error),
    }
}

fn run_bounded(command: &mut Command, label: &str) -> Result<std::process::Output, String> {
    let output = output(command, COMMAND_TIMEOUT, MAX_OUTPUT_BYTES)?;
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!(
            "{label} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

/// Splits command output safely when filenames contain whitespace or newlines.
pub fn split_nul_paths(bytes: &[u8]) -> Vec<PathBuf> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .take(MAX_ITEMS)
        .map(path_from_bytes)
        .collect()
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Parses ripgrep's JSON event stream, preserving escaped newlines in paths.
pub fn parse_ripgrep_json(bytes: &[u8]) -> Vec<MatchLocation> {
    let mut matches = Vec::new();
    for line in bytes.split(|byte| *byte == b'\n') {
        let Ok(event) = serde_json::from_slice::<serde_json::Value>(line) else {
            continue;
        };
        if event.get("type").and_then(|value| value.as_str()) != Some("match") {
            continue;
        }
        let Some(data) = event.get("data") else {
            continue;
        };
        let Some(path) = data
            .get("path")
            .and_then(|path| path.get("text"))
            .and_then(|path| path.as_str())
        else {
            continue;
        };
        let line = data
            .get("line_number")
            .and_then(|line| line.as_u64())
            .and_then(|line| usize::try_from(line).ok());
        let column = data
            .get("submatches")
            .and_then(|matches| matches.as_array())
            .and_then(|matches| matches.first())
            .and_then(|matched| matched.get("start"))
            .and_then(|start| start.as_u64())
            .and_then(|start| usize::try_from(start).ok())
            .and_then(|start| start.checked_add(1));
        matches.push(MatchLocation {
            path: PathBuf::from(path),
            line,
            column,
        });
        if matches.len() >= MAX_ITEMS {
            break;
        }
    }
    matches
}

fn absolute_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn location_label(path: &Path, line: Option<usize>, column: Option<usize>) -> String {
    let mut label = path.to_string_lossy().into_owned();
    if let Some(line) = line {
        label.push(':');
        label.push_str(&line.to_string());
        if let Some(column) = column {
            label.push(':');
            label.push_str(&column.to_string());
        }
    }
    label
}
