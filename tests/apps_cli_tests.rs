use clap::{CommandFactory, Parser};
use mox::cli::{AppCommand, Commands, TrackCommand};

#[derive(Parser, Debug)]
#[command(name = "mox")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[test]
fn test_cli_debug_assert() {
    Cli::command().debug_assert();
}

#[test]
fn test_parse_apps() {
    let cli = Cli::try_parse_from(["mox", "apps"]).expect("parse mox apps");
    match cli.command {
        Commands::Apps { socket, popup } => {
            assert_eq!(socket, None);
            assert!(!popup);
        }
        _ => panic!("expected Commands::Apps"),
    }

    let cli_popup =
        Cli::try_parse_from(["mox", "apps", "--popup"]).expect("parse mox apps --popup");
    match cli_popup.command {
        Commands::Apps { socket, popup } => {
            assert_eq!(socket, None);
            assert!(popup);
        }
        _ => panic!("expected Commands::Apps"),
    }

    let cli_socket = Cli::try_parse_from(["mox", "apps", "--socket", "test-sock", "--popup"])
        .expect("parse mox apps with socket");
    match cli_socket.command {
        Commands::Apps { socket, popup } => {
            assert_eq!(socket.as_deref(), Some("test-sock"));
            assert!(popup);
        }
        _ => panic!("expected Commands::Apps"),
    }
}

#[test]
fn test_parse_floax() {
    let cli = Cli::try_parse_from(["mox", "floax"]).expect("parse mox floax");
    match cli.command {
        Commands::Floax { socket, cwd } => {
            assert_eq!(socket, None);
            assert_eq!(cwd, None);
        }
        _ => panic!("expected Commands::Floax"),
    }

    let cli_args = Cli::try_parse_from(["mox", "floax", "--socket", "s1", "--cwd", "/tmp"])
        .expect("parse mox floax with args");
    match cli_args.command {
        Commands::Floax { socket, cwd } => {
            assert_eq!(socket.as_deref(), Some("s1"));
            assert_eq!(cwd.as_deref(), Some("/tmp"));
        }
        _ => panic!("expected Commands::Floax"),
    }
}

#[test]
fn test_parse_track() {
    // track add
    let cli = Cli::try_parse_from(["mox", "track", "add"]).expect("track add");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(command, TrackCommand::Add { window: None });
        }
        _ => panic!("expected Commands::Track"),
    }

    // track add --window @1
    let cli =
        Cli::try_parse_from(["mox", "track", "add", "--window", "@1"]).expect("track add --window");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(
                command,
                TrackCommand::Add {
                    window: Some("@1".to_string()),
                }
            );
        }
        _ => panic!("expected Commands::Track"),
    }

    // track remove
    let cli = Cli::try_parse_from(["mox", "track", "remove"]).expect("track remove");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(command, TrackCommand::Remove { window: None });
        }
        _ => panic!("expected Commands::Track"),
    }

    // track remove --window @2
    let cli = Cli::try_parse_from(["mox", "track", "remove", "--window", "@2"])
        .expect("track remove --window");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(
                command,
                TrackCommand::Remove {
                    window: Some("@2".to_string()),
                }
            );
        }
        _ => panic!("expected Commands::Track"),
    }

    // track toggle
    let cli = Cli::try_parse_from(["mox", "track", "toggle"]).expect("track toggle");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(command, TrackCommand::Toggle { window: None });
        }
        _ => panic!("expected Commands::Track"),
    }

    // track toggle --window @3
    let cli = Cli::try_parse_from(["mox", "track", "toggle", "--window", "@3"])
        .expect("track toggle --window");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(
                command,
                TrackCommand::Toggle {
                    window: Some("@3".to_string()),
                }
            );
        }
        _ => panic!("expected Commands::Track"),
    }

    // track switch
    let cli = Cli::try_parse_from(["mox", "track", "switch"]).expect("track switch");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(command, TrackCommand::Switch { window: None });
        }
        _ => panic!("expected Commands::Track"),
    }

    // track switch --window @4
    let cli = Cli::try_parse_from(["mox", "track", "switch", "--window", "@4"])
        .expect("track switch --window");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(
                command,
                TrackCommand::Switch {
                    window: Some("@4".to_string()),
                }
            );
        }
        _ => panic!("expected Commands::Track"),
    }

    // track list
    let cli = Cli::try_parse_from(["mox", "track", "list"]).expect("track list");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(command, TrackCommand::List);
        }
        _ => panic!("expected Commands::Track"),
    }

    // global socket option
    let cli = Cli::try_parse_from(["mox", "track", "--socket", "sock1", "list"])
        .expect("track --socket list");
    match cli.command {
        Commands::Track { command, socket } => {
            assert_eq!(socket.as_deref(), Some("sock1"));
            assert_eq!(command, TrackCommand::List);
        }
        _ => panic!("expected Commands::Track"),
    }
}

#[test]
fn test_parse_app() {
    // app run <name>
    let cli = Cli::try_parse_from(["mox", "app", "run", "lazygit"]).expect("app run lazygit");
    match cli.command {
        Commands::App { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(
                command,
                AppCommand::Run {
                    name: "lazygit".to_string(),
                    cwd: None,
                    window: false,
                }
            );
        }
        _ => panic!("expected Commands::App"),
    }

    // app run <name> --cwd <path> --window
    let cli = Cli::try_parse_from(["mox", "app", "run", "btop", "--cwd", "/tmp", "--window"])
        .expect("app run btop with flags");
    match cli.command {
        Commands::App { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(
                command,
                AppCommand::Run {
                    name: "btop".to_string(),
                    cwd: Some("/tmp".to_string()),
                    window: true,
                }
            );
        }
        _ => panic!("expected Commands::App"),
    }

    // app list
    let cli = Cli::try_parse_from(["mox", "app", "list"]).expect("app list");
    match cli.command {
        Commands::App { command, socket } => {
            assert_eq!(socket, None);
            assert_eq!(command, AppCommand::List);
        }
        _ => panic!("expected Commands::App"),
    }

    // global socket option
    let cli = Cli::try_parse_from(["mox", "app", "--socket", "sock2", "list"])
        .expect("app --socket list");
    match cli.command {
        Commands::App { command, socket } => {
            assert_eq!(socket.as_deref(), Some("sock2"));
            assert_eq!(command, AppCommand::List);
        }
        _ => panic!("expected Commands::App"),
    }
}

#[test]
fn test_app_run_not_found() {
    let result = mox::cli::run(Commands::App {
        command: AppCommand::Run {
            name: "nonexistent_app_xyz".to_string(),
            cwd: None,
            window: false,
        },
        socket: None,
    });
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("App 'nonexistent_app_xyz' not found in configuration")
    );
}

#[test]
fn test_app_list_runs_successfully() {
    let result = mox::cli::run(Commands::App {
        command: AppCommand::List,
        socket: None,
    });
    assert!(result.is_ok());
}
