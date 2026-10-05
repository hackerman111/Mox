# Apps Panel, Floax Scratchpad & Tmux Window Tracking Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Extract and generalize `lazygit` into a modular `apps` engine with `btop` support, integrate Floax persistent floating scratchpad terminal, provide an interactive Apps Panel, and support tracking and quick-toggling arbitrary tmux windows.

**Architecture:** A dedicated `src/apps/` module containing `model.rs`, `runner.rs`, `floax.rs`, `tracking.rs`, and `ui.rs`, exposed via CLI commands, keymaps, and the Mox configuration system, while keeping complete backward compatibility with `mox lazygit`.

**Tech Stack:** Rust 2024, tmux CLI integration (`display-popup`, `list-panes`, `list-windows`, sessions), Crossterm TUI picker, serde/toml.

---

### Task 1: Domain Models and Configuration for Apps

**Files:**
- Create: `src/apps/model.rs`
- Modify: `src/config/model.rs`
- Modify: `src/config/loader.rs`
- Test: `tests/apps_model_tests.rs`

**Step 1: Write the failing unit tests for app models and configuration parsing**
In `tests/apps_model_tests.rs`:
```rust
use mox::apps::model::{AppConfig, AppLaunchMode};
use mox::config::model::Config;

#[test]
fn test_default_config_includes_default_apps() {
    let config = Config::default();
    assert_eq!(config.apps.len(), 2);
    assert_eq!(config.apps[0].name, "lazygit");
    assert_eq!(config.apps[1].name, "btop");
}

#[test]
fn test_parse_custom_app_config() {
    let toml = r#"
        [[apps]]
        name = "htop"
        title = "Htop Monitor"
        command = "htop"
        key = "h"
        mode = { type = "popup", width = "90%", height = "80%" }
        focus_existing = true
        check_binary = true
    "#;
    let config: Config = toml::from_str(toml).expect("valid toml");
    assert_eq!(config.apps.len(), 1);
    assert_eq!(config.apps[0].name, "htop");
    assert_eq!(
        config.apps[0].mode,
        AppLaunchMode::Popup {
            width: "90%".into(),
            height: "80%".into()
        }
    );
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test apps_model_tests`
Expected: FAIL (types and fields do not exist yet)

**Step 3: Implement minimal domain models and config updates**
- Create `src/apps/model.rs` with `AppConfig`, `AppLaunchMode`, `TrackedWindow`, `default_apps()`.
- Add `pub apps: Vec<AppConfig>` to `Config` in `src/config/model.rs`.
- Add default toml serialization in `src/config/loader.rs`.

**Step 4: Run test to verify it passes**
Run: `cargo test --test apps_model_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/apps/model.rs src/config/model.rs src/config/loader.rs tests/apps_model_tests.rs
git commit -m "feat(apps): define AppConfig and wire into configuration models"
```

---

### Task 2: Generalized Runner & Backward-Compatible LazyGit Facade

**Files:**
- Create: `src/apps/runner.rs`
- Create: `src/apps/mod.rs`
- Modify: `src/lazygit.rs` (forward calls to `mox::apps::open_lazygit`)
- Modify: `src/lib.rs` (expose `pub mod apps;`)
- Test: `tests/logging_lazygit_tests.rs`

**Step 1: Verify existing lazygit tests pass before migration**
Run: `cargo test --test logging_lazygit_tests`
Expected: PASS

**Step 2: Implement `src/apps/runner.rs` and `src/apps/mod.rs`**
- In `src/apps/runner.rs`, generalize:
  - Checking binary availability via `check_binary(command)`
  - Querying existing panes with `find_existing_pane(socket, session, command, cwd)`
  - Focusing existing pane or spawning popup/window.
- In `src/apps/mod.rs`, expose `run_app(socket, &AppConfig, Option<&str>)` and `open_lazygit(socket, cwd, window)`.
- Replace `src/lazygit.rs` implementation with delegation to `crate::apps::open_lazygit`.

**Step 3: Run existing tests to verify zero regressions**
Run: `cargo test --test logging_lazygit_tests`
Expected: PASS

**Step 4: Commit**
```bash
git add src/apps/runner.rs src/apps/mod.rs src/lazygit.rs src/lib.rs
git commit -m "refactor(apps): generalize application runner and migrate lazygit"
```

---

### Task 3: Floax Persistent Scratchpad Terminal

**Files:**
- Create: `src/apps/floax.rs`
- Modify: `src/apps/mod.rs`
- Test: `tests/apps_floax_tests.rs`

**Step 1: Write test for Floax session and popup commands**
In `tests/apps_floax_tests.rs`:
```rust
#[test]
fn test_floax_session_name_and_popup_construction() {
    let session = mox::apps::floax::FloaxSession::default();
    assert_eq!(session.session_name, "mox-scratch");
    let popup_cmd = session.build_attach_command();
    assert!(popup_cmd.contains("mox-scratch"));
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test apps_floax_tests`
Expected: FAIL

**Step 3: Implement `src/apps/floax.rs`**
- Structure `FloaxSession` with configurable session name, title, width, height.
- Methods:
  - `has_session(socket)`
  - `ensure_session(socket, cwd)`
  - `toggle(socket, cwd)`: opens `display-popup -E ... "tmux attach-session -t <name>"`.

**Step 4: Run test to verify it passes**
Run: `cargo test --test apps_floax_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/apps/floax.rs src/apps/mod.rs tests/apps_floax_tests.rs
git commit -m "feat(apps): implement native floax persistent scratchpad terminal"
```

