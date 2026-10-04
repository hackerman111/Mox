//! Terminal user interface and fuzzy picker for extracted tokens.
//!
//! Provides a categorized fuzzy search picker inside a tmux popup,
//! enabling quick yanking, shell insertion, or opening in `$EDITOR` or browser.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::extract::model::{EntityKind, ExtractedToken};
use crate::extract::scanner::scan_lines;
use crate::tmux::{execute_tmux, execute_tmux_raw};
use crate::tui::TuiGuard;
use crate::ui::picker::PickerState;
use crate::ui::preview::{Preview, PreviewWorker};
use crate::ui::render::{
    ListRow, PreviewPanel, draw_divider, render_empty_state, render_hints, render_list_row,
    render_preview_panel, truncate_str,
};
use crate::ui::theme::Theme;

/// The ordered list of categories cycled via Tab and BackTab.
const CATEGORIES: [Option<EntityKind>; 14] = [
    None,
    Some(EntityKind::Path),
    Some(EntityKind::Url),
    Some(EntityKind::Hash),
    Some(EntityKind::Ip),
    Some(EntityKind::Command),
    Some(EntityKind::Uuid),
    Some(EntityKind::Quoted),
    Some(EntityKind::Number),
    Some(EntityKind::DockerImage),
    Some(EntityKind::KubernetesResource),
    Some(EntityKind::HexColor),
    Some(EntityKind::IpfsCid),
    Some(EntityKind::Word),
];

/// The interaction mode of the extract picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    /// Active search input (characters typed go to the query).
    Search,
    /// Vim-style navigation (keys act as single-stroke commands: y=copy, q=quit, i=search).
    Normal,
}

/// Interactive state for the extraction picker.
pub struct ExtractState {
    /// All extracted tokens available in the session.
    pub tokens: Vec<ExtractedToken>,
    /// Active fuzzy filter query string.
    pub query: String,
    /// Indices into `tokens` that match the current query and category filter.
    pub filtered_indices: Vec<usize>,
    /// Currently highlighted selection index within `filtered_indices`.
    pub selected_index: usize,
    /// Currently active entity category filter, or `None` for all entities.
    pub current_category: Option<EntityKind>,
    /// Active interaction mode (Search or Normal).
    pub mode: PickerMode,
    picker: PickerState,
}

impl ExtractState {
    /// Constructs a new `ExtractState` initialized with the provided tokens.
    pub fn new(tokens: Vec<ExtractedToken>) -> Self {
        let mut state = Self {
            tokens,
            query: String::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            current_category: None,
            mode: PickerMode::Normal,
            picker: PickerState::default(),
        };
        state.apply_filter();
        state
    }

    /// Cycles the active category filter, retaining the original category order first.
    pub fn cycle_category_forward(&mut self) {
        let current_pos = CATEGORIES
            .iter()
            .position(|&c| c == self.current_category)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % CATEGORIES.len();
        self.current_category = CATEGORIES[next_pos];
        self.apply_filter();
    }

    /// Cycles the active category filter backward.
    pub fn cycle_category_backward(&mut self) {
        let current_pos = CATEGORIES
            .iter()
            .position(|&c| c == self.current_category)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            CATEGORIES.len() - 1
        } else {
            current_pos - 1
        };
        self.current_category = CATEGORIES[prev_pos];
        self.apply_filter();
    }

    /// Filters and ranks tokens based on the active query and category.
    pub fn apply_filter(&mut self) {
        self.picker.query.clone_from(&self.query);
        self.picker.selected = self.selected_index;
        self.picker
            .filter(self.tokens.iter().enumerate().filter_map(|(i, token)| {
                (self.current_category.is_none_or(|kind| kind == token.kind))
                    .then_some((i, token.raw_text.as_str()))
            }));
        self.filtered_indices.clone_from(&self.picker.indices);
        self.selected_index = self.picker.selected;
    }

    /// Moves selection up by one item.
    pub fn move_selection_up(&mut self) {
        self.picker.selected = self.selected_index;
        self.picker.selected = self.picker.selected.saturating_sub(1);
        self.selected_index = self.picker.selected;
    }

    /// Moves selection down by one item.
    pub fn move_selection_down(&mut self) {
        self.picker.selected = self.selected_index;
        self.picker.selected =
            (self.picker.selected + 1).min(self.filtered_indices.len().saturating_sub(1));
        self.selected_index = self.picker.selected;
    }

    /// Returns a reference to the currently selected token, if any.
    pub fn selected_token(&self) -> Option<&ExtractedToken> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.tokens.get(idx))
    }
}

