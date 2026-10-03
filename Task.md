# tmux-nvim

## Задача

Сделать небольшой tmux-инструмент на Rust, который переносит Vim-подобную модель управления на уровень tmux.

Основной UX:

**нажал одну клавишу входа → дальше управляешь tmux через Vim-подобные последовательности → после каждого префикса видишь доступные продолжения и не обязан помнить все хоткеи.**

Первая версия должна заменить связку `tmux-modal + tmux-which-key` и добавить нормальный keyboard scrolling. Внутри также должен жить небольшой tea timer как пасхалка.

Не строить универсальный terminal framework. Не делать daemon, plugin SDK, сложную event architecture или десятки абстракций заранее. Проект должен оставаться маленьким, быстрым и понятным.

## Что изучить перед реализацией

В первую очередь изучить исходники, конфиги и поведение:

- `whame/tmux-modal`
- `alexwforsythe/tmux-which-key`
- существующую реализацию моего tea timer, если она есть в репозитории или доступных файлах

От `tmux-modal` нужна **полная функциональная совместимость с его default keymap**, включая nested sequences, sticky window mode, sticky resize mode, session/window/pane operations и go-to commands.

Не переписывать его shell-код построчно. Сначала получить полный набор `{sequence → command → description}`, затем реализовать ту же модель нормально.

От `tmux-which-key` взять идею discoverable keymaps: после `w`, `g`, `s`, `w s` и других промежуточных последовательностей пользователь сразу видит допустимые следующие клавиши и их действия. Меню должно поддерживать вложенные и sticky состояния.

После первой версии изучить как референсы:

`schasse/tmux-jump` → Easymotion-подобные прыжки по содержимому pane.

`fcsonline/tmux-thumbs` → поиск паттернов в содержимом pane и короткие hint labels.

`laktak/extrakto` → извлечение URL, путей, строк и другого текста из `capture-pane`.

На первой итерации эти три механизма **не реализовывать**. Архитектура лишь не должна мешать добавить их позже.

## Архитектура

Использовать один Rust binary и минимальный `.tmux` loader.

```text
tmux
  │
  ├── plugin.tmux
  │      └── загрузка binary + регистрация entry binding
  │
  └── tmux-nvim
         ├── keymap      modal key tree
         ├── ui          which-key / popup
         ├── tmux        выполнение tmux commands
         ├── scroll      Vim-style scroll/copy interaction
         └── tea         tea timer
```

Не нужен постоянно работающий процесс.

**Keymap должен быть единственным источником истины.** Из одного дерева должны получаться и выполняемые команды, и подписи which-key. Не создавать отдельно tmux bindings и отдельно описание меню, которые затем начинают расходиться.

Логическая модель узла:

```rust
KeyNode {
    key,
    label,
    action,
    children,
    sticky,
}
```

Это описание структуры, а не требование буквально использовать такой `struct`.

Tmux остается исполнителем операций с pane/window/session. Rust не должен пытаться реализовывать собственный multiplexer.

Для связи использовать обычный tmux CLI и возможности tmux `key-table`, `display-popup`, `display-menu`, `copy-mode` и `capture-pane` там, где они подходят.

Предпочтение отдать native tmux mechanisms перед эмуляцией терминала внутри Rust.

Runtime-зависимости первой версии: **tmux + один Rust binary**. Python, Ruby, Node, fzf и отдельные background services не нужны.

Минимальная структура репозитория:

```text
Cargo.toml
plugin.tmux
src/
  main.rs
  keymap.rs
  tmux.rs
  ui.rs
  scroll.rs
  tea.rs
tests/
```

Дробить дальше только когда файл действительно получил отдельную ответственность.

## Поведение v0.1

### Modal UX

Оставить `M-m` стандартной точкой входа, как в `tmux-modal`, если пользователь не переопределил ее.

После входа появляется минимальный индикатор modal mode.

Все default bindings текущего `tmux-modal` должны работать. Не переносить список вручную из этого ТЗ: получить его непосредственно из актуальных `keybindings.conf`, `commands.conf` и README upstream.

Для проверки создать небольшой `docs/tmux-modal-parity.md`:

