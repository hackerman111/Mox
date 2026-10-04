# Part 2 validation evidence

This note records the latest reported verification for this checkout. It distinguishes automated and isolated-tmux smoke evidence from untested user configurations and broader plugin parity. Performance measurements below are workload-specific and are not cross-machine comparisons.

## Verification status

- `rtk cargo test --workspace --offline`: **passed**, 124 tests across 28 suites, 0.83 seconds aggregate test time.
- `rtk cargo fmt --all -- --check`: **passed**.
- `rtk cargo check --workspace --all-targets`: **passed**.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings`: **passed**, no warnings/errors.
- `rtk cargo build --release`: **passed**.
- `tests/tmux_popup_smoke.py`: **passed**. Covered Flash `s/S`, `f/F`, blank-row handling, selection, cancellation, Extract copy, and search.
- `tests/native_tmux_smoke.py`: **passed**. Covered suspend key forwarding and resume with `C-Space`/table/status, palette filtering, Projects search cancellation, Files → registered running Neovim RPC, wheel handling for shell scroll position, `less`, `fzf`, and Neovim SGR mouse reporting, plus Neovim vertical-split navigation (`Ctrl-h/l` within Neovim and `Ctrl-h` across the boundary to a neighboring tmux pane).
- Persistence coverage: four tests in `tests/persistence_integration_tests.rs` and one in `tests/persistence_hooks_tests.rs` passed. This includes isolated-server linked-window restore and explicit-path autosave/autorestore checks for snapshot mtime and repeated-restore idempotency.

The tested Neovim vertical-split smart navigation path is covered by the extended smoke. Agent workflows remain outside this smoke coverage. These results do not establish compatibility with an arbitrary personal `.tmux.conf`, all terminal emulators, every remote/nested setup, or complete behavior parity with every plugin customization.

## Focused automated coverage inventory

| Surface | Evidence | What it establishes | Remaining limits |
|---|---|---|---|
| Shared picker/render/preview | `tests/ui_rendering_tests.rs`, `tests/preview_tests.rs`, `tests/native_core_tests.rs` | Shared rendering primitives, picker selection/refilter behavior, bounded local preview, FIFO rejection and worker lifecycle cases | Does not cover every screen size, every preview provider, or stalled remote/FUSE kernel I/O. |
| Extract scanner/categories | `tests/test_extract_scanner.rs`, `tests/test_extract_model.rs`, `tests/test_extract_ui.rs`, `tests/fixtures_tests.rs`; popup smoke | Classification/precedence, coordinates and model labels, category/filter state, fixture behavior; selected real popup actions/search | Parser/state tests alone are not GUI parity evidence. Smoke is a bounded set of terminal workflows, not every category or custom Extrakto action. |
| Flash/modal | `tests/test_flash_labeler.rs`, `tests/test_flash_overlay.rs`, `tests/test_flash_text_object.rs`, `tests/keymap_tests.rs`; popup smoke | Label assignment, text-object spans, generated bindings, and listed Flash popup scenarios | Does not establish all visual layouts, terminal fonts/colors, or custom thumbs regex/label parity. |
| Editor bridge | `tests/editor_bridge_tests.rs`, `tests/editor_files_tests.rs`, `tests/native_core_tests.rs`, native tmux smoke | Location parsing, file source parsing, registered Neovim RPC flow in the smoke environment | Requires explicit `mox editor setup`/registration. Does not guarantee arbitrary user Neovim configs or all multi-editor/edge-window cases. |
| Projects/palette | `tests/projects_tests.rs`, UI/keymap tests, native tmux smoke | Root/repository/session parsing and the tested palette filtering / Projects cancellation paths | Does not establish performance for arbitrary huge roots or all zoxide/environment configurations. |
| Persistence | `tests/persistence_integration_tests.rs`, `tests/persistence_hooks_tests.rs`, `tests/native_core_tests.rs` | Snapshot validation/atomic write; isolated restore, linked windows, explicit-path autosave mtime behavior, autorestore idempotency | Does not resume arbitrary TUI/editor process state or prove safety under every live server topology. |
| Logging/LazyGit | `tests/logging_lazygit_tests.rs` | Isolated tmux logging lifecycle, external-pipe preservation, capture/clear, LazyGit launch/error paths | Does not prove interactive LazyGit behavior on every terminal or mounted filesystem. |
| Clipboard | `tests/clipboard_tests.rs` plus popup smoke Extract copy | tmux buffer transfer, URL scheme validation, and selected popup copy flow | External clipboard delivery remains terminal/client/platform dependent; verify the path actually used. |
| Suspend/scroll/navigation | native tmux smoke plus keymap/integration tests | Listed local suspend forwarding/status, shell/less/fzf/Neovim wheel scenarios, and Neovim vertical-split navigation across an internal window and tmux-pane boundary | Nested remote/multi-client combinations and all applications remain unverified. |

## Release-workload measurements

Reported measurements from the release-mode example/native workload:

| Workload | Measurement | Scope |
|---|---:|---|
| Shared fuzzy filtering | Five runs, 10,000 rows × 100 filters: median **39.749 ms total**, or **0.39749 ms/filter** | One recorded workload; not a comparison against every plugin or other hardware. |
| Scanner | 2,000 lines: median **7.195 ms**, producing 15,990 tokens | One recorded fixture/workload; no claim about all terminal scrollback contents. |
| Full-history export | 200,000 rows, **7,400,001 bytes** in **53.500 ms**; first and last markers asserted | One isolated release-mode export workload; excludes other machines and filesystem conditions. |
| Peak resident memory | `/proc/self/status` `VmHWM`: **7,584 KiB** | Single process/run; platform-specific measure. |

These values are recorded as observations, not performance guarantees. They do not substitute for measuring different row counts, terminal workloads, machines, or end-to-end popup latency.

## Operational caveats

- The tested install snippet and native smoke exercise Mox against isolated tmux setups; the user's complete TPM block and plugin-specific customizations have not been certified as equivalent.
- `mox editor setup` remains opt-in. Smoke success applies to the registered Neovim instance prepared by the test, not every runtime configuration.
- Clipboard system delivery varies with OSC52 support, SSH client, terminal, compositor and installed fallback. The tmux buffer path is separately covered.
- Scanner tests are parser evidence; the popup smoke adds selected GUI evidence, while unusual patterns and customization parity remain outside its scope.
- Remote/FUSE filesystems can still stall in kernel I/O even when preview subprocess deadlines and byte caps are enforced.
- Agent workflows remain outside this verification snapshot until their tests are added and run.