/// Launches a tmux popup running the extract picker.
pub fn launch_extract_popup(socket: Option<&str>, lines: usize) -> std::io::Result<()> {
    let bin_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "mox".to_string());

    let target_pane = execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])
        .unwrap_or_else(|_| "%0".to_string())
        .trim()
        .to_string();

    let lines = lines.min(200_000);
    let mut cmd = format!(
        "{} extract --lines {lines} --target-pane {}",
        crate::tmux::shell_quote(&bin_path),
        crate::tmux::shell_quote(&target_pane)
    );
    if let Some(sock) = socket {
        cmd.push_str(&format!(" --socket {}", crate::tmux::shell_quote(sock)));
    }

    let popup_args = ["display-popup", "-E", "-w", "75%", "-h", "65%", &cmd];

    execute_tmux(socket, &popup_args)
        .map(|_| ())
        .map_err(std::io::Error::other)
}

/// Runs the interactive extract fuzzy picker inside the terminal.
pub fn run_extract_picker(
    socket: Option<&str>,
    lines: usize,
    target_pane: Option<&str>,
) -> std::io::Result<()> {
    let lines = lines.min(200_000);
    let active_pane_id = if let Some(tp) = target_pane {
        tp.trim().to_string()
    } else {
        execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])
            .unwrap_or_else(|_| "%0".to_string())
            .trim()
            .to_string()
    };

    let lines_arg = format!("-{lines}");
    let raw = execute_tmux_raw(
        socket,
        &[
            "capture-pane",
            "-p",
            "-t",
            &active_pane_id,
            "-S",
            &lines_arg,
        ],
    )
    .or_else(|_| execute_tmux_raw(socket, &["capture-pane", "-p", "-t", &active_pane_id]))
    .unwrap_or_default();

    let captured_lines: Vec<String> = raw.lines().map(|s| s.to_string()).collect();
    let raw_tokens = scan_lines(&captured_lines, &active_pane_id);

    // Deduplicate tokens keeping the most recent instance (iterating from newest to oldest)
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for token in raw_tokens.into_iter().rev() {
        if seen.insert(token.clean_text.clone()) {
            deduped.push(token);
        }
    }

    // Preserve established entity navigation when plain words are also available.
    deduped.sort_by_key(|token| token.kind == EntityKind::Word);
    let mut state = ExtractState::new(deduped);
    let _guard = TuiGuard::enter()?;

    let mut out = stdout();
    let mut pending_g = false;
    let mut scroll_offset = 0;
    let mut status = None::<String>;
    let worker = PreviewWorker::new(socket);
    let mut preview_source = None::<Preview>;
    let mut preview_token_index = None;
    let mut preview_generation = 0u64;
    let mut preview_requested = false;
    let mut preview_lines = Vec::new();
    let mut pane_cwds = HashMap::<String, Result<PathBuf, String>>::new();

    loop {
        let selected_token_index = state.filtered_indices.get(state.selected_index).copied();
        if selected_token_index != preview_token_index {
            preview_token_index = selected_token_index;
            preview_generation = preview_generation.wrapping_add(1);
            preview_lines.clear();
            if selected_token_index.is_some() {
                preview_lines.push("Loading preview…".into());
            }
            preview_requested = false;
            preview_source = selected_token_index.map(|index| {
                match preview_for_token(
                    socket,
                    &state.tokens[index],
                    &captured_lines,
                    &mut pane_cwds,
                ) {
                    Ok(preview) => preview,
                    Err(error) => Preview::Text(format!("Preview unavailable: {error}")),
                }
            });
        }
        if !preview_requested && let Some(source) = &preview_source {
            preview_requested = worker.request(preview_generation, source);
        }
        if let Some(lines) = worker.result(preview_generation) {
            preview_lines = lines;
        }

        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let term_w = term_w as usize;
        let term_h = term_h as usize;

        render_extract_ui(
            &state,
            pending_g,
            scroll_offset,
            term_w,
            term_h,
            status.as_deref(),
            &preview_lines,
            &mut out,
        )?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key_event) = event::read()?
        {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }

            // Cancellation always works, including while waiting for gf/gx.
            if key_event.code == KeyCode::Esc
                || (key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL))
            {
                if key_event.code == KeyCode::Esc && state.mode == PickerMode::Search {
                    state.mode = PickerMode::Normal;
                    pending_g = false;
                    continue;
                }
                break;
            }
            if pending_g {
                pending_g = false;
                match key_event.code {
                    KeyCode::Char('f') | KeyCode::Char('o') => {
                        if let Some(tok) = state.selected_token() {
                            match open_in_editor(socket, tok) {
                                Ok(()) => break,
                                Err(error) => status = Some(error),
                            }
                        }
                    }
                    KeyCode::Char('x') => {
                        if let Some(tok) = state.selected_token() {
                            match open_in_browser(&tok.clean_text) {
                                Ok(()) => break,
                                Err(error) => status = Some(error),
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // Global actions that work regardless of mode:
            match key_event.code {
                KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    break;
                }
                KeyCode::Char('y')
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        || key_event.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(tok) = state.selected_token() {
                        match crate::clipboard::copy(socket, &tok.clean_text) {
                            Ok(()) => break,
                            Err(error) => status = Some(error),
                        }
                    }
                }
                KeyCode::Char('o')
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        || key_event.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(tok) = state.selected_token() {
                        match open_in_editor(socket, tok) {
                            Ok(()) => break,
                            Err(error) => status = Some(error),
                        }
                    }
                }
                KeyCode::Char('x')
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        || key_event.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(tok) = state.selected_token() {
                        match open_in_browser(&tok.clean_text) {
                            Ok(()) => break,
                            Err(error) => status = Some(error),
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(tok) = state.selected_token() {
                        // 1. Reset target pane's key-table to root
                        let _ = execute_tmux(
                            socket,
                            &["set-option", "-t", &active_pane_id, "key-table", "root"],
                        );
                        // 2. Cancel copy-mode if pane was in copy-mode
                        let _ = execute_tmux(
                            socket,
                            &["send-keys", "-t", &active_pane_id, "-X", "cancel"],
                        );
                        // 3. Paste cleanly via tmux paste-buffer (bracketed paste, safe from shell execution or key evaluation)
                        let _ = execute_tmux(
                            socket,
                            &["set-buffer", "-b", "mox-paste", "--", &tok.clean_text],
                        );
                        let _ = execute_tmux(
                            socket,
                            &[
                                "paste-buffer",
                                "-b",
                                "mox-paste",
                                "-p",
                                "-t",
                                &active_pane_id,
                            ],
                        );
                        let _ = execute_tmux(socket, &["delete-buffer", "-b", "mox-paste"]);
                        break;
                    }
                }
                KeyCode::Tab => {
                    state.cycle_category_forward();
                    scroll_offset = 0;
                }
                KeyCode::BackTab => {
                    state.cycle_category_backward();
                    scroll_offset = 0;
                }
                KeyCode::Up if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.move_selection_up();
                    adjust_scroll(&mut state, &mut scroll_offset, term_h);
                }
                KeyCode::Down if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.move_selection_down();
                    adjust_scroll(&mut state, &mut scroll_offset, term_h);
                }
                KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.move_selection_up();
                    adjust_scroll(&mut state, &mut scroll_offset, term_h);
                }
                KeyCode::Char('n') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.move_selection_down();
                    adjust_scroll(&mut state, &mut scroll_offset, term_h);
                }
                _ => {
                    // Mode-specific input handling:
                    match state.mode {
                        PickerMode::Search => match key_event.code {
                            KeyCode::Esc => {
                                if state.query.is_empty() {
                                    break;
                                } else {
                                    state.mode = PickerMode::Normal;
                                }
                            }
                            KeyCode::Up => {
                                state.move_selection_up();
                                adjust_scroll(&mut state, &mut scroll_offset, term_h);
                            }
                            KeyCode::Down => {
                                state.move_selection_down();
                                adjust_scroll(&mut state, &mut scroll_offset, term_h);
                            }
                            KeyCode::Backspace => {
                                state.query.pop();
                                state.apply_filter();
                                adjust_scroll(&mut state, &mut scroll_offset, term_h);
                            }
                            KeyCode::Char(c) => {
                                state.query.push(c);
                                state.apply_filter();
                                adjust_scroll(&mut state, &mut scroll_offset, term_h);
                            }
                            _ => {}
                        },
                        PickerMode::Normal => match key_event.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                break;
                            }
                            KeyCode::Char('y') => {
                                if let Some(tok) = state.selected_token() {
                                    match crate::clipboard::copy(socket, &tok.clean_text) {
                                        Ok(()) => break,
                                        Err(error) => status = Some(error),
                                    }
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                state.move_selection_down();
                                adjust_scroll(&mut state, &mut scroll_offset, term_h);
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                state.move_selection_up();
                                adjust_scroll(&mut state, &mut scroll_offset, term_h);
                            }
                            KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Char('/') => {
                                state.mode = PickerMode::Search;
                            }
                            KeyCode::Char('g') => {
                                pending_g = true;
                            }
                            KeyCode::Char('o') => {
                                if let Some(tok) = state.selected_token() {
                                    match open_in_editor(socket, tok) {
                                        Ok(()) => break,
                                        Err(error) => status = Some(error),
                                    }
                                }
                            }
                            KeyCode::Char('x') => {
                                if let Some(tok) = state.selected_token() {
                                    match open_in_browser(&tok.clean_text) {
                                        Ok(()) => break,
                                        Err(error) => status = Some(error),
                                    }
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }

    Ok(())
}

/// Adjusts window scroll offset to keep selected index within visible viewport.
fn adjust_scroll(state: &mut ExtractState, scroll_offset: &mut usize, term_h: usize) {
    let content_height = term_h.saturating_sub(4).max(1);
    state.picker.selected = state.selected_index;
    state.picker.offset = *scroll_offset;
    state
        .picker
        .viewport(state.filtered_indices.len(), content_height);
    *scroll_offset = state.picker.offset;
}

/// Renders the extract picker interface.
// Keep the render inputs explicit; this is the single Extract view renderer.
#[allow(clippy::too_many_arguments)]
fn render_extract_ui(
    state: &ExtractState,
    pending_g: bool,
    scroll_offset: usize,
    width: usize,
    height: usize,
    status: Option<&str>,
    preview_lines: &[String],
    out: &mut std::io::Stdout,
) -> std::io::Result<()> {
    let split = width >= 100;
    let list_width = if split { width / 2 } else { width };
    let theme = Theme::nord();
    execute!(
        out,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All),
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.fg),
    )?;

    // 1. Header (> query [N matches] [MODE])
    let total_matches = state.filtered_indices.len();
    let g_indicator = if pending_g {
        " (g: f/o=editor, x=browser)"
    } else {
        ""
    };
    let mode_str = match state.mode {
        PickerMode::Search => "[SEARCH]",
        PickerMode::Normal => "[NORMAL]",
    };
    let header_text = format!(
        "> {} [{} matches] {} {}",
        state.query, total_matches, mode_str, g_indicator
    );
    let header_line = truncate_str(&header_text, list_width);
    execute!(
        out,
        cursor::MoveTo(0, 0),
        SetForegroundColor(theme.accent),
        Print(&header_line),
        ResetColor,
    )?;

    // 2. Category tabs
    execute!(out, cursor::MoveTo(0, 1))?;
    execute!(out, SetForegroundColor(theme.fg), Print("Tabs: "),)?;

    let mut tabs_width = "Tabs: ".len();
    for cat in CATEGORIES {
        let name = cat.map_or("All", category_name);
        let is_active = state.current_category == cat;
        let tab = if is_active {
            format!(" [{name}] ")
        } else {
            format!(" {name}  ")
        };
        let tab_width = tab.chars().count();
        if tabs_width + tab_width > list_width {
            break;
        }
        tabs_width += tab_width;
        if is_active {
            execute!(
                out,
                SetBackgroundColor(theme.accent),
                SetForegroundColor(theme.bg),
                Print(&tab),
                SetBackgroundColor(theme.bg),
                SetForegroundColor(theme.fg),
                Print(" "),
            )?;
        } else {
            execute!(out, SetForegroundColor(theme.fg), Print(&tab),)?;
        }
    }

    // 3. Separator line
    execute!(out, cursor::MoveTo(0, 2))?;
    draw_divider(out, list_width, &theme)?;

    // 4. List items
    let content_height = height.saturating_sub(4).max(1);
    if state.filtered_indices.is_empty() {
        execute!(out, cursor::MoveTo(0, 3))?;
        render_empty_state(out, "no matching entities", list_width, &theme)?;
    } else {
        let end_idx = state
            .filtered_indices
            .len()
            .min(scroll_offset + content_height);
        let visible_indices = if scroll_offset < state.filtered_indices.len() {
            &state.filtered_indices[scroll_offset..end_idx]
        } else {
            &[][..]
        };

        for (row_offset, &token_idx) in visible_indices.iter().enumerate() {
            let row = 3 + row_offset;
            let is_selected = scroll_offset + row_offset == state.selected_index;
            let token = &state.tokens[token_idx];

            execute!(out, cursor::MoveTo(0, row as u16))?;

            let (badge, badge_color) = badge_for_kind(token.kind, &theme);
            let display_text = token.line_number.map_or_else(
                || Cow::Borrowed(token.clean_text.as_str()),
                |line| Cow::Owned(format!("{}:{line}", token.clean_text)),
            );

            let list_row = ListRow {
                prefix: if is_selected { "> " } else { "  " },
                badge: Some((badge, badge_color)),
                text: &display_text,
                is_selected,
            };

            render_list_row(out, &list_row, list_width, &theme)?;
        }
    }

    if split {
        render_preview_panel(
            out,
            &PreviewPanel {
                title: "Preview",
                lines: preview_lines,
                x: list_width,
                y: 1,
                width: width - list_width,
                height: height.saturating_sub(2),
            },
            &theme,
        )?;
    }

    // 5. Footer actions
    let footer_row = height.saturating_sub(1) as u16;
    let footer_text = if let Some(status) = status {
        status
    } else {
        match state.mode {
            PickerMode::Search => {
                "<Enter> Paste | <C-y> Copy | <Esc> Normal | <Tab> Tab | <C-c> Quit"
            }
            PickerMode::Normal => {
                "<Enter> Paste | <y> Copy | <o/gf> Edit | <gx> Web | <i> Search | <q/Esc> Quit"
            }
        }
    };
    execute!(out, cursor::MoveTo(0, footer_row))?;
    render_hints(out, footer_text, width, &theme)?;

    out.flush()?;
    Ok(())
}

