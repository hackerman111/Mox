# Mox UI Toolkit and Theme Architecture

This document describes the unified terminal user interface (TUI) layer, Theme engine, and presentation widgets introduced in Mox Part 1.

---

## 1. Architectural Principles

1. **Pure Presentation Separation:**
   The UI toolkit is strictly decoupled from tmux CLI calls, system clipboard binaries, file editors, and process spawning. Render functions receive borrowed data, terminal dimensions, query/selection state, and a `Theme` instance, outputting formatted terminal cells via Crossterm.
2. **Single Theme Owner:**
   All color roles, highlights, badges, and mode indicators are resolved through `Theme`. Individual features do not maintain independent color palettes or hardcoded ANSI escape sequences.
3. **Borrowed Data & Zero Cloning on Redraw:**
   Widgets like `ListRow` and `TreeRow` borrow labels and badges (`&'a str`, `Option<(&'a str, Color)>`). Redrawing the screen during user typing or navigation does not clone models or capture scrollback history.
4. **Narrow Terminal & Resize Resilience:**
   Every widget supports clamping and truncation (`truncate_or_pad`, `truncate_str`). Extreme terminal dimensions (e.g. 0 to 20 columns) render gracefully without panicking or wrapping lines unexpectedly.

---

## 2. Nord Palette and Semantic Roles

The default theme is built upon the Arctic Ice Studio **Nord** palette (`src/ui/theme.rs`):

- **Polar Night (Backgrounds & Borders):**
  - `nord0` (`#2E3440`): Primary background (`theme.bg`).
  - `nord1` (`#3B4252`): Elevated component background.
  - `nord2` (`#434C5E`): Active selection background (`theme.selection_bg`).
  - `nord3` (`#4C566A`): Borders, dividers, muted text, comments (`theme.border`, `theme.muted`).
- **Snow Storm (Foregrounds):**
  - `nord4` (`#D8DEE9`): Subtitle and secondary text.
  - `nord5` (`#E5E9F0`): Clean foreground text.
  - `nord6` (`#ECEFF4`): Highlighted / active text (`theme.fg`, `theme.selection_fg`).
- **Frost (Accents & Controls):**
  - `nord7` (`#8FBCBB`): Teal accent (hash badge, IPFS).
  - `nord8` (`#88C0D0`): Primary cyan accent (prompt badge, command badge, `theme.accent`).
  - `nord9` (`#81A1C1`): Light blue info / preview header (`theme.info`).
  - `nord10` (`#5E81AC`): Deep blue (copy mode).
- **Aurora (States & Warnings):**
  - `nord11` (`#BF616A`): Red (errors, dangerous confirmation `[CONFIRM]`, sync mode).
  - `nord12` (`#D08770`): Orange (IP badge, visual mode).
  - `nord13` (`#EBCB8B`): Yellow (URL badge, rename buffer prompt, prefix mode, `theme.warning`).
  - `nord14` (`#A3BE8C`): Green (path badge, success toast notifications, normal mode, `theme.success`).
  - `nord15` (`#B48EAD`): Purple / Magenta (AI agent badge, resize mode).

---

## 3. Core Presentation Primitives

### `ListRow`
Renders a row in flat lists (such as Extract tokens, search results, or command palette rows):
```rust
let row = ListRow {
    prefix: if is_selected { "> " } else { "  " },
    badge: Some(("[PATH]", theme.badge_path)),
    text: "src/main.rs:42",
    is_selected,
};
render_list_row(&mut out, &row, width, &theme)?;
```

### `TreeRow`
Renders hierarchical branch rows for the tree navigator (`Session -> Window -> Pane`):
```rust
let row = TreeRow {
    depth: 1,
    is_collapsed: Some(false),
    label: "1: editor",
    badge: Some(("[AI]", theme.mode_agent)),
    is_active: false,
    is_selected: true,
};
render_tree_row(&mut out, &row, width, &theme)?;
```

### `Viewport`
Stateless scroll window calculation ensuring the selected index remains in view:
```rust
let scroll_offset = Viewport::adjust_scroll(selected_index, current_offset, visible_rows);
```

### `render_prompt`
Renders top-level search inputs with mode badges and count indicators:
```rust
render_prompt(&mut out, "[SEARCH]", &query, Some("12 matches"), width, &theme)?;
```

### `render_toast`
Renders transient feedback messages (success or error):
```rust
render_toast(&mut out, "Yanked to clipboard", false, width, &theme)?;
```

### `render_confirm`
Renders danger / action confirmation prompts:
```rust
render_confirm(&mut out, "kill-window editor? (y/n)", &theme)?;
```

### `render_preview_panel`
Renders side-by-side preview panes with vertical dividers:
```rust
render_preview_panel(&mut out, "Preview", &lines, start_x, start_y, preview_w, preview_h, &theme)?;
```

### `render_status_badge` & `Mode`
Supports all 8 operational modes: `NORMAL`, `PREFIX`, `COPY`, `VISUAL`, `RESIZE`, `SYNC`, `SUSPEND`, `AGENT`.
```rust
render_status_badge(&mut out, Mode::Normal, false, &theme)?;
```
