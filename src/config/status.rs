//! Modular status bar generator, clock widgets, and mode indicators.
//!
//! Provides functions to construct tmux status line formatting strings
//! (`status-left`, `status-right`, and `window-status-format`) based on
//! user configuration and semantic theme color roles, with seamless
//! color transitions between adjacent segments.

use crate::config::{Config, SegmentConfig, SegmentKind, SeparatorStyle};
use crate::ui::status::{
    replace_mode_indicator_placeholder, tmux_color, tmux_indicator_with_config,
};
use crate::ui::theme::Theme;

/// Glyphs associated with a specific separator visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeparatorGlyphs {
    /// Left-to-right hard transition glyph (e.g. ).
    pub hard_left: &'static str,
    /// Left-to-right soft transition glyph (e.g. ).
    pub soft_left: &'static str,
    /// Right-to-left hard transition glyph (e.g. ).
    pub hard_right: &'static str,
    /// Right-to-left soft transition glyph (e.g. ).
    pub soft_right: &'static str,
}

/// Resolves the glyph set for a given separator style.
pub fn separator_glyphs(style: SeparatorStyle) -> SeparatorGlyphs {
    match style {
        SeparatorStyle::Powerline => SeparatorGlyphs {
            hard_left: "",
            soft_left: "",
            hard_right: "",
            soft_right: "",
        },
        SeparatorStyle::Rounded => SeparatorGlyphs {
            hard_left: "",
            soft_left: ")",
            hard_right: "",
            soft_right: "(",
        },
        SeparatorStyle::Slanted => SeparatorGlyphs {
            hard_left: "",
            soft_left: "/",
            hard_right: "",
            soft_right: "\\",
        },
        SeparatorStyle::Flat => SeparatorGlyphs {
            hard_left: "|",
            soft_left: "|",
            hard_right: "|",
            soft_right: "|",
        },
        SeparatorStyle::None => SeparatorGlyphs {
            hard_left: " ",
            soft_left: " ",
            hard_right: " ",
            soft_right: " ",
        },
    }
}

/// Generates the tmux mode indicator string for the active configuration.
pub fn generate_mode_indicator(config: &Config, theme: &Theme) -> String {
    tmux_indicator_with_config(&config.mode_indicator, theme)
}

/// Formatted properties of a status segment.
struct FormattedSegment {
    content: String,
    fg: String,
    bg: String,
    bold: bool,
}

fn format_segment(seg: &SegmentConfig, config: &Config, theme: &Theme) -> Option<FormattedSegment> {
    match seg.kind {
        SegmentKind::Mode => None,
        SegmentKind::Session => {
            let session_text = match seg.icon.as_deref() {
                Some(icon) => {
                    if icon.ends_with(' ') {
                        format!("{icon}#S")
                    } else {
                        format!("{icon} #S")
                    }
                }
                None => "#S".to_string(),
            };
            Some(FormattedSegment {
                content: format!(" {session_text} "),
                fg: tmux_color(theme.bg),
                bg: tmux_color(theme.accent),
                bold: true,
            })
        }
        SegmentKind::Date => {
            let icon = match seg.icon.as_deref() {
                Some(ic) => ic.to_string(),
                None if config.clock.nerd_icons => "󰃭 ".to_string(),
                None => String::new(),
            };
            let date_fmt = &config.clock.date_format;
            let date_str = if icon.is_empty() {
                date_fmt.to_string()
            } else if icon.ends_with(' ') {
                format!("{icon}{date_fmt}")
            } else {
                format!("{icon} {date_fmt}")
            };
            let (bg, fg) = if config.clock.pills {
                (tmux_color(theme.info), tmux_color(theme.bg))
            } else {
                (
                    tmux_color(theme.selection_bg),
                    tmux_color(theme.selection_fg),
                )
            };
            Some(FormattedSegment {
                content: format!(" {date_str} "),
                fg,
                bg,
                bold: false,
            })
        }
        SegmentKind::Clock => {
            let icon = match seg.icon.as_deref() {
                Some(ic) => ic.to_string(),
                None if config.clock.nerd_icons => " ".to_string(),
                None => String::new(),
            };
            let time_fmt = if config.clock.style_12h {
                "%I:%M %p"
            } else {
                config.clock.format.as_str()
            };
            let time_str = if icon.is_empty() {
                time_fmt.to_string()
            } else if icon.ends_with(' ') {
                format!("{icon}{time_fmt}")
            } else {
                format!("{icon} {time_fmt}")
            };
            Some(FormattedSegment {
                content: format!(" {time_str} "),
                fg: tmux_color(theme.selection_fg),
                bg: tmux_color(theme.selection_bg),
                bold: false,
            })
        }
        SegmentKind::Host => {
            let icon = match seg.icon.as_deref() {
                Some(ic) => ic.to_string(),
                None if config.clock.nerd_icons => "󰒋 ".to_string(),
                None => String::new(),
            };
            let host_str = if icon.is_empty() {
                "#H".to_string()
            } else if icon.ends_with(' ') {
                format!("{icon}#H")
            } else {
                format!("{icon} #H")
            };
            Some(FormattedSegment {
                content: format!(" {host_str} "),
                fg: tmux_color(theme.bg),
                bg: tmux_color(theme.accent),
                bold: true,
            })
        }
        SegmentKind::Custom => {
            let cmd = seg.command.as_deref().unwrap_or("");
            let base = format!("#({cmd})");
            let custom_str = match seg.icon.as_deref() {
                Some(icon) => {
                    if icon.ends_with(' ') {
                        format!("{icon}{base}")
                    } else {
                        format!("{icon} {base}")
                    }
                }
                None => base,
            };
            Some(FormattedSegment {
                content: format!(" {custom_str} "),
                fg: tmux_color(theme.selection_fg),
                bg: tmux_color(theme.selection_bg),
                bold: false,
            })
        }
        SegmentKind::Windows => None,
    }
}

