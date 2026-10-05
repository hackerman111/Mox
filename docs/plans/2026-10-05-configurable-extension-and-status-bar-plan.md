# Configurable Extension & Status Bar Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Реализовать систему конфигурации (TOML + tmux-опции), поддержку кастомных тем (Nord, Catppuccin, Tokyo Night, Gruvbox, Dracula, Custom), модульный статус-бар с красивыми часами и индикатором режимов, полностью заменяющие функционал плагинов из `insp/`.

**Architecture:** В `src/config/` реализуются модели данных (serde), парсер цветов `ColorSpec`, реестр тем, сегментный генератор статус-бара с автоматическим расчётом стыковки разделителей (Powerline/rounded/flat) и иерархический загрузчик с обратной совместимостью по tmux-опциям. Существующие `src/ui/theme.rs`, `src/ui/status.rs`, `src/tmux/config.rs` и CLI интегрируются с новой конфигурацией.

**Tech Stack:** Rust (edition 2024), `toml = "0.8"`, `serde`, `crossterm`, `clap`.

---

### Task 1: Подключение зависимости `toml` в `Cargo.toml`

**Files:**
- Modify: `Cargo.toml`
- Test: `Cargo.toml`

**Step 1: Обновить Cargo.toml**
Добавить `toml = "0.8"` в секцию `[dependencies]`.

```toml
[dependencies]
unicode-width = { version = "0.2", default-features = false }
clap = { version = "4.5", features = ["derive"] }
crossterm = "0.29"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
libc = "0.2"
```

**Step 2: Проверить сборку зависимостей**
Run: `cargo check`  
Expected: Успешная компиляция и разрешение зависимостей без ошибок.

**Step 3: Зафиксировать коммит**
```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add toml dependency for configuration support"
```

---

### Task 2: Модели данных конфигурации (`src/config/model.rs`, `src/config/mod.rs`)

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/model.rs`
- Modify: `src/lib.rs`
- Test: `tests/config_model_tests.rs`

**Step 1: Написать падающий тест на десериализацию конфигурации**
Создать `tests/config_model_tests.rs`:
- Проверка, что пустая строка TOML десериализуется в валидный `Config` со всеми дефолтами (тема Nord, powerline разделители, 24h часы).
- Проверка десериализации кастомного TOML с секциями `[theme]`, `[status]`, `[clock]`, `[mode_indicator]`.

**Step 2: Запустить тест и убедиться, что он не компилируется / падает**
Run: `cargo test --test config_model_tests`  
Expected: FAIL (модуль `mox::config` не найден).

**Step 3: Написать реализацию моделей данных**
- `src/config/model.rs`:
  - `Config`: корневая структура (`theme`, `status`, `clock`, `mode_indicator`, `windows`).
  - `SeparatorStyle`: `Powerline`, `Rounded`, `Slanted`, `Flat`, `None`.
  - `StatusConfig`: `enable`, `position`, `separator_style`, `left: Vec<SegmentConfig>`, `right: Vec<SegmentConfig>`.
  - `SegmentKind`: `Mode`, `Session`, `Date`, `Clock`, `Host`, `Custom { command: String }`.
  - `ClockConfig`: `format`, `date_format`, `style_12h`, `show_date`, `nerd_icons`, `pills`.
  - `ModeIndicatorConfig`: `labels: ModeLabels`, `icons: ModeIcons`.
  - `WindowsConfig`: `style`, `show_flags`.
- `src/config/mod.rs`: реэкспорт типов.
- `src/lib.rs`: добавить `pub mod config;`.

**Step 4: Запустить тесты и убедиться в прохождении**
Run: `cargo test --test config_model_tests`  
Expected: PASS.

**Step 5: Зафиксировать коммит**
```bash
git add src/config/ src/lib.rs tests/config_model_tests.rs
git commit -m "feat(config): implement config data models with serde defaults"
```

---

### Task 3: Цветовой движок и встроенные темы (`src/config/theme.rs`, `src/ui/theme.rs`)

**Files:**
- Create: `src/config/theme.rs`
- Modify: `src/config/mod.rs`
- Modify: `src/ui/theme.rs`
- Test: `tests/config_theme_tests.rs`

**Step 1: Написать падающий тест на парсинг цветов и тем**
Создать `tests/config_theme_tests.rs`:
- Тест парсинга `ColorSpec`: `#2e3440`, `#fff`, `black`, `brightcyan`, `colour236`.
- Тест получения палитр: `ThemePreset::Nord`, `ThemePreset::CatppuccinMocha`, `ThemePreset::TokyoNight`, `ThemePreset::Gruvbox`, `ThemePreset::Dracula`.
- Тест наложения кастомных переопределений из `[theme.custom]`.