fn category_name(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Path => "Path",
        EntityKind::Url => "Url",
        EntityKind::Hash => "Hash",
        EntityKind::Ip => "IP",
        EntityKind::Command => "Command",
        EntityKind::Uuid => "UUID",
        EntityKind::Quoted => "String",
        EntityKind::Number => "Number",
        EntityKind::Word => "Word",
        EntityKind::DockerImage => "Docker",
        EntityKind::KubernetesResource => "K8s",
        EntityKind::HexColor => "Color",
        EntityKind::IpfsCid => "IPFS",
    }
}

/// Returns the badge text and accent color for an entity kind.
fn badge_for_kind(kind: EntityKind, theme: &Theme) -> (&'static str, Color) {
    match kind {
        EntityKind::Path => ("[PATH]", theme.badge_path),
        EntityKind::Url => ("[URL ]", theme.badge_url),
        EntityKind::Hash => ("[HASH]", theme.badge_hash),
        EntityKind::Ip => ("[IP  ]", theme.badge_ip),
        EntityKind::Command => ("[CMD ]", theme.badge_command),
        EntityKind::Uuid => ("[UUID]", theme.info),
        EntityKind::Quoted => ("[STR ]", theme.fg),
        EntityKind::Number => ("[NUM ]", theme.muted),
        EntityKind::Word => ("[WORD]", theme.badge_word),
        EntityKind::DockerImage => ("[IMG ]", theme.badge_docker),
        EntityKind::KubernetesResource => ("[K8S ]", theme.badge_k8s),
        EntityKind::HexColor => ("[HEX ]", theme.badge_color),
        EntityKind::IpfsCid => ("[CID ]", theme.badge_ipfs),
    }
}

