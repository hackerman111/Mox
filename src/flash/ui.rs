//! Flash overlay terminal user interface and interactive event runner.
//!
//! Renders dimmed pane text with highlighted home-row jump badges using Crossterm,
//! executes immediate entity yanking/opening, character motions, and remote text operations.

use std::io::{Write, stdout};
use std::process::Command;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::extract::scanner::scan_lines;
use crate::flash::labeler::{LabeledTarget, MatchTarget, assign_labels};
use crate::flash::matcher::{find_char_motion_matches, find_matches, find_token_matches};
use crate::flash::overlay::{FlashConfig, FlashMode};
use crate::flash::text_object::{TextObject, resolve_text_object};
use crate::tmux::{execute_tmux, execute_tmux_raw};
use crate::tui::TuiGuard;

/// Copies text to the tmux paste buffer and the system clipboard.
pub fn copy_to_clipboard_and_tmux(socket: Option<&str>, text: &str) {
    // 1. Set tmux buffer
    let _ = execute_tmux(socket, &["set-buffer", "--", text]);

    // 2. Wayland clipboard (wl-copy)
    if let Ok(mut child) = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        // Clipboard daemons must not keep the popup PTY alive after selection.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return;
    }

    // 3. X11 clipboard (xclip)
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        // Clipboard daemons must not keep the popup PTY alive after selection.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return;
    }

    // 4. macOS clipboard (pbcopy)
    if let Ok(mut child) = Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        // Clipboard daemons must not keep the popup PTY alive after selection.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