**Step 2: Запустить тест и убедиться в ошибке**
Run: `cargo test --test config_theme_tests`  
Expected: FAIL (`ColorSpec` или пресеты не найдены).

**Step 3: Написать реализацию `src/config/theme.rs` и адаптировать `src/ui/theme.rs`**
- Реализовать `ColorSpec` с конвертацией в `crossterm::style::Color` и строку цвета tmux (`#rrggbb`, `black`, и т.д.).
- Определить палитры для `Nord`, `Catppuccin Mocha`, `Tokyo Night`, `Gruvbox`, `Dracula`.
- В `src/ui/theme.rs` добавить конструктор `Theme::from_config(config: &Config) -> Self`.

**Step 4: Запустить тесты и проверить успешное прохождение**
Run: `cargo test --test config_theme_tests`  
Expected: PASS.

**Step 5: Зафиксировать коммит**
```bash
git add src/config/theme.rs src/config/mod.rs src/ui/theme.rs tests/config_theme_tests.rs
git commit -m "feat(theme): add multi-theme presets, ColorSpec parser, and Theme::from_config"
```

---

### Task 4: Модульный сборщик статус-бара, красивые часы и индикатор режимов (`src/config/status.rs`)

**Files:**
- Create: `src/config/status.rs`
- Modify: `src/config/mod.rs`
- Modify: `src/ui/status.rs`
- Test: `tests/config_status_tests.rs`

**Step 1: Написать падающий тест на сборку статус-бара и сопряжение цветов**
Создать `tests/config_status_tests.rs`:
- Тест генерации `status-left` с расчётом цветов разделителей для стилей `powerline` и `rounded`.
- Тест генерации красивых часов (pills vs combined, Nerd Font иконки).
- Тест генерации индикатора режимов со всеми 8 состояниями и обратной совместимостью с `#{tmux_mode_indicator}`.
- Тест генерации `window-status-format` и `window-status-current-format`.

**Step 2: Запустить тест и убедиться в ошибке**
Run: `cargo test --test config_status_tests`  
Expected: FAIL (`status.rs` не существует).

**Step 3: Написать реализацию генератора `src/config/status.rs`**
- Реализовать функцию сопряжения сегментов:
  - Вычисление `#[fg=...,bg=...]` разделителей между смежными сегментами и на границе со `status-bg`.
  - Поддержка глифов `powerline` (``/``, ``/``), `rounded` (``/``), `slanted` (``/``), `flat` (`|`), `none`.
- Реализовать генерацию сегмента часов с учётом формата, иконок и пилюль.
- Реализовать генератор индикатора режимов в `src/ui/status.rs` с поддержкой кастомных меток и цветов.
- Реализовать генератор стилей оконных вкладок.

**Step 4: Запустить тест и убедиться в успехе**
Run: `cargo test --test config_status_tests`  
Expected: PASS.

**Step 5: Зафиксировать коммит**
```bash
git add src/config/status.rs src/config/mod.rs src/ui/status.rs tests/config_status_tests.rs
git commit -m "feat(status): implement modular status bar generator and clock widgets"
```

---

### Task 5: Иерархический загрузчик и обратная совместимость (`src/config/loader.rs`)

