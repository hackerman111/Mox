# tmux-modal Parity Map

| sequence | upstream action | moch action | status |
|---|---|---|---|
| `<prefix>` (M-m) | `set-option key-table ktm-cmd` | Enter modal mode (`moch` key-table) | Done |
| `<root> M-m` | `set-option key-table root` | Exit modal mode | Done |
| `<root> q` / `Escape` | Exit modal | Exit modal mode | Done |
| `<root> i` | Exit modal (insert) | Exit modal mode | Done |
| `<root> :` | `command-prompt` | `command-prompt` | Done |
| `<root> c` | `copy-mode` | `copy-mode` (Vim scrollback) | Done |
| `<root> y` | `paste-buffer` | `paste-buffer` | Done |
| `<root> ?` | help / list-keys | Which-key popup (root) | Done |
| `<root> T` | *(moch extension)* | Built-in Tea Timer | Done |
| `<root> g` | `switch-client -T ktm-goto` | Go-to submenu + Which-key popup | Done |
| `<root> s` | `switch-client -T ktm-session` | Session submenu + Which-key popup | Done |
| `<root> w` | `set-option key-table ktm-window-pane` | Sticky Window/Pane mode + Which-key popup | Done |
| `w h` | `select-pane -L` | `select-pane -L` | Done |
| `w j` | `select-pane -D` | `select-pane -D` | Done |
| `w k` | `select-pane -U` | `select-pane -U` | Done |
| `w l` | `select-pane -R` | `select-pane -R` | Done |
| `w H` | `select-window -t :-` | `select-window -t :-` (previous window) | Done |
| `w L` | `select-window -t :+` | `select-window -t :+` (next window) | Done |
| `w 0`..`9` | `select-window -t :0`..`:9` | `select-window -t :N` | Done |
| `w c` | `new-window` | `new-window` | Done |
| `w d` | `kill-pane` (confirm) | `confirm-before -p "kill-pane #P? (y/n)" kill-pane` | Done |
| `w D` | `kill-window` (confirm) | `confirm-before -p "kill-window #W? (y/n)" kill-window` | Done |
| `w b` | `break-pane` | `break-pane` | Done |
| `w n` | `display-panes` | `display-panes` | Done |
| `w o` | `last-window` | `last-window` | Done |
| `w z` | `resize-pane -Z` | `resize-pane -Z` | Done |
| `w ,` | `command-prompt -I "#W" "rename-window -- '%%'"` | Rename window prompt | Done |
| `w i` | `command-prompt -p index "select-window -t ':%%'"` | Select window by index prompt | Done |
| `w t` | `choose-tree -Zw` | `choose-tree -Zw` | Done |
| `w a` | `switch-client -T ktm-window-arrange` | Arrange submenu + Which-key popup | Done |
| `w m` | `switch-client -T ktm-window-move` | Move pane submenu + Which-key popup | Done |
| `w r` | `set-option key-table ktm-window-resize` | Sticky Resize mode + Which-key popup | Done |
| `w s` | `switch-client -T ktm-window-split` | Split submenu + Which-key popup | Done |
| `w q` / `Escape` | `set-option key-table ktm-cmd` | Return to root modal mode | Done |
| `w M-m` | `set-option key-table root` | Exit modal mode | Done |
| `w s j` | `split-window` | `split-window` (horizontal) | Done |
| `w s l` | `split-window -h` | `split-window -h` (vertical) | Done |
| `w s f` | `split-window -f` | `split-window -f` (full horizontal) | Done |
| `w s F` | `split-window -f -h` | `split-window -f -h` (full vertical) | Done |
| `w a 1` | `select-layout even-horizontal` | `select-layout even-horizontal` | Done |
| `w a 2` | `select-layout even-vertical` | `select-layout even-vertical` | Done |
| `w a 3` | `select-layout main-horizontal` | `select-layout main-horizontal` | Done |
| `w a 4` | `select-layout main-vertical` | `select-layout main-vertical` | Done |
| `w m j` | `swap-pane -D` | `swap-pane -D` | Done |
| `w m k` | `swap-pane -U` | `swap-pane -U` | Done |
| `w r h` | `resize-pane -L` | `resize-pane -L 1` | Done |
| `w r j` | `resize-pane -D` | `resize-pane -D 1` | Done |
| `w r k` | `resize-pane -U` | `resize-pane -U 1` | Done |
| `w r l` | `resize-pane -R` | `resize-pane -R 1` | Done |
| `w r H` | `resize-pane -L 5` | `resize-pane -L 5` | Done |
| `w r J` | `resize-pane -D 5` | `resize-pane -D 5` | Done |
| `w r K` | `resize-pane -U 5` | `resize-pane -U 5` | Done |
| `w r L` | `resize-pane -R 5` | `resize-pane -R 5` | Done |
| `w r q` / `Escape` | `set-option key-table ktm-window-pane` | Return to window mode | Done |
| `s ,` | `command-prompt -I "#S" "rename-session '%%'"` | Rename session prompt | Done |
| `s D` | `kill-session` (confirm) | `confirm-before -p "kill-session #S? (y/n)" kill-session` | Done |
| `s d` | `detach-client` | `detach-client` | Done |
| `s h` | `switch-client -p` | `switch-client -p` (previous session) | Done |
| `s l` | `switch-client -n` | `switch-client -n` (next session) | Done |
| `s t` | `choose-tree -Zs` | `choose-tree -Zs` | Done |
| `s c` / `s n` | `new-session` | `new-session` | Done |
| `g s` | `switch-client -T ktm-goto-session` | Goto session submenu | Done |
| `g s h` | `switch-client -p` | `switch-client -p` | Done |
| `g s l` | `switch-client -n` | `switch-client -n` | Done |
| `g s t` | `choose-tree -Zs` | `choose-tree -Zs` | Done |
| `g w` | `switch-client -T ktm-goto-window` | Goto window submenu | Done |
| `g w 0`..`9` | `select-window -t :0`..`:9` | `select-window -t :N` | Done |
| `g w h` | `select-window -t :-` | `select-window -t :-` | Done |
| `g w l` | `select-window -t :+` | `select-window -t :+` | Done |
| `g w i` | `command-prompt -p index "select-window -t ':%%'"` | Window index prompt | Done |
| `g w o` | `last-window` | `last-window` | Done |
| `g w t` | `choose-tree -Zw` | `Mox Navigator` | Done |
| `<root> a` | *(mox extension)* | AI Agent fast-toggle | Done |
| `<root> A` | *(mox extension)* | AI Agent window create | Done |
| `<root> t` | *(mox extension)* | Vim-like Fuzzy Tree Navigator | Done |
| `w s` / `w t` | `choose-tree -Zw` | Vim-like Fuzzy Tree Navigator | Done |
| `S s` / `S t` | `choose-tree -Zs` | Vim-like Fuzzy Tree Navigator | Done |
| `g s` / `g w` | `choose-tree` | Vim-like Fuzzy Tree Navigator | Done |

