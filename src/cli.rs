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