**Files:**
- Create: `src/config/loader.rs`
- Modify: `src/config/mod.rs`
- Test: `tests/config_loader_tests.rs`

**Step 1: Написать падающий тест на загрузчик и оверрайды**
Создать `tests/config_loader_tests.rs`:
- Тест загрузки файла по явному пути и по умолчанию.
- Тест наложения оверрайдов tmux-опций (`@nord_tmux_date_format`, `@nord_tmux_no_patched_font`, `@mode_indicator_*`).
- Тест генерации дефолтного файла конфигурации (`Config::default_toml()`).

**Step 2: Запустить тест и убедиться в ошибке**
Run: `cargo test --test config_loader_tests`  
Expected: FAIL (`loader.rs` не существует).

**Step 3: Написать реализацию `src/config/loader.rs`**
- Функция поиска конфига: `$XDG_CONFIG_HOME/mox/config.toml` -> `~/.config/mox/config.toml`.
- Загрузка и десериализация с информативными ошибками синтаксиса.
- Функция `apply_tmux_overrides(&mut Config, socket: Option<&str>)` — чтение tmux опций (`show-option -gqv`) и переопределение полей.
- Генератор шаблона `default_toml()` с подробными комментариями.

**Step 4: Запустить тест и убедиться в прохождении**
Run: `cargo test --test config_loader_tests`  
Expected: PASS.

**Step 5: Зафиксировать коммит**
```bash
git add src/config/loader.rs src/config/mod.rs tests/config_loader_tests.rs
git commit -m "feat(config): implement hierarchical loader with tmux option overrides"
```

---

### Task 6: Интеграция в `src/tmux/config.rs`, CLI и TUI

**Files:**
- Modify: `src/tmux/config.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `tests/native_init_tests.rs`
- Test: `tests/keymap_tests.rs`

**Step 1: Написать тест на генерацию `mox init` с кастомным конфигом**
Добавить в `tests/native_init_tests.rs`:
- Проверка, что `generate_init_script` формирует правильные стили tmux (`pane-border-style`, `message-style`, `clock-mode-colour`, `status-left`, `status-right`, `window-status-format`) из активной темы.

**Step 2: Запустить тест и зафиксировать поведение**
Run: `cargo test --test native_init_tests`  
Expected: FAIL до обновления `generate_init_script`.

**Step 3: Обновить код генератора и CLI**
- В `src/tmux/config.rs`: `generate_init_script` принимает `&Config` и генерирует полный скрипт стилизации tmux и статус-бара на основе модульного генератора.
- В `src/cli.rs` и `src/main.rs`:
  - Добавить опцию `--config <path>` в команду `Init`.
  - Добавить подкоманду `Config` (`mox config default` / `mox config show`).
  - Обеспечить загрузку темы в командах `Nav`, `WhichKey`, `Extract`, `Flash`, `Tea`.

**Step 4: Запустить все затронутые тесты**
Run: `cargo test --test native_init_tests --test keymap_tests`  
Expected: PASS.

**Step 5: Зафиксировать коммит**
```bash
git add src/tmux/config.rs src/cli.rs src/main.rs tests/native_init_tests.rs tests/keymap_tests.rs
git commit -m "feat: integrate config and multi-theme status generation into init script and CLI"
```

---

### Task 7: Комплексная верификация и регрессионное тестирование

**Files:**
- All touched files

**Step 1: Запустить форматирование**
Run: `cargo fmt --all -- --check`  
Expected: PASS без расхождений.

**Step 2: Запустить проверку сборки всего workspace**
Run: `cargo check --workspace --all-targets`  
Expected: PASS без ошибок.

**Step 3: Запустить все тесты проекта**
Run: `cargo test --workspace`  
Expected: PASS все тесты.

**Step 4: Запустить Clippy со строгими предупреждениями**
Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`  
Expected: PASS без warnings.

**Step 5: Зафиксировать итоговый коммит верификации**
```bash
git commit --allow-empty -m "chore: verify workspace tests, formatting, and clippy passes"
```
