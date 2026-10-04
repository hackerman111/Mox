# TPM to native Mox migration checklist

This checklist describes the native setup implemented in this checkout and the evidence boundary for replacing each local plugin. “Tests pass” means the named automated tests pass; parser or command-generation coverage alone does not establish interactive GUI or full TPM parity. See [compatibility inventory](compatibility-inventory.md) and [part 2 validation](part-2-validation.md).

## 1. Minimal tmux setup

Install the `mox` binary in `PATH`, then use:

```tmux
set -g prefix C-Space
set -g mouse on
set -g history-limit 200000
run-shell 'mox init --apply'
```

`mox init --apply` installs Mox's generated bindings, mode/status formatting, scroll configuration, and startup hooks. The snippet explicitly sets a prefix and history size; keep any other personal tmux settings you need. Do not source the old `plugin.tmux` alongside the new bindings during final migration unless intentionally testing a hybrid setup.

The current isolated-tmux smoke results and their limits are recorded in [part 2 validation](part-2-validation.md). Flash/Extract, selected file-to-Neovim, palette, Projects-cancel, suspend, wheel handling, and Neovim vertical-split navigation across the tmux-pane boundary have smoke coverage. Agent workflows do not yet.

## 2. Optional feature commands

The init command does not install external tools or configure Neovim for you. These workflows are opt-in:

| Workflow | Command / configuration | Notes |
|---|---|---|
| Neovim bridge | `mox editor setup`, then ensure the Mox Lua module loads/registers in Neovim | Required for Neovim endpoint discovery and editor-aware navigation. Setup and parser tests do not prove a user's runtime config is active. |
| Project roots | `mox projects --root ~/src --root ~/work` or `MOX_PROJECT_ROOTS` | Discovers recent/session directories, Git repositories under configured roots, and zoxide entries when zoxide is installed. Keep roots bounded and relevant. |
| Snapshot lifecycle | `mox persist save`, `restore`, `autosave`, `autorestore`; tmux init exposes the configured hooks | Restore policy should be exercised on a disposable isolated tmux server before relying on it for recovery. It does not recreate arbitrary editor/TUI process state. |
| Pane logging | `mox log start PATH`, `stop`, `save PATH [--full]`, `clear [--yes]` | Paths are caller-supplied. Clear prompts unless `--yes` is passed. A pre-existing non-Mox pipe is preserved and causes start to report an error. |
| LazyGit | `mox lazygit [--cwd PATH] [--window]` | Requires the LazyGit executable; popup/window execution is external. |
| Clipboard | `mox clipboard TEXT` | Uses the tmux buffer and available OSC52/system clipboard paths. Verify end-to-end delivery in the actual terminal/SSH client. |
| Navigator, Extract, Flash, Palette | `mox nav`, `mox extract`, `mox flash`, `mox palette` | Also reachable through Mox key bindings where configured. Interactive UI still needs real-terminal validation beyond state/render tests. |

## 3. Plugin-by-plugin migration gate

| Legacy plugin | Native path | Automated evidence | Before removing the plugin |
|---|---|---|---|
| **nord-tmux / mode indicator** | `mox init --apply` + `src/ui/theme.rs` / `src/ui/status.rs` | Generated status and keymap tests | Inspect actual colors and mode labels in the user's tmux/terminal, including copy mode. |
| **tmux-yank** | `src/clipboard.rs`, `mox clipboard`; Extract copy | `tests/clipboard_tests.rs` | Verify paste into both tmux and an external application for local, SSH, and OSC52 paths used. |
| **tmux-thumbs / tmux-copycat / tmux-jump** | Flash labels/matching and Extract entity picker | `test_flash_*`, `test_extract_*`, popup smoke covers `s/S`, `f/F`, blank rows, selection and cancellation | Validate any still-used customized regex/actions and target-terminal appearance. Smoke does not establish exact thumbs/copycat parity. |
| **extrakto / fzf-url** | `mox extract` shared Picker/Preview and copy/insert/editor/browser actions | scanner/model/state and preview/clipboard tests; popup smoke covers Extract copy and search | Exercise categories and actions you rely on, especially preview, relative file opening, URL and shell insertion. Not every action/category has end-to-end smoke coverage. |
| **tmux-open / tmux-open-nvim / fzf-open-files-nvim** | `mox editor open`, `mox editor files|git|rg`, shared URL opener | parser/bridge/file tests; native smoke covers Files → registered running Neovim RPC | `mox editor setup` is required. Confirm your own Neovim config and fallback URL opener behavior. |
| **tmux-sessionx** | Navigator for current tmux tree; `mox projects` for directories/repos | navigator/Projects tests; native smoke covers Projects search cancellation | Test actual project activation, roots and optional zoxide on representative data. Projects is not a clone of every sessionx fzf mode. |
| **tmux-resurrect / tmux-continuum** | `mox persist` plus init hooks | 4 persistence integration tests + 1 hooks test; linked-window restore and repeated explicit-path autosave/autorestore pass | Do not expect arbitrary foreground programs or internal editor state to resume; validate the topology and restore policy you depend on. |
| **vim-tmux-navigator** | `mox Navigate` + Neovim bridge | editor bridge tests; native smoke covers `Ctrl-h/l` within a Neovim vertical split and `Ctrl-h` across the tmux-pane boundary | Verify remaining directions, layouts, and multi-client/nested cases you use. |
| **tmux-suspend** | client-scoped `mox suspend` / resume state | native smoke covers local key forwarding/resume and table/status | Nested remote and multiple-client combinations in the user's topology remain to be tested. |
| **mighty-scroll** | generated Mox copy-mode scroll bindings | native smoke covers first wheel event for shell scroll position, `less`, `fzf`, and Neovim SGR | Test the applications, mouse-reporting modes and nested/remote sessions you use. |
| **tmux-logging** | `mox log` | `tests/logging_lazygit_tests.rs` isolated tmux lifecycle | Verify output path/permissions and full-history capture size on the target server. |
| **tmux-neolazygit** | `mox lazygit` | `tests/logging_lazygit_tests.rs` command integration | Launch LazyGit interactively and confirm popup/window cwd and exit behavior. |
| **tmux-fzf / tmux-menus** | shared Palette and which-key | palette/rendering/keymap tests | Confirm all desired dynamic sources and destructive-action prompts interactively. |

## 4. Safe transition

1. Keep a copy of the existing tmux configuration and note local plugin options/bindings.
2. Build/install `mox` and test its init script on a disposable tmux server or separate socket.
3. Enable only the feature integrations you use: Neovim setup, project roots, persistence policy, and optional system clipboard/open tools.
4. Exercise each workflow in the migration table. Automated parser, reducer, and generated-command tests are useful evidence but are not a substitute for a live terminal where the table calls for one.
5. Remove only the TPM plugin entries whose replacement you have exercised. Keep a plugin while an unverified interactive workflow is still a dependency.

Rollback by restoring the saved tmux configuration, sourcing the previous TPM loader, and reloading tmux. The repository's compatibility loader may remain available, but a native install should not require it.
