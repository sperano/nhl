use crate::commands::parse_game_date;
use crate::config::Config;
use crate::data_provider::NHLDataProvider;
use crate::formatting::BoxChars;
use anyhow::{bail, Context, Result};
use chrono::Datelike;
use nhl_api::{GameDate, GameLog, GameState, GameType, PlayerSearchResult, SkaterStats};

/// Run the player-stats command
///
/// Searches for a player by name and displays their stats from today's game (or specified date).
pub async fn run(
    client: &dyn NHLDataProvider,
    player_name: &str,
    date: Option<String>,
    config: &Config,
) -> Result<()> {
    // Parse the date
    let game_date = parse_game_date(date)?;
    let date_str = game_date.to_api_string();
    let season = get_season_for_date(&game_date);

    // Search for the player
    let search_results = client
        .search_player(player_name, Some(10))
        .await
        .context("Failed to search for player")?;

    if search_results.is_empty() {
        bail!("No players found matching '{}'", player_name);
    }

    // Filter results to those whose name contains all search terms and have a team
    let matching = filter_matching_players(&search_results, player_name);

    if matching.is_empty() {
        bail!("No players found matching '{}'", player_name);
    }

    let player = match resolve_single_match(&matching, player_name) {
        Some(player) => player,
        None => std::process::exit(1),
    };
    let player_id = player.player_id;

    // Get the player's game log for the season
    let game_log = client
        .player_game_log(player_id, season, GameType::RegularSeason)
        .await
        .context("Failed to fetch player game log")?;

    // Find the game for the specified date
    let game_entry = game_log.game_log.iter().find(|g| g.game_date == date_str);

    let log_stats = match game_entry {
        Some(stats) => stats,
        None => {
            println!("No data for {} on {}", player.name, date_str);
            return Ok(());
        }
    };

    let boxscore_stats = fetch_boxscore_stats_for_player(client, &game_date, player).await?;

    print_player_game_stats(player, log_stats, boxscore_stats.as_ref(), config);

    Ok(())
}

/// If there is exactly one matching player, return it. If there are several,
/// print them (so the caller can be more specific) and return `None`.
///
/// Extracted from `run` to keep the ambiguous-match printing separate from
/// the main control flow.
fn resolve_single_match<'a>(
    matching: &[&'a PlayerSearchResult],
    player_name: &str,
) -> Option<&'a PlayerSearchResult> {
    if matching.len() > 1 {
        println!("Multiple players found matching '{}':", player_name);
        println!();
        for player in matching {
            let team = player.team_abbrev.as_deref().unwrap_or("N/A");
            let number = player
                .sweater_number
                .map(|n| format!("#{}", n))
                .unwrap_or_default();
            println!(
                "  {} {} - {} {}",
                player.name,
                number,
                team,
                player.position.map_or("N/A", |p| p.code())
            );
        }
        println!();
        println!("Please be more specific.");
        return None;
    }

    Some(matching[0])
}

/// Fetch the player's boxscore-derived stats (hits, blocks, FO%, etc.) for the
/// game on `game_date`, if their team has a game that has started.
///
/// Extracted from `run` to isolate the schedule/boxscore lookup chain.
async fn fetch_boxscore_stats_for_player(
    client: &dyn NHLDataProvider,
    game_date: &GameDate,
    player: &PlayerSearchResult,
) -> Result<Option<SkaterStats>> {
    let player_team = player.team_abbrev.as_deref().unwrap_or("");

    let schedule = client
        .daily_schedule(Some(game_date.clone()))
        .await
        .context("Failed to fetch schedule")?;

    let team_game = schedule
        .games
        .iter()
        .find(|g| g.away_team.abbrev == player_team || g.home_team.abbrev == player_team);

    let Some(game) = team_game else {
        return Ok(None);
    };

    // Only fetch boxscore if the game has started
    if game.game_state == GameState::Future || game.game_state == GameState::PreGame {
        return Ok(None);
    }

    let boxscore = client.boxscore(game.id).await.ok();
    Ok(boxscore.and_then(|bs| {
        let team_stats = if bs.away_team.abbrev == player_team {
            &bs.player_by_game_stats.away_team
        } else {
            &bs.player_by_game_stats.home_team
        };

        // Find player in forwards or defense
        team_stats
            .forwards
            .iter()
            .find(|s| s.player_id == player.player_id)
            .or_else(|| {
                team_stats
                    .defense
                    .iter()
                    .find(|s| s.player_id == player.player_id)
            })
            .cloned()
    }))
}

