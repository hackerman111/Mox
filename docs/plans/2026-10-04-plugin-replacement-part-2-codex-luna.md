# Часть 2 — Codex + Luna: общая логика и завершение замены TPM

Вход: собирающаяся часть 1, общий Theme/rendering, сохранённые текущие workflows, fixtures, parity table и handoff. Цель: завершить исходное задание полностью, без обязательного fzf/TPM/plugin.tmux и без дублирующих UI или tmux execution paths.

## Распределение между Codex и Luna

**Codex:** архитектурные контракты, сложные state machines и algorithms, tmux boundary, Picker integration, Editor Bridge, Persistence, process routing, общая интеграция и решение review findings.

**Luna:** исследование отдельного модуля, ограниченные implementations по зафиксированному контракту, fixtures/tests для парсинга и command generation, Projects sources, Logging/LazyGit orchestration и независимое review изменений Codex. Codex независимо проверяет implementations Luna. Для одного patch автор не выступает его независимым reviewer.

Работать по `.agent/MULTI_AGENT.md`: Planner → Implementer → Reviewer → Fixer → Reviewer. Роли можно выполнять последовательно двумя агентами. Не запускать параллельные изменения общих файлов: `main.rs`, `keymap`, `tmux/config`, registry и общей UI state machine. Для каждой задачи назначать владельца файлов; учитывать чужие изменения и не откатывать их.

## 1. Core boundary, Picker и единый источник действий

- Принять rendering части 1 и проверить отсутствие domain logic в UI.
- Расширить существующий `src/tmux/client.rs` в единственный tmux API, а не создавать конкурирующий клиент. Изолировать tmux execution, socket/target handling, capture, command generation и необходимое escaping.
- Удалить обходы API, включая самостоятельный shell dispatch в which-key. Внешние программы относятся к соответствующим feature/platform adapters; tmux side effects проходят через tmux API.
- Определить владельцев state/hooks/config. Учитывать scope server/session/window/pane/client, особенно для modal, suspend и status.
- Обобщить имеющийся fuzzy search и picker state: filtering, selection identity, viewport, empty results, resize, tree projection, preview lifecycle. Navigator сохраняет управление деревом, Extract — свои entity actions; общий UI не узнаёт о них.
- Pane/file/directory/command preview использует общий Preview. Ограничить объём чтения и длительность процессов, игнорировать устаревшие результаты. Command preview не должен автоматически выполнять произвольную выбранную команду.
- Определить единый action source на базе существующего keymap/action model. Which-key, palette, bindings и CLI adapters ссылаются на него; palette включает также небиндированные действия.
- Перевести Navigator/Extract/which-key на общую state/input инфраструктуру и удалить заменённые локальные picker paths.

Это фундамент. Его контракты должны быть готовы до реализации зависящих features, без массовой перестановки папок ради схемы из задания.

## 2. Editor Bridge и источники файлов

Codex реализует canonical `mox editor open file:line:column`:

- парсинг path/line/column, относительные paths относительно source pane cwd, безопасная передача filenames;
- обнаружение работающих Neovim panes и способа связи с ними;
- 1 editor — открыть там; несколько — общий picker; 0 — создать editor pane/window;
- минимальный Lua bridge, если нужен для регистрации Neovim endpoint и smart navigation;
- корректное поведение при закрытии выбранного pane/editor и ошибке транспорта.

Не полагаться только на process name и не отправлять Ex-команды в pane, который может оказаться shell. Не выполнять содержимое path как shell code.

Luna подключает file picker, git changed files, ripgrep results, `gf` и extract paths к этому API после его фиксации. Учитывать filenames с пробелами, Unicode, двоеточиями и multiline tool output; использовать структурированный/NUL-safe output, где программа его предоставляет. Удалить дублирование открывания файлов из Flash и Extract. URL opening имеет отдельного canonical owner.

## 3. Projects

Luna реализует sources и tests; Codex — правила объединения и интеграцию:

