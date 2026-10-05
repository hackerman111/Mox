//! Terminal UI rendering and event loop for the Gongfu Cha tea timer.

use super::model::TeaTimer;
use crate::tui::RawModeGuard;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Write, stdout};
use std::time::{Duration, Instant};

/// Runs the tea timer in the current terminal window or tmux popup.
pub fn run_tea_timer() -> Result<(), String> {
    let _guard = RawModeGuard::new()?;
    let mut timer = TeaTimer::new();
    let mut last_tick = Instant::now();

    loop {
        render_tea_ui(&timer)?;

        let timeout = Duration::from_millis(200);
        if event::poll(timeout).map_err(|e| e.to_string())?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char(' ') => timer.toggle_running(),
                KeyCode::Char('n') => timer.next_round(),
                KeyCode::Char('p') => timer.prev_round(),
                KeyCode::Char('r') => timer.reset_current_round(),
                KeyCode::Char('+') | KeyCode::Char('=') => timer.adjust_time(5),
                KeyCode::Char('-') => timer.adjust_time(-5),
                KeyCode::Char('q') | KeyCode::Esc => break,
                _ => {}
            }
        }

        if last_tick.elapsed() >= Duration::from_secs(1) {
            timer.tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn render_tea_ui(timer: &TeaTimer) -> Result<(), String> {
    let theme = crate::ui::Theme::load(None);
    let mut out = stdout();
    execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0)).map_err(|e| e.to_string())?;

    let total = timer.current_round().duration_secs.max(1);
    let rem = timer.remaining_secs;
    let elapsed = total.saturating_sub(rem);
    let progress_pct = ((elapsed as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as usize;

    let bar_len = 24;
    let filled_len = (bar_len * progress_pct) / 100;
    let empty_len = bar_len.saturating_sub(filled_len);
    let bar = format!("{}{}", "█".repeat(filled_len), "░".repeat(empty_len));

    let mins = rem / 60;
    let secs = rem % 60;

    // Header: round info
    execute!(
        out,
        SetForegroundColor(theme.warning),
        Print(format!("🍵 {}\r\n", timer.current_round().name)),
        ResetColor
    )
    .map_err(|e| e.to_string())?;

    // Timer display
    if timer.finished {
        execute!(
            out,
            SetBackgroundColor(theme.success),
            SetForegroundColor(theme.bg),
            Print("  >>> TEA READY! ENJOY! <<<  "),
            ResetColor,
            Print("\r\n\r\n")
        )
        .map_err(|e| e.to_string())?;
    } else {
        let status = if timer.is_running {
            "[BREWING...]"
        } else {
            "[PAUSED]"
        };
        let status_color = if timer.is_running {
            theme.info
        } else {
            theme.muted
        };

        execute!(
            out,
            SetForegroundColor(status_color),
            Print(format!("   {:02}:{:02}   {}\r\n", mins, secs, status)),
            ResetColor,
            SetForegroundColor(theme.success),
            Print(format!("   [{bar}] {:>3}%\r\n\r\n", progress_pct)),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
    }

    // Controls
    execute!(
        out,
        SetForegroundColor(theme.muted),
        Print("[Space] Start/Pause   [n] Next   [p] Prev   [r] Reset   [+/-] Time   [q] Exit\r\n"),
        ResetColor
    )
    .map_err(|e| e.to_string())?;

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}
