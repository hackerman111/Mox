use clap::{Parser, Subcommand};
use moch::keymap::build_default_keymap;
use moch::tea::run_tea_timer;
use moch::tmux::{apply_tmux_commands, generate_init_script};
use moch::ui::run_which_key;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "moch")]
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
    /// Generate or apply tmux configuration for moch
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
                .unwrap_or_else(|_| "moch".to_string());
            let script_lines = generate_init_script(&keymap, &entry_key, &bin_path);

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
    }

    ExitCode::SUCCESS
}
