//! Vim-style scrollback navigation using tmux `copy-mode-vi`.

/// Returns the tmux configuration commands to setup Vim-like scrollback navigation.
pub fn generate_scroll_config() -> Vec<String> {
    vec![
        // Ensure vi mode keys are active in copy mode
        "set -g mode-keys vi".to_string(),
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
    ]
}
