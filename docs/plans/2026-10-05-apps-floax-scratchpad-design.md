# Design Document: Apps Panel, Floax Scratchpad & Tmux Window Tracking

**Date:** 2026-10-05  
**Status:** Approved  

---

## 1. Overview & Goals

Mox provides fast, native terminal ergonomics for tmux workflows. Currently, `lazygit` is implemented in a single standalone module (`src/lazygit.rs`).
This design extracts and generalizes application management into a dedicated `apps` module, integrates a native persistent floating scratchpad terminal based on the `floax` architecture (`insp/tmux-floax`), provides an interactive Apps Panel (TUI picker) for managing applications, and introduces a flexible window tracking and toggling mechanism for arbitrary tmux windows (e.g. running AI agents/codex in long-running sessions).

### Key Requirements
1. **Extract & Generalize Apps (`src/apps/`)**:
   - Move `src/lazygit.rs` into `src/apps/`.
   - Generalize the execution logic: checking binary availability, detecting and focusing already running instances across tmux panes, launching in popup or window mode.
   - Built-in support for `lazygit` and `btop`, extensible via `mox.toml` configuration (`[[apps]]`).
   - Preserve 100% backward compatibility with `mox lazygit` and existing tests.
2. **Floax Scratchpad Integration**:
   - Native Rust implementation of the Floax workflow from `insp/tmux-floax`.
   - Dedicated persistent background session (`mox-scratch` or configurable) with `detach-on-destroy on` and `status off`.
   - Displayed via `tmux display-popup -E "tmux attach-session -t mox-scratch"`.
   - Allows instant hiding/toggling without terminating long-running processes (CLI agents, codex, long compilations, interactive shells).
3. **Tmux Window Tracking & Quick-Toggling**:
   - Track arbitrary tmux windows using window options (`@mox_tracked 1`, `@mox_prev_window`).
   - Quick toggle to jump between current work window and tracked windows and back.
   - Any window can be added/removed from tracking interactively via the Apps Panel or CLI (`mox track add|remove|toggle`).
