# Часть 1 — Gemini Flash 3.8: объёмная работа с простой логикой

Цель общего проекта: один бинарник `mox`, общий core и UI toolkit, независимые features, сохранение всех workflows из исходного задания после удаления TPM. Эта часть готовит общий слой отображения и материалы для второй части. Она не означает, что плагины уже можно удалять.

## Исходное состояние

Изучить исходники и тесты перед изменениями. В текущем checkout уже есть:

- `src/tmux/{client,config,scroll}.rs`: выполнение tmux-команд, генерация init/bindings, copy-mode и mouse wheel bindings.
- `src/keymap/{types,defaults}.rs`, `src/ui/{runner,view}.rs`: дерево действий, modal/which-key и confirm.
- `src/nav/{model,state,fuzzy,runner,view}.rs`: session/window/pane tree, fuzzy search, действия и pane preview.
- `src/extract/{model,scanner,ui}.rs`: классификация сущностей, scanner, категоризированный picker, copy/insert/open.
- `src/flash/*`: jump, quick-yank/open, remote operations, character motions, labels и overlay.
- `src/agent/*`: создание и переключение agent windows.
- `src/tui/*`: управление terminal/raw mode; `src/tea/*`: существующая функция, которую нужно сохранить.

Не создавать заново Navigator, Flash, Extract, agent manager, terminal guard или tmux client. Текущие папки не нужно массово переименовывать ради совпадения с концептуальной схемой `core/ui/features`.

## 1. Инвентаризация совместимости

Подготовить одну таблицу: plugin → workflow → существующая функция Mox → недостающая функция → тест/критерий проверки.

Покрыть все плагины исходного задания: resurrect, continuum, open/open-nvim/fzf-open-files-nvim, sessionx, vim-tmux-navigator, suspend, mighty-scroll, logging, yank/copycat/thumbs/extrakto/fzf-url/jump, fzf/menus, Nord/mode-indicator, neolazygit.

Изучить доступную текущую конфигурацию и соответствующие plugin sources/docs. Записывать, что проверено по исходникам, а что пока является требованием. Если точный TPM-блок или источник недоступен, явно отметить пробел; не придумывать текущие bindings. Доступность и версии внешних источников должен проверять исполнитель при их использовании.

Сохранить baseline bindings, CLI и сценариев. Отдельно различать `Navigator` — существующее дерево tmux — и `Projects` — открытие рабочих директорий.

## 2. Theme и общий слой отображения

Работать в `src/ui/`, используя имеющийся crossterm. Не добавлять TUI framework или обязательный `fzf`.

- Ввести встроенную Nord theme и расширяемый `Theme`: семантические роли для фона, текста, выделения, границ, ошибок, предупреждений, success, hints, entity badges и mode colors.
- Все значения цветов хранить в одном владельце theme. Не раскладывать палитру по features.
- Реализовать общий rendering для list rows, выделенной строки, viewport, рамок, tree indentation, preview pane, prompt, confirm, hints, toast и status badge.
- Обрабатывать пустые списки, узкий терминал и resize. Не копировать целые модели ради отображения; использовать заимствованные данные.
- Использовать существующий terminal guard. Граница terminal session, popup lifecycle и event loop остаётся второй части.

**Контракт общего UI:** renderer получает данные строки/дерева, query, selection, viewport, preview text, message и Theme; рисует их, но не вызывает tmux, не запускает процессы, не открывает файлы и не выбирает domain action. Tree использует тот же list/selection/preview rendering с глубиной и признаком раскрытия строки. Features предоставляют содержимое и действия через свои adapters.

В этой части допускаются только реальные общие структуры представления. Не создавать собственный action registry, универсальный global context или второй fuzzy engine. Логику фильтрации, выбора, обновления preview и reducer/event loop унифицирует часть 2.

## 3. Механическая миграция существующих экранов

Перевести rendering Navigator, Extract и which-key на общий слой. Flash использует общий Theme, hints/status/toast, сохраняя специализированное отображение labels поверх текста.

