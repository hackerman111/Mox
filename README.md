# Mox

Vim-style controls for tmux, with key hints when you need them.

Press one key to enter modal mode, then use short key sequences to move between panes, manage windows and sessions, browse scrollback, or jump to text in a pane. Mox shows the available keys as you go. It is a single Rust binary; it does not run a background service.

## Install

Requirements: tmux and, when building from source, Rust/Cargo.

Build the binary from this repository:

```sh
cargo build --release
```

Add the loader to `~/.tmux.conf`, using the absolute path to this checkout:

```tmux
set -g @mox_entry_key M-m
run-shell "/path/to/moch/plugin.tmux"
```

Reload tmux configuration with `tmux source-file ~/.tmux.conf`. The loader uses the built binary when available and builds it in release mode otherwise. The entry key defaults to `M-m`; set `@mox_entry_key` to change it.

## Use

Press `M-m` to enter modal mode. Press `?` to see the available commands. `q` or `Escape` leaves the current submenu; `i` or another `M-m` exits modal mode.

| Keys | Action |
| --- | --- |
| `w` | Window and pane controls |
| `g` | Open a path or URL; navigate the tmux tree |
| `s` / `S` | Flash jump in this pane / across panes |
| `y` | Quick entity and hash labels, extraction, paste |
| `v` or `[` | Scrollback and visual selection |
| `e` | Extract text from the pane |
| `t` / `T` | Tea timer |

Mox also includes Vim-style copy and extract modes, pane selection, rename prompts, and pane/window/session actions. See the [modal command map](docs/tmux-modal-parity.md) and [visual navigation guide](docs/tmux-visual-navigation.md) for the full key reference.

## Development

```sh
cargo test --workspace
cargo build
python3 tests/tmux_popup_smoke.py
```

The popup smoke check requires Python 3 and tmux. It starts a private tmux server and uses a fake clipboard daemon.

## License

Mox is licensed under the [MIT License](LICENSE).