4. **Apps Panel UI**:
   - Interactive TUI picker (using Mox's native `Picker` / `ListRow` / `PreviewPanel`).
   - Shows registered applications (`lazygit`, `btop`, user apps), Floax scratchpad, and tracked windows with live status.
   - Quick action to track/untrack the current window.
   - Hotkeys both inside the panel and globally in tmux keymaps.

---

## 2. Architecture & Module Structure

```
src/
├── apps/
│   ├── mod.rs          // Public facade, re-exports, open_lazygit backward compatibility
│   ├── model.rs        // AppConfig, AppLaunchMode, AppStatus, TrackedWindow models
│   ├── runner.rs       // Generalized process/tmux launcher, existing pane finder & focuser
│   ├── floax.rs        // Native Floax scratchpad session management & popup toggle
│   ├── tracking.rs     // Window tracking (@mox_tracked), toggle back-and-forth
│   └── ui.rs           // Interactive Apps Panel built on crate::ui::picker
├── config/
│   └── model.rs        // Updated Config struct with `apps: Vec<AppConfig>`
├── actions.rs          // FeatureAction::Apps, FeatureAction::Floax, FeatureAction::TrackToggle
├── keymap/
│   ├── types.rs        // KeyAction::Apps, KeyAction::Floax, KeyAction::TrackToggle
│   └── defaults.rs     // Bindings in default keymap
├── cli.rs              // CLI commands: mox apps, mox floax, mox track, mox lazygit (compat)
└── lib.rs              // pub mod apps; pub mod lazygit (facade pointing to apps)
```

---

## 3. Component Details

### 3.1 `src/apps/model.rs`
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub title: String,
    pub command: String,
    pub key: Option<String>,
    pub mode: AppLaunchMode,
    #[serde(default = "default_true")]
    pub focus_existing: bool,
    #[serde(default = "default_true")]
    pub check_binary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppLaunchMode {
    Popup {
        #[serde(default = "default_width")]
        width: String,
        #[serde(default = "default_height")]
        height: String,
    },
    Window,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedWindow {
    pub id: String,
    pub name: String,
    pub session_id: String,
    pub active: bool,
}
```

### 3.2 `src/apps/runner.rs`
1. Resolve target working directory (`cwd` parameter or `#{pane_current_path}`).
2. If `focus_existing` is true:
   - Query `tmux list-panes -a -F "#{session_id}\t#{pane_id}\t#{pane_current_command}\t#{pane_current_path}"`.
   - If an existing pane matches `session_id`, `command` name, and `cwd`, run `select-window -t pane` and `select-pane -t pane`, returning early with `Ok(())`.
3. If `check_binary` is true:
   - Check executable availability using `Command::new(binary).arg("--version")` with a bounded timeout.
4. Launching:
   - For `AppLaunchMode::Window`: run `tmux new-window -c <cwd> <command>`.
   - For `AppLaunchMode::Popup`: if `std::io::stdout().is_terminal()`, execute directly or via `tmux popup -w <w> -h <h> -b rounded -T <title> -d <cwd> <command>`.

### 3.3 `src/apps/floax.rs`
Native Floax persistent scratchpad:
1. `FLOAX_SESSION_NAME`: default `"mox-scratch"`.
2. Check if the session exists via `tmux has-session -t mox-scratch`.
3. If not, create it in the background:
   - `tmux new-session -d -s mox-scratch -c <cwd>`
   - `tmux set-option -t mox-scratch status off`
   - `tmux set-option -t mox-scratch detach-on-destroy on`
4. Toggle display:
   - Run `tmux display-popup -E -w 85% -h 85% -b rounded -T " FloaX Scratchpad " "tmux attach-session -t mox-scratch"`.
   - Closing the popup detaches the client; long-running processes (e.g. codex, agent, cargo) remain alive.
   - Subsequent calls re-attach to the same session with all state preserved.

### 3.4 `src/apps/tracking.rs`
Window tracking & toggle:
- `track_current_window(socket)`: sets `@mox_tracked 1` on current window.
- `untrack_current_window(socket)`: unsets `@mox_tracked` on current window.
- `list_tracked_windows(socket)`: queries `list-windows` filtering for `@mox_tracked == 1`.
- `toggle_tracked_window(socket, target_window_id)`:
  - If currently in the tracked window: inspects `@mox_prev_window` and selects it (fallback to `last-window`).
  - If in another window: saves current window id to `@mox_prev_window` and selects the tracked window.

### 3.5 `src/apps/ui.rs` (The Apps Panel)
Built on `crate::ui::picker`:
- Lists sections:
  1. **Built-in / Configured Apps**: `LazyGit`, `btop`, any extra configured apps from `mox.toml`.
  2. **Floax Scratchpad**: Persistent floating terminal (`[Running]` / `[Idle]`).
  3. **Tracked Windows**: Active tracked tmux windows.
  4. **Actions**: `[+] Track Current Window`, `[-] Untrack Current Window`.
- Allows instant fuzzy search, selection, and preview showing status and command details.

---

## 4. Keymap & CLI Integration

- **CLI**:
  - `mox apps [--popup]`
  - `mox app run <name>`
  - `mox floax`
  - `mox track add|remove|toggle`
  - `mox lazygit` (alias/compat)
- **Keymaps**:
  - `build_default_keymap`:
    - `a` / `apps`: opens Apps Panel.
    - `p` / `floax`: toggles Floax scratchpad.
    - `G` / `N`: lazygit.
    - `B`: btop.

---

## 5. Testing & Verification

1. **Unit Tests**:
   - `test_app_config_serde`: serialization/deserialization of `AppConfig` and defaults.
   - `test_floax_session_creation_args`: verifies tmux command construction for session creation and popup attachment.
   - `test_tracked_window_parsing`: verifies parsing of `list-windows` output for `@mox_tracked` attributes.
2. **Integration Tests**:
   - In `tests/logging_lazygit_tests.rs`: verify `mox::lazygit::open` continues to work seamlessly via the new `src/apps/` backend.
   - Isolated tmux server tests in `tests/apps_integration_tests.rs`:
     - Test floax session creation and idempotent toggle.
     - Test window tracking and toggle roundtrip.
3. **Verification Commands**:
   - `cargo check --workspace --all-targets`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