/// Filter player search results to those whose name contains every whitespace-separated
/// term in `player_name` (case-insensitive) and that have a known current team. Players
/// without a team (e.g. free agents) can't be matched to a game, so they're excluded.
fn filter_matching_players<'a>(
    search_results: &'a [PlayerSearchResult],
    player_name: &str,
) -> Vec<&'a PlayerSearchResult> {
    let search_lower = player_name.to_lowercase();
    let search_terms: Vec<&str> = search_lower.split_whitespace().collect();

    search_results
        .iter()
        .filter(|p| {
            let name_lower = p.name.to_lowercase();
            let matches_name = search_terms.iter().all(|term| name_lower.contains(term));
            let has_team = p.team_abbrev.is_some();
            matches_name && has_team
        })
        .collect()
}

/// Get the NHL season for a given date.
/// NHL seasons run from October to June, so:
/// - Oct-Dec of year Y = season YYYYYYYY+1 (e.g., Oct 2024 = 20242025)
/// - Jan-Sep of year Y = season YYYY-1YYYY (e.g., Apr 2025 = 20242025)
fn get_season_for_date(date: &GameDate) -> i32 {
    let naive_date = match date {
        GameDate::Now => chrono::Local::now().date_naive(),
        GameDate::Date(d) => *d,
    };

    let year = naive_date.year();
    let month = naive_date.month();

    if month >= 10 {
        // October-December: season starts this year
        year * 10000 + (year + 1)
    } else {
        // January-September: season started last year
        (year - 1) * 10000 + year
    }
}

fn print_player_game_stats(
    player: &PlayerSearchResult,
    log_stats: &GameLog,
    boxscore_stats: Option<&SkaterStats>,
    config: &Config,
) {
    print!(
        "{}",
        format_player_game_stats(player, log_stats, boxscore_stats, &config.display.box_chars)
    );
}

/// Build the player header box, matchup line, and stats table as a single string.
///
/// Extracted from `print_player_game_stats` so the formatting logic can be
/// unit-tested without capturing stdout.
fn format_player_game_stats(
    player: &PlayerSearchResult,
    log_stats: &GameLog,
    boxscore_stats: Option<&SkaterStats>,
    box_chars: &BoxChars,
) -> String {
    let box_width = 50;

    let team = player.team_abbrev.as_deref().unwrap_or("N/A");
    let number = player
        .sweater_number
        .map(|n| format!("#{}", n))
        .unwrap_or_default();
    let content = format!(
        "{} {} - {} {}",
        player.name,
        number,
        team,
        player.position.map_or("N/A", |p| p.code())
    );

    let mut output = String::new();

    // Player header
    output.push_str(&format!(
        "\n{}{}{}\n",
        box_chars.top_left,
        box_chars.horizontal.repeat(box_width),
        box_chars.top_right
    ));
    output.push_str(&format!(
        "{} {:<width$} {}\n",
        box_chars.vertical,
        content,
        box_chars.vertical,
        width = box_width - 2
    ));
    output.push_str(&format!(
        "{}{}{}\n",
        box_chars.bottom_left,
        box_chars.horizontal.repeat(box_width),
        box_chars.bottom_right
    ));

    // Game info
    let opponent = if log_stats.home_road_flag == nhl_api::HomeRoad::Home {
        format!("vs {}", log_stats.opponent_abbrev)
    } else {
        format!("@ {}", log_stats.opponent_abbrev)
    };
    output.push_str(&format!("\n{} - {}\n", log_stats.game_date, opponent));

    // Stats table
    output.push('\n');
    output.push_str(&format_skater_stats_table(
        log_stats,
        boxscore_stats,
        box_chars,
    ));

    output
}

