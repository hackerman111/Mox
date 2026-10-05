# Дизайн настраиваемости расширения Mox: конфигурация, темы, модульный статус-бар и часы

**Дата:** 2026-10-05  
**Статус:** Утверждён  
**Цель:** Создать обобщённую, гибкую систему конфигурации для Mox, которая полностью заменяет плагины `nord-tmux` и `tmux-mode-indicator` из `insp/`, объединяет стилизацию TUI и tmux, и даёт широкие возможности кастомизации тем, разделителей, индикаторов режимов и красивых часов.

---

## 1. Контекст и решаемые задачи

В каталоге `insp/` находятся два плагина:
1. `insp/tmux` (`nord-tmux`):
   - Оформление статус-бара, границ панелей, подсветки команд и часов в палитре Nord.
   - Опции: `@nord_tmux_show_status_content`, `@nord_tmux_no_patched_font`, `@nord_tmux_date_format`, 12h/24h часы.
   - Фиксированная разметка: `#S` слева, окна по центру/слева, дата/время/хост справа.
2. `insp/tmux-mode-indicator`:
   - Настройка текстовых меток (`WAIT`, `COPY`, `SYNC`, `TMUX`) и стилей для режимов tmux.
   - Подстановка в статус-бар через placeholder `#{tmux_mode_indicator}`.

В текущей версии Mox палитра Nord захардкожена в `src/ui/theme.rs`, а статус-бар генерируется в `src/tmux/config.rs` с фиксированным форматом.

### Требования к новой системе
- **Единая конфигурация**: Декларативный TOML-конфиг (`~/.config/mox/config.toml`) с поддержкой переопределений через tmux-опции (`@mox_*` и legacy `@nord_*`, `@mode_indicator_*`).
- **Обобщённые темы**: Поддержка не только Nord, но и других популярных тем (`catppuccin-mocha`, `tokyo-night`, `gruvbox`, `dracula`) и полностью кастомных палитр `[theme.custom]`.
- **Синхронизация**: Выбранная тема автоматически стилизует как tmux (статус-бар, границы, меню, всплывающие окна), так и внутренние TUI-окна Mox (Navigator, WhichKey, Extract, Flash, Tea).
- **Модульный статус-бар**: Генерация статус-бара из настраиваемых типизированных сегментов (`mode`, `session`, `windows`, `clock`, `host`, `custom`) с автоматическим сопряжением цветов разделителей.
- **Красивые часы**: Настройка форматов даты/времени, глифов Nerd Font (``, `󰥔`, `󰃭`), 12/24-часового формата и стилей отображения (единый блок или раздельные капсулы/pills).
- **Стили разделителей**: Поддержка стилей `powerline` ( ), `rounded` ( ), `slanted` ( ), `flat` (|) и `none`.
- **Индикатор режимов**: Полное покрытие всех 8 состояний Mox (`NORMAL`, `PREFIX`, `COPY`, `VISUAL`, `RESIZE`, `SYNC`, `SUSPEND`, `AGENT`) с быстрой оценкой в формате tmux и обратной совместимостью с `#{tmux_mode_indicator}`.

---

## 2. Архитектура и структура модулей

Новый функционал инкапсулируется в модуле `src/config/`:

```text
src/
└── config/
    ├── mod.rs          # Публичный API модуля, экспорт типов
    ├── model.rs        # Структуры данных конфигурации (serde-десериализация)
    ├── theme.rs        # Реестр тем (Nord, Catppuccin, Tokyo Night, Gruvbox, Dracula) и ColorSpec
    ├── status.rs       # Алгоритм генерации сегментов статус-бара и сопряжения разделителей
    └── loader.rs       # Поиск, чтение TOML, наложение оверрайдов из tmux-опций
```

Интеграция с существующими модулями:
- `src/ui/theme.rs`: адаптируется для создания рабочего `Theme` на основе `Config` / `ThemeConfig`.
- `src/tmux/config.rs`: использует `StatusConfig` и активный `Theme` для сборки скрипта инициализации tmux.
- `src/ui/status.rs`: генерирует строку формата tmux индикатора на основе `ModeIndicatorConfig`.
- `src/cli.rs` и `src/main.rs`: получают флаг `--config <path>` и команду `mox config default`.

---

## 3. Схема конфигурационного файла (`config.toml`)

```toml
# Выбор темы: "nord", "catppuccin-mocha", "tokyo-night", "gruvbox", "dracula", "custom"
theme = "nord"

[theme.custom]
# Опциональное переопределение цветов палитры (hex, ansi имя или colourN):
# bg = "#2e3440"
# fg = "#eceff4"
# accent = "#88c0d0"

[status]
enable = true
position = "bottom"          # "top" | "bottom"
separator_style = "powerline"    # "powerline" | "rounded" | "slanted" | "flat" | "none"

# Сегменты левой части
left = [
  { kind = "mode" },
  { kind = "session", icon = " " }
]

# Сегменты правой части
right = [
  { kind = "custom", command = "neoamp tmux --track-width 36", when = "has_neoamp" },
  { kind = "date", format = "%Y-%m-%d", icon = "󰃭 " },
  { kind = "clock", format = "%H:%M", icon = " " },
  { kind = "host", icon = "󰒋 " }
]

[clock]
format = "%H:%M"             # strftime-формат времени
date_format = "%Y-%m-%d"     # strftime-формат даты
style_12h = false            # 12/24-часовой формат (или авто-синхронизация с clock-mode-style)
show_date = true
nerd_icons = true            # использовать Nerd Font символы
pills = false                # false: единый блок даты и времени; true: раздельные капсулы

[mode_indicator]
# Текстовые метки режимов
labels.normal  = "NORMAL"
labels.prefix  = "PREFIX"
labels.copy    = "COPY"
labels.visual  = "VISUAL"
labels.resize  = "RESIZE"
labels.sync    = "SYNC"
labels.suspend = "SUSPEND"
labels.agent   = "AGENT"

# Иконки режимов
icons.normal  = "●"
icons.prefix  = "◆"
icons.copy    = "⎘"
icons.visual  = "◈"
icons.resize  = "↕"
icons.sync    = "⇄"
icons.suspend = "⏸"
icons.agent   = "▲"

[windows]
style = "powerline"          # "powerline" | "rounded" | "flat" | "classic"
show_flags = true            # отображение флагов окон (#F)
```

