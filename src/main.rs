use clap::{Parser, Subcommand};
use mox::keymap::build_default_keymap;
use mox::tea::run_tea_timer;
use mox::tmux::{apply_tmux_commands, generate_init_script};
use mox::ui::run_which_key;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "Mox")]
#[command(
    about = "Vim-style modal control, which-key, and tea timer for tmux",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate or apply tmux configuration for Mox
    Init {
        /// Entry key to toggle modal mode (default: M-m)
        #[arg(short, long, default_value = "M-m")]
        entry_key: String,

        /// Automatically apply commands to the running tmux instance
        #[arg(short, long)]
        apply: bool,

        /// Tmux socket name (-L)
        #[arg(short, long)]
        socket: Option<String>,
    },

    /// Launch which-key popup menu for a key prefix
    WhichKey {
        /// Sequence prefix (e.g. "w", "w s", "g")
        #[arg(short, long, default_value = "")]
        prefix: String,

        /// Tmux socket name (-L)
        #[arg(short, long)]
        socket: Option<String>,
    },

    /// Launch the built-in Gongfu Cha tea timer
    Tea,

    /// Launch the unified Vim-like fuzzy tree navigator
    Nav {
        /// Tmux socket name (-L)
        #[arg(short, long)]
        socket: Option<String>,
    },

    /// Create a new AI agent window and jump to it
    AgentCreate {
        /// Tmux socket name (-L)
        #[arg(short, long)]
        socket: Option<String>,
    },

    /// Toggle between current work window and AI agent window
    AgentToggle {
        /// Tmux socket name (-L)
        #[arg(short, long)]
        socket: Option<String>,
    },

    /// Launch Flash overlay for visual navigation, yanking, and remote operations
    Flash {
        /// Navigation mode (jump, quick-yank, quick-open, remote-yank, char-motion)
        #[arg(short, long, default_value = "jump")]
        mode: String,

        /// Match across all window panes instead of current pane
        #[arg(short, long)]
        multi_pane: bool,

        /// Entity filter for quick-yank or quick-open (path, url, hash, ip)
        #[arg(short, long)]
        filter: Option<String>,

        /// Single character target for char-motion
        #[arg(short, long)]
        r#char: Option<char>,

        /// Scan backward instead of forward for char-motion
        #[arg(short, long)]
        backward: bool,

        /// Target tmux pane id (e.g. %0)
        #[arg(long)]
        target_pane: Option<String>,

        /// Tmux socket name (-L)
        #[arg(short, long)]
        socket: Option<String>,

        /// Launch borderless display-popup overlay
        #[arg(long)]
        launch_popup: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            entry_key,
            apply,
            socket,
        } => {
            let keymap = build_default_keymap();
            let bin_path = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "mox".to_string());
            let current_status_left =
                mox::tmux::execute_tmux(socket.as_deref(), &["show-option", "-gv", "status-left"])
                    .ok();
            let script_lines = generate_init_script(
                &keymap,
                &entry_key,
                &bin_path,
                current_status_left.as_deref(),
            );

            if apply {
                if let Err(e) = apply_tmux_commands(socket.as_deref(), &script_lines) {
                    eprintln!("Error applying tmux config: {e}");
                    return ExitCode::FAILURE;
                }
            } else {
                use std::io::Write;
                let mut out = std::io::stdout().lock();
                for line in script_lines {
                    if writeln!(out, "{line}").is_err() {
                        break;
                    }
                }
            }
        }
        Commands::WhichKey { prefix, socket } => {
            if let Err(e) = run_which_key(&prefix, socket.as_deref()) {
                eprintln!("Error in which-key: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Tea => {
            if let Err(e) = run_tea_timer() {
                eprintln!("Error in tea timer: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Nav { socket } => {
            if let Err(e) = mox::nav::run_navigator(socket.as_deref()) {
                eprintln!("Error in navigator: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::AgentCreate { socket } => {
            if let Err(e) = mox::agent::create_agent_window(socket.as_deref()) {
                eprintln!("Error creating agent window: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::AgentToggle { socket } => {
            if let Err(e) = mox::agent::toggle_agent_window(socket.as_deref()) {
                eprintln!("Error toggling agent window: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Flash {
            mode,
            multi_pane,
            filter,
            r#char,
            backward,
            target_pane,
            socket,
            launch_popup,
        } => {
            let flash_mode = match mode.to_lowercase().as_str() {
                "quick-yank" | "quickyank" => mox::flash::FlashMode::QuickYank,
                "quick-open" | "quickopen" => mox::flash::FlashMode::QuickOpen,
                "remote-yank" | "remoteyank" => mox::flash::FlashMode::RemoteYank,
                "char-motion" | "charmotion" => mox::flash::FlashMode::CharMotion,
                _ => mox::flash::FlashMode::Jump,
            };

            let entity_filter = filter
                .as_deref()
                .and_then(|f| match f.to_lowercase().as_str() {
                    "path" => Some(mox::extract::EntityKind::Path),
                    "url" => Some(mox::extract::EntityKind::Url),
                    "hash" => Some(mox::extract::EntityKind::Hash),
                    "ip" => Some(mox::extract::EntityKind::Ip),
                    _ => None,
                });

            let cfg = mox::flash::FlashConfig {
                mode: flash_mode,
                multi_pane,
                entity_filter,
                target_pane,
                motion_char: r#char,
                motion_forward: !backward,
            };

            if launch_popup {
                if let Err(e) = mox::flash::launch_flash_popup(socket.as_deref(), &cfg) {
                    eprintln!("Error launching flash popup: {e}");
                    return ExitCode::FAILURE;
                }
            } else if let Err(e) = mox::flash::run_flash_overlay(&cfg, socket.as_deref()) {
                eprintln!("Error running flash overlay: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}
