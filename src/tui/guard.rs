//! Shared terminal raw mode guard for TUI interactive components.

use crossterm::{cursor, execute, terminal};
use std::io::stdout;

/// RAII guard that enables raw terminal mode and hides the cursor,
/// ensuring both are restored when dropped.
pub struct RawModeGuard;

impl RawModeGuard {
    pub fn new() -> Result<Self, String> {
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