/// Build the skater stats table (header row, separator, and value row) as a string.
///
/// Extracted from `print_skater_stats` so the two formatting branches (with and
/// without boxscore-derived stats) can be unit-tested without capturing stdout.
fn format_skater_stats_table(
    log: &GameLog,
    boxscore: Option<&SkaterStats>,
    box_chars: &BoxChars,
) -> String {
    let pim = log.pim.unwrap_or(0);
    let mut output = String::new();

    if let Some(bs) = boxscore {
        // Full stats with boxscore data
        output.push_str(&format!(
            "{:>2} {:>2} {:>2} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>4} {:>2} {:>2} {:>6}\n",
            "G", "A", "P", "+/-", "PIM", "SOG", "PPP", "HIT", "BLK", "FO%", "GV", "TK", "TOI"
        ));
        output.push_str(&format!("{}\n", box_chars.horizontal.repeat(52)));
        output.push_str(&format!(
            "{:>2} {:>2} {:>2} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>4.0} {:>2} {:>2} {:>6}\n",
            log.goals,
            log.assists,
            log.points,
            format_plus_minus(log.plus_minus),
            pim,
            log.shots,
            log.power_play_points,
            bs.hits,
            bs.blocked_shots,
            bs.faceoff_winning_pctg,
            bs.giveaways,
            bs.takeaways,
            log.toi
        ));
    } else {
        // Basic stats without boxscore
        output.push_str(&format!(
            "{:>3} {:>3} {:>3} {:>4} {:>4} {:>4} {:>4} {:>7}\n",
            "G", "A", "P", "+/-", "PIM", "SOG", "PPP", "TOI"
        ));
        output.push_str(&format!("{}\n", box_chars.horizontal.repeat(42)));
        output.push_str(&format!(
            "{:>3} {:>3} {:>3} {:>4} {:>4} {:>4} {:>4} {:>7}\n",
            log.goals,
            log.assists,
            log.points,
            format_plus_minus(log.plus_minus),
            pim,
            log.shots,
            log.power_play_points,
            log.toi
        ));
    }

    output
}

