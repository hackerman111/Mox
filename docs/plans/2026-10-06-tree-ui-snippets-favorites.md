# Tree UI Enhancements, Navi-Style Snippets Panel, and Favorite Projects Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Modernize all tree interfaces across Mox with clean branch connectors and semantic icons, build an interactive navi-style terminal command snippets panel with parameter interpolation and fuzzy search, and integrate favorite projects pinned to the top of `mox projects`.

**Architecture:** Extend `TreeRow` in `src/ui/render.rs` with icon and depth color hierarchy; implement directory tree preview in `src/ui/preview.rs`; integrate icons in `src/nav/view.rs`; create a modular `src/snippets/` engine with template parsing, persistent TOML storage, and interactive TUI runner; enhance `src/projects.rs` with persistent project favorites toggled via `f`; wire up CLI commands and keymaps.

**Tech Stack:** Rust (2024 edition), crossterm 0.29, serde / toml, unicode-width.

---

### Task 1: Extended `TreeRow` Model and Rendering in `src/ui/render.rs`

**Files:**
- Modify: `src/ui/render.rs:141-205`
- Test: `tests/tree_render_test.rs`

**Step 1: Write the failing test**

Create `tests/tree_render_test.rs`:
```rust
use crossterm::style::Color;
use mox::ui::render::{TreeRow, render_tree_row};
use mox::ui::theme::Theme;

#[test]
fn test_tree_row_renders_icon_and_clean_branch() {
    let theme = Theme::load(None);
    let mut out = Vec::new();
    let row = TreeRow {
        depth: 1,
        is_collapsed: Some(false),
        icon: Some(("󰖲 ", Color::Yellow)),
        label: "window 1: editor",
        badge: Some(("[AI]", Color::Cyan)),
        is_active: true,
        is_selected: false,
    };

    render_tree_row(&mut out, &row, 50, &theme).expect("render succeeds");
    let output = String::from_utf8_lossy(&out);
    assert!(output.contains("├─"));
    assert!(output.contains("󰖲"));
    assert!(output.contains("window 1: editor"));
    assert!(output.contains("●"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test tree_render_test`  
Expected: FAIL (fields `icon` missing on `TreeRow`).

**Step 3: Write minimal implementation**

In `src/ui/render.rs`:
1. Add `pub icon: Option<(&'a str, Color)>` to `TreeRow<'a>`.
2. Update `render_tree_row` to render:
   - Branch indentation (`├─ `, `└─ `, `│  `) in `theme.border`.
   - Fold chevron `▶ ` / `▼ ` in `theme.accent`.
   - Node icon with its designated `Color`.
   - Active bullet `● ` in `theme.accent` (replacing `*`).
   - Depth-dependent text styling.

**Step 4: Run test to verify it passes**

