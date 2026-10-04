# Native Mox workflows

Mox can install its tmux bindings directly from the `mox` binary. Install it with `cargo install --path .` and make sure `~/.cargo/bin` is on `PATH` for the tmux server and any Neovim process that should register with Mox.

Add this to `~/.tmux.conf`:

```tmux
run-shell 'mox init --apply'
```

Reload it with `tmux source-file ~/.tmux.conf`. No TPM plugin entry is needed for this setup. The modal entry key defaults to `M-m`; for another key, use `run-shell 'mox init --apply --entry-key M-x'`.

## Prefix shortcuts

`<prefix>` means the tmux prefix configured on your machine. Mox adds these direct prefix bindings:

| Keys | Action |
| --- | --- |
| `<prefix> Space` | Search the command palette, including Mox actions and tmux targets |
| `<prefix> o` | Pick a project, recent directory, or existing session |
| `<prefix> f` | Pick a file and open it in a registered Neovim instance |
| `<prefix> C-f` | Pick changed Git files and open one in Neovim |
| `<prefix> /` | Search file contents with ripgrep and open a match in Neovim |
| `<prefix> N` | Open LazyGit |
| `<prefix> C-s` | Save a tmux session snapshot |
| `<prefix> C-r` | Restore a saved snapshot |

`M-m` enters the modal key table; `?` shows its key hints. `M-z` temporarily suspends Mox's global bindings for the current tmux client; press `M-z` again to resume. `C-h/j/k/l` moves between panes, while an active Neovim pane receives those keys for split navigation. In tmux copy mode, `C-f` pages down and `C-b` pages up; this is separate from `<prefix> C-f`, which opens the Git file picker.

## Projects

The project picker includes existing tmux session directories, recently selected directories, each configured root itself, Git repositories found under those roots, and directories from `zoxide query -l` when zoxide is installed. Configure roots in the tmux server environment with a colon-separated variable, or pass roots to the command:

```sh
export MOX_PROJECT_ROOTS="$HOME/code:$HOME/work"
mox projects --root /srv/projects
```

The recursive scan stops at depth 4 and examines at most 5,000 directory entries total. It skips directory symlinks. Recent directories are kept in Mox's state directory, up to 100 entries. The state directory is `MOX_STATE_DIR`, then `$XDG_STATE_HOME/mox`, then `~/.local/state/mox`.

Choosing an existing session switches to that exact session. Choosing a directory creates a detached session there, then switches to it. Session names are derived from the directory name and made unique when needed.

## Neovim file bridge

The bridge lets `mox editor files`, `mox editor git`, and `mox editor rg QUERY` open a location in an already running Neovim pane. Add this snippet to Neovim's `init.lua`:

```lua
local code = vim.fn.system({ 'mox', 'editor', 'setup' })
assert(vim.v.shell_error == 0, code)
assert(loadstring(code))()
```

Neovim must run inside a tmux pane and be able to find `mox` on `PATH`. The setup registers its Neovim server endpoint and PID with the owning pane. Mox checks that the endpoint still belongs to the same pane process before opening a file. If no Neovim pane exists, `mox editor open path/to/file:line:column` starts a new Neovim window. An existing Neovim pane without the bridge reports a setup error. Paths can be relative to a source pane with `--pane %PANE_ID`.

The direct commands are:

```sh
mox editor open src/main.rs:20:5
mox editor files
mox editor git
mox editor rg 'TODO|FIXME'
```

Each picker is bounded: external listing/search commands have a two-second timeout and an 8 MiB output cap, and pickers keep at most 10,000 entries.

## Session snapshots

Save and restore explicitly with:

```sh
mox persist save
mox persist restore
mox persist restore --commands
```

Snapshots preserve tmux sessions and window/pane layout, working directories, active selections, and zoom state. Restore leaves sessions that already exist untouched. By default, new panes start with their normal shell. `--commands` additionally restarts only captured Neovim and LazyGit command lines when their argument vectors are available; it does not save or restore arbitrary process state, shell jobs, application memory, terminal scrollback, or unsaved buffers.

`mox init --apply` enables automatic snapshots and autorestore by default. The autosave interval defaults to 900 seconds and can be changed with the tmux option `@mox_autosave_interval`.
