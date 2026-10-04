//! Vim-style scrollback navigation using tmux `copy-mode-vi`.

/// Returns the tmux configuration commands to setup Vim-like scrollback navigation.
pub fn generate_scroll_config() -> Vec<String> {
    generate_scroll_config_with_bin("mox")
}

/// Returns the tmux configuration commands to setup Vim-like scrollback navigation with custom binary path.
pub fn generate_scroll_config_with_bin(bin: &str) -> Vec<String> {
    let theme = crate::ui::Theme::default();
    let color = crate::ui::status::tmux_color;
    let style = |foreground, background, bold| {
        super::quote(&format!(
            "fg={},bg={}{}",
            color(foreground),
            color(background),
            if bold { ",bold" } else { "" }
        ))
    };
    let binary = super::shell_quote(bin);
    let run = |args| super::quote(&format!("{binary} {args}"));
    let mut lines=vec![
        // Ensure vi mode keys are active in copy mode
        "set -g mode-keys vi".to_string(),
        format!("set -wg mode-style {}",style(theme.bg,theme.accent,true)),
        format!("set -wg copy-mode-match-style {}",style(theme.bg,theme.warning,false)),
        format!("set -wg copy-mode-current-match-style {}",style(theme.bg,theme.success,true)),
        format!("set -wg copy-mode-selection-style {}",style(theme.bg,theme.accent,false)),
        format!("set -wg copy-mode-position-style {}",style(theme.bg,theme.mode_agent,true)),
        "set -wg copy-mode-position-format ' ◈ #{?selection_present,VISUAL,COPY} · #{scroll_position}/#{history_size} '".to_string(),
        // Core vertical motions
        "bind-key -T copy-mode-vi j send-keys -X cursor-down".to_string(),
        "bind-key -T copy-mode-vi k send-keys -X cursor-up".to_string(),
        // Half-page navigation
        "bind-key -T copy-mode-vi C-d send-keys -X halfpage-down".to_string(),
        "bind-key -T copy-mode-vi C-u send-keys -X halfpage-up".to_string(),
        // Full-page navigation
        "bind-key -T copy-mode-vi C-f send-keys -X page-down".to_string(),
        "bind-key -T copy-mode-vi C-b send-keys -X page-up".to_string(),
        // Top and bottom of history
        "bind-key -T copy-mode-vi G send-keys -X history-bottom".to_string(),
        "bind-key -T copy-mode-vi g switch-client -T copy-mode-vi-g".to_string(),
        "bind-key -T copy-mode-vi-g g send-keys -X history-top".to_string(),
        "bind-key -T copy-mode-vi-g Escape switch-client -T copy-mode-vi".to_string(),
        "bind-key -T copy-mode-vi-g q switch-client -T copy-mode-vi".to_string(),
        // Exit copy-mode immediately without needing prefix
        "bind-key -T copy-mode-vi Escape send-keys -X cancel".to_string(),
        "bind-key -T copy-mode-vi q send-keys -X cancel".to_string(),
        // Vim visual selection and yank
        "bind-key -T copy-mode-vi v send-keys -X begin-selection".to_string(),
        "bind-key -T copy-mode-vi y send-keys -X copy-selection-and-cancel".to_string(),
        // Flash Jump and Extract integration in copy-mode-vi
        format!("bind-key -T copy-mode-vi s run-shell -b {}",run("flash --launch-popup")),
        format!("bind-key -T copy-mode-vi S run-shell -b {}",run("flash --launch-popup --multi-pane")),
        format!("bind-key -T copy-mode-vi e run-shell -b {}",run("extract --launch-popup")),
        format!("bind-key -T copy-mode-vi R run-shell -b {}",run("flash --launch-popup --mode remote-yank")),
        format!("bind-key -T copy-mode-vi f run-shell -b {}",run("flash --launch-popup --mode char-motion")),
        format!("bind-key -T copy-mode-vi F run-shell -b {}",run("flash --launch-popup --mode char-motion --backward")),
    ];
    lines.extend(crate::navigation::scroll_bindings());
    lines
}
