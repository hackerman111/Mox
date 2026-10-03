# Design Document: AI Agents Support and Unified Tree Navigator for Mox

Date: 2026-10-04
Status: Approved

## 1. Overview

This design addresses two key enhancements in `Mox`:
1. **AI Agent Window Management**:
   - Create AI agent windows via `A` (in modal mode).
   - Fast toggle between active work window and AI agent window via `a`.
   - Automatic return from AI agent window back to the previous work window upon `Alt-m` -> `a`.
   - Windows remember their AI agent status and transition automatically between modal mode and insert mode (`root`).
2. **Unified Vim-like Fuzzy Tree Navigator**:
   - Replaces `choose-tree` across all Mox menus and bindings (`s`, `t`, `w s`, `S s`, `g s`).
   - Collects tmux hierarchy (`Session -> Window -> Pane`) in a single snapshot call: `tmux list-panes -a -F '...'`.
   - Native Rust TUI using `crossterm` in a popup:
     - Left pane: Hierarchical tree with fold/unfold status (`Tab`), search bar (`/`), visual indicators.
     - Right pane: Live preview of pane content (`tmux capture-pane -ep -t <id>`), toggleable via `?`.
   - Vim navigation: `j/k`, `gg/G`, `/` (fuzzy search), `Tab` (fold/unfold), `?` (preview toggle), `Esc` (exit).
   - Node actions: `Enter` (focus/switch), `dd` (kill node with confirmation), `r` (rename session/window).

---

## 2. AI Agent Windows Workflow

### 2.1 State & Identification
- AI agent windows are tracked by setting the tmux window option `@mox_is_agent 1`.
- When switching away from a normal window to an agent window, the previous window ID is stored in `@mox_prev_window` (tmux session or global option).
- Default agent window naming: `agent`, `agent-2`, `agent-3`, etc.
- Default command: configurable via `@mox_agent_cmd`, falling back to `$SHELL`.

### 2.2 Key Actions & Subcommands
- **`A` (`mox agent-create`)**:
  - Save current window ID into `@mox_prev_window`.
  - Check existing agent windows to calculate the next window name (`agent`, `agent-2`, etc.).
  - Run `tmux new-window -n <name> <cmd>`.
  - Set `@mox_is_agent 1` on the new window.
  - Switch tmux key-table to `root` (insert mode) so the user can immediately talk to the agent.
- **`a` (`mox agent-toggle`)**:
  - Check whether the current active window has `@mox_is_agent == 1`:
    - **If currently in an agent window**:
      - Read `@mox_prev_window`. If valid, switch to it (`select-window -t <id>`); otherwise call `last-window`.
      - Switch key-table to `root` (insert mode).
    - **If in a normal work window**:
      - Record current window ID in `@mox_prev_window`.
      - Find the latest active agent window (or first window having `@mox_is_agent == 1`).
      - If found, switch to it (`select-window -t <agent_win_id>`).
      - If no agent window exists yet, create one (same as `agent-create`).
      - Switch key-table to `root` (insert mode).

---

## 3. Unified Fuzzy Tree Navigator Architecture

### 3.1 Data Model
Module: `src/nav/model.rs` (or `src/nav.rs`)

```rust
pub struct PaneInfo {
    pub id: String,          // e.g. "%1"
    pub index: u32,
    pub active: bool,
    pub command: String,     // e.g. "nvim", "zsh"
    pub cwd: String,         // e.g. "~/code/mox"
    pub title: String,
}

pub struct WindowInfo {
    pub id: String,          // e.g. "@1"
    pub index: u32,
    pub name: String,        // e.g. "editor", "agents"
    pub active: bool,
    pub is_agent: bool,
    pub panes: Vec<PaneInfo>,
    pub collapsed: bool,
}

pub struct SessionInfo {
    pub id: String,          // e.g. "$1"
    pub name: String,        // e.g. "project"
    pub active: bool,
    pub windows: Vec<WindowInfo>,
    pub collapsed: bool,
}
```

### 3.2 Tmux Snapshot
Obtained with a single command:
```bash
tmux list-panes -a -F '#{session_name}	#{session_id}	#{session_attached}	#{window_index}	#{window_id}	#{window_name}	#{window_active}	#{pane_id}	#{pane_index}	#{pane_active}	#{pane_current_command}	#{pane_current_path}	#{pane_title}	#{@mox_is_agent}'
```
Grouped into `Vec<SessionInfo>` preserving natural order.

### 3.3 Flattened Visible Tree Item
```rust
pub enum TreeNode<'a> {
    Session(&'a SessionInfo),
    Window(&'a SessionInfo, &'a WindowInfo),
    Pane(&'a SessionInfo, &'a WindowInfo, &'a PaneInfo),
}
```
Only expanded nodes produce items in the flat list for navigation.

### 3.4 Fuzzy Search
- Query matches against:
  - Session name
  - Window name
  - Pane command, cwd, title
- Subsequence matching with scoring (higher score for prefix, word boundary, command match).
- Hierarchy preservation: if a Pane matches, its parent Window and Session are shown and expanded so context is not lost.

### 3.5 TUI Layout & Navigation
Popup command:
`tmux display-popup -w 90% -h 85% -b rounded -T " Mox Navigator " -E "mox nav"`

- Split view:
  - Left column (e.g. 50-60% width): Tree list with search prompt at top.
  - Right column (remainder): Live preview with border and title.
- Vim navigation keys:
  - `j` / Down: Next visible node
  - `k` / Up: Previous visible node
  - `gg`: Top of tree
  - `G`: Bottom of tree
  - `Tab`: Expand / collapse node (`Session` or `Window`)
  - `/`: Enter search mode
  - `?`: Toggle preview panel visibility
  - `Esc`: Clear search if in search mode; exit navigator if search is empty
  - `Enter`: Focus / switch to selected node (Session -> `switch-client`, Window -> `select-window`, Pane -> `select-pane ; select-window`)
  - `dd`: Kill node with confirmation (`kill-session`, `kill-window`, `kill-pane`). Refreshes tree on success!
  - `r`: Prompt inline to rename session or window.

### 3.6 Preview
- Pane: `tmux capture-pane -ep -t <pane_id>`
- Window: captured output of its active pane
- Session: captured output of its active window's active pane
- ANSI colors preserved via crossterm formatting or direct stdout passthrough.

---

## 4. Keymap Integration

In `src/keymap.rs`:
- Root NORMAL mode:
  - `a`: `KeyAction::AgentToggle` (calls `mox agent-toggle`)
  - `A`: `KeyAction::AgentCreate` (calls `mox agent-create`)
  - `s`: `KeyAction::Navigator` (opens `mox nav` popup)
  - `t`: `KeyAction::Navigator` (opens `mox nav` popup)
- Window submenu (`w`):
  - `s`, `t`: `KeyAction::Navigator`
- Session submenu (`S`):
  - `s`, `t`: `KeyAction::Navigator`
- Go-to submenu (`g`):
  - `s`, `w`: `KeyAction::Navigator`