- existing sessions, recent cwd, git repositories, configurable roots, optional zoxide;
- deduplication директорий и repositories, bounded scanning, исключения и errors;
- existing session → switch; directory без session → create с cwd;
- collision-safe session naming, исчезнувшие cwd и одновременно созданная session;
- общий Picker/Preview и отдельная ответственность от Navigator.

Git и zoxide остаются внешними инструментами. Отсутствие zoxide не ломает Projects. Не сканировать файловую систему целиком при каждом вводе символа.

## 4. Persistence

Codex владеет snapshot/schema, restore algorithm и autosave lifecycle. Luna готовит независимые tests и проверяет восстановление.

- Сохранять sessions, windows, panes, cwd, layout, active window/pane, zoom и доступные сведения о foreground command.
- Versioned format, atomic save, проверка повреждённого snapshot, сохранение последнего корректного состояния.
- Restore сопоставляет старые и новые IDs; не использовать сохранённые IDs как действующие targets.
- Определить merge/conflict policy с существующим сервером, порядок восстановления layout/active/zoom и поведение при частичной ошибке. Не уничтожать рабочие sessions для удобства восстановления.
- Foreground command запускать только по безопасной явной policy. Имя `pane_current_command` не является полным argv; неподтверждённые команды не реконструировать догадками.
- Autosave/autorestore: идемпотентная установка hooks, предотвращение параллельных saves/restores, первая/повторная загрузка, shutdown и empty-server behavior.
- Выбрать lifecycle из возможностей tmux и необходимости периодического autosave. Если нужен worker, он запускается тем же `mox`, имеет одного владельца и проверяемое завершение; отдельный бинарник/service не обязателен.

Не обещать восстановление внутреннего состояния Neovim, произвольного TUI или процессов: восстановление безопасных команд и topology — предмет этой функции.

## 5. Smart navigation, Suspend и process-aware scroll

Codex реализует общую process/context classification и маршрутизацию; Luna — таблицы правил и edge-case tests.

- `C-h/j/k/l`: внутри Neovim переместиться по его windows; на границе перейти в соседний tmux pane. Из tmux pane переходить обычным способом. Lua bridge вызывает `mox`, без циклического dispatch.
- Suspend/resume действует на конкретного local client для nested/remote tmux; сохранить способ resume и корректно восстановить input/mouse/status. Один client не должен менять режим остальных.
- Scroll: shell → history/copy mode; Neovim → mouse; fzf/man/less → line/page; other TUI → configurable rules.
- Проверять foreground process/context, copy mode, mouse reporting и remote/nested поведение. Не считать `pane_current_command` достаточным для всех случаев.
- Расширить существующий copy-mode implementation, сохранить visual/yank/Flash bindings.

Прямой вызов `mox` на каждое mouse event может быть дорогим. Измерить стоимость и выбрать tmux format predicates/bindings либо иной ограниченный механизм там, где это сохраняет корректность.

## 6. Logging и LazyGit

Luna реализует по core contracts; Codex проверяет filesystem/process lifecycle и tmux integration.

Logging:

- start/stop live pane logging;
- save visible pane и complete history;
- clear history с общим confirm;
- понятные ошибки, безопасные filenames, сохранение raw capture;
- stop/shutdown без потерянного writer; повторный start и существующий pipe-pane не должны незаметно перехватывать чужой logger;
- complete history не требует лишних копий всего capture в памяти.

LazyGit:

- найти существующий подходящий lazygit и focus;
- иначе открыть popup/window в нужном cwd;
- документировать scope поиска и выбор при нескольких instances;
- использовать общий popup lifecycle и process detection. Не реализовывать интерфейс LazyGit заново.

Не добавлять shell scripts для логики этих features. Необходимые tmux command strings собираются безопасно через общий слой.

## 7. Extraction и Clipboard

Codex определяет classification/action contracts; Luna реализует ограниченные recognizers и tests после фиксации правил.

