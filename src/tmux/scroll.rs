//! Vim-style scrollback navigation using tmux `copy-mode-vi`.

/// Returns the tmux configuration commands to setup Vim-like scrollback navigation.
pub fn generate_scroll_config() -> Vec<String> {
    generate_scroll_config_with_bin("mox")
}

/// Returns the tmux configuration commands to setup Vim-like scrollback navigation with custom binary path.
pub fn generate_scroll_config_with_bin(bin: &str) -> Vec<String> {
    vec![
        // Ensure vi mode keys are active in copy mode
        "set -g mode-keys vi".to_string(),
        // Seamless mouse wheel scrolling into copy mode
        "bind-key -n WheelUpPane if-shell -F \"#{||:#{pane_in_mode},#{mouse_any_flag}}\" \"send-keys -M\" \"copy-mode -e; send-keys -M\"".to_string(),
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
        format!("bind-key -T copy-mode-vi s run-shell \"{bin} flash --launch-popup\""),
        format!("bind-key -T copy-mode-vi S run-shell \"{bin} flash --launch-popup --multi-pane\""),
        format!("bind-key -T copy-mode-vi e run-shell \"{bin} extract --launch-popup\""),
        format!("bind-key -T copy-mode-vi R run-shell \"{bin} flash --launch-popup --mode remote-yank\""),
        format!("bind-key -T copy-mode-vi f command-prompt -1 -p \"(char forward)\" \"run-shell '{bin} flash --launch-popup --mode char-motion --char %%%'\""),
        format!("bind-key -T copy-mode-vi F command-prompt -1 -p \"(char backward)\" \"run-shell '{bin} flash --launch-popup --mode char-motion --backward --char %%%'\""),
    ]
}
