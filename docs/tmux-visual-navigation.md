# Visual navigation

After rebuilding, run `bash plugin.tmux` in tmux to reload bindings and styles.

| Mode | Keys |
| --- | --- |
| Modal | `s`: Flash in current pane; `S`: Flash across panes; `e`: Extract; `v` or `[`: copy mode |
| Quick Entity Labels | `y f`: copy visible entity; `y h`: copy hash; `g f`: open path; `g x`: open URL |
| Flash | Type search, then `Enter` and label; `Esc` backs out; `Ctrl-C` exits |
| Copy mode | `v`: begin selection; `f/F`: choose forward/backward character on current line; `R`: remote yank; `q/Esc`: exit |
| Extract | Starts in NORMAL: `j/k` or arrows select; `y` copies; `Enter` pastes; `gf/gx` open; `Tab` changes category |
| Extract search | `/` or `i` starts search; `Esc` returns to NORMAL; `q/Esc` in NORMAL or `Ctrl-C` exits |
| Navigator | `r` renames selected session, window, or pane title |

Copy mode shows `◈ COPY`; an active selection shows `◈ VISUAL`. Selection,
search matches, the current match and the scroll position use distinct colors.
Flash respects the displayed scrollback viewport and preserves an existing selection.

Regression checks:

```sh
cargo test --workspace
cargo build
python3 tests/tmux_popup_smoke.py
```

The popup check uses a private tmux server and a fake clipboard daemon. It does
not modify existing sessions or the system clipboard. It needs Python 3 and tmux
with mode-screen capture (`capture-pane -M`) and copy-mode styling support;
verified with tmux 3.7c.
