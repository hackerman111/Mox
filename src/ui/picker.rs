//! Feature-neutral fuzzy selection and interactive List/Preview runner.
use super::preview::{Preview, PreviewWorker};
use super::{
    ListRow, PreviewPanel, Theme, Viewport, render_hints, render_list_row, render_preview_panel,
    render_prompt,
};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PickerItem {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub preview: Preview,
}

#[derive(Debug, Default)]
pub struct PickerState {
    pub query: String,
    pub selected: usize,
    pub offset: usize,
    pub indices: Vec<usize>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Input {
    Changed,
    Accept,
    Cancel,
    None,
}

impl PickerState {
    pub fn filter<'a>(&mut self, values: impl IntoIterator<Item = (usize, &'a str)>) {
        let previous = self.indices.get(self.selected).copied();
        let mut ranked = values
            .into_iter()
            .filter_map(|(index, text)| {
                super::fuzzy::fuzzy_match(text, &self.query).map(|score| (index, score))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        self.indices = ranked.into_iter().map(|(index, _)| index).collect();
        self.selected = previous
            .and_then(|id| self.indices.iter().position(|&i| i == id))
            .unwrap_or(0);
        self.offset = 0;
    }
    pub fn input(&mut self, key: KeyEvent, len: usize, search: bool) -> Input {
        if key.kind != KeyEventKind::Press {
            return Input::None;
        }
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => Input::Cancel,
            KeyCode::Enter => Input::Accept,
            KeyCode::Up | KeyCode::Char('k') if !search || control || key.code == KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                Input::None
            }
            KeyCode::Down | KeyCode::Char('j')
                if !search || control || key.code == KeyCode::Down =>
            {
                self.selected = (self.selected + 1).min(len.saturating_sub(1));
                Input::None
            }
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(10);
                Input::None
            }
            KeyCode::PageDown => {
                self.selected = (self.selected + 10).min(len.saturating_sub(1));
                Input::None
            }
            KeyCode::Backspace if search => {
                self.query.pop();
                Input::Changed
            }
            KeyCode::Char(c)
                if search && !control && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.query.push(c);
                Input::Changed
            }
            _ => Input::None,
        }
    }
    pub fn viewport(&mut self, len: usize, height: usize) {
        self.selected = self.selected.min(len.saturating_sub(1));
        self.offset = Viewport::adjust_scroll(self.selected, self.offset, height);
    }
}

pub fn run_picker(
    title: &str,
    items: &[PickerItem],
    socket: Option<&str>,
) -> Result<Option<usize>, String> {
    let _guard = crate::tui::TuiGuard::enter().map_err(|e| e.to_string())?;
    let mut state = PickerState::default();
    state.filter(
        items
            .iter()
            .enumerate()
            .map(|(i, item)| (i, item.label.as_str())),
    );
    let theme = Theme::default();
    let mut out = stdout();
    let worker = PreviewWorker::new(socket);
    let mut selected_id = None;
    let mut generation = 0;
    let mut sent = false;
    let mut lines = Vec::new();
    loop {
        let (width, height) = terminal::size().map_err(|e| e.to_string())?;
        let (width, height) = (width as usize, height as usize);
        let rows = height.saturating_sub(4);
        state.viewport(state.indices.len(), rows);
        let original = state.indices.get(state.selected).copied();
        if original != selected_id {
            selected_id = original;
            generation += 1;
            sent = false;
            lines.clear();
        }
        if !sent && let Some(index) = original {
            sent = worker.request(generation, &items[index].preview);
        }
        if let Some(result) = worker.result(generation) {
            lines = result;
        }
        let list_width = if width >= 70 { width / 2 } else { width };
        execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0)).map_err(|e| e.to_string())?;
        render_prompt(
            &mut out,
            title,
            &state.query,
            Some(&format!("{} results", state.indices.len())),
            width,
            &theme,
        )
        .map_err(|e| e.to_string())?;
        for (row, &index) in state
            .indices
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(rows)
        {
            execute!(out, cursor::MoveTo(0, (row - state.offset + 2) as u16))
                .map_err(|e| e.to_string())?;
            render_list_row(
                &mut out,
                &ListRow {
                    prefix: "",
                    badge: None,
                    text: &items[index].label,
                    is_selected: row == state.selected,
                },
                list_width,
                &theme,
            )
            .map_err(|e| e.to_string())?;
        }
        if list_width < width {
            render_preview_panel(
                &mut out,
                &PreviewPanel {
                    title: "Preview",
                    lines: &lines,
                    x: list_width,
                    y: 1,
                    width: width - list_width,
                    height: height.saturating_sub(2),
                },
                &theme,
            )
            .map_err(|e| e.to_string())?;
        }
        execute!(out, cursor::MoveTo(0, height.saturating_sub(1) as u16))
            .map_err(|e| e.to_string())?;
        render_hints(
            &mut out,
            "type: filter · ↑↓/C-j/k: select · Enter: accept · Esc: cancel",
            width,
            &theme,
        )
        .map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        if !event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
            continue;
        }
        if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
            match state.input(key, state.indices.len(), true) {
                Input::Changed => state.filter(
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| (i, item.label.as_str())),
                ),
                Input::Accept => return Ok(original),
                Input::Cancel => return Ok(None),
                Input::None => {}
            }
        }
    }
}
