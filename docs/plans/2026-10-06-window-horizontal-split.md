# Window Horizontal Split Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Bind key `s` in window mode (`M-m w`) to execute `split-window -v -c "#{pane_current_path}"` labeled "split horizontal".

**Architecture:** Update `window_children` in `src/keymap/defaults.rs` and verify via unit tests in `tests/keymap_tests.rs`.

**Tech Stack:** Rust 2024, tmux keybinding generator.

---

### Task 1: Bind Key 's' to Horizontal Split in Window Mode

**Files:**
- Modify: `src/keymap/defaults.rs:155-165`
- Test: `tests/keymap_tests.rs:85-100`

**Step 1: Write the failing test in `tests/keymap_tests.rs`**
In `test_sticky_window_and_resize_modes`:
```rust
    let split_h = window_node.find_child("s").expect("split horizontal missing in window mode");
    assert_eq!(split_h.label, "split horizontal");
    assert_eq!(
        split_h.action,
        Some(KeyAction::Tmux("split-window -v -c \"#{pane_current_path}\"".into()))
    );
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test keymap_tests test_sticky_window_and_resize_modes`
Expected: FAIL (assertion failed: label == "split horizontal", currently "tree navigator")

**Step 3: Implement minimal change in `src/keymap/defaults.rs`**
Update line in `window_children`:
```rust
        KeyNode::new_action(
            "s",
            "split horizontal",
            KeyAction::Tmux("split-window -v -c \"#{pane_current_path}\"".into()),
        ),
```

**Step 4: Run test to verify it passes**
Run: `cargo test --test keymap_tests test_sticky_window_and_resize_modes`
Expected: PASS

**Step 5: Run all workspace tests**
Run: `cargo test --workspace`
Expected: PASS

**Step 6: Run clippy and fmt**
Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`
Expected: PASS

**Step 7: Commit**
```bash
git add src/keymap/defaults.rs tests/keymap_tests.rs
git commit -m "feat(keymap): bind 's' to horizontal split in window mode"
```

---

### Task 2: Build, Deploy, and Push

**Step 1: Build release and apply to tmux**
Run: `cargo build --release && cp target/release/mox ~/.cargo/bin/mox && mox init --apply`

**Step 2: Push to GitHub**
Run: `git push origin main`
