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
