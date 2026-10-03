# AI Agents Support and Unified Tree Navigator Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Implement AI agent window tracking & fast-toggling (`A` create, `a` toggle) and a unified native Vim-like fuzzy tree navigator (`Session -> Window -> Pane`) with live preview and actions in Mox.

**Architecture:**
- AI agent module in Rust managing tmux window options (`@mox_is_agent`) and switching logic, switching key-table to `root` (insert mode) on arrival.
- Snapshot tmux hierarchy in a single `tmux list-panes -a -F '...'` command, parsing into a type-safe tree.
- Native Rust fuzzy search engine matching across session name, window name, pane command, cwd, and title, preserving tree hierarchy.
- Crossterm-based dual-panel TUI popup for tree navigation (`j/k`, `gg/G`, `Tab`, `?`, `/`, `Enter`, `dd`, `r`) with live pane preview (`tmux capture-pane`).
- Keymap integration replacing legacy `choose-tree` with `mox nav` and binding `a`/`A`.

**Tech Stack:** Rust 2024, `crossterm` 0.29, `clap` 4.5, tmux CLI.

---

### Task 1: AI Agent Window Management Module

**Files:**
- Create: `src/agent.rs`
- Modify: `src/lib.rs`
- Test: `tests/agent_tests.rs`

**Step 1: Write the failing tests**
Create `tests/agent_tests.rs` testing window name generation (`agent`, `agent-2`), agent detection, and toggle target selection.

```rust
use mox::agent::{calculate_next_agent_name, parse_window_list_for_agents, WindowSummary};

#[test]
fn test_calculate_next_agent_name() {
    let existing = vec!["editor".to_string(), "bash".to_string()];
    assert_eq!(calculate_next_agent_name(&existing), "agent");

    let with_agent = vec!["editor".to_string(), "agent".to_string()];
    assert_eq!(calculate_next_agent_name(&with_agent), "agent-2");

    let with_agent_2 = vec!["editor".to_string(), "agent".to_string(), "agent-2".to_string()];
    assert_eq!(calculate_next_agent_name(&with_agent_2), "agent-3");
}

#[test]
fn test_parse_window_list_for_agents() {
    let raw = "@1\teditor\t0\n@2\tagent\t1\n";
    let windows = parse_window_list_for_agents(raw);
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].id, "@1");
    assert!(!windows[0].is_agent);
    assert_eq!(windows[1].id, "@2");
    assert!(windows[1].is_agent);
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test agent_tests`
Expected: FAIL (module `agent` not found).

**Step 3: Implement minimal code**
Create `src/agent.rs` implementing `calculate_next_agent_name`, `parse_window_list_for_agents`, `create_agent_window(socket)`, and `toggle_agent_window(socket)`. Export in `src/lib.rs`.

**Step 4: Run test to verify it passes**
Run: `cargo test --test agent_tests`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/agent.rs src/lib.rs tests/agent_tests.rs
git commit -m "feat(agent): add AI agent window tracking and toggle logic"
```

---

### Task 2: Tmux Snapshot & Hierarchy Data Model

**Files:**
- Create: `src/nav/mod.rs`
- Create: `src/nav/model.rs`
- Modify: `src/lib.rs`
- Test: `tests/nav_model_tests.rs`

**Step 1: Write the failing tests**
Create `tests/nav_model_tests.rs` verifying TSV snapshot parsing into `SessionInfo`, `WindowInfo`, and `PaneInfo`.

```rust
use mox::nav::model::parse_tmux_snapshot;

#[test]
fn test_parse_tmux_snapshot() {
    let raw = "project\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/home/user/code\tmain.rs\t0\n\
               project\t$1\t1\t1\t@1\teditor\t1\t%2\t1\t0\tzsh\t/home/user/code\tzsh\t0\n\
               project\t$1\t1\t2\t@2\tagents\t0\t%3\t0\t1\tclaude\t/home/user/code\tclaude\t1\n";
    let sessions = parse_tmux_snapshot(raw);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].name, "project");
    assert_eq!(sessions[0].windows.len(), 2);
    assert_eq!(sessions[0].windows[0].panes.len(), 2);
    assert_eq!(sessions[0].windows[1].name, "agents");
    assert!(sessions[0].windows[1].is_agent);
    assert_eq!(sessions[0].windows[1].panes[0].command, "claude");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test nav_model_tests`
Expected: FAIL (`mox::nav` not found).

**Step 3: Implement data structures and parser**
Implement `SessionInfo`, `WindowInfo`, `PaneInfo`, `parse_tmux_snapshot`, and snapshot collection in `src/nav/model.rs`.

**Step 4: Run test to verify it passes**
Run: `cargo test --test nav_model_tests`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/nav/ tests/nav_model_tests.rs src/lib.rs
git commit -m "feat(nav): add tmux hierarchy snapshot parser and data model"
```

---

### Task 3: Fuzzy Search Engine with Tree Preservation

**Files:**
- Create: `src/nav/fuzzy.rs`
- Modify: `src/nav/mod.rs`
- Test: `tests/nav_fuzzy_tests.rs`