Run: `cargo test --test tree_render_test`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui/render.rs tests/tree_render_test.rs
git commit -m "feat(ui): extend TreeRow with icons, clean branches, and active bullets"
```

---

### Task 2: Recursive Directory Tree Preview in `src/ui/preview.rs`

**Files:**
- Modify: `src/ui/preview.rs:55-79`
- Modify: `tests/tree_render_test.rs`

**Step 1: Write the failing test**

Add to `tests/tree_render_test.rs`:
```rust
#[test]
fn test_local_directory_tree_formats_hierarchy() {
    let temp_dir = std::env::temp_dir().join("mox_tree_test_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("sub")).unwrap();
    std::fs::write(temp_dir.join("file1.txt"), "hello").unwrap();
    std::fs::write(temp_dir.join("sub").join("file2.rs"), "fn main() {}").unwrap();

    let lines = mox::ui::preview::local_directory(&temp_dir).expect("loads directory tree");
    assert!(!lines.is_empty());
    assert!(lines.iter().any(|l| l.contains("sub") && l.contains("")));
    assert!(lines.iter().any(|l| l.contains("file1.txt") && l.contains("")));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test tree_render_test -- test_local_directory_tree_formats_hierarchy`  
Expected: FAIL (icons `` or `` not in output).

**Step 3: Write minimal implementation**

In `src/ui/preview.rs`:
Implement recursive helper `collect_tree_lines(path: &Path, prefix: &str, depth: usize, max_depth: usize, out: &mut Vec<String>)`:
- Sort entries directories first, then files alphabetically.
- Format with `├──  dir` and `└──  file.txt`.
- Limit entries to 200 lines and max depth to 3.
- Update `local_directory` to call this tree builder.

**Step 4: Run test to verify it passes**

Run: `cargo test --test tree_render_test -- test_local_directory_tree_formats_hierarchy`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui/preview.rs tests/tree_render_test.rs
git commit -m "feat(preview): render directory previews as formatted hierarchical trees"
```

---

### Task 3: Modernized Tree UI in Tmux Navigator (`src/nav/view.rs`)

**Files:**
- Modify: `src/nav/view.rs:60-170`
- Test: `tests/nav_test.rs` or `cargo test --lib nav`

**Step 1: Write the failing test**

Add unit test in `src/nav/view.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav::model::{PaneInfo, SessionInfo, TreeItem, WindowInfo};

    #[test]
    fn test_tree_row_parts_populates_icons() {
        let session = SessionInfo {
            id: "$0".into(),
            name: "main".into(),
            active: true,
            windows: vec![],
            collapsed: false,
        };
        let item = TreeItem::Session(&session);
        let row = tree_item_to_row(&item, 0, false, true);
        assert_eq!(row.icon.unwrap().0, "󰍹 ");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib nav::view::tests`  
Expected: FAIL (function `tree_item_to_row` not found).

**Step 3: Write minimal implementation**

In `src/nav/view.rs`:
- Implement `tree_item_to_row<'a>(item: &'a TreeItem<'a>, item_idx: usize, is_selected: bool, preserve_collapse: bool, theme: &Theme) -> TreeRow<'a>`.
- Assign icons:
  - Session: `󰍹 ` (`theme.accent`)
  - Window: `󰖲 ` (`theme.warning`) or `󰚩 ` (`theme.mode_agent` if `window.is_agent`)
  - Pane: ` ` (`theme.fg`)
- Use `render_tree_row` in `render_navigator_with_theme`.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib nav::view::tests`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/nav/view.rs
git commit -m "feat(nav): integrate modern icons and tree row rendering into tmux navigator"
```

---

### Task 4: Snippets Model & Template Engine in `src/snippets/`

**Files:**
- Create: `src/snippets/mod.rs`
- Create: `src/snippets/model.rs`
- Create: `src/snippets/template.rs`
- Modify: `src/lib.rs`

**Step 1: Write the failing test**

In `src/snippets/template.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_and_substitute_parameters() {
        let cmd = "docker exec -it <container_id> <shell:bash>";
        let params = extract_parameters(cmd);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "container_id");
        assert_eq!(params[0].default, None);
        assert_eq!(params[1].name, "shell");
        assert_eq!(params[1].default, Some("bash".into()));

        let values = [
            ("container_id".into(), "my-app".into()),
            ("shell".into(), "sh".into()),
        ];
        let result = substitute_parameters(cmd, &values);
        assert_eq!(result, "docker exec -it my-app sh");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib snippets::template::tests`  
Expected: FAIL (module `snippets` not found).

**Step 3: Write minimal implementation**

Create `src/snippets/model.rs` with `Snippet` struct.  
Create `src/snippets/template.rs` with:
- `TemplateParam { name: String, default: Option<String> }`
- `extract_parameters(&str) -> Vec<TemplateParam>`
- `substitute_parameters(&str, &[(String, String)]) -> String`  
Register `pub mod snippets;` in `src/lib.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib snippets::template::tests`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/lib.rs src/snippets/mod.rs src/snippets/model.rs src/snippets/template.rs
git commit -m "feat(snippets): add snippet models and template interpolation engine"
```

---

### Task 5: Snippets Persistent TOML Storage in `src/snippets/storage.rs`

**Files:**
- Create: `src/snippets/storage.rs`
- Modify: `src/snippets/mod.rs`

**Step 1: Write the failing test**

In `src/snippets/storage.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_snippets_and_serialization() {
        let defaults = default_snippets();
        assert!(!defaults.is_empty());
        let toml_str = serialize_snippets(&defaults).expect("serializes to toml");
        let parsed = deserialize_snippets(&toml_str).expect("deserializes from toml");
        assert_eq!(parsed.len(), defaults.len());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib snippets::storage::tests`  
Expected: FAIL (functions not defined).

**Step 3: Write minimal implementation**

In `src/snippets/storage.rs`:
- `default_snippets() -> Vec<Snippet>` with starter cheatsheets (Git, Docker, Tmux, Curl).
- `default_snippets_path() -> PathBuf` pointing to `~/.config/mox/snippets.toml`.
- `load_snippets(path: Option<&Path>) -> Result<Vec<Snippet>, String>`.
- `save_snippets(path: Option<&Path>, snippets: &[Snippet]) -> Result<(), String>` using `crate::state::atomic_write`.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib snippets::storage::tests`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/snippets/storage.rs src/snippets/mod.rs
git commit -m "feat(snippets): add persistent TOML storage and default cheatsheets"
```

---

### Task 6: Interactive Snippets TUI Panel (`src/snippets/ui.rs`, `runner.rs`)

**Files:**
- Create: `src/snippets/ui.rs`
- Create: `src/snippets/runner.rs`
- Modify: `src/snippets/mod.rs`

**Step 1: Write the failing test**

In `src/snippets/runner.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::snippets::model::Snippet;

    #[test]
    fn test_fuzzy_filter_snippets() {
        let snippets = vec![
            Snippet {
                id: "1".into(),
                title: "Docker Shell".into(),
                command: "docker exec -it <id> sh".into(),
                description: "Open container shell".into(),
                tags: vec!["docker".into(), "shell".into()],
            },
            Snippet {
                id: "2".into(),
                title: "Git Log".into(),
                command: "git log --oneline".into(),
                description: "Compact history".into(),
                tags: vec!["git".into()],
            },
        ];
        let matches = filter_snippets(&snippets, "docker");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, "1");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib snippets::runner::tests`  
Expected: FAIL (function `filter_snippets` not found).

**Step 3: Write minimal implementation**

1. In `src/snippets/runner.rs`:
   - Implement `filter_snippets` using `crate::ui::fuzzy::fuzzy_match` across `title`, `description`, and `tags`.
   - Implement `run_snippets_panel(socket: Option<&str>) -> Result<(), String>`.
   - Event loop handling:
     - `/` for search query, `j`/`k`/Up/Down for selection.
     - `Enter`: if parameters exist, interactively prompt parameter values via terminal input; send command to active tmux pane using `tmux send-keys`.
     - `y`: prompt parameters and copy to clipboard (`crate::clipboard::copy`).
     - `a`: interactive prompt flow to add a new snippet, append to list, and call `save_snippets`.
     - `d`: delete selected snippet with confirmation prompt.
2. In `src/snippets/ui.rs`:
   - Render two-pane layout: list on left with `#tag` badges, rich preview on right showing description and highlighted command template.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib snippets::runner::tests`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/snippets/ui.rs src/snippets/runner.rs src/snippets/mod.rs
git commit -m "feat(snippets): implement interactive TUI panel with fuzzy search and parameter prompting"
```

---

### Task 7: Favorite Projects in `src/projects.rs`

**Files:**
- Modify: `src/projects.rs`
- Test: `tests/projects_favorites_test.rs`

**Step 1: Write the failing test**

Create `tests/projects_favorites_test.rs`:
```rust
use std::path::PathBuf;

#[test]
fn test_favorite_projects_are_pinned_to_top() {
    let dir1 = PathBuf::from("/tmp/mox_proj_a");
    let dir2 = PathBuf::from("/tmp/mox_proj_b");
    let favorites = vec![dir2.clone()];

    let mut projects = vec![
        mox::projects::ProjectForTest { path: dir1.clone(), is_favorite: false },
        mox::projects::ProjectForTest { path: dir2.clone(), is_favorite: true },
    ];
    mox::projects::sort_projects_with_favorites(&mut projects, &favorites);

    assert_eq!(projects[0].path, dir2);
    assert!(projects[0].is_favorite);
    assert_eq!(projects[1].path, dir1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test projects_favorites_test`  
Expected: FAIL.

**Step 3: Write minimal implementation**

In `src/projects.rs`:
- Add `pub is_favorite: bool` to `Project`.
- Implement `favorites_file() -> Result<PathBuf, String>`.
- Implement `read_favorites() -> HashSet<PathBuf>`.
- Implement `toggle_favorite(path: &Path) -> Result<bool, String>`.
- Pin favorite projects at the top of the discovery list with `★ ` prefix in `item(project: &Project)`.
- Update `run(socket, roots)` to support key `f` in `run_picker` (or custom runner loop) to toggle favorite status and refresh list.

**Step 4: Run test to verify it passes**

Run: `cargo test --test projects_favorites_test`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/projects.rs tests/projects_favorites_test.rs
git commit -m "feat(projects): support persistent favorite projects pinned to top with star icon"
```

---

### Task 8: CLI, Feature Actions, and Keymap Integration

**Files:**
- Modify: `src/cli.rs:6-120, 230-475`
- Modify: `src/actions.rs:10-60`
- Modify: `src/keymap/defaults.rs:245-485`
- Modify: `src/main.rs:20-140`

**Step 1: Write the failing test**

In `src/keymap/defaults.rs`:
```rust
#[test]
fn test_keymap_contains_snippets_bindings() {
    let keymap = build_default_keymap();
    // 'C' in root table
    let root_c = keymap.children.iter().find(|n| n.key == "C");
    assert!(root_c.is_some(), "Key 'C' must be bound to snippets");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib keymap::defaults::tests::test_keymap_contains_snippets_bindings`  
Expected: FAIL.

**Step 3: Write minimal implementation**

1. In `src/actions.rs`:
   - Add `FeatureAction::Snippets` to enum and `label()` ("favorite commands & snippets").
   - Handle in `actions::run`: launch snippets runner.
2. In `src/cli.rs`:
   - Add `Commands::Snippets { socket: Option<String>, popup: bool }`.
   - Dispatch to `crate::snippets::run_snippets_panel`.
3. In `src/keymap/defaults.rs`:
   - Bind `C` (Shift+c) in root to `KeyAction::Feature(FeatureAction::Snippets)`.
   - Bind `c` in `goto_children` (`g c`) to `KeyAction::Feature(FeatureAction::Snippets)`.
4. In `src/main.rs`:
   - Dispatch `Commands::Native(Commands::Snippets { .. })`.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib keymap::defaults::tests::test_keymap_contains_snippets_bindings`  
Expected: PASS.

**Step 5: Commit**

```bash
git add src/cli.rs src/actions.rs src/keymap/defaults.rs src/main.rs
git commit -m "feat(cli,keymap): wire up snippets panel CLI command and keybindings"
```

---

### Task 9: Full Workspace Verification and Documentation

**Files:**
- Modify: `README.md`
- Check: workspace clippy, tests, fmt

**Step 1: Update documentation**

Document the new tree styling, `mox snippets` command, keybindings (`C`, `g c`), and favorite projects (`f` in projects picker) in `README.md`.

**Step 2: Run all workspace verification commands**

Run:
```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Expected: All checks exit 0 with 0 warnings.

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document snippets cheatsheet, tree UI, and favorite projects"
```
