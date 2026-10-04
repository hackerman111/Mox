//! Status indicators, mode badges, and presentation state for tmux and Mox TUI.

use crate::ui::theme::Theme;
use crossterm::{
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
};
use std::io::Write;

/// Supported operational modes for Mox status display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Normal,
    Prefix,
    Copy,
    Visual,
    Resize,
    Sync,
    Suspend,
    Agent,
}

impl Mode {
    /// Returns the uppercase presentation string for the mode.
    pub fn label(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Prefix => "PREFIX",
            Mode::Copy => "COPY",
            Mode::Visual => "VISUAL",
            Mode::Resize => "RESIZE",
            Mode::Sync => "SYNC",
            Mode::Suspend => "SUSPEND",
            Mode::Agent => "AGENT",
        }
    }

    /// Resolves the theme color assigned to this mode.
    pub fn color(&self, theme: &Theme) -> Color {
        match self {
            Mode::Normal => theme.mode_normal,
            Mode::Prefix => theme.mode_prefix,
            Mode::Copy => theme.mode_copy,
            Mode::Visual => theme.mode_visual,
            Mode::Resize => theme.mode_resize,
            Mode::Sync => theme.mode_sync,
            Mode::Suspend => theme.mode_suspend,
            Mode::Agent => theme.mode_agent,
        }
    }
}

pub fn tmux_color(color: Color) -> String {
    match color {
        Color::Rgb { r, g, b } => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "black".into(),
        Color::White => "white".into(),
        Color::AnsiValue(n) => format!("colour{n}"),
        _ => "default".into(),
    }
}

/// Priority is client suspension, selection/copy, resize, sync, prefix, agent, normal.
pub fn tmux_indicator(theme: &Theme) -> String {
    let badge = |mode: Mode| {
        let (icon, label) = match mode {
            Mode::Normal => ("●", "NORMAL"),
            Mode::Prefix => ("◆", "PREFIX"),
            Mode::Visual => ("◈", "VISUAL"),
            Mode::Agent => ("▲", "AGENT"),
            Mode::Copy => ("⎘", "COPY"),
            Mode::Resize => ("↕", "RESIZE"),
            Mode::Sync => ("⇄", "SYNC"),
            Mode::Suspend => ("⏸", "SUSPEND"),
        };
        format!(
            "#[fg={}]#[bg={}]#[bold] {icon} {label} #[default] ",
            tmux_color(theme.bg),
            tmux_color(mode.color(theme)),
        )
    };
    let mut value = badge(Mode::Normal);
    for (condition, mode) in [
        ("#{==:#{@mox_is_agent},1}", Mode::Agent),
        (
            "#{||:#{client_prefix},#{m:mox*,#{client_key_table}}}",
            Mode::Prefix,
        ),
        ("#{pane_synchronized}", Mode::Sync),
        ("#{m:mox*r,#{client_key_table}}", Mode::Resize),
        ("#{pane_in_mode}", Mode::Copy),
        ("#{selection_present}", Mode::Visual),
        ("#{==:#{client_key_table},mox_suspend}", Mode::Suspend),
    ] {
        value = format!("#{{?{condition},{},{value}}}", badge(mode));
    }
    value
}

/// Renders a standardized mode badge with optional sticky indicator.
pub fn render_status_badge<W: Write>(
    out: &mut W,
    mode: Mode,
    sticky: bool,
    theme: &Theme,
) -> std::io::Result<()> {
    let bg = mode.color(theme);
    let (icon, label) = match mode {
        Mode::Normal => ("●", "NORMAL"),
        Mode::Prefix => ("◆", "PREFIX"),
        Mode::Visual => ("◈", "VISUAL"),
        Mode::Agent => ("▲", "AGENT"),
        Mode::Copy => ("⎘", "COPY"),
        Mode::Resize => ("↕", "RESIZE"),
        Mode::Sync => ("⇄", "SYNC"),
        Mode::Suspend => ("⏸", "SUSPEND"),
    };
    queue!(
        out,
        SetBackgroundColor(bg),
        SetForegroundColor(theme.bg),
        SetAttribute(Attribute::Bold),
        Print(format!(" {icon} {label} ")),
        SetAttribute(Attribute::Reset),
        ResetColor,
    )?;

    if sticky {
        queue!(
            out,
            SetForegroundColor(theme.warning),
            Print(" [STICKY]"),
            ResetColor,
        )?;
    }
    Ok(())
}

/// Renders a full-width bottom status bar with left content and right metadata.
pub fn render_status_bar<W: Write>(
    out: &mut W,
    left: &str,
    right: &str,
    width: usize,
    theme: &Theme,
) -> std::io::Result<()> {
    let left_len = super::render::display_width(left);
    let right_len = super::render::display_width(right);
    let padding = width.saturating_sub(left_len + right_len);

    queue!(
        out,
        SetBackgroundColor(theme.bg),
        SetForegroundColor(theme.fg),
        Print(left),
        Print(" ".repeat(padding)),
        SetForegroundColor(theme.muted),
        Print(right),
        ResetColor,
    )
}
