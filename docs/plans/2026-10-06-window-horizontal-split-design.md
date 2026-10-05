# Design Document: Horizontal Split in Window Menu

## Summary
Add horizontal pane splitting to the window submenu (`M-m w`) using key `s`, providing symmetric Vim-like split navigation alongside vertical split `v`.

## Requirements
- Key `s` in window mode (`M-m w`) executes `split-window -v -c "#{pane_current_path}"`.
- Label is "split horizontal".
- Existing key `v` remains "split vertical" (`split-window -h -c "#{pane_current_path}"`).
- Existing key `S` remains "session menu".

## Architecture & Code Changes
1. `src/keymap/defaults.rs`:
   Update `window_children` definition so key `s` maps to `KeyAction::Tmux("split-window -v -c \"#{pane_current_path}\"".into())` with label `"split horizontal"`.
2. `tests/keymap_tests.rs`:
   Add test assertion verifying `w` mode contains key `s` with action `split-window -v`.

## Verification
- `cargo test --test keymap_tests`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