fn preview_for_token(
    socket: Option<&str>,
    token: &ExtractedToken,
    captured_lines: &[String],
    pane_cwds: &mut HashMap<String, Result<PathBuf, String>>,
) -> Result<Preview, String> {
    if token.kind != EntityKind::Path {
        return Ok(Preview::Text(text_context(
            captured_lines,
            token.screen_row,
        )));
    }

    let location = crate::editor::Location {
        path: token.clean_text.clone().into(),
        line: token.line_number,
        column: token.col_number,
    };
    if location.path.is_absolute() {
        return Ok(Preview::File(location.path));
    }

    let cwd = pane_cwds
        .entry(token.pane_id.clone())
        .or_insert_with(|| {
            execute_tmux(
                socket,
                &[
                    "display-message",
                    "-p",
                    "-t",
                    &token.pane_id,
                    "#{pane_current_path}",
                ],
            )
            .map(PathBuf::from)
        })
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(Preview::File(resolve_location_path(location, cwd)))
}

fn resolve_location_path(location: crate::editor::Location, cwd: &Path) -> PathBuf {
    if location.path.is_absolute() {
        location.path
    } else {
        cwd.join(location.path)
    }
}

fn text_context(lines: &[String], screen_row: usize) -> String {
    if lines.is_empty() {
        return "No source context available".into();
    }
    let first = screen_row.saturating_sub(2);
    let end = screen_row.saturating_add(3).min(lines.len());
    let mut context = String::new();
    for (row, line) in lines.iter().enumerate().take(end).skip(first) {
        let marker = if row == screen_row { '▶' } else { ' ' };
        context.push_str(&format!("{:>5} {marker} {line}\n", row + 1));
    }
    context
}

