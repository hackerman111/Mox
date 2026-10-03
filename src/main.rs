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
    }

    ExitCode::SUCCESS
}
