use nhl::commands;
use nhl::config;
use nhl::data_provider::NHLDataProvider;
use nhl::tui;

#[cfg(feature = "development")]
use nhl::dev::mock_client::MockClient;

use clap::{Parser, Subcommand, ValueEnum};
use nhl_api::Client;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

// Default Configuration Constants
/// Default log level when not specified
const DEFAULT_LOG_LEVEL: &str = "info";

/// Default log file path (no logging to file)
const DEFAULT_LOG_FILE: &str = "/dev/null";

#[derive(Parser)]
#[command(name = "nhl")]
#[command(
    about = "NHL stats and standings CLI",
    long_about = "NHL stats and standings CLI\n\nIf no command is specified, the program starts in interactive mode."
)]
struct Cli {
    /// Set log level (trace, debug, info, warn, error)
    #[arg(short = 'L', long, global = true, default_value = DEFAULT_LOG_LEVEL)]
    log_level: String,

    /// Log file path (default: /dev/null for no logging)
    #[arg(short = 'F', long, global = true, default_value = DEFAULT_LOG_FILE)]
    log_file: String,

    /// Use mock data instead of real API calls (development feature only)
    #[cfg(feature = "development")]
    #[arg(long, global = true)]
    mock: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Clone, Copy, ValueEnum)]
enum GroupBy {
    /// Group by division
    #[value(name = "d")]
    Division,
    /// Group by conference
    #[value(name = "c")]
    Conference,
    /// Show league-wide standings
    #[value(name = "l")]
    League,
}

impl GroupBy {
    /// Convert CLI GroupBy enum to commands::standings::GroupBy
    fn to_standings_groupby(self) -> commands::standings::GroupBy {
        match self {
            Self::Division => commands::standings::GroupBy::Division,
            Self::Conference => commands::standings::GroupBy::Conference,
            Self::League => commands::standings::GroupBy::League,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Display NHL standings
    Standings {
        /// Season year (optional, defaults to current standings)
        #[arg(short, long)]
        season: Option<i64>,

        /// Date in YYYY-MM-DD format (optional)
        #[arg(short, long)]
        date: Option<String>,

        /// Group standings by: d=division, c=conference, l=league
        #[arg(short, long, default_value = "d")]
        by: GroupBy,
    },
    /// Display boxscore for a specific game
    Boxscore {
        /// Game ID (e.g., 2024020001)
        game_id: i64,
    },
    /// Display daily schedule of games
    Schedule {
        /// Date in YYYY-MM-DD format (optional, defaults to today)
        #[arg(short, long)]
        date: Option<String>,
    },
    /// Display scores for games with period-by-period breakdown
    Scores {
        /// Date in YYYY-MM-DD format (optional, defaults to today)
        #[arg(short, long)]
        date: Option<String>,
    },
    /// Display all NHL franchises
    Franchises,
    /// Display current configuration
    Config,
}

fn create_client(#[allow(unused_variables)] mock_mode: bool) -> Arc<dyn NHLDataProvider> {
    #[cfg(feature = "development")]
    if mock_mode {
        tracing::info!("Using mock client for development");
        return Arc::new(MockClient::new());
    }

    match Client::new() {
        Ok(client) => Arc::new(client),
        Err(e) => {
            let error_msg = format!("Failed to create NHL API client: {}", e);
            tracing::error!("{}", error_msg);
            eprintln!("{}", error_msg);
            std::process::exit(1);
        }
    }
}

fn init_logging(log_level: &str, log_file: &str) {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_file)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open log file {}: {}", log_file, e);
            return;
        }
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .finish();
    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Failed to set tracing subscriber: {}", e);
    }
}

// TODO update this command, output is not up to date
/// Handle the config command - display current configuration
fn handle_config_command() {
    let cfg = config::read();

    let (path_str, exists) = match config::get_config_path() {
        Some(path) => {
            let exists = path.exists();
            (path.display().to_string(), exists)
        }
        None => ("Unable to determine config path".to_string(), false),
    };

    println!(
        "Configuration File: {} (Exists: {})",
        path_str,
        if exists { "yes" } else { "no" }
    );
    println!();
    println!("Current Configuration:");
    println!("=====================");
    println!("log_level: {}", cfg.log_level);
    println!("log_file: {}", cfg.log_file);
    println!("refresh_interval: {} seconds", cfg.refresh_interval);
    println!(
        "display_standings_western_first: {}",
        cfg.display_standings_western_first
    );
    println!("time_format: {}", cfg.time_format);
    println!();
    println!("[theme]");
}

/// Resolve log configuration from CLI args and config file
/// CLI arguments take precedence over config file
fn resolve_log_config<'a>(cli: &'a Cli, config: &'a config::Config) -> (&'a str, &'a str) {
    let log_level = if cli.log_level != DEFAULT_LOG_LEVEL {
        cli.log_level.as_str()
    } else {
        config.log_level.as_str()
    };

    let log_file = if cli.log_file != DEFAULT_LOG_FILE {
        cli.log_file.as_str()
    } else {
        config.log_file.as_str()
    };

    (log_level, log_file)
}

/// Run TUI mode
async fn run_tui_mode(config: config::Config, mock_mode: bool) -> Result<(), std::io::Error> {
    tracing::info!("Running in experimental React-like mode");
    let client = create_client(mock_mode);
    tui::run(client, config).await
}

/// Execute a CLI command by routing it to the appropriate command handler
async fn execute_command(
    client: &dyn NHLDataProvider,
    command: Commands,
    config: &config::Config,
) -> anyhow::Result<()> {
    match command {
        Commands::Config => unreachable!("Config command should be handled before execute_command"),
        Commands::Standings { season, date, by } => {
            let group_by = by.to_standings_groupby();
            commands::standings::run(client, season, date, group_by, config).await
        }
        Commands::Boxscore { game_id } => commands::boxscore::run(client, game_id, config).await,
        Commands::Schedule { date } => commands::schedule::run(client, date).await,
        Commands::Scores { date } => commands::scores::run(client, date).await,
        Commands::Franchises => commands::franchises::run(client).await,
    }
}

#[tokio::main]
async fn main() {
    let config = config::read();
    let cli = Cli::parse();

    // Resolve and initialize logging
    let (log_level, log_file) = resolve_log_config(&cli, &config);
    if log_file != DEFAULT_LOG_FILE {
        init_logging(log_level, log_file);
    }

    // Extract mock flag (only available in development feature)
    #[cfg(feature = "development")]
    let mock_mode = cli.mock;
    #[cfg(not(feature = "development"))]
    let mock_mode = false;

    // If no subcommand, run TUI
    if cli.command.is_none() {
        if let Err(e) = run_tui_mode(config, mock_mode).await {
            eprintln!("Error running TUI: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let command = cli.command.unwrap();

    // Handle Config command separately (doesn't need a client)
    if let Commands::Config = command {
        handle_config_command();
        return;
    }

    // Create client and execute command
    let client = create_client(mock_mode);
    if let Err(e) = execute_command(&*client, command, &config).await {
        eprintln!("Error: {:#}", e);
        tracing::error!("Command failed: {:#}", e);
        std::process::exit(1);
    }
}
