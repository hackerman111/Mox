//! Terminal user interface and fuzzy picker for extracted tokens.
//!
//! Provides a categorized fuzzy search picker inside a tmux popup,
//! enabling quick yanking, shell insertion, or opening in `$EDITOR` or browser.

use std::collections::HashSet;
use std::io::{Write, stdout};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::extract::model::{EntityKind, ExtractedToken};
use crate::extract::scanner::scan_lines;
use crate::tmux::execute_tmux;
use crate::tui::TuiGuard;

/// The ordered list of categories cycled via Tab and BackTab.
const CATEGORIES: [Option<EntityKind>; 6] = [
    None,
    Some(EntityKind::Path),
    Some(EntityKind::Url),
    Some(EntityKind::Hash),
    Some(EntityKind::Ip),
    Some(EntityKind::Command),
];

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
        };
        state.apply_filter();
        state
    }

    /// Cycles the active category filter forward (`All -> Path -> Url -> Hash -> Ip -> Command -> All`).
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
        let q = self.query.trim();
        if q.is_empty() {
            self.filtered_indices = (0..self.tokens.len())
                .filter(|&i| {
                    if let Some(cat) = self.current_category {
                        self.tokens[i].kind == cat
                    } else {
                        true
                    }
                })
                .collect();
        } else {
            let mut matches: Vec<(usize, i64)> = Vec::new();
            for (i, token) in self.tokens.iter().enumerate() {
                if let Some(cat) = self.current_category
                    && token.kind != cat
                {
                    continue;
                }
                let score_clean = crate::nav::fuzzy::fuzzy_match(&token.clean_text, q);
                let score_raw = crate::nav::fuzzy::fuzzy_match(&token.raw_text, q);
                let best = match (score_clean, score_raw) {
                    (Some(s1), Some(s2)) => Some(s1.max(s2)),
                    (Some(s1), None) => Some(s1),
                    (None, Some(s2)) => Some(s2),
                    (None, None) => None,
                };
                if let Some(score) = best {
                    matches.push((i, score));
                }
            }
            matches.sort_by_key(|b| std::cmp::Reverse(b.1));
            self.filtered_indices = matches.into_iter().map(|(i, _)| i).collect();
        }

        if self.filtered_indices.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = self.filtered_indices.len() - 1;
        }
    }

    /// Moves selection up by one item.
    pub fn move_selection_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Moves selection down by one item.
    pub fn move_selection_down(&mut self) {
        if !self.filtered_indices.is_empty()
            && self.selected_index + 1 < self.filtered_indices.len()
        {
            self.selected_index += 1;
        }
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

    let mut cmd = format!("{bin_path} extract --lines {lines}");
    if let Some(sock) = socket {
        cmd.push_str(&format!(" --socket '{sock}'"));
    }

    let popup_args = ["display-popup", "-E", "-w", "75%", "-h", "65%", &cmd];

    execute_tmux(socket, &popup_args)
        .map(|_| ())
        .map_err(std::io::Error::other)
}