**Step 1: Write the failing tests**
Create `tests/nav_fuzzy_tests.rs` testing query matching and hierarchy expansion.

```rust
use mox::nav::fuzzy::filter_sessions;
use mox::nav::model::parse_tmux_snapshot;

#[test]
fn test_fuzzy_filtering_preserves_hierarchy() {
    let raw = "work\t$1\t1\t1\t@1\teditor\t1\t%1\t0\t1\tnvim\t/code/mox\tedit\t0\n\
               work\t$1\t1\t2\t@2\tshell\t0\t%2\t0\t1\tzsh\t/home\tzsh\t0\n";
    let sessions = parse_tmux_snapshot(raw);
    let filtered = filter_sessions(&sessions, "nvim");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].windows.len(), 1);
    assert_eq!(filtered[0].windows[0].panes.len(), 1);
    assert_eq!(filtered[0].windows[0].panes[0].command, "nvim");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test nav_fuzzy_tests`
Expected: FAIL (`fuzzy` module not found).

**Step 3: Implement fuzzy search logic**
Implement subsequence scoring and tree filtering in `src/nav/fuzzy.rs`.

**Step 4: Run test to verify it passes**
Run: `cargo test --test nav_fuzzy_tests`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/nav/fuzzy.rs tests/nav_fuzzy_tests.rs src/nav/mod.rs
git commit -m "feat(nav): add fuzzy search engine with tree hierarchy retention"
```

---

### Task 4: Interactive TUI Tree Navigator & Live Preview

**Files:**
- Create: `src/nav/ui.rs`
- Modify: `src/nav/mod.rs`
- Test: `tests/nav_action_tests.rs`

**Step 1: Write the failing tests**
Test node action command generation for switch, kill, and rename.

```rust
use mox::nav::ui::{format_kill_command, format_switch_command};
use mox::nav::model::TreeItem;

#[test]
fn test_switch_command_generation() {
    assert_eq!(format_switch_command(&TreeItem::Pane { pane_id: "%1".into(), window_id: "@1".into() }), "select-pane -t %1 ; select-window -t @1");
    assert_eq!(format_switch_command(&TreeItem::Window { window_id: "@2".into() }), "select-window -t @2");
    assert_eq!(format_switch_command(&TreeItem::Session { session_id: "$1".into() }), "switch-client -t $1");
}
```

**Step 2: Run test to verify it fails**
Run: `cargo test --test nav_action_tests`
Expected: FAIL (`TreeItem` / functions not found).

**Step 3: Implement TUI Navigator**
- Left tree column: box drawing (`├─`, `└─`, `▼`, `▶`), badges (`[AI]`), cursor highlight.
- Right preview column: `capture-pane -ep -t <id>`, toggleable with `?`.
- Vim controls: `j/k`, `gg/G`, `Tab` (fold/unfold), `?`, `/`, `Esc`, `Enter` (switch), `dd` (kill confirm), `r` (rename inline).

**Step 4: Run test to verify it passes**
Run: `cargo test --test nav_action_tests`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/nav/ui.rs src/nav/mod.rs tests/nav_action_tests.rs
git commit -m "feat(nav): implement dual-pane TUI tree navigator with live preview"
```

---

### Task 5: Keymap & CLI Integration

**Files:**
- Modify: `src/keymap.rs`
- Modify: `src/tmux.rs`
- Modify: `src/main.rs`
- Test: `tests/keymap_tests.rs`

**Step 1: Write test for new bindings**
Update `tests/keymap_tests.rs` to verify `a`, `A`, and navigator bindings (`s`, `t`, `w s`, `S s`, `g s`).

**Step 2: Run test to verify it fails**
Run: `cargo test --test keymap_tests`
Expected: FAIL.

**Step 3: Update Keymap, Tmux Config Generation, and CLI Commands**
- In `src/keymap.rs`:
  - `a`: `KeyAction::AgentToggle`
  - `A`: `KeyAction::AgentCreate`
  - `s`, `t`: `KeyAction::Navigator`
  - Update `w`, `S`, `g` submenus to use `KeyAction::Navigator`.
- In `src/tmux.rs`:
  - Handle `KeyAction::Navigator`, `KeyAction::AgentToggle`, `KeyAction::AgentCreate`.
- In `src/main.rs`:
  - Add subcommands `Nav`, `AgentCreate`, `AgentToggle`.

**Step 4: Run test to verify it passes**
Run: `cargo test --test keymap_tests`
Expected: PASS.

**Step 5: Commit**
```bash
git add src/keymap.rs src/tmux.rs src/main.rs tests/keymap_tests.rs
git commit -m "feat: integrate agent toggle and tree navigator into mox keymap and CLI"
```

---

### Task 6: Full Workspace Verification & Parity Check

**Files:**
- Modify: `docs/tmux-modal-parity.md`
- Run:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`

**Step 1: Update parity documentation**
Document `a`, `A`, and navigator commands in `docs/tmux-modal-parity.md`.

**Step 2: Run full verification suite**
Run:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
Expected: All clean and passing with zero warnings.

**Step 3: Commit**
```bash
git add docs/tmux-modal-parity.md
git commit -m "docs: update modal parity doc with agent and tree navigator features"
```
