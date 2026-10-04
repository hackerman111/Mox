use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};
pub fn input(title: &str) -> Result<Option<String>, String> {
    let _guard = crate::tui::RawModeGuard::new()?;
    let mut text = String::new();
    let mut out = stdout();
    loop {
        execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0)).map_err(|e| e.to_string())?;
        super::render_prompt(
            &mut out,
            title,
            &text,
            None,
            terminal::size().map_err(|e| e.to_string())?.0 as usize,
            &super::Theme::default(),
        )
        .map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => return Ok(Some(text)),
                KeyCode::Backspace => {
                    text.pop();
                }
                KeyCode::Char(ch) => text.push(ch),
                _ => {}
            }
        }
    }
}

/// Interactive confirmation owns terminal mode when invoked directly from CLI.
pub fn confirm(message: &str) -> Result<bool, String> {
    let _guard = crate::tui::TuiGuard::enter().map_err(|e| e.to_string())?;
    crate::ui::view::prompt_confirm(message)
}

pub fn toast(message: &str, error: bool) -> Result<(), String> {
    use std::io::Write;
    let _guard = crate::tui::TuiGuard::enter().map_err(|e| e.to_string())?;
    let mut out = std::io::stdout();
    let (width, _) = crossterm::terminal::size().map_err(|e| e.to_string())?;
    crossterm::execute!(out, crossterm::cursor::MoveTo(0, 0)).map_err(|e| e.to_string())?;
    crate::ui::render_toast(
        &mut out,
        message,
        error,
        width as usize,
        &crate::ui::Theme::default(),
    )
    .map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    if crossterm::event::poll(std::time::Duration::from_millis(1500)).map_err(|e| e.to_string())? {
        let _ = crossterm::event::read().map_err(|e| e.to_string())?;
    }
    Ok(())
}
