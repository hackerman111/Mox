# Design Document: Tmux UI & Pane Indicator in FloaX Scratchpad

## Summary
Provide visible tmux status line and pane indicators inside FloaX floating scratchpad windows, allowing users to clearly see which window/pane is active when splitting terminals within the popup.

## Requirements & Scope
1. **Status Bar**: Enable status bar (`status on`) inside the `mox-scratch` session by default, inheriting Mox status widgets and theme styles.
2. **Pane Borders & Active Indicator**:
   - Enable `pane-border-status top`.
   - Configure `pane-border-format` with clear distinctions between active (`● Pane #P: #{pane_current_command}`) and inactive (`○ Pane #P: #{pane_current_command}`) panes.
   - Use `pane-border-lines rounded` where supported.
3. **Configuration**:
   - Expose `show_status: bool` and `show_pane_borders: bool` in `FloaxConfig`.
   - Provide builder methods `.with_status(bool)` and `.with_pane_borders(bool)`.
4. **Idempotence & Live Sessions**:
   - Apply these options when creating the floax session and update existing sessions so currently running background sessions gain the UI without needing manual restarts.

## Technical Architecture

### 1. `FloaxConfig`
Extend `FloaxConfig` in `src/apps/floax.rs`:
```rust
pub struct FloaxConfig {
    pub session_name: String,
    pub title: String,
    pub width: String,
    pub height: String,
    pub change_path: bool,
    pub show_status: bool,
    pub show_pane_borders: bool,
}
```
Default values:
- `show_status: true`
- `show_pane_borders: true`

### 2. Session Options Configuration
In `src/apps/floax.rs`:
Define helper `apply_floax_session_options(socket: Option<&str>, config: &FloaxConfig) -> Result<(), String>`:
- `set-option -t <session> detach-on-destroy on`
- If `config.show_status`: `set-option -t <session> status on` (else `status off`)
- If `config.show_pane_borders`:
  - `set-option -t <session> pane-border-status top`
  - `set-option -t <session> pane-border-lines rounded`
  - `set-option -t <session> pane-border-format " #{?pane_active,#[fg=cyan,bold]● Pane #P: #{pane_current_command}#[default],#[fg=brightblack]○ Pane #P: #{pane_current_command}#[default]} "`
  - `set-option -t <session> pane-active-border-style "fg=cyan,bold"`

Call `apply_floax_session_options` in:
- `ensure_floax_session`
- `toggle_floax` (to guarantee any existing session is updated)

## Verification Plan
1. Unit tests in `tests/apps_floax_tests.rs`:
   - Verify `FloaxConfig` defaults for `show_status` and `show_pane_borders`.
   - Verify builder methods.
   - Verify `ensure_floax_session` sets `status on`, `pane-border-status top`, and expected `pane-border-format`.
2. Workspace checks:
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo fmt --all -- --check`
3. Live build and test in tmux.
