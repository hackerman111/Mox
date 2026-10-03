# Visual Navigation, Flash Jump & Extract Picker Design

## Overview & Philosophy

The goal is to bring modern, ergonomic Vim/Neovim navigation and text extraction paradigms into tmux via **Mox**, splitting terminal interaction into four specialized, highly optimized mechanisms:

1. **Extract Picker**: Typed entity extraction (`Path`, `Url`, `Hash`, `IP`, `Command`) across deep terminal scrollback with categorized fuzzy search.
2. **Jump Mode**: Flash/EasyMotion-style instant jump labels (`asdfjkl...`) for visible screen targets with arbitrary search prefixes (`s`/`S`) or typed entity markers.
3. **Copy/Scroll Mode**: Seamless native tmux `copy-mode-vi` navigation and mouse wheel scrolling without friction.
4. **Remote Operator Mode**: Flash.nvim-inspired remote operations (`ys`) to yank/copy remote text objects (words, paths, URLs, quotes) without losing cursor position in shell.

Strict project constraints per `AGENTS.md` and `Task.md`:
- **Single Self-Contained Rust Binary**: Zero external runtime dependencies (no external `fzf`, Python, Ruby, or background daemons).
- **Sub-millisecond Performance**: Instant pane capture, zero-copy entity scanning, flicker-free `display-popup` overlay rendering.
- **Vim Ergonomics**: Dual entry points (Modal mode via `M-m` and direct keys inside `copy-mode-vi`).

---

## 1. UX Grammar & Keymap Matrix

### A. Modal Mode (`M-m`) Entry
From any active shell, pressing `M-m` activates the modal keymap with immediate access to:

| Key Sequence | Action | Target / Behavior |
|---|---|---|
| `s` | Flash Jump (Current Pane) | Incremental search overlay → labels → jump into `copy-mode-vi` |
| `S` | Flash Jump (Multi-Pane) | Full-window overlay → jump across split panes into `copy-mode-vi` |
| `yf` | Quick Entity Yank | Immediate labels on all visible tokens → copy to clipboard → return to shell |
| `yh` | Quick Hash Yank | Immediate labels on visible Git commit SHAs → copy to clipboard |
| `gf` | Quick File Open | Immediate labels on file paths → open in `$EDITOR +line:col path` |
| `gx` | Quick URL Open | Immediate labels on URLs → open in browser via `xdg-open` / `open` |
| `ys` | Remote Flash Yank | Flash search → select label → yank text object without moving cursor |
| `e` (or `y e`) | Extract Picker | Built-in TUI popup for deep scrollback with categorized fuzzy filter |
| `[` or `v` | Enter Copy Mode | Direct transition to native `copy-mode-vi` |

### B. Copy-Mode-Vi (Native Vim Navigation)
Inside tmux `copy-mode-vi`, the keys operate as native motions and operators:

| Key Sequence | Mechanism | Description |
|---|---|---|
| `j`, `k`, `h`, `l` | Vim Motion | 1-line vertical / horizontal cursor movement |
| `w`, `b`, `e` | Vim Motion | Word navigation |
| `C-d`, `C-u` | Vim Motion | Half-page down / up |
| `C-f`, `C-b` | Vim Motion | Full-page down / up |
| `gg`, `G` | Vim Motion | Top / bottom of scrollback history |
| `f<char>`, `F<char>` | Flash Motion | Search char forward/backward; instant labels if multiple hits |
| `t<char>`, `T<char>` | Flash Motion | Search till char forward/backward with labels |
| `;`, `,` | Repeat Motion | Repeat last `f`/`F`/`t`/`T` forward / backward |
| `s`, `S` | Flash Jump | Jump cursor directly to target in current / all panes |
| `v`, `V` | Visual Selection | Character / line-wise selection |
| `v s` | Range Flash | Flash-jump to target and select enclosing semantic text object |
| `y` | Operator | Copy active selection and cancel copy-mode |
| `ys` | Remote Operator | Remote yank target without moving current copy-mode cursor |
| `gf`, `gx` | Action | Open path under cursor in editor / URL in browser |
| `q`, `Esc` | Cancel | Exit copy-mode immediately |

### C. Seamless Mouse Wheel
- If foreground app uses mouse tracking (Neovim, less, fzf), mouse events pass through directly (`send-keys -M`).
- If in standard shell, wheel scrolling up enters `copy-mode-vi` and scrolls scrollback naturally.

---

## 2. Extraction Pipeline & Data Model

Located in `src/extract/`:

