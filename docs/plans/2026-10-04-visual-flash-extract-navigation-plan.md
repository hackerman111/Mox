# Visual Navigation, Flash Jump & Extract Picker Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Implement a four-tier terminal navigation and text manipulation suite in Mox (Extract Picker, Flash Jump, Copy/Scroll Mode, Remote Operators) with zero external runtime dependencies and sub-millisecond responsiveness.

**Architecture:** A unified tokenization pipeline (`src/extract/`) powers both deep scrollback fuzzy picking (`Extract Picker`) and instant home-row jump badges (`Flash Engine`). A borderless `display-popup` overlay renders Flash labels pixel-perfect above pane splits, while remote operators (`ys`) resolve semantic text objects directly to system and tmux clipboards without moving the shell cursor.

**Tech Stack:** Rust (edition 2024), `crossterm 0.29`, `clap 4.5`, native tmux CLI (`capture-pane`, `display-popup`, `copy-mode-vi`).

---

### Task 1: Token Model & EntityKind Structures

**Files:**
- Create: `src/extract/mod.rs`
- Create: `src/extract/model.rs`
- Modify: `src/lib.rs:1-9`
- Test: `tests/test_extract_model.rs`

**Step 1: Write the failing test**

```rust
// tests/test_extract_model.rs
use mox::extract::model::{EntityKind, ExtractedToken};

#[test]
fn test_extracted_token_creation() {
    let token = ExtractedToken {
        kind: EntityKind::Path,
        raw_text: "src/main.rs:42:10".to_string(),
        clean_text: "src/main.rs".to_string(),
        line_number: Some(42),
        col_number: Some(10),
        pane_id: "%0".to_string(),
        screen_row: 5,
        col_start: 12,
        col_end: 29,
    };

    assert_eq!(token.kind, EntityKind::Path);
    assert_eq!(token.clean_text, "src/main.rs");
    assert_eq!(token.line_number, Some(42));
    assert_eq!(token.col_number, Some(10));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_extract_model`
Expected: FAIL with "cannot find module or unresolved import `extract`"

**Step 3: Write minimal implementation**

```rust
// src/extract/model.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Path,
    Url,
    Hash,
    Ip,
    Uuid,
    Command,
    Quoted,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedToken {
    pub kind: EntityKind,
    pub raw_text: String,
    pub clean_text: String,
    pub line_number: Option<usize>,
    pub col_number: Option<usize>,
    pub pane_id: String,
    pub screen_row: usize,
    pub col_start: usize,
    pub col_end: usize,
}

// src/extract/mod.rs
pub mod model;
pub use model::{EntityKind, ExtractedToken};
```

Export `pub mod extract;` in `src/lib.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_extract_model`
Expected: PASS

**Step 5: Commit**

```bash
git add src/extract src/lib.rs tests/test_extract_model.rs
git commit -m "feat(extract): add ExtractedToken data model and EntityKind"
```

---

### Task 2: Token Scanner & Entity Extractors

**Files:**
- Create: `src/extract/scanner.rs`
- Modify: `src/extract/mod.rs`
- Test: `tests/test_extract_scanner.rs`

**Step 1: Write the failing test**

```rust
// tests/test_extract_scanner.rs
use mox::extract::model::EntityKind;
use mox::extract::scanner::scan_line;

#[test]
fn test_scan_entities_in_line() {
    let line = "error at src/lib.rs:25:3 see https://example.com commit d8a37b2f4a";
    let tokens = scan_line(line, 0, "%0");

    assert!(tokens.iter().any(|t| t.kind == EntityKind::Path && t.clean_text == "src/lib.rs" && t.line_number == Some(25)));
    assert!(tokens.iter().any(|t| t.kind == EntityKind::Url && t.clean_text == "https://example.com"));
    assert!(tokens.iter().any(|t| t.kind == EntityKind::Hash && t.clean_text == "d8a37b2f4a"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_extract_scanner`
Expected: FAIL with "cannot find function `scan_line`"