---

### Task 4: Arbitrary Tmux Window Tracking & Quick-Toggling

**Files:**
- Create: `src/apps/tracking.rs`
- Modify: `src/apps/mod.rs`
- Test: `tests/apps_tracking_tests.rs`

**Step 1: Write unit tests for window tracking parsing and state transitions**
In `tests/apps_tracking_tests.rs`:
```rust
#[test]
fn test_parse_tracked_windows() {
    let raw = "@1\tcode\t0\n@2\tcodex\t1\n@3\tagent\t1\n";
    let list = mox::apps::tracking::parse_tracked_windows(raw);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "@2");
    assert_eq!(list[0].name, "codex");
    assert_eq!(list[1].id, "@3");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test apps_tracking_tests`
Expected: FAIL

**Step 3: Implement `src/apps/tracking.rs`**
- `track_current_window(socket)`
- `untrack_current_window(socket)`
- `is_window_tracked(socket, window_id)`
- `list_tracked_windows(socket)`
- `toggle_tracked_window(socket, target_window_id)` (saves `@mox_prev_window` and restores it).

**Step 4: Run test to verify it passes**
Run: `cargo test --test apps_tracking_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/apps/tracking.rs src/apps/mod.rs tests/apps_tracking_tests.rs
git commit -m "feat(apps): implement tmux window tracking and quick-toggle"
```

---

### Task 5: Interactive Apps Panel UI

**Files:**
- Create: `src/apps/ui.rs`
- Modify: `src/apps/mod.rs`
- Test: `tests/apps_ui_tests.rs`

**Step 1: Write unit test for Apps Panel item builder**
In `tests/apps_ui_tests.rs`:
```rust
#[test]
fn test_build_apps_picker_items() {
    let items = mox::apps::ui::build_picker_items(&[], &[], false);
    assert!(items.iter().any(|i| i.id == "floax"));
    assert!(items.iter().any(|i| i.id == "track_current"));
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test apps_ui_tests`
Expected: FAIL

**Step 3: Implement `src/apps/ui.rs`**
- Implement `run_apps_panel(socket, config)` using `crate::ui::picker::run_picker`.
- Build list items for:
  - Applications (LazyGit, btop, custom configured apps)
  - Floax Scratchpad (with status indication)
  - Tracked Windows (with quick jump)
  - Track / Untrack actions.
- Execute selection: launch app / toggle Floax / toggle window / toggle tracking.

**Step 4: Run test to verify it passes**
Run: `cargo test --test apps_ui_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/apps/ui.rs src/apps/mod.rs tests/apps_ui_tests.rs
git commit -m "feat(apps): implement interactive apps and scratchpad picker panel"
```

---

### Task 6: CLI Subcommands

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `tests/apps_cli_tests.rs`

**Step 1: Add CLI subcommands**
- `Apps { popup: bool, socket: Option<String> }`
- `Floax { socket: Option<String> }`
- `Track { command: TrackCommand, socket: Option<String> }`
  - `TrackCommand::Add`, `TrackCommand::Remove`, `TrackCommand::Toggle`, `TrackCommand::List`
- `AppRun { name: String, socket: Option<String> }`

**Step 2: Write and run test verifying CLI argument parsing**
Run: `cargo test --test apps_cli_tests`
Expected: PASS

**Step 3: Commit**
```bash
git add src/cli.rs src/main.rs tests/apps_cli_tests.rs
git commit -m "feat(cli): wire up apps, floax, and track commands"
```

---

### Task 7: Keymap & Tmux Config Integration

**Files:**
- Modify: `src/actions.rs`
- Modify: `src/keymap/types.rs`
- Modify: `src/keymap/defaults.rs`
- Modify: `src/tmux/config.rs`
- Modify: `src/ui/runner.rs`

**Step 1: Add KeyAction variants and FeatureAction variants**
- `KeyAction::AppsPanel`
- `KeyAction::FloaxToggle`
- `KeyAction::TrackToggle`

**Step 2: Wire bindings into `build_default_keymap`**
- `a` / `apps`: Apps Panel
- `p` / `floax`: Floax floating scratchpad
- `t` / `track-toggle`: Tracked window toggle
- Preserve `G` / `N` for LazyGit and add `B` for btop.

**Step 3: Wire into `src/tmux/config.rs` generator**
- Generate tmux `display-popup` / `run-shell` bindings for `AppsPanel`, `FloaxToggle`, and `TrackToggle`.

**Step 4: Verify keymap tests pass**
Run: `cargo test --test keymap_tests`
Expected: PASS

**Step 5: Commit**
```bash
git add src/actions.rs src/keymap/types.rs src/keymap/defaults.rs src/tmux/config.rs src/ui/runner.rs
git commit -m "feat(keymap): register apps panel, floax, and track actions in keymaps"
```

---

### Task 8: Full Verification & Integration Tests

**Files:**
- Create: `tests/apps_integration_tests.rs`
- Workspace checks

**Step 1: Write isolated tmux server integration tests**
Test full end-to-end flow:
- Floax session creation, toggle, detach preservation.
- Window tracking mark, toggle to previous window, return back.
- LazyGit and btop launching logic.

**Step 2: Run verification commands**
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

**Step 3: Commit**
```bash
git add tests/apps_integration_tests.rs
git commit -m "test(apps): add end-to-end integration tests for apps, floax, and tracking"
```
