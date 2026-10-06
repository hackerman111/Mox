# Design Document: Tree UI Enhancements, Navi-Style Snippets Panel, and Favorite Projects

**Date:** 2026-10-06  
**Status:** Approved  
**Author:** Pair Programming Session  

---

## 1. Overview

This design enhances the visual and operational UX of Mox in three cohesive areas:
1. **Tree UI Enhancements:** Unified visual styling across all hierarchical representations in Mox (the tmux session/window/pane navigator, directory previews, and menus) using modern branch glyphs, semantic Nerd Font/Unicode icons, distinct depth color hierarchy, and clean active/fold indicators.
2. **Favorite Commands / Snippets Panel (Navi-style):** A dedicated, interactive TUI panel for managing and executing favorite terminal commands with parameterized templates (`<param>` or `<param:default>`), descriptions, tags, fuzzy search, inline parameter filling, interactive snippet creation/deletion (`a` / `d`), and direct execution into active tmux panes or copying to clipboard (`y`).
3. **Favorite Projects:** Persistent project favorites in `mox projects` toggled on-the-fly via key `f`, pinned to the top of the picker list with a gold `★` star indicator, and optional static configuration in `config.toml`.

---

## 2. Tree UI Improvements

### 2.1 Extended `TreeRow` Model (`src/ui/render.rs`)
The existing `TreeRow` struct will be extended to support node-specific icons and colors:
```rust
pub struct TreeRow<'a> {
    pub depth: usize,
    pub is_collapsed: Option<bool>, // None = leaf, Some(true) = ▶, Some(false) = ▼
    pub icon: Option<(&'a str, Color)>,
    pub label: &'a str,
    pub badge: Option<(&'a str, Color)>,
    pub is_active: bool,
    pub is_selected: bool,
}
```

### 2.2 Visual Glyphs & Styling
- **Branch Connectors:** Clean box-drawing glyphs `├─ `, `└─ `, `│  ` with `theme.border` color.
- **Node Icons:**
  - Tmux Session: `󰍹 ` (`theme.accent`)
  - Tmux Window: `󰖲 ` (`theme.fg` or `theme.warning`)
  - AI Agent Window: `󰚩 ` (`theme.mode_agent`)
  - Tmux Pane: ` ` (`theme.fg`)
  - Directory: ` ` (`theme.accent`)
  - File: ` ` (`theme.fg`)
  - Favorite Project: `★ ` (`theme.warning`)
- **Fold Glyphs:** `▶ ` (collapsed) and `▼ ` (expanded) styled in `theme.accent`.
- **Activity Marker:** Replaced raw `*` with a round bullet `●` in active theme color (`theme.accent` / `theme.mode_insert`).
- **Depth Color Gradient:** Top-level nodes (depth 0) render bold/accented, depth 1 uses primary foreground, and depth 2+ uses dimmed secondary text.

### 2.3 Recursive Directory Tree Preview (`src/ui/preview.rs`)
Replace the flat line listing in `local_directory` with `local_directory_tree`, rendering directories first with ` ` and files with ` `, with hierarchical branch indentation up to 3 levels deep.

---

## 3. Navi-Style Snippets Panel

### 3.1 Data Model (`src/snippets/model.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub command: String, // e.g. "docker exec -it <container> <shell:sh>"
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}
```

### 3.2 Template Engine (`src/snippets/template.rs`)
- **Syntax:** `<name>` or `<name:default_value>`.
- **Parsing:** Extract template variables in order of appearance.
- **Substitution:** Prompt user for missing variables; fallback to defaults if input is empty.

### 3.3 Storage (`src/snippets/storage.rs`)
- Persistent file: `~/.config/mox/snippets.toml`.
- Populated with built-in default snippets (Docker, Git, Tmux, Curl) on first launch.
- Atomic file writes using `crate::state::atomic_write`.

### 3.4 Interactive TUI Runner (`src/snippets/runner.rs`, `src/snippets/ui.rs`)
- **Search Header:** `[SEARCH]` badge with instant fuzzy filtering over title, tags, and description.
- **Left Panel:** List of snippets displaying title, tag pills (`#tag`), and concise description.
- **Right Panel:** Live preview displaying full metadata, highlighted command, and parameters.
- **Key Bindings:**
  - `Enter`: Select snippet; sequentially prompt for parameters `<param>` using `render_prompt`, then send the interpolated command to the target tmux pane via `tmux send-keys`.
  - `y`: Copy interpolated command to clipboard instead of running.
  - `a`: Interactively add a new snippet via prompt prompts (`title`, `command`, `description`, `tags`).
  - `d`: Delete selected snippet with `[CONFIRM]` prompt.
  - `/`: Focus search filter.
  - `Esc` / `q`: Close panel.

---

## 4. Favorite Projects in `mox projects`

### 4.1 Persistence & Data Flow (`src/projects.rs`)
- Storage file: `~/.local/share/mox/projects-favorites`.
- Config integration: `favorite_projects: Vec<PathBuf>` in `Config` (optional static overrides).
- Internal representation:
  ```rust
  pub(crate) struct Project {
      path: PathBuf,
      session: Option<ExistingSession>,
      is_favorite: bool,
  }
  ```

### 4.2 Ordering & Visuals
- Favorite projects are pinned to the top of the list above existing sessions and recent paths.
- Displayed with `★ ` prefix in `theme.warning` gold color.
- Pressing `f` in the projects picker toggles favorite status immediately, updates the state file, and refreshes the ordered list.

---

## 5. CLI & Keymap Integration

### 5.1 CLI Commands (`src/cli.rs`)
```rust
Commands::Snippets {
    #[arg(long)]
    socket: Option<String>,
    #[arg(long)]
    popup: bool,
}
```

### 5.2 Actions & Palette (`src/actions.rs`)
- Add `FeatureAction::Snippets` ("favorite commands & snippets").
- Displayed in `mox palette` (`Space`).

### 5.3 Keymap (`src/keymap/defaults.rs`)
- `C` (Shift+c) in modal root table: Open snippets panel in popup.
- `g c` in `go to` submenu: Navigate to snippets panel.

---

## 6. Verification Plan

1. **Unit Tests:**
   - Template parameter extraction and substitution unit tests in `src/snippets/template.rs`.
   - Snippet serialization and deserialization in `src/snippets/storage.rs`.
   - Favorite projects ordering and toggling in `src/projects.rs`.
   - Tree formatting and row rendering tests in `src/ui/render.rs` and `src/ui/preview.rs`.
2. **Integration Verification:**
   - `cargo check --workspace --all-targets`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - Verify popup execution via `python3 tests/tmux_popup_smoke.py` if applicable.