**Step 3: Write minimal implementation**

Implement single-pass line scanning with URL, path/line/col, SHA hex hash, IP, and quote tokenizers in `src/extract/scanner.rs`, exporting `scan_line` and `scan_lines`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_extract_scanner`
Expected: PASS

**Step 5: Commit**

```bash
git add src/extract/scanner.rs src/extract/mod.rs tests/test_extract_scanner.rs
git commit -m "feat(extract): implement high-speed token scanner for paths, urls and hashes"
```

---

### Task 3: Flash Matcher & Labeler Algorithm

**Files:**
- Create: `src/flash/mod.rs`
- Create: `src/flash/matcher.rs`
- Create: `src/flash/labeler.rs`
- Modify: `src/lib.rs`
- Test: `tests/test_flash_labeler.rs`

**Step 1: Write the failing test**

```rust
// tests/test_flash_labeler.rs
use mox::flash::labeler::{assign_labels, MatchTarget};

#[test]
fn test_label_assignment_ergonomics() {
    let targets = vec![
        MatchTarget { row: 1, col: 5, pane_id: "%0".into() },
        MatchTarget { row: 2, col: 10, pane_id: "%0".into() },
    ];
    let labeled = assign_labels(&targets, 1, 0); // cursor at row 1, col 0
    assert_eq!(labeled.len(), 2);
    assert_eq!(labeled[0].label, "a");
    assert_eq!(labeled[1].label, "s");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_flash_labeler`
Expected: FAIL with "cannot find module `flash`"

**Step 3: Write minimal implementation**

Implement home-row prioritized labeler (`asdfjkl;...`), distance scoring relative to current cursor, 1-char and 2-char expansion in `src/flash/labeler.rs`, and incremental prefix matching in `src/flash/matcher.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_flash_labeler`
Expected: PASS

**Step 5: Commit**

```bash
git add src/flash src/lib.rs tests/test_flash_labeler.rs
git commit -m "feat(flash): implement incremental matcher and home-row labeler"
```

---

### Task 4: Remote Operator Text Objects & Resolution

**Files:**
- Create: `src/flash/text_object.rs`
- Modify: `src/flash/mod.rs`
- Test: `tests/test_flash_text_object.rs`

**Step 1: Write the failing test**

```rust
// tests/test_flash_text_object.rs
use mox::flash::text_object::{resolve_text_object, TextObject};

#[test]
fn test_resolve_word_and_token_objects() {
    let line = "let candidate = \"https://antigravity.dev\";";
    // Target is inside the URL
    let col = 20;
    let resolved = resolve_text_object(line, col, TextObject::AutoToken).unwrap();
    assert_eq!(resolved, "https://antigravity.dev");

    // Inner word
    let word = resolve_text_object(line, 4, TextObject::InnerWord).unwrap();
    assert_eq!(word, "candidate");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_flash_text_object`
Expected: FAIL with "cannot find module `text_object`"

**Step 3: Write minimal implementation**

Implement `TextObject` enum (`InnerWord`, `AWord`, `AutoToken`, `Line`, `ToEndOfLine`, `Quoted`) and extraction logic in `src/flash/text_object.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_flash_text_object`
Expected: PASS

**Step 5: Commit**

```bash
git add src/flash/text_object.rs src/flash/mod.rs tests/test_flash_text_object.rs
git commit -m "feat(flash): implement semantic text objects and remote resolution"
```

---

### Task 5: Flash Overlay Display-Popup Runner & UI

**Files:**
- Create: `src/flash/overlay.rs`
- Create: `src/flash/ui.rs`
- Modify: `src/main.rs`
- Test: `tests/test_flash_overlay.rs`

**Step 1: Write the failing test**

```rust
// tests/test_flash_overlay.rs
use mox::flash::overlay::FlashConfig;

#[test]
fn test_flash_config_construction() {
    let cfg = FlashConfig::new_jump(false);
    assert!(!cfg.multi_pane);
    assert!(!cfg.remote_yank);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_flash_overlay`
Expected: FAIL with "cannot find module `overlay`"

**Step 3: Write minimal implementation**

Implement crossterm terminal overlay renderer in `src/flash/ui.rs`, tmux popup launcher in `src/flash/overlay.rs`, and wire `Commands::Flash` in `src/main.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_flash_overlay`
Expected: PASS

**Step 5: Commit**

```bash
git add src/flash/overlay.rs src/flash/ui.rs src/main.rs tests/test_flash_overlay.rs
git commit -m "feat(flash): implement crossterm display-popup overlay runner and CLI command"
```

---

### Task 6: Built-in Extract TUI Picker

**Files:**
- Create: `src/extract/ui.rs`
- Modify: `src/extract/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/test_extract_ui.rs`

**Step 1: Write the failing test**

```rust
// tests/test_extract_ui.rs
use mox::extract::ui::ExtractState;
use mox::extract::model::{EntityKind, ExtractedToken};

#[test]
fn test_extract_state_category_cycling() {
    let mut state = ExtractState::new(vec![]);
    assert_eq!(state.current_category, None);
    state.cycle_category_forward();
    assert_eq!(state.current_category, Some(EntityKind::Path));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_extract_ui`
Expected: FAIL with "cannot find module `ui` in `extract`"

**Step 3: Write minimal implementation**

Implement interactive categorized TUI with fuzzy search, category tabs (`Tab`), clipboard yank (`y`), shell insert (`Enter`), and editor/web open (`gf`/`gx`) in `src/extract/ui.rs`. Wire `Commands::Extract` in `src/main.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_extract_ui`
Expected: PASS

**Step 5: Commit**

```bash
git add src/extract/ui.rs src/extract/mod.rs src/main.rs tests/test_extract_ui.rs
git commit -m "feat(extract): implement categorized TUI fuzzy picker and CLI command"
```

---

### Task 7: Modal Keymap & Copy-Mode-Vi Integration

**Files:**
- Modify: `src/keymap/defaults.rs`
- Modify: `src/tmux/scroll.rs`
- Modify: `src/tmux/config.rs`
- Test: `tests/test_keymap_visual_bindings.rs`

**Step 1: Write the failing test**

```rust
// tests/test_keymap_visual_bindings.rs
use mox::keymap::build_default_keymap;

#[test]
fn test_visual_and_flash_keybindings_exist() {
    let keymap = build_default_keymap();
    assert!(keymap.find_child('s').is_some(), "Modal mode must contain 's' for Flash jump");
    assert!(keymap.find_child('S').is_some(), "Modal mode must contain 'S' for multi-pane Flash jump");
    assert!(keymap.find_child('e').is_some(), "Modal mode must contain 'e' for Extract picker");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test test_keymap_visual_bindings`
Expected: FAIL

**Step 3: Write minimal implementation**

Add `s`, `S`, `e`, `yf`, `yh`, `gf`, `gx`, `ys` to `src/keymap/defaults.rs`.
Add vi motions (`f`, `F`, `t`, `T`, `s`, `ys`) and seamless mouse config into `src/tmux/scroll.rs`.

**Step 4: Run test to verify it passes**

Run: `cargo test --test test_keymap_visual_bindings`
Expected: PASS

**Step 5: Commit**

```bash
git add src/keymap/defaults.rs src/tmux/scroll.rs src/tmux/config.rs tests/test_keymap_visual_bindings.rs
git commit -m "feat(keymap): integrate flash jump, extract picker and copy-mode-vi bindings"
```

---

### Task 8: Verification Suite & Documentation

**Files:**
- Modify: `README.md` or `docs/`
- Test: Full integration test suite

**Step 1: Run comprehensive workspace verification**

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Expected: All checks pass with 0 errors and 0 warnings.

**Step 2: Commit**

```bash
git add docs/
git commit -m "chore: verify test suite and complete visual navigation documentation"
```