---

## 4. Спецификация компонентов

### 4.1. Цветовой движок и темы (`src/config/theme.rs`)
- `ColorSpec`: тип, десериализующий строковое представление цвета (`#rrggbb`, `#rgb`, `black`, `brightcyan`, `colour236`) в crossterm `Color` и валидную строку для tmux (`fg=...`, `bg=...`).
- Встроенные палитры:
  - `nord`: классические полярные цвета Nord 0..15.
  - `catppuccin-mocha`: цвета Base `#1e1e2e`, Text `#cdd6f4`, Sapphire, Mauve, Rosewater и т.д.
  - `tokyo-night`: цвета `#1a1b26`, `#c0caf5`, `#7aa2f7` и т.д.
  - `gruvbox`: ретро-цвета `#282828`, `#ebdbb2`, `#fe8019`, `#b8bb26`.
  - `dracula`: цвета `#282a36`, `#f8f8f2`, `#bd93f9`, `#50fa7b`, `#ff79c6`.
- Поддержка частичных переопределений: базовая тема + точечные замены из `[theme.custom]`.

### 4.2. Алгоритм стыковки сегментов и разделителей (`src/config/status.rs`)
- Сегмент описывается как блок контента с известным цветом фона ($Bg_i$) и текста ($Fg_i$).
- При обходе цепочки сегментов для левой стороны:
  - Между сегментом $i$ и сегментом $i+1$ генерируется разделитель с цветом: `#[fg=Bg_i,bg=Bg_{i+1}]<символ>`.
  - На стыке последнего сегмента со свободным фоном статус-бара разделитель окрашивается в `#[fg=Bg_{last},bg=StatusBg]<символ>#[default]`.
- Для правой стороны разделители направлены влево (``, ``, ``), а цвета сопрягаются от внешнего фона статус-бара к крайнему правому блоку.
- Если у соседних сегментов одинаковый фон (например, дата и время в блоке часов), используется тонкий разделитель (`` или `|`).
- Если `separator_style = "flat"` или активен режим `no_patched_font`, применяются ASCII/текстовые разделители.

### 4.3. Часы и виджеты
- `ClockSegment`:
  - Если `pills = true`: дата и время выводятся в двух отдельных капсулах с разными акцентными цветами.
  - Если `pills = false`: дата и время объединяются в одном сегменте через тонкий разделитель.
  - Поддержка иконок Nerd Font (, 󰥔, 󰃭).
  - Учёт флага `style_12h` и fallback на tmux-опцию `clock-mode-style`.

### 4.4. Индикатор режимов и совместимость
- Генерация вложенного выражения tmux format:
  1. `Agent`: `#{==:#{@mox_is_agent},1}`
  2. `Suspend`: `#{==:#{client_key_table},mox_suspend}`
  3. `Visual`: `#{selection_present}`
  4. `Copy`: `#{pane_in_mode}`
  5. `Resize`: `#{m:mox*r,#{client_key_table}}`
  6. `Sync`: `#{pane_synchronized}`
  7. `Prefix`: `#{||:#{client_prefix},#{m:mox*,#{client_key_table}}}`
  8. `Normal` (fallback).
- Обратная совместимость с плейсхолдером `#{tmux_mode_indicator}` и опциями `@mode_indicator_*_prompt` / `@mode_indicator_*_mode_style`.

### 4.5. Порядок разрешения параметров (`loader.rs`)
1. Параметры командной строки (`--config`, `--theme`).
2. Опции tmux сервера (`@mox_*`, `@nord_*`, `@mode_indicator_*`).
3. Файл `~/.config/mox/config.toml`.
4. Встроенные значения по умолчанию (эквивалентны Nord).

---

## 5. План тестирования и критерии приёмки

1. **Unit-тесты сериализации/десериализации**:
   - Чтение пустого TOML (проверка дефолтов).
   - Чтение полной конфигурации со всеми секциями.
   - Корректная обработка некорректных значений (graceful fallback).
2. **Тесты парсинга цветов**:
   - Проверка преобразования `#hex`, названий ANSI и `colourN` в crossterm `Color` и строки стилей tmux.
3. **Тесты сопряжения сегментов**:
   - Проверка корректности `#[fg=...,bg=...]` для стилей `powerline`, `rounded` и `flat`.
4. **Тесты обратной совместимости**:
   - Проверка, что `@nord_tmux_date_format`, `@nord_tmux_no_patched_font` и `@mode_indicator_*` корректно подхватываются и применяются.
5. **Интеграционные тесты `mox init`**:
   - Проверка генерации полного init-скрипта с новой конфигурацией.
6. **Workspace checks**:
   - `cargo fmt --all -- --check`
   - `cargo check --workspace --all-targets`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