- Сохранить существующий scanner и расширить его: word, Docker image, Kubernetes resource, hex color, IPFS CID; проверить полноту URL/path/location/hash/IP/command.
- Установить precedence для пересекающихся сущностей и правила неоднозначности; не классифицировать любую строку как git hash/resource без контекста.
- Сохранить screen coordinates, Unicode correctness и raw/clean text. Правила распознавания совместно используются Flash и Extract.
- Actions: copy, insert, URL open, file open через Editor Bridge, jump через существующий Flash/copy mode.
- Один Clipboard owner: tmux buffer/OSC52 в приоритете; wl-copy/xclip/pbcopy — fallback. Проверять успех backend, не считать успешный spawn успешным copy.
- Учесть remote/nested tmux, большой payload, закрытие popup и clipboard subprocesses, которые могут удерживать PTY.

Не оставлять clipboard/open implementations в UI feature files после их замены.

## 8. Command palette, Status и установка

- Searchable palette: Mox actions, tmux commands, keybindings, buffers, processes, projects, sessions/windows/panes через общий Picker/Preview и единый action source.
- Dynamic sources предоставляют entries; регистрация static actions и which-key общая. Raw tmux commands и destructive actions проходят boundary validation/confirm.
- Единый Status получает реальные `NORMAL/PREFIX/COPY/VISUAL/RESIZE/SYNC/SUSPEND/AGENT` states. Зафиксировать scopes и приоритет; Theme применяется и к TUI, и к tmux status/copy-mode styles.
- Toast/confirm используется во всех workflows. `display-message -p` как запрос tmux metadata не является пользовательским уведомлением и не заменяется toast.
- `mox init --apply` устанавливает всё без loader, идемпотентно: bindings/hooks/status не размножаются; пользовательские настройки и прежние Mox bindings сохраняются, если замена не требуется.
- Основной путь установки — бинарник в PATH. `plugin.tmux` может оставаться совместимым необязательным loader; установка не зависит от него.
- Только после проверки parity удалить ставшие лишними plugin bindings/config. Не удалять TPM-блок из реальной пользовательской конфигурации по предположению о его содержимом.

## PERFORMANCE CONTRACT

- Required: UI не блокируется долгим preview, project scan или process query; bounded I/O, history capture и queues.
- Required: сохранение raw coordinates; отсутствие повторного capture всего scrollback на каждый key; отсутствие необязательных external clipboard calls после успешного preferred backend.
- Candidate: caching/invalidation snapshot и process classification, debounce preview, borrowed/index-based filtering вместо cloning дерева. Проверить реальные bottlenecks до реализации cache.
- Измерить picker/filter/preview latency и память на больших fixtures, complete-history export при `history-limit 200000`, scroll/navigation dispatch, save/restore на большом числе panes.
- Передать baseline части 1; каждую оптимизацию сохранить как candidate/validated/rejected/deferred с причиной. Не объявлять улучшения по одному наблюдению.

## Verification и итоговый gate

Перед каждым большим feature читать текущий owning code/callers/tests и соответствующие plugin sources/docs. Не переносить plugin implementation буквально.

Unit/integration coverage: parsers, reducer/state, action dispatch, tmux generation, escaping, IDs/layout restore, repeated init, errors и процессные lifecycle. Tmux integration использовать на отдельном сервере; включить Neovim transport, multiple clients и nested suspend/scroll сценарии. Проверить degraded operation без fzf/zoxide/system clipboard tools.

Через RTK выполнить focused checks и все применимые workspace checks: fmt, check all-targets, tests, clippy all-targets/all-features с `-D warnings`, popup smoke. Не выдавать пропущенный integration test за доказательство parity.

Итоговая acceptance matrix должна подтверждать: modal/which-key, tree navigation, Projects, file/git/rg → running Neovim, flash/jump, extract/copy/open, persistence, nested suspend, smart scroll, logging, LazyGit, status и palette. Затем проверить минимальную конфигурацию:

```tmux
set -g prefix C-Space
set -g mouse on
set -g history-limit 200000
run-shell 'mox init --apply'
```

Definition of Done: все workflows работают с одним `mox` и общим UI без TPM; нет заменённых дублирующих paths, unresolved BLOCKER/MAJOR findings или потерянных performance candidates. Если реальный TPM-блок недоступен, явно отделить подтверждённую функциональную parity от непроверенной персональной конфигурации.
