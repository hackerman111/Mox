//! Tmux configuration and init script generation.

use super::scroll;
use crate::keymap::{KeyAction, KeyNode};

const VISUAL_INDICATOR: &str = "#{?#{==:#{pane_mode},copy-mode},#[fg=#2e3440]#[bg=#88c0d0]#[bold] ◈ #{?selection_present,VISUAL,COPY} #[default] ,}";

/// Strips any previously injected mox or moch indicator from a status-left string.
pub fn clean_status_left(s: &str) -> String {
    let mut cleaned = s.trim().to_string();
    cleaned = cleaned
        .replace(VISUAL_INDICATOR, "")
        .replace("#{E:@mox_autosave}", "");
    if let Some(pos) = cleaned.find("#{E:@mox_indicator}") {
        cleaned.replace_range(pos..pos + "#{E:@mox_indicator}".len(), "");
    }
    if let Some(pos) = cleaned.find("#{E:@moch_indicator}") {
        cleaned.replace_range(pos..pos + "#{E:@moch_indicator}".len(), "");
    }
    while let Some(start) = cleaned.find("#{?#{==") {
        if cleaned[start..].starts_with("#{?#{==:#{key-table},mox")
            || cleaned[start..].starts_with("#{?#{==:#{key-table},moch")
        {
            let mut depth = 0;
            let mut end = None;
            for (i, c) in cleaned[start..].char_indices() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
            }
            if let Some(end_idx) = end {
                cleaned.replace_range(start..end_idx, "");
            } else {
                break;
            }
        } else {
            break;
        }
    }
    cleaned
}

/// Generates the complete tmux script for configuring mox.
pub fn generate_init_script(
    root_keymap: &KeyNode,
    entry_key: &str,
    bin_path: &str,
    current_status_left: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let (inner, border) = popup_styles();

    lines.push("# === Mox tmux initialization ===".into());

    // 1. Vim scroll configuration
    lines.extend(scroll::generate_scroll_config_with_bin(bin_path));

    // 2. Status indicator configuration
    lines.push("# Modal status indicator".into());
    lines.push("set -g status-left-length 250".into());

    let raw_status_left = current_status_left.unwrap_or("#[fg=black,bg=blue,bold] #S #[default]");
    let base_status_left = clean_status_left(raw_status_left);

    let theme = crate::ui::Theme::default();
    lines.push(format!(
        "set -g @mox_indicator {}",
        super::quote(&crate::ui::status::tmux_indicator(&theme))
    ));
    lines.push(format!(
        "set -g status-style {}",
        super::quote(&format!(
            "fg={},bg={}",
            crate::ui::status::tmux_color(theme.fg),
            crate::ui::status::tmux_color(theme.bg)
        ))
    ));
    let save_command = format!(
        "{} persist autosave --interval '#{{@mox_autosave_interval}}'",
        super::shell_quote(bin_path)
    );
    let auto = format!("#({save_command})");
    let base_status_left = base_status_left.replace("#{E:@mox_autosave}", "");
    lines.push(format!("set -g @mox_autosave {}", super::quote(&auto)));
    lines.push(
        "if-shell -F '#{==:#{@mox_autosave_interval},}' { set -g @mox_autosave_interval 900 }"
            .into(),
    );
    lines.push("if-shell -F '#{==:#{@mox_autorestore},}' { set -g @mox_autorestore on }".into());
    lines.push(format!(
        "set -g status-left {}",
        super::quote(&format!(
            "#{{E:@mox_indicator}}#{{E:@mox_autosave}}{base_status_left}"
        ))
    ));
    let restore = format!("{} persist autorestore", super::shell_quote(bin_path));
    lines.push(format!(
        "if-shell -F '#{{==:#{{@mox_autorestore}},on}}' {{ run-shell -b {} }}",
        super::quote(&restore)
    ));
    lines.extend(crate::navigation::bindings(bin_path));
    for (key, action) in [
        ("Space", "palette"),
        ("o", "projects"),
        ("f", "files"),
        ("C-f", "git-files"),
        ("/", "search"),
        ("N", "lazy-git"),
        ("C-s", "save"),
        ("C-r", "restore"),
    ] {
        let command = format!("{} action {action}", super::shell_quote(bin_path));
        lines.push(format!("bind-key -T prefix {key} display-popup -E -w 85% -h 85% -b rounded -T \" Mox: {action} \" -s \"{inner}\" -S \"{border}\" {}",super::quote(&command)));
    }

    // 3. Popup border styling: transparent border background ensures rounded corners do not bleed opaque pixels
    lines.push(format!(
        "set -g popup-border-style {}",
        super::quote(&border)
    ));

    // 4. Entry binding (default: M-m)
    lines.push(format!(
        "bind-key -n {} {{ set-option key-table mox ; refresh-client -S }}",
        entry_key
    ));

    // 5. Generate key-table bindings for root and sub-tables
    generate_table_bindings(root_keymap, "mox", bin_path, &mut lines);

    lines
}

