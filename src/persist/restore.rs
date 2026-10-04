use super::{Pane, Snapshot};
use crate::tmux::{execute_tmux, shell_quote};
use std::collections::HashMap;

fn start_command(pane: &Pane, commands: bool) -> Option<String> {
    if !commands {
        return None;
    }
    // Never reconstruct argv from a foreground process name. Restore only captured,
    // explicitly supported programs, with every argument individually shell-quoted.
    let name = std::path::Path::new(pane.argv.first()?)
        .file_name()?
        .to_str()?;
    if !matches!(name, "nvim" | "lazygit") {
        return None;
    }
    Some(
        pane.argv
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Substitute pane IDs in leaf cells and recompute the tmux layout checksum.
pub fn remap_layout(layout: &str, ids: &HashMap<String, String>) -> Result<String, String> {
    fn number<'a>(body: &'a str, offset: &mut usize) -> Result<&'a str, String> {
        let start = *offset;
        while body.as_bytes().get(*offset).is_some_and(u8::is_ascii_digit) {
            *offset += 1;
        }
        if start == *offset {
            return Err("Malformed layout number".into());
        }
        Ok(&body[start..*offset])
    }
    fn punctuation(body: &str, offset: &mut usize, expected: u8) -> Result<(), String> {
        if body.as_bytes().get(*offset) != Some(&expected) {
            return Err("Malformed layout separator".into());
        }
        *offset += 1;
        Ok(())
    }
    fn cell(
        body: &str,
        offset: &mut usize,
        out: &mut String,
        ids: &HashMap<String, String>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("Layout nesting limit exceeded".into());
        }
        let start = *offset;
        number(body, offset)?;
        punctuation(body, offset, b'x')?;
        number(body, offset)?;
        punctuation(body, offset, b',')?;
        number(body, offset)?;
        punctuation(body, offset, b',')?;
        number(body, offset)?;
        out.push_str(&body[start..*offset]);
        match body.as_bytes().get(*offset).copied() {
            Some(open @ (b'[' | b'{')) => {
                *offset += 1;
                out.push(open as char);
                cell(body, offset, out, ids, depth + 1)?;
                while body.as_bytes().get(*offset) == Some(&b',') {
                    *offset += 1;
                    out.push(',');
                    cell(body, offset, out, ids, depth + 1)?;
                }
                let close = if open == b'[' { b']' } else { b'}' };
                punctuation(body, offset, close)?;
                out.push(close as char);
            }
            Some(b',') => {
                *offset += 1;
                let id = number(body, offset)?;
                let mapped = ids
                    .get(&format!("%{id}"))
                    .ok_or_else(|| format!("Layout references missing pane %{id}"))?;
                out.push(',');
                out.push_str(mapped.trim_start_matches('%'));
            }
            _ => return Err("Malformed layout leaf".into()),
        }
        Ok(())
    }
    let (_, body) = layout.split_once(',').ok_or("Malformed layout")?;
    let mut result = String::new();
    let mut offset = 0;
    cell(body, &mut offset, &mut result, ids, 0)?;
    if offset != body.len() {
        return Err("Trailing layout data".into());
    }
    let mut checksum: u16 = 0;
    for byte in result.bytes() {
        checksum = checksum.rotate_right(1).wrapping_add(byte as u16);
    }
    Ok(format!("{checksum:04x},{result}"))
}

