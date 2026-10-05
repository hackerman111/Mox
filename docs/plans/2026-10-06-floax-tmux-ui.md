# FloaX Tmux UI & Pane Indicator Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Enable tmux status bar and pane split border indicators with active pane highlight inside the FloaX floating scratchpad terminal.

**Architecture:** Extend `FloaxConfig` with `show_status` and `show_pane_borders`, and configure `mox-scratch` session options (`status on`, `pane-border-status top`, `pane-border-format`) dynamically on creation and toggle.

**Tech Stack:** Rust 2024, tmux CLI integration, clap, cargo test.

---

### Task 1: Extend FloaxConfig with UI & Pane Indicator Options

**Files:**
- Modify: `src/apps/floax.rs:30-75`
- Test: `tests/apps_floax_tests.rs:40-75`

**Step 1: Write the failing test in `tests/apps_floax_tests.rs`**
Add assertions to `test_default_floax_config_values` and `test_floax_config_custom_builder_options`:
```rust
#[test]
fn test_default_floax_config_ui_values() {
    let config = FloaxConfig::default();
    assert!(config.show_status);
    assert!(config.show_pane_borders);
    assert_eq!(config.pane_border_position, "top");
}

#[test]
fn test_floax_config_ui_builder_options() {
    let custom = FloaxConfig::default()
        .with_status(false)
        .with_pane_borders(false)
        .with_pane_border_position("bottom");

    assert!(!custom.show_status);
    assert!(!custom.show_pane_borders);
    assert_eq!(custom.pane_border_position, "bottom");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test apps_floax_tests test_default_floax_config_ui_values`
Expected: FAIL (fields `show_status` not found)

**Step 3: Implement minimal code in `src/apps/floax.rs`**
Extend `FloaxConfig`:
```rust
pub struct FloaxConfig {
    pub session_name: String,
    pub title: String,
    pub width: String,
    pub height: String,
    pub change_path: bool,
    pub show_status: bool,
    pub show_pane_borders: bool,
    pub pane_border_position: String,
}
```
In `Default for FloaxConfig`:
- `show_status: true`
- `show_pane_borders: true`
- `pane_border_position: "top".to_string()`

Add builder methods:
- `.with_status(mut self, show_status: bool) -> Self`
- `.with_pane_borders(mut self, show_pane_borders: bool) -> Self`
- `.with_pane_border_position(mut self, position: impl Into<String>) -> Self`

**Step 4: Run tests to verify they pass**
Run: `cargo test --test apps_floax_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/apps/floax.rs tests/apps_floax_tests.rs
git commit -m "feat(floax): add UI and pane border options to FloaxConfig"
```

---

### Task 2: Apply Status & Pane Border Options to FloaX Session

**Files:**
- Modify: `src/apps/floax.rs:130-180`
- Test: `tests/apps_floax_tests.rs:150-185`

**Step 1: Write the failing test in `tests/apps_floax_tests.rs`**
In `test_floax_session_lifecycle_and_options`, verify `status` is `"on"`, `pane-border-status` is `"top"`, and `pane-border-format` contains `#P`:
```rust
    let status_opt = mox::tmux::execute_tmux(
        socket,
        &["show-option", "-t", &config.session_name, "-qv", "status"],
    )
    .expect("get status option");
    assert_eq!(status_opt.trim(), "on");

    let border_status = mox::tmux::execute_tmux(
        socket,
        &["show-option", "-t", &config.session_name, "-qv", "pane-border-status"],
    )
    .expect("get pane-border-status option");
    assert_eq!(border_status.trim(), "top");

    let border_fmt = mox::tmux::execute_tmux(
        socket,
        &["show-option", "-t", &config.session_name, "-qv", "pane-border-format"],
    )
    .expect("get pane-border-format option");
    assert!(border_fmt.contains("#P"));
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test apps_floax_tests test_floax_session_lifecycle_and_options`
Expected: FAIL (assertion failed: status == "on", currently "off")

**Step 3: Implement `apply_floax_session_options` and wire into `ensure_floax_session` and `toggle_floax`**
In `src/apps/floax.rs`:
```rust
pub fn apply_floax_session_options(
    socket: Option<&str>,
    config: &FloaxConfig,
) -> Result<(), String> {
    execute_tmux(
        socket,
        &[
            "set-option",
            "-t",
            &config.session_name,
            "detach-on-destroy",
            "on",
        ],
    )?;

    if config.show_status {
        execute_tmux(
            socket,
            &["set-option", "-t", &config.session_name, "status", "on"],
        )?;
    } else {
        execute_tmux(
            socket,
            &["set-option", "-t", &config.session_name, "status", "off"],
        )?;
    }

    if config.show_pane_borders {
        let _ = execute_tmux(
            socket,
            &[
                "set-option",
                "-t",
                &config.session_name,
                "pane-border-status",
                &config.pane_border_position,
            ],
        );
        let _ = execute_tmux(
            socket,
            &[
                "set-option",
                "-t",
                &config.session_name,
                "pane-border-lines",
                "rounded",
            ],
        );
        let _ = execute_tmux(
            socket,
            &[
                "set-option",
                "-t",
                &config.session_name,
                "pane-border-format",
                " #{?pane_active,#[fg=cyan,bold]● Pane #P: #{pane_current_command}#[default],#[fg=brightblack]○ Pane #P: #{pane_current_command}#[default]} ",
            ],
        );
        let _ = execute_tmux(
            socket,
            &[
                "set-option",
                "-t",
                &config.session_name,
                "pane-active-border-style",
                "fg=cyan,bold",
            ],
        );
    }

    Ok(())
}
```
Update `ensure_floax_session` and `toggle_floax` to invoke `apply_floax_session_options`.

**Step 4: Run tests to verify they pass**
Run: `cargo test --test apps_floax_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/apps/floax.rs tests/apps_floax_tests.rs
git commit -m "feat(floax): apply status on and pane-border-status indicators to scratchpad"
```

---

### Task 3: Full Verification, Build, Deploy and Push

**Files:**
- All workspace files

**Step 1: Run workspace checks**
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

**Step 2: Build release binary and deploy to live tmux**
- `cargo build --release`
- `cp target/release/mox ~/.cargo/bin/mox`
- `mox init --apply`

**Step 3: Commit and push**
```bash
git push origin main
```