/// Runs the interactive extract fuzzy picker inside the terminal.
pub fn run_extract_picker(socket: Option<&str>, lines: usize) -> std::io::Result<()> {
    let active_pane_id = execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])
        .unwrap_or_else(|_| "%0".to_string())
        .trim()
        .to_string();

    let lines_arg = format!("-{lines}");
    let raw = execute_tmux(socket, &["capture-pane", "-p", "-S", &lines_arg])
        .or_else(|_| execute_tmux(socket, &["capture-pane", "-p"]))
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

    let mut state = ExtractState::new(deduped);
    let _guard = TuiGuard::enter()?;

    let mut out = stdout();
    let mut pending_g = false;
    let mut scroll_offset = 0;

    loop {
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let term_w = term_w as usize;
        let term_h = term_h as usize;

        render_extract_ui(&state, pending_g, scroll_offset, term_w, term_h, &mut out)?;

        if let Event::Key(key_event) = event::read()? {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }

            if pending_g {
                pending_g = false;
                match key_event.code {
                    KeyCode::Char('f') | KeyCode::Char('o') => {
                        if let Some(tok) = state.selected_token() {
                            open_in_editor(socket, &tok.clean_text, tok.line_number);
                            break;
                        }
                    }
                    KeyCode::Char('x') => {
                        if let Some(tok) = state.selected_token() {
                            open_in_browser(&tok.clean_text);
                            break;
                        }
                    }
                    _ => {
                        // Cancel pending chord
                    }
                }
                continue;
            }

            match key_event.code {
                KeyCode::Esc => {
                    break;
                }
                KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    break;
                }
                KeyCode::Char('q')
                    if state.query.is_empty()
                        || key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    break;
                }
                KeyCode::Enter => {
                    if let Some(tok) = state.selected_token() {
                        let mut args = vec!["send-keys", "-l"];
                        if !active_pane_id.is_empty() {
                            args.push("-t");
                            args.push(&active_pane_id);
                        }
                        args.push(&tok.clean_text);
                        let _ = execute_tmux(socket, &args);
                        break;
                    }
                }
                KeyCode::Char('y')
                    if state.query.is_empty()
                        || key_event.modifiers.contains(KeyModifiers::CONTROL)
                        || key_event.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if let Some(tok) = state.selected_token() {
                        crate::flash::ui::copy_to_clipboard_and_tmux(socket, &tok.clean_text);
                        break;
                    }
                }
                KeyCode::Char('g')
                    if state.query.is_empty()
                        || key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    pending_g = true;
                }
                KeyCode::Tab => {
                    state.cycle_category_forward();
                    scroll_offset = 0;
                }
                KeyCode::BackTab => {
                    state.cycle_category_backward();
                    scroll_offset = 0;
                }
                KeyCode::Up => {
                    state.move_selection_up();
                    adjust_scroll(&state, &mut scroll_offset, term_h);
                }
                KeyCode::Down => {
                    state.move_selection_down();
                    adjust_scroll(&state, &mut scroll_offset, term_h);
                }
                KeyCode::Char('p') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.move_selection_up();
                    adjust_scroll(&state, &mut scroll_offset, term_h);
                }
                KeyCode::Char('n') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.move_selection_down();
                    adjust_scroll(&state, &mut scroll_offset, term_h);
                }
                KeyCode::Backspace => {
                    state.query.pop();
                    state.apply_filter();
                    adjust_scroll(&state, &mut scroll_offset, term_h);
                }
                KeyCode::Char(c) => {
                    state.query.push(c);
                    state.apply_filter();
                    adjust_scroll(&state, &mut scroll_offset, term_h);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Adjusts window scroll offset to keep selected index within visible viewport.
fn adjust_scroll(state: &ExtractState, scroll_offset: &mut usize, term_h: usize) {
    let content_height = term_h.saturating_sub(4).max(1);
    if state.selected_index < *scroll_offset {
        *scroll_offset = state.selected_index;
    } else if state.selected_index >= *scroll_offset + content_height {
        *scroll_offset = state.selected_index + 1 - content_height;
    }
}

/// Renders the extract picker interface.
fn render_extract_ui(
    state: &ExtractState,
    pending_g: bool,
    scroll_offset: usize,
    width: usize,
    height: usize,
    out: &mut std::io::Stdout,
) -> std::io::Result<()> {
    let bg = Color::Rgb {
        r: 46,
        g: 52,
        b: 64,
    };
    let fg = Color::Rgb {
        r: 236,
        g: 239,
        b: 244,
    };

    execute!(
        out,
        cursor::MoveTo(0, 0),
        Clear(ClearType::All),
        SetBackgroundColor(bg),
        SetForegroundColor(fg),
    )?;

    // 1. Header (> query [N matches])
    let total_matches = state.filtered_indices.len();
    let g_indicator = if pending_g {
        " (g: f/o=editor, x=browser)"
    } else {
        ""
    };
    let header_text = format!(
        "> {} [{} matches]{}",
        state.query, total_matches, g_indicator
    );
    let header_line = truncate_str(&header_text, width);
    execute!(
        out,
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::Rgb {
            r: 136,
            g: 192,
            b: 208
        }),
        Print(&header_line),
        ResetColor,
    )?;

    // 2. Category tabs
    execute!(out, cursor::MoveTo(0, 1))?;
    let tabs = [
        ("All", None),
        ("Path", Some(EntityKind::Path)),
        ("Url", Some(EntityKind::Url)),
        ("Hash", Some(EntityKind::Hash)),
        ("IP", Some(EntityKind::Ip)),
        ("Command", Some(EntityKind::Command)),
    ];

    execute!(
        out,
        SetForegroundColor(Color::Rgb {
            r: 216,
            g: 222,
            b: 233
        }),
        Print("Tabs: "),
    )?;

    for (name, cat) in &tabs {
        let is_active = state.current_category == *cat;
        if is_active {
            execute!(
                out,
                SetBackgroundColor(Color::Rgb {
                    r: 136,
                    g: 192,
                    b: 208
                }),
                SetForegroundColor(Color::Rgb {
                    r: 46,
                    g: 52,
                    b: 64
                }),
                Print(format!(" [{name}] ")),
                SetBackgroundColor(bg),
                SetForegroundColor(fg),
                Print(" "),
            )?;
        } else {
            execute!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: 216,
                    g: 222,
                    b: 233
                }),
                Print(format!(" {name}  ")),
            )?;
        }
    }

    // 3. Separator line
    let sep = "─".repeat(width);
    execute!(
        out,
        cursor::MoveTo(0, 2),
        SetForegroundColor(Color::Rgb {
            r: 76,
            g: 86,
            b: 106
        }),
        Print(&sep),
        ResetColor,
    )?;

    // 4. List items
    let content_height = height.saturating_sub(4).max(1);
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

        if is_selected {
            execute!(
                out,
                SetBackgroundColor(Color::Rgb {
                    r: 59,
                    g: 66,
                    b: 82
                }),
                SetForegroundColor(Color::Rgb {
                    r: 235,
                    g: 203,
                    b: 139
                }),
                Print("> "),
            )?;
        } else {
            execute!(
                out,
                SetBackgroundColor(bg),
                SetForegroundColor(Color::Rgb {
                    r: 76,
                    g: 86,
                    b: 106
                }),
                Print("  "),
            )?;
        }

        let (badge, badge_color) = badge_for_kind(token.kind);
        execute!(
            out,
            SetForegroundColor(badge_color),
            Print(badge),
            Print(" "),
        )?;

        let mut display_text = token.clean_text.clone();
        if let Some(l) = token.line_number {
            display_text.push_str(&format!(":{l}"));
        }

        let max_text_width = width.saturating_sub(12);
        let truncated = truncate_str(&display_text, max_text_width);

        if is_selected {
            execute!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: 236,
                    g: 239,
                    b: 244
                }),
                Print(&truncated),
                ResetColor,
            )?;
        } else {
            execute!(
                out,
                SetForegroundColor(Color::Rgb {
                    r: 216,
                    g: 222,
                    b: 233
                }),
                Print(&truncated),
                ResetColor,
            )?;
        }
    }

    // 5. Footer actions
    let footer_row = height.saturating_sub(1) as u16;
    let footer_text =
        "<Enter> Paste | <y> Copy | <g f/o> Editor | <g x> Browser | <Tab> Tab | <Esc/q> Quit";
    let truncated_footer = truncate_str(footer_text, width);
    execute!(
        out,
        cursor::MoveTo(0, footer_row),
        SetBackgroundColor(Color::Rgb {
            r: 59,
            g: 66,
            b: 82
        }),
        SetForegroundColor(Color::Rgb {
            r: 216,
            g: 222,
            b: 233
        }),
        Print(&truncated_footer),
        ResetColor,
    )?;

    out.flush()?;
    Ok(())
}