/// Opens a filesystem path in `$EDITOR` or `$VISUAL`.
pub fn open_in_editor(socket: Option<&str>, token: &ExtractedToken) -> Result<(), String> {
    let location = crate::editor::Location {
        path: token.clean_text.clone().into(),
        line: token.line_number,
        column: token.col_number,
    };
    crate::editor::open(socket, Some(&token.pane_id), location)
}

/// Opens a URL in the default browser.
pub fn open_in_browser(url: &str) -> Result<(), String> {
    crate::clipboard::open_url(url)
}

#[cfg(test)]
mod preview_tests {
    use super::{resolve_location_path, text_context};
    use crate::editor::Location;
    use std::path::PathBuf;

    #[test]
    fn relative_preview_path_uses_source_pane_cwd() {
        let location = Location {
            path: PathBuf::from("src/main.rs"),
            line: Some(12),
            column: Some(4),
        };
        assert_eq!(
            resolve_location_path(location, &PathBuf::from("/work/project")),
            PathBuf::from("/work/project/src/main.rs")
        );
    }

    #[test]
    fn non_path_preview_marks_selected_capture_context() {
        let lines = vec!["before".into(), "selected URL".into(), "after".into()];
        let context = text_context(&lines, 1);
        assert!(context.contains("1   before"));
        assert!(context.contains("2 ▶ selected URL"));
        assert!(context.contains("3   after"));
    }
}
