//! Domain model and state machine for the Gongfu Cha tea timer.

use std::io::{Write, stdout};

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