/// Returns the badge text and accent color for an entity kind.
fn badge_for_kind(kind: EntityKind) -> (&'static str, Color) {
    match kind {
        EntityKind::Path => (
            "[PATH]",
            Color::Rgb {
                r: 136,
                g: 192,
                b: 208,
            },
        ),
        EntityKind::Url => (
            "[URL ]",
            Color::Rgb {
                r: 163,
                g: 190,
                b: 140,
            },
        ),
        EntityKind::Hash => (
            "[HASH]",
            Color::Rgb {
                r: 235,
                g: 203,
                b: 139,
            },
        ),
        EntityKind::Ip => (
            "[IP  ]",
            Color::Rgb {
                r: 180,
                g: 142,
                b: 173,
            },
        ),
        EntityKind::Command => (
            "[CMD ]",
            Color::Rgb {
                r: 208,
                g: 135,
                b: 112,
            },
        ),
        EntityKind::Uuid => (
            "[UUID]",
            Color::Rgb {
                r: 129,
                g: 161,
                b: 193,
            },
        ),
        EntityKind::Quoted => (
            "[STR ]",
            Color::Rgb {
                r: 229,
                g: 233,
                b: 240,
            },
        ),
        EntityKind::Number => (
            "[NUM ]",
            Color::Rgb {
                r: 216,
                g: 222,
                b: 233,
            },
        ),
    }
}

/// Truncates string to at most `max_len` characters, appending ellipsis if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        s.to_string()
    } else if max_len > 1 {
        let mut truncated: String = chars[..max_len - 1].iter().collect();
        truncated.push('…');
        truncated
    } else {
        chars[..max_len].iter().collect()
    }
}

/// Opens a filesystem path in `$EDITOR` or `$VISUAL`.
pub fn open_in_editor(socket: Option<&str>, path: &str, line: Option<usize>) {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "nvim".to_string());
    let cmd = if let Some(l) = line {
        format!("{editor} +{l} {path}")
    } else {
        format!("{editor} {path}")
    };

    if execute_tmux(socket, &["new-window", "-n", "editor", "--", &cmd]).is_err() {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

/// Opens a URL in the default browser.
pub fn open_in_browser(url: &str) {
    let _ = std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .or_else(|_| std::process::Command::new("open").arg(url).spawn());
}
