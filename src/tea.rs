//! Built-in tea timer for tea ceremonies (Gongfu Cha style).

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{Write, stdout};
use std::time::{Duration, Instant};

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self, String> {
        terminal::enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
        let mut out = stdout();
        let _ = execute!(out, cursor::Hide);
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = execute!(out, cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}

#[derive(Debug, Clone)]
pub struct TeaRound {
    pub name: String,
    pub duration_secs: u64,
}

pub struct TeaTimer {
    pub rounds: Vec<TeaRound>,
    pub current_round_idx: usize,
    pub remaining_secs: u64,
    pub is_running: bool,
    pub finished: bool,
}

impl TeaTimer {
    pub fn new() -> Self {
        let rounds = vec![
            TeaRound {
                name: "Rinse / Warming (Промывка)".into(),
                duration_secs: 10,
            },
            TeaRound {
                name: "Steep 1 (1-й пролив)".into(),
                duration_secs: 15,
            },
            TeaRound {
                name: "Steep 2 (2-й пролив)".into(),
                duration_secs: 20,
            },
            TeaRound {
                name: "Steep 3 (3-й пролив)".into(),
                duration_secs: 25,
            },
            TeaRound {
                name: "Steep 4 (4-й пролив)".into(),
                duration_secs: 30,
            },
            TeaRound {
                name: "Steep 5 (5-й пролив)".into(),
                duration_secs: 40,
            },
            TeaRound {
                name: "Steep 6 (6-й пролив)".into(),
                duration_secs: 50,
            },
        ];
        let initial_duration = rounds[0].duration_secs;
        Self {
            rounds,
            current_round_idx: 0,
            remaining_secs: initial_duration,
            is_running: false,
            finished: false,
        }
    }

    pub fn current_round(&self) -> &TeaRound {
        &self.rounds[self.current_round_idx]
    }

    pub fn reset_current_round(&mut self) {
        self.remaining_secs = self.current_round().duration_secs;
        self.is_running = false;
        self.finished = false;
    }

    pub fn next_round(&mut self) {
        if self.current_round_idx + 1 < self.rounds.len() {
            self.current_round_idx += 1;
        } else {
            // Add another round dynamically
            let num = self.rounds.len();
            let last_dur = self.rounds.last().map(|r| r.duration_secs).unwrap_or(30);
            self.rounds.push(TeaRound {
                name: format!("Steep {} ({}-й пролив)", num, num),
                duration_secs: last_dur + 10,
            });
            self.current_round_idx += 1;
        }
        self.reset_current_round();
    }

    pub fn prev_round(&mut self) {
        if self.current_round_idx > 0 {
            self.current_round_idx -= 1;
            self.reset_current_round();
        }
    }

    pub fn adjust_time(&mut self, delta_secs: i64) {
        if delta_secs < 0 {
            self.remaining_secs = self
                .remaining_secs
                .saturating_sub(delta_secs.unsigned_abs());
        } else {
            self.remaining_secs = self.remaining_secs.saturating_add(delta_secs as u64);
        }
    }

    pub fn toggle_running(&mut self) {
        if self.finished {
            self.reset_current_round();
        }
        self.is_running = !self.is_running;
    }

    pub fn tick(&mut self) {
        if self.is_running && self.remaining_secs > 0 {
            self.remaining_secs -= 1;
            if self.remaining_secs == 0 {
                self.is_running = false;
                self.finished = true;
                // Terminal bell
                print!("\x07");
                let _ = stdout().flush();
            }
        }
    }
}

impl Default for TeaTimer {
    fn default() -> Self {
        Self::new()
    }
}

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
        SetForegroundColor(Color::Yellow),
        Print(format!("🍵 {}\r\n", timer.current_round().name)),
        ResetColor
    )
    .map_err(|e| e.to_string())?;

    // Timer display
    if timer.finished {
        execute!(
            out,
            SetBackgroundColor(Color::Green),
            SetForegroundColor(Color::Black),
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
            Color::Cyan
        } else {
            Color::DarkGrey
        };

        execute!(
            out,
            SetForegroundColor(status_color),
            Print(format!("   {:02}:{:02}   {}\r\n", mins, secs, status)),
            ResetColor,
            SetForegroundColor(Color::Green),
            Print(format!("   [{bar}] {:>3}%\r\n\r\n", progress_pct)),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
    }

    // Controls
    execute!(
        out,
        SetForegroundColor(Color::DarkGrey),
        Print("[Space] Start/Pause   [n] Next   [p] Prev   [r] Reset   [+/-] Time   [q] Exit\r\n"),
        ResetColor
    )
    .map_err(|e| e.to_string())?;

    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}