/// Generates the formatted `status-left` string with color coupling and separators.
pub fn generate_status_left(config: &Config, theme: &Theme) -> String {
    let glyphs = separator_glyphs(config.status.separator_style);
    let status_bg = tmux_color(theme.bg);

    let mut out = String::new();
    let mut normal_segments: Vec<FormattedSegment> = Vec::new();

    for seg in &config.status.left {
        if seg.kind == SegmentKind::Mode {
            out.push_str(&generate_mode_indicator(config, theme));
        } else if let Some(formatted) = format_segment(seg, config, theme) {
            normal_segments.push(formatted);
        }
    }

    if !normal_segments.is_empty() {
        for (i, seg) in normal_segments.iter().enumerate() {
            let bold_attr = if seg.bold { ",bold" } else { "" };
            out.push_str(&format!(
                "#[fg={},bg={}{bold_attr}]{}",
                seg.fg, seg.bg, seg.content
            ));

            if i + 1 < normal_segments.len() {
                let next = &normal_segments[i + 1];
                if seg.bg == next.bg {
                    out.push_str(&format!(
                        "#[fg={},bg={}]{}",
                        seg.fg, seg.bg, glyphs.soft_left
                    ));
                } else {
                    out.push_str(&format!(
                        "#[fg={},bg={}]{}",
                        seg.bg, next.bg, glyphs.hard_left
                    ));
                }
            } else {
                // Last segment to status bar background transition
                out.push_str(&format!(
                    "#[fg={},bg={}]{}#[default]",
                    seg.bg, status_bg, glyphs.hard_left
                ));
            }
        }
    }

    let indicator = generate_mode_indicator(config, theme);
    replace_mode_indicator_placeholder(&out, &indicator)
}

/// Generates the formatted `status-right` string with right-to-left arrows and clock widgets.
pub fn generate_status_right(config: &Config, theme: &Theme) -> String {
    let glyphs = separator_glyphs(config.status.separator_style);
    let status_bg = tmux_color(theme.bg);

    let mut out = String::new();
    let mut normal_segments: Vec<FormattedSegment> = Vec::new();

    for seg in &config.status.right {
        if seg.kind == SegmentKind::Mode {
            out.push_str(&generate_mode_indicator(config, theme));
        } else if let Some(formatted) = format_segment(seg, config, theme) {
            normal_segments.push(formatted);
        }
    }

    if !normal_segments.is_empty() {
        // Leading separator from status bar background to first segment
        let first = &normal_segments[0];
        out.push_str(&format!(
            "#[fg={},bg={}]{}",
            first.bg, status_bg, glyphs.hard_right
        ));

        for (i, seg) in normal_segments.iter().enumerate() {
            let bold_attr = if seg.bold { ",bold" } else { "" };
            out.push_str(&format!(
                "#[fg={},bg={}{bold_attr}]{}",
                seg.fg, seg.bg, seg.content
            ));

            if i + 1 < normal_segments.len() {
                let next = &normal_segments[i + 1];
                if seg.bg == next.bg {
                    out.push_str(&format!(
                        "#[fg={},bg={}]{}",
                        seg.fg, seg.bg, glyphs.soft_right
                    ));
                } else {
                    out.push_str(&format!(
                        "#[fg={},bg={}]{}",
                        next.bg, seg.bg, glyphs.hard_right
                    ));
                }
            }
        }
    }

    let indicator = generate_mode_indicator(config, theme);
    replace_mode_indicator_placeholder(&out, &indicator)
}

/// Generates window tab formatting string (`window-status-format` or `window-status-current-format`).
pub fn generate_window_status(config: &Config, theme: &Theme, current: bool) -> String {
    let status_bg = tmux_color(theme.bg);
    let tab_bg = tmux_color(theme.selection_bg);
    let tab_fg = tmux_color(theme.selection_fg);
    let accent_bg = tmux_color(theme.accent);
    let accent_fg = tmux_color(theme.bg);

    let flags = if config.windows.show_flags { "#F" } else { "" };

    let is_rounded = config.windows.style.eq_ignore_ascii_case("rounded")
        || config.status.separator_style == SeparatorStyle::Rounded;
    let is_flat = config.windows.style.eq_ignore_ascii_case("flat")
        || config.status.separator_style == SeparatorStyle::Flat;

    if is_rounded {
        if current {
            format!(
                "#[fg={accent_bg},bg={status_bg}]#[fg={accent_fg},bg={accent_bg},bold]#I #W{flags}#[fg={accent_bg},bg={status_bg}]"
            )
        } else {
            format!(
                "#[fg={tab_bg},bg={status_bg}]#[fg={tab_fg},bg={tab_bg}]#I #W{flags}#[fg={tab_bg},bg={status_bg}]"
            )
        }
    } else if is_flat {
        if current {
            format!(" #[fg={accent_fg},bg={accent_bg},bold]#I #W{flags} ")
        } else {
            format!(" #[fg={tab_fg},bg={tab_bg}]#I #W{flags} ")
        }
    } else {
        // Powerline (default)
        if current {
            format!(
                "#[fg={status_bg},bg={accent_bg}] #[fg={accent_fg},bg={accent_bg},bold]#I  #W{flags} #[fg={accent_bg},bg={status_bg}]"
            )
        } else {
            format!(
                "#[fg={status_bg},bg={tab_bg}] #[fg={tab_fg},bg={tab_bg}]#I  #W{flags} #[fg={tab_bg},bg={status_bg}]"
            )
        }
    }
}

/// Returns the separator string between window status tabs (`window-status-separator`).
pub fn generate_window_status_separator(_config: &Config) -> &'static str {
    ""
}