fn popup_styles() -> (String, String) {
    let theme = crate::ui::Theme::default();
    let color = crate::ui::status::tmux_color;
    (
        format!("fg={},bg={}", color(theme.fg), color(theme.bg)),
        format!("fg={},bg=default", color(theme.info)),
    )
}

fn generate_table_bindings(
    node: &KeyNode,
    table_name: &str,
    bin_path: &str,
    lines: &mut Vec<String>,
) {
    let (inner, border) = popup_styles();
    for child in &node.children {
        // Exit shortcuts: q, Escape, i, M-m
        if child.key == "i" || child.key == "M-m" {
            lines.push(format!(
                "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S }}",
                child.key
            ));
            continue;
        }

        if (child.key == "q" || child.key == "Escape")
            || (child.key == "Enter"
                && matches!(
                    child.action,
                    Some(KeyAction::ExitModal) | Some(KeyAction::SwitchTable(_)) | None
                ))
        {
            let cmd = match &child.action {
                Some(KeyAction::ExitModal) => {
                    "{ set-option key-table root ; refresh-client -S }".to_string()
                }
                Some(KeyAction::SwitchTable(target)) => {
                    format!("{{ set-option key-table {target} ; refresh-client -S }}")
                }
                _ => "{ set-option key-table root ; refresh-client -S }".to_string(),
            };
            lines.push(format!("bind-key -T {table_name} {} {cmd}", child.key));
            continue;
        }

        if !child.children.is_empty() {
            let subtable_name = format!("{table_name}_{}", child.key);
            let prefix_arg = if node.key.is_empty() {
                child.key.clone()
            } else {
                format!("{} {}", node.key, child.key)
            };

            let title = if child.sticky {
                format!(" Mox: {} [STICKY] ", child.label)
            } else {
                format!(" Mox: {} ", child.label)
            };

            let (w, h) =
                if child.key == "r" || child.key == "s" || child.key == "g" || child.key == "y" {
                    (68, 11)
                } else {
                    (78, 14)
                };

            let which_key = format!(
                "{} which-key --prefix {}",
                super::shell_quote(bin_path),
                super::shell_quote(&prefix_arg)
            );
            let popup_cmd = format!(
                "display-popup -w {w} -h {h} -y S -b rounded -T \"{title}\" -s \"{inner}\" -S \"{border}\" -E {}",
                super::quote(&which_key)
            );

            if child.sticky {
                if child.key == "r" {
                    // For sticky resize: switch table without blocking screen, popup available via ?
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table {subtable_name} ; refresh-client -S }}",
                        child.key
                    ));
                } else {
                    // For sticky window mode (w): switch table and show compact which-key
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table {subtable_name} ; refresh-client -S ; {popup_cmd} }}",
                        child.key
                    ));
                }
            } else {
                // Transient submenu
                lines.push(format!(
                    "bind-key -T {table_name} {} {{ switch-client -T {subtable_name} ; {popup_cmd} }}",
                    child.key
                ));
            }

            // Also generate sub-table bindings
            generate_table_bindings(child, &subtable_name, bin_path, lines);
        } else if let Some(action) = &child.action {
            match action {
                KeyAction::Feature(action) => {
                    let command = format!(
                        "{} action {}",
                        crate::tmux::shell_quote(bin_path),
                        action.name()
                    );
                    lines.push(format!("bind-key -T {table_name} {} {{ set-option key-table root ; display-popup -E -w 85% -h 85% -b rounded -T \" Mox: {} \" -s \"{inner}\" -S \"{border}\" {} }}",child.key,action.label(),super::quote(&command)));
                }
                KeyAction::Tmux(cmd) => {
                    let cmd = cmd.replace("{{bin_path}}", bin_path);
                    if cmd.contains("choose-tree") {
                        lines.push(format!(
                            "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; {cmd} }}",
                            child.key
                        ));
                    } else {
                        lines.push(format!("bind-key -T {table_name} {} {cmd}", child.key));
                    }
                }
                KeyAction::SwitchTable(target) => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table {target} ; refresh-client -S }}",
                        child.key
                    ));
                }
                KeyAction::Confirm { prompt, command } => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} confirm-before -p \"{prompt}\" \"{command}\"",
                        child.key
                    ));
                }
                KeyAction::Flash(cfg) => {
                    let command = crate::flash::build_flash_command(bin_path, cfg, None);
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; run-shell -b {} }}",
                        child.key,
                        super::quote(&format!("{command} --launch-popup"))
                    ));
                }
                KeyAction::Extract => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; run-shell -b {} }}",
                        child.key,
                        super::quote(&format!("{} extract --launch-popup", super::shell_quote(bin_path)))
                    ));
                }
                KeyAction::CopyMode => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; copy-mode }}",
                        child.key
                    ));
                }
                KeyAction::WhichKey => {
                    let command = format!("{} which-key", super::shell_quote(bin_path));
                    lines.push(format!(
                        "bind-key -T {table_name} {} display-popup -w 82 -h 15 -y S -b rounded -T \" Mox: which-key \" -s \"{inner}\" -S \"{border}\" -E {}",
                        child.key,
                        super::quote(&command)
                    ));
                }
                KeyAction::TeaTimer => {
                    let command = format!("{} tea", super::shell_quote(bin_path));
                    lines.push(format!(
                        "bind-key -T {table_name} {} display-popup -w 50 -h 10 -y C -b rounded -T \" Tea Timer \" -s \"{inner}\" -S \"{border}\" -E {}",
                        child.key,
                        super::quote(&command)
                    ));
                }
                KeyAction::Navigator => {
                    let command = format!("{} nav", super::shell_quote(bin_path));
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S ; display-popup -w 90% -h 85% -b rounded -T \" Mox: Navigator \" -s \"{inner}\" -S \"{border}\" -E {} }}",
                        child.key,
                        super::quote(&command)
                    ));
                }
                KeyAction::AgentToggle => {
                    let command = format!("{} agent-toggle", super::shell_quote(bin_path));
                    lines.push(format!(
                        "bind-key -T {table_name} {} run-shell {}",
                        child.key,
                        super::quote(&command)
                    ));
                }
                KeyAction::AgentCreate => {
                    let command = format!("{} agent-create", super::shell_quote(bin_path));
                    lines.push(format!(
                        "bind-key -T {table_name} {} run-shell {}",
                        child.key,
                        super::quote(&command)
                    ));
                }
                KeyAction::ExitModal => {
                    lines.push(format!(
                        "bind-key -T {table_name} {} {{ set-option key-table root ; refresh-client -S }}",
                        child.key
                    ));
                }
            }
        }
    }
}
