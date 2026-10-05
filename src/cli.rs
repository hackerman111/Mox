//! Thin CLI adapters for native features.
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Commands {
    Action {
        #[arg(value_enum)]
        action: crate::actions::FeatureAction,
        #[arg(long)]
        socket: Option<String>,
    },
    Palette {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        popup: bool,
    },
    Projects {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        root: Vec<PathBuf>,
        #[arg(long)]
        popup: bool,
    },
    Editor {
        #[command(subcommand)]
        command: EditorCommand,
        #[arg(long, global = true)]
        socket: Option<String>,
    },
    Persist {
        #[command(subcommand)]
        command: PersistCommand,
        #[arg(long, global = true)]
        socket: Option<String>,
        #[arg(long, global = true)]
        path: Option<PathBuf>,
    },
    Log {
        #[command(subcommand)]
        command: LogCommand,
        #[arg(long, global = true)]
        socket: Option<String>,
        #[arg(long, global = true)]
        pane: Option<String>,
    },
    #[command(name = "lazygit", alias = "lazy-git")]
    LazyGit {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        window: bool,
    },
    Navigate {
        direction: String,
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        pane: Option<String>,
        #[arg(long)]
        from_editor: bool,
    },
    Suspend {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        client: Option<String>,
        #[arg(long)]
        resume: bool,
    },
    #[command(hide = true)]
    PreviewFile {
        #[arg(long)]
        path: PathBuf,
    },
    #[command(hide = true)]
    PreviewDirectory {
        #[arg(long)]
        path: PathBuf,
    },
    Clipboard {
        text: String,
        #[arg(long)]
        socket: Option<String>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
        #[arg(long, global = true)]
        socket: Option<String>,
    },
    Apps {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        popup: bool,
    },
    Floax {
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
    },
    Track {
        #[command(subcommand)]
        command: TrackCommand,
        #[arg(long, global = true)]
        socket: Option<String>,
    },
    App {
        #[command(subcommand)]
        command: AppCommand,
        #[arg(long, global = true)]
        socket: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the starter commented configuration file (default.toml)
    Default,
    /// Print the path to the active configuration file
    Path,
    /// Show current configuration as TOML
    Show {
        #[arg(long)]
        config: Option<PathBuf>,
    },
}
#[derive(Debug, Subcommand)]
pub enum EditorCommand {
    Open {
        location: String,
        #[arg(long)]
        pane: Option<String>,
    },
    Setup,
    Register {
        #[arg(long)]
        pane: String,
        #[arg(long)]
        server: String,
        #[arg(long)]
        pid: u32,
    },
    Files {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Git {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Rg {
        query: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
}
#[derive(Debug, Subcommand)]
pub enum PersistCommand {
    Save,
    Restore {
        #[arg(long)]
        commands: bool,
    },
    Autosave {
        #[arg(long, default_value_t = 900)]
        interval: u64,
    },
    Autorestore,
}
#[derive(Debug, Subcommand)]
pub enum LogCommand {
    Start {
        path: PathBuf,
    },
    Stop,
    Save {
        path: PathBuf,
        #[arg(long)]
        full: bool,
    },
    Clear {
        #[arg(long)]
        yes: bool,
    },
    Sink {
        path: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum TrackCommand {
    Add {
        #[arg(long)]
        window: Option<String>,
    },
    Remove {
        #[arg(long)]
        window: Option<String>,
    },
    Toggle {
        #[arg(long)]
        window: Option<String>,
    },
    Switch {
        #[arg(long)]
        window: Option<String>,
    },
    List,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum AppCommand {
    Run {
        name: String,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        window: bool,
    },
    List,
}

pub fn run(command: Commands) -> Result<(), String> {
    match command {
        Commands::Action { action, socket } => crate::actions::run(action, socket.as_deref()),
        Commands::Palette { socket, popup } => {
            if popup {
                launch(socket.as_deref(), "palette")
            } else {
                crate::actions::palette(socket.as_deref())
            }
        }
        Commands::Projects {
            socket,
            root,
            popup,
        } => {
            if popup {
                let roots = root
                    .iter()
                    .map(|p| format!(" --root {}", crate::tmux::shell_quote(&p.to_string_lossy())))
                    .collect::<String>();
                launch(socket.as_deref(), &format!("projects{roots}"))
            } else {
                crate::projects::run(socket.as_deref(), &root)
            }
        }
        Commands::Editor { command, socket } => {
            let socket = socket.as_deref();
            match command {
                EditorCommand::Setup => {
                    print!("{}", crate::editor::LUA);
                    Ok(())
                }
                EditorCommand::Register { pane, server, pid } => {
                    crate::editor::register(socket, &pane, &server, pid)
                }
                EditorCommand::Open { location, pane } => crate::editor::open(
                    socket,
                    pane.as_deref(),
                    crate::editor::Location::parse(&location)?,
                ),
                EditorCommand::Files { root } => crate::editor::files::pick(
                    socket,
                    crate::editor::files::Source::Files,
                    None,
                    root.as_deref(),
                ),
                EditorCommand::Git { root } => crate::editor::files::pick(
                    socket,
                    crate::editor::files::Source::Git,
                    None,
                    root.as_deref(),
                ),
                EditorCommand::Rg { query, root } => crate::editor::files::pick(
                    socket,
                    crate::editor::files::Source::Ripgrep,
                    Some(&query),
                    root.as_deref(),
                ),
            }
        }
        Commands::Persist {
            command,
            socket,
            path,
        } => {
            let socket = socket.as_deref();
            let path = path
                .map(Ok)
                .unwrap_or_else(|| crate::persist::default_path(socket))?;
            match command {
                PersistCommand::Save => crate::persist::save(socket, &path),
                PersistCommand::Restore { commands } => {
                    crate::persist::restore(socket, &path, commands)
                }
                PersistCommand::Autosave { interval } => {
                    crate::persist::autosave(socket, &path, interval)
                }
                PersistCommand::Autorestore => crate::persist::autorestore(socket, &path),
            }
        }
        Commands::Log {
            command,
            socket,
            pane,
        } => {
            if let LogCommand::Sink { path } = command {
                return crate::logging::sink(&path);
            }
            let socket = socket.as_deref();
            let pane = match pane {
                Some(pane) => pane,
                None => {
                    crate::tmux::execute_tmux(socket, &["display-message", "-p", "#{pane_id}"])?
                }
            };
            match command {
                LogCommand::Start { path } => crate::logging::start(socket, &pane, &path),
                LogCommand::Stop => crate::logging::stop(socket, &pane),
                LogCommand::Save { path, full } => crate::logging::save(socket, &pane, &path, full),
                LogCommand::Clear { yes } => {
                    if yes || crate::ui::prompt::confirm("Clear history?")? {
                        crate::logging::clear(socket, &pane)?;
                    }
                    Ok(())
                }
                LogCommand::Sink { .. } => unreachable!(),
            }
        }
        Commands::LazyGit {
            socket,
            cwd,
            window,
        } => crate::lazygit::open(socket.as_deref(), cwd.as_deref(), window),
        Commands::Navigate {
            socket,
            pane,
            direction,
            from_editor,
        } => {
            crate::navigation::navigate(socket.as_deref(), pane.as_deref(), &direction, from_editor)
        }
        Commands::Suspend {
            socket,
            client,
            resume,
        } => crate::navigation::suspend(socket.as_deref(), client.as_deref(), resume),
        Commands::PreviewFile { path } => {
            println!(
                "{}",
                serde_json::to_string(&crate::ui::preview::local_file(&path)?)
                    .map_err(|e| e.to_string())?
            );
            Ok(())
        }
        Commands::PreviewDirectory { path } => {
            println!(
                "{}",
                serde_json::to_string(&crate::ui::preview::local_directory(&path)?)
                    .map_err(|e| e.to_string())?
            );
            Ok(())
        }
        Commands::Clipboard { socket, text } => crate::clipboard::copy(socket.as_deref(), &text),
        Commands::Config { command, socket } => match command {
            ConfigCommand::Default => {
                print!("{}", crate::config::loader::default_toml());
                Ok(())
            }
            ConfigCommand::Path => {
                if let Some(path) = crate::config::loader::default_config_path() {
                    println!("{}", path.display());
                    Ok(())
                } else {
                    Err("No default configuration path resolved".into())
                }
            }
            ConfigCommand::Show { config } => {
                let mut cfg = crate::config::loader::load_config(config.as_deref());
                crate::config::loader::apply_tmux_server_overrides(&mut cfg, socket.as_deref());
                let toml_str = toml::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
                print!("{toml_str}");
                Ok(())
            }
        },
        Commands::Apps { socket, popup } => {
            if popup {
                launch(socket.as_deref(), "apps")
            } else {
                crate::apps::run_apps_panel(socket.as_deref(), None)
            }
        }
        Commands::Floax { socket, cwd } => crate::apps::toggle_floax(
            socket.as_deref(),
            &crate::apps::floax::FloaxConfig::default(),
            cwd.as_deref(),
        ),
        Commands::Track { command, socket } => match command {
            TrackCommand::Add { window } => {
                crate::apps::track_window(socket.as_deref(), window.as_deref()).map(|_| ())
            }
            TrackCommand::Remove { window } => {
                crate::apps::untrack_window(socket.as_deref(), window.as_deref()).map(|_| ())
            }
            TrackCommand::Toggle { window } => {
                let tracked = if let Some(w) = window {
                    let is_tracked = crate::apps::is_window_tracked(socket.as_deref(), Some(&w))?;
                    if is_tracked {
                        crate::apps::untrack_window(socket.as_deref(), Some(&w))?;
                        false
                    } else {
                        crate::apps::track_window(socket.as_deref(), Some(&w))?;
                        true
                    }
                } else {
                    crate::apps::toggle_current_window_tracking(socket.as_deref())?
                };
                let win_info = crate::tmux::execute_tmux(
                    socket.as_deref(),
                    &["display-message", "-p", "#{window_name} (#{window_id})"],
                )
                .unwrap_or_default();
                let msg = if tracked {
                    format!("Tracked window {}", win_info.trim())
                } else {
                    format!("Untracked window {}", win_info.trim())
                };
                let _ = crate::tmux::execute_tmux(socket.as_deref(), &["display-message", &msg]);
                println!("{msg}");
                Ok(())
            }
            TrackCommand::Switch { window } => {
                if let Err(e) =
                    crate::apps::toggle_tracked_window(socket.as_deref(), window.as_deref())
                {
                    let msg = format!("Mox: {e}");
                    let _ =
                        crate::tmux::execute_tmux(socket.as_deref(), &["display-message", &msg]);
                    return Err(e);
                }
                Ok(())
            }
            TrackCommand::List => {
                let list = crate::apps::list_tracked_windows(socket.as_deref())?;
                for w in list {
                    println!("{}\t{}\t{}", w.id, w.name, w.session_id);
                }
                Ok(())
            }
        },
        Commands::App { command, socket } => match command {
            AppCommand::Run { name, cwd, window } => {
                let cfg = crate::config::loader::load_config(None);
                if let Some(app) = cfg.apps.iter().find(|a| a.name == name) {
                    crate::apps::run_app(socket.as_deref(), app, cwd.as_deref(), window)
                } else {
                    Err(format!("App '{name}' not found in configuration"))
                }
            }
            AppCommand::List => {
                let cfg = crate::config::loader::load_config(None);
                for app in &cfg.apps {
                    println!("{}\t{}\t{}", app.name, app.command, app.title);
                }
                Ok(())
            }
        },
    }
}
fn launch(socket: Option<&str>, subcommand: &str) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = format!(
        "{} {subcommand}",
        crate::tmux::shell_quote(&executable.to_string_lossy())
    );
    if let Some(socket) = socket {
        command.push_str(&format!(" --socket {}", crate::tmux::shell_quote(socket)));
    }
    crate::tmux::popup(socket, "Mox", None, &command)
}