```text
sequence | upstream action | tmux-nvim action | status
```

Он нужен для контроля полноты, а не как большая документация.

`q` и `Esc` должны предсказуемо выходить из текущего submenu/sticky mode. Повторный `M-m` выходит из modal mode полностью.

Опасные операции вроде `kill-pane`, `kill-window`, `kill-session` должны поддерживать confirmation без специального hardcoded исключения в UI.

### Which-key

Если введенная последовательность еще не завершена, показать доступные продолжения.

Пример:

```text
w

h  pane left       l  pane right
j  pane down       k  pane up
c  new window      d  kill pane
s  split…          r  resize…
w  window mode
```

Меню должно показывать только действия текущего узла дерева.

UI должен быть компактным, без полноэкранного TUI. Ориентироваться на аккуратный tmux popup/menu с нормальными padding, border и выделением текущего prefix.

Не добавлять анимации.

Цвета должны иметь разумные defaults и возможность переопределения. Не завязывать layout на конкретную Nerd Font.

Sticky mode визуально отличать от обычной незавершенной последовательности.

### Scroll

Нужно уметь читать scrollback только с клавиатуры так же естественно, как Vim buffer.

Использовать tmux copy-mode, а не собственное хранение history.

Минимально должны нормально работать Vim-привычные:

```text
j / k
Ctrl-d / Ctrl-u
Ctrl-f / Ctrl-b
gg / G
```

Переход в scroll/copy interaction и выход из него не должны требовать постоянного использования tmux prefix.

Не ломать обычный mouse scrolling, если mouse support включен в tmux.

### Tea timer

Tea timer встроить как небольшой внутренний модуль, а не как отдельный tmux plugin.

Он должен поддерживать последовательность раундов чайной церемонии: прогрев/промывку и последовательные проливы, ручной переход к следующему раунду, start/pause/reset и настраиваемые интервалы.

Интерфейс должен занимать небольшой popup или участок status line. После окончания интервала достаточно визуального сигнала и terminal bell; тяжелая notification subsystem не нужна.

Tea timer не должен занимать существующий binding из `tmux-modal`. Добавить его как отдельное действие в discoverable keymap.

## Задел на следующую версию

Следующая задача после стабильного modal UX:

```text
capture-pane
    ↓
token/pattern detection
    ↓
hint labels
    ↓
jump / yank / open / insert
```

Это позволит позже объединить идеи `tmux-jump`, `tmux-thumbs` и `extrakto`.

Нужны будут:

- подсветка URL, file paths, hashes и других полезных tokens;
- hint selection в стиле Vimium/Easymotion/Flash/Leap;
- неявный поиск по видимому содержимому;
- переход к совпадению в copy-mode;
- yank выбранного значения.

Не проектировать эти API подробно сейчас. Достаточно не смешивать keymap/UI код с логикой анализа содержимого pane.

## Проверка

После изменений агент обязан запускать:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Для интеграционных тестов использовать отдельный tmux server, например:

```bash
tmux -L tmux-nvim-test -f /dev/null ...
```

Минимально тестировать key tree, ambiguous/nested sequences, sticky mode, выход по `q/Esc` и соответствие sequence → tmux command.

## Готовность первой версии

Работа считается законченной, когда:

1. TPM/manual installation загружает plugin без дополнительного runtime.
2. В modal mode работают все default команды `tmux-modal`.
3. После промежуточной клавиши which-key показывает правильные допустимые продолжения.
4. Sticky window/resize modes работают и отображаются корректно.
5. Scrollback полностью управляется с клавиатуры через Vim motions.
6. Tea timer работает как встроенная функция.
7. `fmt`, `clippy` и tests проходят.
8. В hot path нет Python/Ruby/fzf и нет фонового daemon.

Если во время реализации приходится выбирать между архитектурной универсальностью и простым решением для текущего поведения, выбирать простое решение.

Перед кодированием не писать многостраничный implementation plan. Сначала изучить указанные репозитории, составить короткую parity map для `tmux-modal`, после чего сделать работающий vertical slice: **вход в modal mode → nested keymap → which-key → выполнение tmux command**. Затем довести parity, scrolling и tea timer.