- Сохранить текущие клавиши, действия, поиск, collapse/expand, confirm, rename и copy/insert/open.
- Перенести форматирование, рамки, цвета, footer и сообщения, а не переизобретать модели features.
- Удалить заменённые rendering helpers; не оставлять два renderer для одного экрана.
- Существующие циклы и feature states могут сохраняться до их объединения во второй части. Перечислить оставшиеся места в handoff; не выдавать эту промежуточную миграцию за завершённый общий Picker.
- Где уже доступно локальное status message, использовать общий toast. Перевод внешних уведомлений и изменение dispatch оставить части 2.

Не менять здесь `open_target`, `open_in_editor`, clipboard backends, Flash coordinates или генерацию tmux bindings: это логические задачи второй части.

## 4. Данные для status и palette

- Подготовить presentation labels и theme roles для `NORMAL`, `PREFIX`, `COPY`, `VISUAL`, `RESIZE`, `SYNC`, `SUSPEND`, `AGENT`.
- Status renderer получает готовый mode. Определение состояния, приоритет одновременных режимов, client/pane scopes и tmux formats принадлежат части 2.
- Which-key продолжает получать действия из существующего KeyNode. Не заводить отдельный список команд для будущей palette.
- Подготовить отображение palette entries через общий List/Preview; регистрация, поиск по источникам и dispatch принадлежат части 2.

## 5. Тестовые данные и документация

Подготовить fixtures с явно указанными ожидаемыми результатами:

- все запрошенные сущности: URL, path, file:line:column, git hash, IP, command, word, Docker image, Kubernetes resource, hex color, IPFS CID;
- пробелы и кавычки в путях, Unicode, punctuation, пересекающиеся распознавания, длинные строки, пустой capture;
- Neovim panes: 0/1/несколько, разные cwd; неоднозначные paths;
- tmux snapshots: несколько sessions/windows/panes, active state, zoom и layout;
- process classifications и комбинации status states.

Это входные данные и ожидаемое поведение, а не реализация новых парсеров. Не добавлять постоянно падающие tests к ещё отсутствующим API. Тесты нового rendering и сохранения действующих UI behavior должны выполняться уже в части 1.

Подготовить migration checklist, минимальный пример конфигурации и документацию общего UI. Целевую установку с `run-shell 'mox init --apply'` описать как целевую до её проверки во второй части. TPM/plugin.tmux пока не удалять.

## Границы ответственности

Gemini владеет presentation modules в `src/ui/`, theme, локальными rendering изменениями в существующих features, UI tests, fixtures и документацией. Не изменять core execution/state/hooks/keymap semantics, feature algorithms, persistence, Neovim transport или process routing.

Если перенос rendering требует сложного изменения event/state flow, записать конкретную зависимость для части 2. Не обходить её новым локальным picker или registry.

## PERFORMANCE CONTRACT

- Чувствительные пути: rendering, поиск Navigator/Extract, pane capture и обработка большой history.
- Required: rendering не запускает subprocesses и не читает history целиком; не добавлять cloning модели на каждой перерисовке.
- Candidate: общий viewport и повторное использование подготовленных строк; внедрять по измерениям, не добавлять cache infrastructure заранее.
- Снять baseline и сравнить UI rendering на одинаковых fixtures и terminal sizes. Существующие затраты поиска зафиксировать отдельно; оптимизация алгоритма принадлежит части 2.
- Не заявлять ускорение без измерений. Передать все найденные performance issues и кандидаты в Task Packet.

## Проверка и передача

Следовать `AGENTS.md`, `.agent/MULTI_AGENT.md` и применимым shared rules. Передавать конкретные ownership, Task Packet и результаты между ролями; независимый reviewer не утверждает собственный patch.

Выполнить focused UI checks, затем `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` через RTK. Popup smoke проверить, если доступны tmux и Python. Не считать skipped checks успешными.

Передать части 2: diff/commits, список новых UI API, таблицу parity, fixtures, подтверждённые проверки, известные ограничения и PERFORMANCE CONTRACT. Результат должен собираться и сохранять прежние workflows; замена всего TPM проверяется только в части 2.