fn format_plus_minus(pm: i32) -> String {
    if pm > 0 {
        format!("+{}", pm)
    } else {
        pm.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::mock_client::MockClient;
    use chrono::NaiveDate;
    use nhl_api::{HomeRoad, Position};

    fn sample_player(name: &str, team: Option<&str>) -> PlayerSearchResult {
        PlayerSearchResult {
            player_id: 8478402.into(),
            name: name.to_string(),
            position: Some(Position::Center),
            team_id: Some(22.into()),
            team_abbrev: team.map(|t| t.to_string()),
            sweater_number: Some(97),
            active: true,
            height: None,
            birth_city: None,
            birth_state_province: None,
            birth_country: None,
        }
    }

    fn sample_game_log(home_road_flag: HomeRoad, pim: Option<i32>) -> GameLog {
        GameLog {
            game_id: 2024020500.into(),
            game_date: "2024-12-28".to_string(),
            team_abbrev: "EDM".to_string(),
            home_road_flag,
            opponent_abbrev: "CGY".to_string(),
            goals: 2,
            assists: 1,
            points: 3,
            plus_minus: 2,
            power_play_goals: 1,
            power_play_points: 2,
            shots: 5,
            shifts: 24,
            toi: "21:45".to_string(),
            game_winning_goals: Some(1),
            ot_goals: None,
            pim,
        }
    }

    fn sample_boxscore_skater() -> SkaterStats {
        SkaterStats {
            player_id: 8478402.into(),
            sweater_number: 97,
            name: nhl_api::LocalizedString {
                default: "Connor McDavid".to_string(),
            },
            position: Some(Position::Center),
            goals: 2,
            assists: 1,
            points: 3,
            plus_minus: 2,
            pim: 0,
            hits: 3,
            power_play_goals: 1,
            sog: 5,
            faceoff_winning_pctg: 0.6,
            toi: "21:45".to_string(),
            blocked_shots: 1,
            shifts: 24,
            giveaways: 2,
            takeaways: 4,
        }
    }

    // --- filter_matching_players ---

    #[test]
    fn test_filter_matching_players_single_term_case_insensitive() {
        let players = vec![sample_player("Connor McDavid", Some("EDM"))];
        let matching = filter_matching_players(&players, "mcdavid");
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name, "Connor McDavid");
    }

    #[test]
    fn test_filter_matching_players_requires_all_terms() {
        let players = vec![
            sample_player("Connor McDavid", Some("EDM")),
            sample_player("Connor Bedard", Some("CHI")),
        ];
        let matching = filter_matching_players(&players, "connor mcdavid");
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name, "Connor McDavid");
    }

    #[test]
    fn test_filter_matching_players_excludes_players_without_team() {
        let players = vec![sample_player("Connor McDavid", None)];
        let matching = filter_matching_players(&players, "mcdavid");
        assert!(matching.is_empty());
    }

    #[test]
    fn test_filter_matching_players_no_match_returns_empty() {
        let players = vec![sample_player("Connor McDavid", Some("EDM"))];
        let matching = filter_matching_players(&players, "matthews");
        assert!(matching.is_empty());
    }

    #[test]
    fn test_filter_matching_players_ambiguous_query_returns_multiple() {
        let players = vec![
            sample_player("Connor McDavid", Some("EDM")),
            sample_player("Connor Bedard", Some("CHI")),
        ];
        let matching = filter_matching_players(&players, "connor");
        assert_eq!(matching.len(), 2);
    }

    // --- get_season_for_date ---

    #[test]
    fn test_get_season_for_date_october_starts_new_season() {
        let date = GameDate::Date(NaiveDate::from_ymd_opt(2024, 10, 15).unwrap());
        assert_eq!(get_season_for_date(&date), 20242025);
    }

    #[test]
    fn test_get_season_for_date_december_uses_current_year_start() {
        let date = GameDate::Date(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
        assert_eq!(get_season_for_date(&date), 20242025);
    }

    #[test]
    fn test_get_season_for_date_september_uses_previous_year_start() {
        let date = GameDate::Date(NaiveDate::from_ymd_opt(2025, 9, 30).unwrap());
        assert_eq!(get_season_for_date(&date), 20242025);
    }

    #[test]
    fn test_get_season_for_date_april_uses_previous_year_start() {
        let date = GameDate::Date(NaiveDate::from_ymd_opt(2025, 4, 1).unwrap());
        assert_eq!(get_season_for_date(&date), 20242025);
    }

    #[test]
    fn test_get_season_for_date_now_matches_todays_month_boundary() {
        let today = chrono::Local::now().date_naive();
        let expected = if today.month() >= 10 {
            today.year() * 10000 + (today.year() + 1)
        } else {
            (today.year() - 1) * 10000 + today.year()
        };
        assert_eq!(get_season_for_date(&GameDate::Now), expected);
    }

    // --- format_plus_minus ---

    #[test]
    fn test_format_plus_minus_positive_has_leading_plus() {
        assert_eq!(format_plus_minus(5), "+5");
    }

    #[test]
    fn test_format_plus_minus_zero_has_no_sign() {
        assert_eq!(format_plus_minus(0), "0");
    }

    #[test]
    fn test_format_plus_minus_negative_uses_default_sign() {
        assert_eq!(format_plus_minus(-3), "-3");
    }

    // --- format_skater_stats_table ---

    #[test]
    fn test_format_skater_stats_table_with_boxscore_includes_hits_and_blocks() {
        let log = sample_game_log(HomeRoad::Home, Some(2));
        let bs = sample_boxscore_skater();
        let table = format_skater_stats_table(&log, Some(&bs), &BoxChars::unicode());

        assert!(table.contains("HIT"));
        assert!(table.contains("BLK"));
        assert!(table.contains("FO%"));
        // Values row should carry through the hits/blocked_shots from the boxscore stats
        let value_line = table.lines().nth(2).expect("value row present");
        assert!(value_line.contains(&bs.hits.to_string()));
    }

    #[test]
    fn test_format_skater_stats_table_without_boxscore_omits_hits_column() {
        let log = sample_game_log(HomeRoad::Road, None);
        let table = format_skater_stats_table(&log, None, &BoxChars::unicode());

        assert!(!table.contains("HIT"));
        assert!(!table.contains("BLK"));
        assert!(table.contains("PPP"));
    }

    #[test]
    fn test_format_skater_stats_table_defaults_missing_pim_to_zero() {
        let log = sample_game_log(HomeRoad::Home, None);
        let table = format_skater_stats_table(&log, None, &BoxChars::unicode());
        let value_line = table.lines().nth(2).expect("value row present");
        // PIM column should render as 0 when the source data has no pim value
        assert!(value_line.contains('0'));
    }

    // --- format_player_game_stats ---

    #[test]
    fn test_format_player_game_stats_home_game_uses_vs_prefix() {
        let player = sample_player("Connor McDavid", Some("EDM"));
        let log = sample_game_log(HomeRoad::Home, Some(0));
        let output = format_player_game_stats(&player, &log, None, &BoxChars::unicode());

        assert!(output.contains("vs CGY"));
        assert!(output.contains("Connor McDavid"));
        assert!(output.contains("#97"));
        assert!(output.contains("EDM"));
    }

    #[test]
    fn test_format_player_game_stats_road_game_uses_at_prefix() {
        let player = sample_player("Connor McDavid", Some("EDM"));
        let log = sample_game_log(HomeRoad::Road, Some(0));
        let output = format_player_game_stats(&player, &log, None, &BoxChars::unicode());

        assert!(output.contains("@ CGY"));
    }

    #[test]
    fn test_format_player_game_stats_no_sweater_number_omits_hash() {
        let mut player = sample_player("Connor McDavid", Some("EDM"));
        player.sweater_number = None;
        let log = sample_game_log(HomeRoad::Home, Some(0));
        let output = format_player_game_stats(&player, &log, None, &BoxChars::unicode());

        assert!(!output.contains('#'));
    }

    // --- run() integration via MockClient ---

    #[tokio::test]
    async fn test_run_errors_when_no_player_matches_query() {
        let client = MockClient::new();
        let config = Config::default();
        let result = run(&client, "Zzzznotarealplayer", None, &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No players found"));
    }

    #[tokio::test]
    async fn test_run_errors_on_invalid_date_format() {
        let client = MockClient::new();
        let config = Config::default();
        let result = run(&client, "McDavid", Some("not-a-date".to_string()), &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_succeeds_with_no_game_on_requested_date() {
        let client = MockClient::new();
        let config = Config::default();
        // McDavid's mock game log has no entry for this date
        let result = run(&client, "McDavid", Some("2024-01-01".to_string()), &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_succeeds_with_matching_game_and_no_schedule_entry() {
        let client = MockClient::new();
        let config = Config::default();
        // EDM doesn't appear in the mock schedule, so this exercises the
        // basic-stats (no boxscore) path end-to-end.
        let result = run(&client, "McDavid", Some("2024-12-28".to_string()), &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_succeeds_when_player_team_has_a_scheduled_game() {
        let client = MockClient::new();
        let config = Config::default();
        // Matthews is on TOR, which appears (live) in the mock schedule, so this
        // exercises the schedule-lookup and boxscore-fetch branches.
        let result = run(&client, "Matthews", Some("2024-12-28".to_string()), &config).await;
        assert!(result.is_ok());
    }
}
