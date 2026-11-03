use super::*;

#[test]
fn test_resolve_log_config_prefers_config_when_no_cli_flags() {
    let cli = Cli::parse_from(["nhl"]);
    let config = config::Config {
        log_level: "debug".to_string(),
        log_file: "/tmp/nhl.log".to_string(),
        ..Default::default()
    };

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
    let config = config::Config {
        log_level: "debug".to_string(),
        ..Default::default()
    };

    let (log_level, _) = resolve_log_config(&cli, &config);

    assert_eq!(log_level, "info");
}

#[test]
fn test_resolve_log_config_explicit_log_file_wins() {
    let cli = Cli::parse_from(["nhl", "-F", "/dev/null"]);
    let config = config::Config {
        log_file: "/tmp/nhl.log".to_string(),
        ..Default::default()
    };

    let (_, log_file) = resolve_log_config(&cli, &config);

    assert_eq!(log_file, "/dev/null");
}

fn parse_standings_season(season_arg: &str) -> Result<Option<Season>, clap::Error> {
    let cli = Cli::try_parse_from(["nhl", "standings", "--season", season_arg])?;
    match cli.command {
        Some(Commands::Standings { season, .. }) => Ok(season),
        _ => panic!("expected the standings subcommand"),
    }
}

#[test]
fn test_standings_season_accepts_hyphenated_range() {
    let season = parse_standings_season("2024-2025").unwrap();

    assert_eq!(season, Some(Season::new(2024)));
}

#[test]
fn test_standings_season_accepts_compact_range() {
    let season = parse_standings_season("20242025").unwrap();

    assert_eq!(season, Some(Season::new(2024)));
}

#[test]
fn test_standings_season_rejects_single_year() {
    assert!(parse_standings_season("2024").is_err());
}

#[test]
fn test_standings_season_rejects_non_numeric_input() {
    assert!(parse_standings_season("abc").is_err());
}