/// Opens a URL or filesystem path in the default browser or `$EDITOR`.
pub fn open_target(socket: Option<&str>, text: &str, line: Option<usize>, _col: Option<usize>) {
    if text.starts_with("http://") || text.starts_with("https://") || text.starts_with("git@") {
        let _ = Command::new("xdg-open")
            .arg(text)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .or_else(|_| {
                Command::new("open")
                    .arg(text)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
            });
    } else {
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "nvim".to_string());
        let cmd = if let Some(l) = line {
            format!("{editor} +{l} {text}")
        } else {
            format!("{editor} {text}")
        };

        if execute_tmux(socket, &["new-window", "-n", "editor", "--", &cmd]).is_err() {
            let _ = Command::new("xdg-open")
                .arg(text)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

/// Jumps cursor to `(row, col)` in `pane_id` and enters `copy-mode-vi`.
pub fn jump_to_target(
    socket: Option<&str>,
    pane_id: &str,
    row: usize,
    col: usize,
) -> Result<(), String> {
    execute_tmux(socket, &["select-pane", "-t", pane_id])?;
    let mode = execute_tmux(
        socket,
        &["display-message", "-p", "-t", pane_id, "#{pane_mode}"],
    )?;
    if mode != "copy-mode" {
        execute_tmux(socket, &["copy-mode", "-t", pane_id])?;
    }
    execute_tmux(socket, &["send-keys", "-X", "-t", pane_id, "top-line"])?;
    if row > 0 {
        let count_str = row.to_string();
        execute_tmux(
            socket,
            &[
                "send-keys",
                "-X",
                "-t",
                pane_id,
                "-N",
                &count_str,
                "cursor-down",
            ],
        )?;
    }
    execute_tmux(socket, &["send-keys", "-X", "-t", pane_id, "start-of-line"])?;
    if col > 0 {
        let count_str = col.to_string();
        execute_tmux(
            socket,
            &[
                "send-keys",
                "-X",
                "-t",
                pane_id,
                "-N",
                &count_str,
                "cursor-right",
            ],
        )?;
    }
    Ok(())
}

/// Represents a pane's layout bounds and captured lines.
struct PaneContent {
    id: String,
    left: usize,
    top: usize,
    lines: Vec<String>,
}

/// Queries visible lines and panes from tmux.
fn capture_panes_content(
    socket: Option<&str>,
    cfg: &FlashConfig,
) -> (Vec<PaneContent>, usize, usize, String) {
    // Determine active cursor position
    let mut cursor_col = 0;
    let mut cursor_row = 0;
    let mut active_pane_id = "%0".to_string();

    let query_args = if let Some(tp) = &cfg.target_pane {
        vec![
            "display-message",
            "-p",
            "-t",
            tp,
            "#{?#{==:#{pane_mode},copy-mode},#{copy_cursor_x},#{cursor_x}},#{?#{==:#{pane_mode},copy-mode},#{copy_cursor_y},#{cursor_y}},#{pane_id}",
        ]
    } else {
        vec![
            "display-message",
            "-p",
            "#{?#{==:#{pane_mode},copy-mode},#{copy_cursor_x},#{cursor_x}},#{?#{==:#{pane_mode},copy-mode},#{copy_cursor_y},#{cursor_y}},#{pane_id}",
        ]
    };

    if let Ok(msg) = execute_tmux(socket, &query_args) {
        let parts: Vec<&str> = msg.trim().split(',').collect();
        if parts.len() >= 3 {
            cursor_col = parts[0].parse().unwrap_or(0);
            cursor_row = parts[1].parse().unwrap_or(0);
            active_pane_id = parts[2].to_string();
        }
    }

    if cfg.multi_pane {
        // Query all panes in the current window
        if let Ok(list) = execute_tmux(
            socket,
            &[
                "list-panes",
                "-F",
                "#{pane_id}:#{pane_left}:#{pane_top}:#{pane_width}:#{pane_height}",
            ],
        ) {
            let mut contents = Vec::new();
            for line in list.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 5 {
                    let id = parts[0].to_string();
                    let left = parts[1].parse().unwrap_or(0);
                    let top = parts[2].parse().unwrap_or(0);
                    let raw = capture_visible_pane(socket, &id).unwrap_or_default();
                    let lines: Vec<String> = raw.lines().map(|s| s.to_string()).collect();
                    contents.push(PaneContent {
                        id,
                        left,
                        top,
                        lines,
                    });
                }
            }
            if !contents.is_empty() {
                return (contents, cursor_row, cursor_col, active_pane_id);
            }
        }
    }

    // Single-pane capture
    let raw = capture_visible_pane(socket, &active_pane_id).unwrap_or_default();
    let lines: Vec<String> = raw.lines().map(|s| s.to_string()).collect();
    let content = PaneContent {
        id: active_pane_id.clone(),
        left: 0,
        top: 0,
        lines,
    };

    (vec![content], cursor_row, cursor_col, active_pane_id)
}

/// Capture the displayed viewport, including an existing scrollback selection.
fn capture_visible_pane(socket: Option<&str>, pane: &str) -> Result<String, String> {
    let mode = execute_tmux(
        socket,
        &["display-message", "-p", "-t", pane, "#{pane_mode}"],
    )?;
    if mode == "copy-mode" {
        execute_tmux_raw(socket, &["capture-pane", "-p", "-M", "-t", pane])
    } else {
        execute_tmux_raw(socket, &["capture-pane", "-p", "-t", pane])
    }
}

/// Renders the terminal overlay with dimmed text, highlighted badges, and a status bar.
fn render_flash_screen(
    out: &mut std::io::Stdout,
    panes: &[PaneContent],
    labeled: &[LabeledTarget],
    prompt_title: &str,
    status_text: &str,
    input_highlight: &str,
) -> std::io::Result<()> {
    let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
    let term_w = term_w as usize;
    let term_h = term_h as usize;
    let max_rows = term_h.saturating_sub(1);

    // Build virtual screen line buffer
    let mut screen_chars: Vec<Vec<char>> = vec![vec![' '; term_w]; max_rows];

    for pane in panes {
        for (r_idx, line) in pane.lines.iter().enumerate() {
            let screen_r = pane.top + r_idx;
            if screen_r >= max_rows {
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            for (c_idx, &ch) in chars.iter().enumerate() {
                let screen_c = pane.left + c_idx;
                if screen_c < term_w {
                    screen_chars[screen_r][screen_c] = ch;
                }
            }
        }
    }

    // Map targets to screen coordinates
    struct ScreenTarget<'a> {
        screen_r: usize,
        screen_c: usize,
        target: &'a LabeledTarget,
    }

    let mut screen_targets: Vec<ScreenTarget> = Vec::new();
    for lt in labeled {
        // Find which pane this target belongs to
        if let Some(pane) = panes.iter().find(|p| p.id == lt.target.pane_id) {
            let screen_r = pane.top + lt.target.row;
            let screen_c = pane.left + lt.target.col;
            if screen_r < max_rows && screen_c < term_w {
                screen_targets.push(ScreenTarget {
                    screen_r,
                    screen_c,
                    target: lt,
                });
            }
        }
    }

    // Render each row
    for (r, row_vec) in screen_chars.iter().enumerate().take(max_rows) {
        execute!(
            out,
            cursor::MoveTo(0, r as u16),
            Clear(ClearType::CurrentLine)
        )?;

        let mut row_targets: Vec<&ScreenTarget> = screen_targets
            .iter()
            .filter(|st| st.screen_r == r)
            .collect();
        row_targets.sort_by_key(|st| st.screen_c);

        let mut curr_c = 0;

        for st in row_targets {
            let label_len = st.target.label.chars().count();
            if st.screen_c > curr_c {
                let dimmed_slice: String = row_vec[curr_c..st.screen_c].iter().collect();
                execute!(
                    out,
                    SetForegroundColor(Color::DarkGrey),
                    Print(&dimmed_slice),
                    ResetColor
                )?;
            }

            // Render label badge
            let is_matched_prefix =
                !input_highlight.is_empty() && st.target.label.starts_with(input_highlight);
            let (bg, fg) = if is_matched_prefix {
                (Color::Green, Color::Black)
            } else {
                (Color::Yellow, Color::Black)
            };

            execute!(
                out,
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
                Print(
                    &st.target.label[..st
                        .target
                        .label
                        .len()
                        .min(term_w.saturating_sub(st.screen_c))]
                ),
                ResetColor
            )?;

            curr_c = (st.screen_c + label_len).min(term_w);
        }

        if curr_c < term_w {
            let remainder: String = row_vec[curr_c..].iter().collect();
            let trimmed = remainder.trim_end();
            if !trimmed.is_empty() {
                execute!(
                    out,
                    SetForegroundColor(Color::DarkGrey),
                    Print(trimmed),
                    ResetColor
                )?;
            }
        }
    }

    // Leave the last cell free: wrapping at the bottom would scroll the overlay.
    let status: String = format!(" {prompt_title} │ {status_text} ")
        .chars()
        .take(term_w.saturating_sub(1))
        .collect();
    // Render status bar at bottom row
    execute!(
        out,
        cursor::MoveTo(0, max_rows as u16),
        Clear(ClearType::CurrentLine),
        SetBackgroundColor(Color::DarkBlue),
        SetForegroundColor(Color::White),
        Print(status),
        ResetColor
    )?;

    out.flush()?;
    Ok(())
}

/// Runs the interactive Flash overlay.
pub fn run_flash_overlay(cfg: &FlashConfig, socket: Option<&str>) -> std::io::Result<()> {
    let _guard = TuiGuard::enter()?;
    let mut out = stdout();

    let (panes, cursor_row, cursor_col, _active_pane_id) = capture_panes_content(socket, cfg);

    match cfg.mode {
        FlashMode::QuickYank | FlashMode::QuickOpen => {
            // Extract all tokens across captured panes
            let mut all_tokens = Vec::new();
            for pane in &panes {
                let pane_tokens = scan_lines(&pane.lines, &pane.id);
                all_tokens.extend(pane_tokens);
            }

            let targets = find_token_matches(&all_tokens, cfg.entity_filter);
            if targets.is_empty() {
                execute!(
                    out,
                    Clear(ClearType::All),
                    cursor::MoveTo(0, 0),
                    Print("No entities found. Press any key to exit.\r\n")
                )?;
                out.flush()?;
                let _ = event::read();
                return Ok(());
            }

            let labeled = assign_labels(&targets, cursor_row, cursor_col);
            let mut input_label = String::new();
            let title = if cfg.mode == FlashMode::QuickYank {
                "QUICK YANK"
            } else {
                "QUICK OPEN"
            };

            loop {
                let status = format!(
                    "Select label: {} (Esc to cancel) │ Entities: {}",
                    input_label,
                    labeled.len()
                );
                render_flash_screen(&mut out, &panes, &labeled, title, &status, &input_label)?;

                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Backspace => {
                            input_label.pop();
                        }
                        KeyCode::Char(c) => {
                            input_label.push(c);
                            if let Some(matched) = labeled.iter().find(|lt| lt.label == input_label)
                            {
                                // Find matching extracted token
                                let token = all_tokens.iter().find(|t| {
                                    t.pane_id == matched.target.pane_id
                                        && t.screen_row == matched.target.row
                                        && t.col_start == matched.target.col
                                });
                                let text = token
                                    .map(|t| t.clean_text.as_str())
                                    .unwrap_or(&matched.target.matched_text);

                                if cfg.mode == FlashMode::QuickYank {
                                    copy_to_clipboard_and_tmux(socket, text);
                                } else {
                                    let line_no = token.and_then(|t| t.line_number);
                                    let col_no = token.and_then(|t| t.col_number);
                                    open_target(socket, text, line_no, col_no);
                                }
                                break;
                            }

                            if !labeled.iter().any(|lt| lt.label.starts_with(&input_label)) {
                                input_label.pop();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        FlashMode::CharMotion => {
            let mut motion_ch = cfg.motion_char;
            if motion_ch.is_none() {
                render_flash_screen(
                    &mut out,
                    &panes,
                    &[],
                    "CHAR MOTION",
                    "Type target character (Esc to cancel):",
                    "",
                )?;
                loop {
                    if let Event::Key(key) = event::read()? {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        match key.code {
                            KeyCode::Esc => return Ok(()),
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                return Ok(());
                            }
                            KeyCode::Char(c) => {
                                motion_ch = Some(c);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            let Some(ch) = motion_ch else {
                return Ok(());
            };

            let mut all_targets = Vec::new();
            for pane in &panes {
                let targets = find_char_motion_matches(
                    &pane.lines,
                    ch,
                    cfg.motion_forward,
                    cursor_row,
                    cursor_col,
                    &pane.id,
                );
                all_targets.extend(targets);
            }

            if all_targets.is_empty() {
                return Ok(());
            }

            if all_targets.len() == 1 {
                let t = &all_targets[0];
                let _ = jump_to_target(socket, &t.pane_id, t.row, t.col);
                return Ok(());
            }

            let labeled = assign_labels(&all_targets, cursor_row, cursor_col);
            let mut input_label = String::new();

            loop {
                let status = format!(
                    "Select label: {} (Esc to cancel) │ Matches: {}",
                    input_label,
                    labeled.len()
                );
                render_flash_screen(
                    &mut out,
                    &panes,
                    &labeled,
                    "CHAR MOTION",
                    &status,
                    &input_label,
                )?;

                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Backspace => {
                            input_label.pop();
                        }
                        KeyCode::Char(c) => {
                            input_label.push(c);
                            if let Some(matched) = labeled.iter().find(|lt| lt.label == input_label)
                            {
                                let _ = jump_to_target(
                                    socket,
                                    &matched.target.pane_id,
                                    matched.target.row,
                                    matched.target.col,
                                );
                                break;
                            }

                            if !labeled.iter().any(|lt| lt.label.starts_with(&input_label)) {
                                input_label.pop();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        FlashMode::Jump => {
            let mut search_query = String::new();
            let mut in_label_mode = false;
            let mut input_label = String::new();

            loop {
                // Collect matches
                let mut all_targets = Vec::new();
                if !search_query.is_empty() {
                    for pane in &panes {
                        let matches = find_matches(&pane.lines, &search_query, &pane.id);
                        all_targets.extend(matches);
                    }
                }

                let labeled = assign_labels(&all_targets, cursor_row, cursor_col);

                let status = if in_label_mode {
                    format!(
                        "Select label: {} │ Matches: {} (Esc to edit search)",
                        input_label,
                        labeled.len()
                    )
                } else {
                    format!(
                        "Search: {} │ Matches: {} (Esc to exit, Enter to pick label)",
                        search_query,
                        labeled.len()
                    )
                };

                render_flash_screen(
                    &mut out,
                    &panes,
                    &labeled,
                    "FLASH JUMP",
                    &status,
                    &input_label,
                )?;

                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Esc => {
                            if in_label_mode {
                                in_label_mode = false;
                                input_label.clear();
                            } else if !search_query.is_empty() {
                                search_query.clear();
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Backspace => {
                            if in_label_mode {
                                if input_label.is_empty() {
                                    in_label_mode = false;
                                } else {
                                    input_label.pop();
                                }
                            } else {
                                search_query.pop();
                            }
                        }
                        KeyCode::Enter => {
                            if all_targets.len() == 1 {
                                let t = &all_targets[0];
                                let _ = jump_to_target(socket, &t.pane_id, t.row, t.col);
                                break;
                            } else if all_targets.len() > 1 {
                                in_label_mode = true;
                                input_label.clear();
                            }
                        }
                        KeyCode::Char(c) => {
                            if in_label_mode {
                                input_label.push(c);
                                if let Some(matched) =
                                    labeled.iter().find(|lt| lt.label == input_label)
                                {
                                    let _ = jump_to_target(
                                        socket,
                                        &matched.target.pane_id,
                                        matched.target.row,
                                        matched.target.col,
                                    );
                                    break;
                                }
                                if !labeled.iter().any(|lt| lt.label.starts_with(&input_label)) {
                                    input_label.pop();
                                }
                            } else {
                                // Check if c is a label and not continuing any match
                                let mut candidate_query = search_query.clone();
                                candidate_query.push(c);

                                let mut continuation_targets = Vec::new();
                                for pane in &panes {
                                    continuation_targets.extend(find_matches(
                                        &pane.lines,
                                        &candidate_query,
                                        &pane.id,
                                    ));
                                }

                                let is_label = labeled.iter().any(|lt| lt.label == c.to_string());

                                if is_label && continuation_targets.is_empty() {
                                    if let Some(matched) =
                                        labeled.iter().find(|lt| lt.label == c.to_string())
                                    {
                                        let _ = jump_to_target(
                                            socket,
                                            &matched.target.pane_id,
                                            matched.target.row,
                                            matched.target.col,
                                        );
                                        break;
                                    }
                                } else {
                                    search_query.push(c);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        FlashMode::RemoteYank => {
            let mut search_query = String::new();
            let mut in_label_mode = false;
            let mut input_label = String::new();
            let mut selected_target: Option<(MatchTarget, String)> = None;

            // Phase 1: Search & Target selection
            loop {
                let mut all_targets = Vec::new();
                if !search_query.is_empty() {
                    for pane in &panes {
                        let matches = find_matches(&pane.lines, &search_query, &pane.id);
                        all_targets.extend(matches);
                    }
                }

                let labeled = assign_labels(&all_targets, cursor_row, cursor_col);

                let status = if in_label_mode {
                    format!(
                        "Select label: {} │ Matches: {} (Esc to edit search)",
                        input_label,
                        labeled.len()
                    )
                } else {
                    format!(
                        "Search: {} │ Matches: {} (Esc to exit, Enter to pick label)",
                        search_query,
                        labeled.len()
                    )
                };

                render_flash_screen(
                    &mut out,
                    &panes,
                    &labeled,
                    "REMOTE YANK",
                    &status,
                    &input_label,
                )?;

                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Esc => {
                            if in_label_mode {
                                in_label_mode = false;
                                input_label.clear();
                            } else if !search_query.is_empty() {
                                search_query.clear();
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Backspace => {
                            if in_label_mode {
                                if input_label.is_empty() {
                                    in_label_mode = false;
                                } else {
                                    input_label.pop();
                                }
                            } else {
                                search_query.pop();
                            }
                        }
                        KeyCode::Enter => {
                            if all_targets.len() == 1 {
                                let t = all_targets[0].clone();
                                let lbl =
                                    labeled.first().map(|l| l.label.clone()).unwrap_or_default();
                                selected_target = Some((t, lbl));
                                break;
                            } else if all_targets.len() > 1 {
                                in_label_mode = true;
                                input_label.clear();
                            }
                        }
                        KeyCode::Char(c) => {
                            if in_label_mode {
                                input_label.push(c);
                                if let Some(matched) =
                                    labeled.iter().find(|lt| lt.label == input_label)
                                {
                                    selected_target =
                                        Some((matched.target.clone(), matched.label.clone()));
                                    break;
                                }
                                if !labeled.iter().any(|lt| lt.label.starts_with(&input_label)) {
                                    input_label.pop();
                                }
                            } else {
                                let mut candidate_query = search_query.clone();
                                candidate_query.push(c);

                                let mut continuation_targets = Vec::new();
                                for pane in &panes {
                                    continuation_targets.extend(find_matches(
                                        &pane.lines,
                                        &candidate_query,
                                        &pane.id,
                                    ));
                                }

                                let is_label = labeled.iter().any(|lt| lt.label == c.to_string());

                                if is_label && continuation_targets.is_empty() {
                                    if let Some(matched) =
                                        labeled.iter().find(|lt| lt.label == c.to_string())
                                    {
                                        selected_target =
                                            Some((matched.target.clone(), matched.label.clone()));
                                        break;
                                    }
                                } else {
                                    search_query.push(c);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Phase 2: Semantic text object selection
            if let Some((target, target_label)) = selected_target {
                let target_labeled = vec![LabeledTarget {
                    target: target.clone(),
                    label: target_label.clone(),
                }];

                loop {
                    let status = "[Enter/repeat] Auto-token │ [w] Word │ [q] Quote │ [l] Line │ [b] Bracket │ [$] EOL";
                    render_flash_screen(
                        &mut out,
                        &panes,
                        &target_labeled,
                        "REMOTE YANK",
                        status,
                        "",
                    )?;

                    if let Event::Key(key) = event::read()? {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }

                        let text_object = match key.code {
                            KeyCode::Esc => break,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                break;
                            }
                            KeyCode::Enter => Some(TextObject::AutoToken),
                            KeyCode::Char('w') => Some(TextObject::InnerWord),
                            KeyCode::Char('a') => Some(TextObject::AWord),
                            KeyCode::Char('q') | KeyCode::Char('\'') | KeyCode::Char('"') => {
                                Some(TextObject::InnerQuoted)
                            }
                            KeyCode::Char('l') => Some(TextObject::Line),
                            KeyCode::Char('b') => Some(TextObject::Bracket),
                            KeyCode::Char('$') => Some(TextObject::ToEndOfLine),
                            KeyCode::Char(c) if c.to_string() == target_label => {
                                Some(TextObject::AutoToken)
                            }
                            _ => None,
                        };

                        if let Some(obj) = text_object {
                            // Find line text for the selected target
                            let line_text = panes
                                .iter()
                                .find(|p| p.id == target.pane_id)
                                .and_then(|p| p.lines.get(target.row))
                                .map(|s| s.as_str())
                                .unwrap_or("");

                            let extracted = resolve_text_object(line_text, target.col, obj)
                                .unwrap_or_else(|| target.matched_text.clone());

                            copy_to_clipboard_and_tmux(socket, &extracted);
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