### Data Structures
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Path,    // e.g. src/main.rs, ~/code/test.py:127:5
    Url,     // e.g. https://github.com/foo/bar, git@github.com:...
    Hash,    // Git commit SHAs (7..40 hex chars)
    Ip,      // IPv4 / IPv6 addresses + ports
    Uuid,    // UUID v4 format
    Command, // Shell commands prefixed by '$ ' or '> '
    Quoted,  // Strings within double/single/backtick quotes
    Number,  // Numeric identifiers, status codes, ports
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
```

### Capture Engine
- **Visible Capture**: `tmux capture-pane -p` for current pane geometry. Fast single-pass tokenization (< 1ms).
- **Scrollback Capture**: `tmux capture-pane -p -S -2000` for deep history extraction.
- **Trimming & Cleaning**: Trailing punctuation marks (`:`, `,`, `)`, `.`) are trimmed from clean URLs and paths while preserving precise column positions.

---

## 3. Flash Engine Architecture

Located in `src/flash/`:

```text
Capture Screen
      │
      ▼
┌──────────────┐
│   Matcher    │ ── Substring search (s/S) or Entity filter (yf/gf/gx)
└──────┬───────┘
       ▼
┌──────────────┐
│   Labeler    │ ── Ergonomic home-row keys (a s d f j k l ; ...)
└──────┬───────┘
       ▼
┌──────────────┐
│   Renderer   │ ── Borderless display-popup overlay matching pane geometry
└──────┬───────┘
       ▼
┌──────────────┐
│    Action    │ ── Jump into copy-mode-vi, Yank to clipboard, Open in editor/web
└──────────────┘
```

### Components
1. **Matcher**:
   - `PatternMatcher`: Incremental character input with smart-case matching.
   - `TokenMatcher`: Direct matching on pre-extracted `ExtractedToken` lists (no input needed).
   - `CharMotionMatcher`: Single-character scan forward/backward (`f`/`F`/`t`/`T`).
2. **Labeler**:
   - Priority keys: `a s d f j k l ; g h q w e r u i o p`.
   - Distributes single-character labels to targets closest to the cursor.
   - Multi-target scale-up: Uses 2-character prefixes (`aa`, `as`, `ad`...) when candidates exceed single-key alphabet size.
3. **Renderer**:
   - Queries pane bounds via `tmux display-message -p "#{pane_left},#{pane_top},#{pane_width},#{pane_height}"`.
   - Opens borderless popup via `tmux display-popup -E -x <x> -y <y> -w <w> -h <h> "mox flash ..."`.
   - Dims background terminal text, renders highlighted colored badges over match targets.
   - For `S` (multi-pane), opens fullscreen (`100% x 100%`) across all panes.

---

## 4. Remote Operators & Semantic Text Objects

### Remote Yank Workflow (`ys`)
1. User presses `ys`.
2. Flash overlay appears; user types target prefix or chooses visible match.
3. User selects label badge (e.g. `A`).
4. **Resolution**:
   - **Auto-Detect**: If target aligns with an extracted token (URL, Path, Hash, Quote), pressing `Enter` or retyping label copies the entire token.
   - **Text Objects**: User can enter standard motion:
     - `iw` / `aw`: inner / a word
     - `i"` / `a"`: inner / a quoted string
     - `p`: path (with `:line:col`)
     - `u`: URL
     - `l`: entire screen line
     - `$`: to end of line
5. **Execution**: Selected text is saved to tmux paste buffer and system clipboard (`wl-copy` / `xclip` / OSC 52). Overlay closes. Shell cursor remains completely unchanged.

---

## 5. Built-in Extract TUI Picker

Located in `src/extract/ui.rs`:

- **Autonomous Crossterm TUI**: Centered popup (approx 75% width, 60% height). Zero external `fzf` requirement.
- **Categorized Sections**: Grouped by `[PATH]`, `[URL]`, `[HASH]`, `[IP]`, `[CMD]`.
- **Fuzzy Filter**: Powered by `mox::nav::fuzzy::fuzzy_match` with matched character highlighting.
- **Category Tabs**: `Tab` / `Shift-Tab` cycles filters (`ALL` → `PATH` → `URL` → `HASH` → `CMD`).
- **Actions**:
  - `Enter`: Paste text directly into active shell (`tmux send-keys -l`).
  - `y`: Copy to clipboard and tmux buffer.
  - `gf` / `o`: Open in `$EDITOR +<line> <path>`.
  - `gx`: Open URL in default browser.
  - `Esc` / `q`: Close popup.

---

## 6. Verification & Architectural Boundaries

- **Separation of Concerns**:
  - `src/extract/`: Pure extraction logic and token models.
  - `src/flash/`: Labeling algorithm, overlay geometry, and flash UI.
  - `src/tmux/`: Low-level CLI commands, popup invocation, buffer management.
  - `src/keymap/`: Integration into Mox modal key tree and `copy-mode-vi` configurations.
- **Verification Suite**:
  - Unit tests for token regexes, path/line parsing, and boundary trimming.
  - Unit tests for Flash `Labeler` key distribution and prefix collisions.
  - Integration tests with isolated tmux socket (`tmux -L mox-test`).
  - Standard checks: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`.
