<img width="1254" height="1254" alt="Moss Terminal MOX Mascot" src="https://github.com/user-attachments/assets/6ea6cdd1-d933-4199-95ad-645d2a87b068" />

# Mox

Vim-style controls for tmux, with key hints when you need them.

[GitHub repository](https://github.com/hackerman111/Mox)

Press one key to enter modal mode, then use short key sequences to move between panes, manage windows and sessions, browse scrollback, or jump to text in a pane. Mox shows the available keys as you go. It is a single Rust binary; it does not run a background service.

## Install

Requirements: tmux. The recommended install uses Cargo and needs Rust/Cargo. The legacy TPM loader can download a prebuilt release when one is available.

### Cargo install (recommended)

Clone the repository and install the binary:

```sh
git clone https://github.com/hackerman111/Mox.git
cd Mox
cargo install --path .
```

Cargo installs `mox` in `~/.cargo/bin` by default. Make sure that directory is on `PATH` for both your shell and the tmux server. Then add this line to `~/.tmux.conf`:

```tmux
run-shell 'mox init --apply'
```

Reload the config with `tmux source-file ~/.tmux.conf`. This applies Mox's native key tables and bindings without TPM. The default modal entry key is `M-m`; use `mox init --apply --entry-key M-x` to choose another key.

See [Native workflows](docs/native-workflows.md) for editor setup, projects, session snapshots, and the new prefix bindings.

### TPM

Add Mox to your `~/.tmux.conf` alongside your other TPM plugins:

```tmux
set -g @mox_entry_key M-m
set -g @plugin 'hackerman111/Mox'
```

Keep the Mox entry after any theme plugin that sets `status-left`; Mox adds its mode indicator to that option. Reload tmux, then press your TPM install key (usually `prefix` + `I`). The loader downloads a prebuilt binary on supported systems and falls back to Cargo if none is available.

### Build from source

For a local development build instead of installing with Cargo:

```sh
git clone https://github.com/hackerman111/Mox.git
cd Mox
cargo build --release
```

The release workflow builds binaries for Linux (glibc, x86-64/ARM64) and macOS (Intel/Apple Silicon). To publish one, push a `v` tag matching the version in `Cargo.toml` (for example, `v0.1.0`).

The optional legacy `plugin.tmux` loader remains available for existing TPM setups. For a direct checkout-based loader without TPM, add this to `~/.tmux.conf`, using the absolute path to the checkout:

```tmux
set -g @mox_entry_key M-m
run-shell "/path/to/Mox/plugin.tmux"
```

Reload tmux configuration with `tmux source-file ~/.tmux.conf`. The entry key defaults to `M-m`; pass `--entry-key` to `mox init` to change it.

## Use

Press `M-m` to enter modal mode. Press `?` to see the available commands. `q` or `Escape` leaves the current submenu; `i` or another `M-m` exits modal mode.

| Keys       | Action                                          |
| ---------- | ----------------------------------------------- |
| `w`        | Window and pane controls                        |
| `g`        | Open a path or URL; navigate the tmux tree      |
| `s` / `S`  | Flash jump in this pane / across panes          |
| `y`        | Quick entity and hash labels, extraction, paste |
| `v` or `[` | Scrollback and visual selection                 |
| `e`        | Extract text from the pane                      |
| `t` / `T`  | Tea timer                                       |

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