pub fn apply(socket: Option<&str>, snapshot: &Snapshot, commands: bool) -> Result<(), String> {
    let existing =
        execute_tmux(socket, &["list-sessions", "-F", "#{session_name}"]).or_else(|error| {
            if error.contains("no server running") || error.contains("No such file or directory") {
                Ok(String::new())
            } else {
                Err(error)
            }
        })?;
    for session in &snapshot.sessions {
        if existing.lines().any(|name| name == session.name) {
            continue;
        }
        for window in &session.windows {
            let ids = window
                .panes
                .iter()
                .map(|p| (p.id.clone(), p.id.clone()))
                .collect();
            remap_layout(&window.layout, &ids)?;
            for pane in &window.panes {
                if !pane.cwd.is_dir() {
                    return Err(format!("Restore cwd missing: {}", pane.cwd.display()));
                }
            }
        }
    }
    let mut shared = HashMap::<String, String>::new();
    for session in &snapshot.sessions {
        // Existing sessions win. Never mutate live work merely to match a snapshot.
        if existing.lines().any(|name| name == session.name) {
            continue;
        }
        let first = &session.windows[0];
        let pane = &first.panes[0];
        if !pane.cwd.is_dir() {
            return Err(format!("Restore cwd missing: {}", pane.cwd.display()));
        }
        let cwd = pane.cwd.to_str().ok_or("Non-UTF8 cwd")?;
        let mut args = vec![
            "new-session",
            "-d",
            "-P",
            "-F",
            "#{session_id}",
            "-s",
            &session.name,
            "-c",
            cwd,
        ];
        let command = start_command(pane, commands);
        if let Some(command) = &command {
            args.push(command);
        }
        let session_id = execute_tmux(socket, &args)?;
        struct Rollback<'a> {
            socket: Option<&'a str>,
            session: String,
            committed: bool,
        }
        impl Drop for Rollback<'_> {
            fn drop(&mut self) {
                if !self.committed {
                    let _ = execute_tmux(self.socket, &["kill-session", "-t", &self.session]);
                }
            }
        }
        let mut rollback = Rollback {
            socket,
            session: session_id.clone(),
            committed: false,
        };
        execute_tmux(
            socket,
            &["set-option", "-t", &session_id, "renumber-windows", "off"],
        )?;
        let mut created = Vec::new();
        for (position, window) in session.windows.iter().enumerate() {
            if let Some(linked) = shared.get(&window.panes[0].id) {
                let index = if position == 0 {
                    execute_tmux(
                        socket,
                        &[
                            "display-message",
                            "-p",
                            "-t",
                            &session_id,
                            "#{window_index}",
                        ],
                    )?
                } else {
                    (10000 + position).to_string()
                };
                execute_tmux(
                    socket,
                    &[
                        "link-window",
                        "-k",
                        "-s",
                        linked,
                        "-t",
                        &format!("{session_id}:{index}"),
                    ],
                )?;
                created.push((window, linked.clone()));
                continue;
            }
            let window_id = if position == 0 {
                execute_tmux(
                    socket,
                    &["display-message", "-p", "-t", &session_id, "#{window_id}"],
                )?
            } else {
                let cwd = window.panes[0].cwd.to_str().ok_or("Non-UTF8 cwd")?;
                let mut args = vec![
                    "new-window",
                    "-d",
                    "-P",
                    "-F",
                    "#{window_id}",
                    "-t",
                    &session_id,
                    "-c",
                    cwd,
                ];
                let command = start_command(&window.panes[0], commands);
                if let Some(command) = &command {
                    args.push(command);
                }
                execute_tmux(socket, &args)?
            };
            execute_tmux(socket, &["rename-window", "-t", &window_id, &window.name])?;
            let first_id = execute_tmux(
                socket,
                &["display-message", "-p", "-t", &window_id, "#{pane_id}"],
            )?;
            let mut ids = HashMap::new();
            ids.insert(window.panes[0].id.clone(), first_id);
            for pane in &window.panes[1..] {
                if !pane.cwd.is_dir() {
                    return Err(format!("Restore cwd missing: {}", pane.cwd.display()));
                }
                let cwd = pane.cwd.to_str().ok_or("Non-UTF8 cwd")?;
                let mut args = vec![
                    "split-window",
                    "-d",
                    "-P",
                    "-F",
                    "#{pane_id}",
                    "-t",
                    &window_id,
                    "-c",
                    cwd,
                ];
                let command = start_command(pane, commands);
                if let Some(command) = &command {
                    args.push(command);
                }
                let id = execute_tmux(socket, &args)?;
                ids.insert(pane.id.clone(), id);
                execute_tmux(socket, &["select-layout", "-t", &window_id, "tiled"])?;
            }
            let layout = remap_layout(&window.layout, &ids)?;
            execute_tmux(socket, &["select-layout", "-t", &window_id, &layout])?;
            if let Some(pane) = window.panes.iter().find(|p| p.active) {
                execute_tmux(
                    socket,
                    &[
                        "select-pane",
                        "-t",
                        ids.get(&pane.id).ok_or("Missing pane mapping")?,
                    ],
                )?;
            }
            if window.zoom {
                execute_tmux(socket, &["resize-pane", "-Z", "-t", &window_id])?;
            }
            shared.insert(window.panes[0].id.clone(), window_id.clone());
            created.push((window, window_id));
        }
        // Move via temporary high indexes to avoid collisions, then set saved indexes.
        for (i, (_, id)) in created.iter().enumerate() {
            execute_tmux(
                socket,
                &[
                    "move-window",
                    "-s",
                    &format!("{session_id}:{id}"),
                    "-t",
                    &format!("{session_id}:{}", 100000 + i),
                ],
            )?;
        }
        for (window, id) in &created {
            execute_tmux(
                socket,
                &[
                    "move-window",
                    "-s",
                    &format!("{session_id}:{id}"),
                    "-t",
                    &format!("{session_id}:{}", window.index),
                ],
            )?;
        }
        execute_tmux(
            socket,
            &["set-option", "-u", "-t", &session_id, "renumber-windows"],
        )?;
        if let Some((_, id)) = created.iter().find(|(w, _)| w.active) {
            execute_tmux(
                socket,
                &["select-window", "-t", &format!("{session_id}:{id}")],
            )?;
        }
        rollback.committed = true;
    }
    if let Some(name) = &snapshot.active_session {
        let sessions = execute_tmux(
            socket,
            &["list-sessions", "-F", "#{session_name}\t#{session_id}"],
        )?;
        if let Some(id) = sessions
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .find_map(|(session, id)| (session == name).then_some(id))
        {
            let _ = execute_tmux(socket, &["switch-client", "-t", id]);
        }
    }
    Ok(())
}
