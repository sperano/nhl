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

/// Default log file path (no logging to file)
const DEFAULT_LOG_FILE: &str = "/dev/null";

#[derive(Parser)]
#[command(name = "nhl")]
#[command(
    about = "NHL stats and standings CLI",
    long_about = "NHL stats and standings CLI\n\nIf no command is specified, the program starts in interactive mode."
)]
struct Cli {
    /// Set log level (trace, debug, info, warn, error) [default: info, or config's log_level]
    #[arg(short = 'L', long, global = true)]
    log_level: Option<String>,

    /// Log file path [default: /dev/null (no logging), or config's log_file]
    #[arg(short = 'F', long, global = true)]
    log_file: Option<String>,

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
    /// Display player stats for the day
    PlayerStats {
        /// Player name to search for
        player: String,

        /// Date in YYYY-MM-DD format (optional, shows recent games if not specified)
        #[arg(short, long)]
        date: Option<String>,
    },
    /// Display current configuration
    Config,
    /// Display play-by-play events (like Unix tail)
    Tail {
        /// Game ID (e.g., 2024020001)
        game_id: i64,

        /// Number of plays to show
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Follow mode - continuously stream new plays
        #[arg(short = 'f', long)]
        follow: bool,

        /// Polling interval in seconds (for follow mode)
        #[arg(long, default_value = "5")]
        interval: u64,

        /// Show verbose output with additional details
        #[arg(short, long)]
        verbose: bool,

        /// Only show goals
        #[arg(long)]
        goals: bool,

        /// Only show penalties
        #[arg(long)]
        penalties: bool,

        /// Only show shots (includes goals)
        #[arg(long)]
        shots: bool,
    },
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
    println!("[display]");
    println!("use_unicode: {}", cfg.display.use_unicode);
    println!(
        "theme: {}",
        cfg.display
            .theme_name
            .as_deref()
            .unwrap_or("(none, using default)")
    );
    println!("error_fg: {:?}", cfg.display.error_fg);
}

/// Resolve log configuration from CLI args and config file
/// CLI arguments take precedence over config file
fn resolve_log_config<'a>(cli: &'a Cli, config: &'a config::Config) -> (&'a str, &'a str) {
    let log_level = cli.log_level.as_deref().unwrap_or(&config.log_level);
    let log_file = cli.log_file.as_deref().unwrap_or(&config.log_file);
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
        Commands::PlayerStats { player, date } => {
            commands::player_stats::run(client, &player, date, config).await
        }
        Commands::Tail {
            game_id,
            count,
            follow,
            interval,
            verbose,
            goals,
            penalties,
            shots,
        } => {
            let filter = commands::tail::EventFilter {
                goals,
                penalties,
                shots,
            };
            if follow {
                commands::tail::follow(client, game_id, count, interval, &filter, verbose).await
            } else {
                commands::tail::run(client, game_id, count, &filter, verbose).await
            }
        }
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
    let Some(command) = cli.command else {
        if let Err(e) = run_tui_mode(config, mock_mode).await {
            eprintln!("Error running TUI: {}", e);
            std::process::exit(1);
        }
        return;
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_log_config_prefers_config_when_no_cli_flags() {
        let cli = Cli::parse_from(["nhl"]);
        let mut config = config::Config::default();
        config.log_level = "debug".to_string();
        config.log_file = "/tmp/nhl.log".to_string();

        let (log_level, log_file) = resolve_log_config(&cli, &config);

        assert_eq!(log_level, "debug");
        assert_eq!(log_file, "/tmp/nhl.log");
    }

    #[test]
    fn test_resolve_log_config_explicit_flag_wins_even_when_it_matches_the_default() {
        // Regression test: passing `-L info` explicitly must take precedence over the config
        // file's log_level, even though "info" also happens to be the flag's implicit default.
        // The bug this guards against: inferring "was the flag set?" by comparing the parsed
        // value against the default string, which can't distinguish "user typed the default
        // value" from "user didn't pass the flag at all".
        let cli = Cli::parse_from(["nhl", "-L", "info"]);
        let mut config = config::Config::default();
        config.log_level = "debug".to_string();

        let (log_level, _) = resolve_log_config(&cli, &config);

        assert_eq!(log_level, "info");
    }

    #[test]
    fn test_resolve_log_config_explicit_log_file_wins() {
        let cli = Cli::parse_from(["nhl", "-F", "/dev/null"]);
        let mut config = config::Config::default();
        config.log_file = "/tmp/nhl.log".to_string();

        let (_, log_file) = resolve_log_config(&cli, &config);

        assert_eq!(log_file, "/dev/null");
    }
}
