//! Canonical feature actions; the key tree is also the palette catalog.
use crate::keymap::{KeyAction, KeyNode};
use crate::tmux::execute_tmux;
use crate::ui::{
    picker::{PickerItem, run_picker},
    preview::Preview,
};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FeatureAction {
    Palette,
    Projects,
    Files,
    GitFiles,
    Search,
    LazyGit,
    Save,
    Restore,
    LogStart,
    LogStop,
    SaveScreen,
    SaveHistory,
    ClearHistory,
    Suspend,
}
impl FeatureAction {
    pub fn name(self) -> String {
        self.to_possible_value()
            .map(|p| p.get_name().to_owned())
            .unwrap_or_default()
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Palette => "command palette",
            Self::Projects => "projects",
            Self::Files => "files → Neovim",
            Self::GitFiles => "git changes → Neovim",
            Self::Search => "ripgrep → Neovim",
            Self::LazyGit => "LazyGit",
            Self::Save => "save sessions",
            Self::Restore => "restore sessions",
            Self::LogStart => "start pane logging",
            Self::LogStop => "stop pane logging",
            Self::SaveScreen => "save visible pane",
            Self::SaveHistory => "save complete history",
            Self::ClearHistory => "clear pane history",
            Self::Suspend => "suspend local client",
        }
    }
}
pub fn run(action: FeatureAction, socket: Option<&str>) -> Result<(), String> {
    use std::io::IsTerminal;
    let result = execute(action, socket);
    if std::io::stdout().is_terminal() {
        match &result {
            Err(error) => crate::ui::prompt::toast(error, true)?,
            Ok(())
                if matches!(
                    action,
                    FeatureAction::Save
                        | FeatureAction::Restore
                        | FeatureAction::LogStart
                        | FeatureAction::LogStop
                        | FeatureAction::SaveScreen
                        | FeatureAction::SaveHistory
                        | FeatureAction::ClearHistory
                ) =>
            {
                crate::ui::prompt::toast(&format!("Completed: {}", action.label()), false)?
            }
            _ => {}
        }
    }
    result
}
fn execute(action: FeatureAction, socket: Option<&str>) -> Result<(), String> {
    use FeatureAction::*;
    match action {
        Palette => palette(socket),
        Projects => crate::projects::run(socket, &[]),
        Files | GitFiles | Search => {
            let source = match action {
                Files => crate::editor::files::Source::Files,
                GitFiles => crate::editor::files::Source::Git,
                _ => crate::editor::files::Source::Ripgrep,
            };
            let query = if action == Search {
                crate::ui::prompt::input("ripgrep pattern")?
            } else {
                None
            };
            crate::editor::files::pick(socket, source, query.as_deref(), None)
        }
        LazyGit => crate::lazygit::open(socket, None, false),
        Save => crate::persist::save(socket, &crate::persist::default_path(socket)?),
        Restore => crate::persist::restore(socket, &crate::persist::default_path(socket)?, false),
        Suspend => crate::navigation::suspend(socket, None, false),
        LogStart | LogStop | SaveScreen | SaveHistory | ClearHistory => {
            let pane = execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])?;
            let logs = crate::state::directory()?.join("logs");
            std::fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let path = logs.join(format!("{}-{time}.log", pane.trim_start_matches('%')));
            match action {
                LogStart => crate::logging::start(socket, &pane, &path),
                LogStop => crate::logging::stop(socket, &pane),
                ClearHistory => {
                    if crate::ui::prompt::confirm("Clear history?")? {
                        crate::logging::clear(socket, &pane)?;
                    }
                    Ok(())
                }
                _ => crate::logging::save(socket, &pane, &path, action == SaveHistory),
            }
        }
    }
}
pub fn catalog(root: &KeyNode) -> Vec<(String, String, KeyAction)> {
    fn walk(node: &KeyNode, path: &str, entries: &mut Vec<(String, String, KeyAction)>) {
        for child in &node.children {
            let path = format!("{path} {}", child.key).trim().to_owned();
            if let Some(action) = &child.action {
                entries.push((path.clone(), child.label.clone(), action.clone()));
            }
            walk(child, &path, entries);
        }
    }
    let mut entries = Vec::new();
    walk(root, "", &mut entries);
    entries
}
pub fn dispatch(action: &KeyAction, socket: Option<&str>) -> Result<(), String> {
    match action {
        KeyAction::Feature(action) => run(*action, socket),
        KeyAction::Tmux(command) => {
            crate::tmux::apply_tmux_commands(socket, std::slice::from_ref(command))
        }
        KeyAction::Confirm { prompt, command } => {
            if crate::ui::prompt::confirm(prompt)? {
                crate::tmux::apply_tmux_commands(socket, std::slice::from_ref(command))?;
            }
            Ok(())
        }
        KeyAction::Navigator => crate::nav::run_navigator(socket),
        KeyAction::Extract => {
            crate::extract::run_extract_picker(socket, 2000, None).map_err(|e| e.to_string())
        }
        KeyAction::Flash(config) => {
            crate::flash::run_flash_overlay(config, socket).map_err(|e| e.to_string())
        }
        KeyAction::TeaTimer => crate::tea::run_tea_timer(),
        KeyAction::AgentToggle => crate::agent::toggle_agent_window(socket),
        KeyAction::AgentCreate => crate::agent::create_agent_window(socket),
        KeyAction::CopyMode => execute_tmux(socket, &["copy-mode"]).map(|_| ()),
        KeyAction::WhichKey => crate::ui::run_which_key("", socket),
        KeyAction::SwitchTable(table) => {
            execute_tmux(socket, &["switch-client", "-T", table]).map(|_| ())
        }
        KeyAction::ExitModal => execute_tmux(socket, &["switch-client", "-T", "root"]).map(|_| ()),
    }
}
pub fn palette(socket: Option<&str>) -> Result<(), String> {
    let catalog = catalog(&crate::keymap::build_default_keymap());
    let mut items = Vec::new();
    let mut actions = Vec::new();
    for (path, label, action) in catalog {
        if action == KeyAction::Feature(FeatureAction::Palette) {
            continue;
        }
        items.push(PickerItem {
            id: path.clone(),
            label: format!("{label} · {path}"),
            detail: path,
            preview: Preview::Text(format!("{action:?}")),
        });
        actions.push(action);
    }
    for (command, format, prefix) in [
        (
            "list-panes",
            "#{pane_id}\t#{session_name}:#{window_name} · #{pane_current_command}",
            "pane",
        ),
        ("list-sessions", "#{session_id}\t#{session_name}", "session"),
        (
            "list-windows",
            "#{window_id}\t#{session_name}:#{window_name}",
            "window",
        ),
        ("list-buffers", "#{buffer_name}\t#{buffer_sample}", "buffer"),
        (
            "list-commands",
            "#{command_list_name}\t#{command_list_usage}",
            "tmux",
        ),
    ] {
        let mut args = vec![command];
        if command == "list-panes" || command == "list-windows" {
            args.push("-a");
        }
        args.extend(["-F", format]);
        let raw = execute_tmux(socket, &args)?;
        for line in raw.lines() {
            let Some((id, label)) = line.split_once('\t') else {
                continue;
            };
            if id.is_empty() {
                continue;
            }
            let action = match prefix {
                "pane" => format!(
                    "switch-client -t {} ; select-pane -t {} ; select-window -t {}",
                    crate::tmux::shell_quote(id),
                    crate::tmux::shell_quote(id),
                    crate::tmux::shell_quote(id)
                ),
                "session" => format!("switch-client -t {}", crate::tmux::shell_quote(id)),
                "window" => format!(
                    "switch-client -t {} ; select-window -t {}",
                    crate::tmux::shell_quote(id),
                    crate::tmux::shell_quote(id)
                ),
                "buffer" => format!("paste-buffer -b {}", crate::tmux::shell_quote(id)),
                _ => format!("command-prompt -I {}", crate::tmux::shell_quote(id)),
            };
            items.push(PickerItem {
                id: format!("{prefix}:{id}"),
                label: format!("{prefix} · {label}"),
                detail: id.into(),
                preview: if prefix == "pane" {
                    Preview::Pane(id.into())
                } else {
                    Preview::Text(label.into())
                },
            });
            actions.push(KeyAction::Tmux(action));
        }
    }
    // A process entry focuses its owning pane; it never executes captured argv.
    let panes = execute_tmux(
        socket,
        &["list-panes", "-a", "-F", "#{pane_id}\t#{pane_tty}"],
    )?;
    let owners: std::collections::HashMap<_, _> = panes
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(pane, tty)| (tty.trim_start_matches("/dev/"), pane))
        .collect();
    let mut command = std::process::Command::new("ps");
    command.args(["-eo", "pid=,tty=,comm=,args="]);
    if let Ok(output) = crate::process::output(
        &mut command,
        std::time::Duration::from_millis(500),
        1024 * 1024,
    ) {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut fields = line.split_whitespace();
            let (Some(pid), Some(tty)) = (fields.next(), fields.next()) else {
                continue;
            };
            let Some(pane) = owners.get(tty) else {
                continue;
            };
            let label = format!("process · {pid} · {}", fields.collect::<Vec<_>>().join(" "));
            items.push(PickerItem {
                id: format!("process:{pid}"),
                label: label.clone(),
                detail: (*pane).into(),
                preview: Preview::Text(label),
            });
            actions.push(KeyAction::Tmux(format!(
                "switch-client -t {pane} ; select-pane -t {pane} ; select-window -t {pane}"
            )));
        }
    }
    let projects = crate::projects::discover(socket, &[])?;
    let project_start = items.len();
    items.extend(projects.iter().map(|project| {
        let mut item = crate::projects::item(project);
        item.label = format!("project · {}", item.label);
        item
    }));
    if let Some(index) = run_picker("Command palette", &items, socket)? {
        if index < project_start {
            dispatch(&actions[index], socket)?;
        } else {
            crate::projects::activate(socket, &projects[index - project_start])?;
        }
    }
    Ok(())
}
