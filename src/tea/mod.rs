//! Built-in tea timer for tea ceremonies (Gongfu Cha style).

pub mod model;
pub mod ui;

pub use model::{TeaRound, TeaTimer};
pub use ui::run_tea_timer;
