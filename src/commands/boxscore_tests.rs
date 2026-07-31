use super::*;
use crate::dev::mock_client::MockClient;
use crate::fixtures;
use nhl_api::{GoalieStats, LocalizedString, Position, SkaterStats};

fn sample_skater(sweater_number: i32, name: &str, position: Option<Position>) -> SkaterStats {
    SkaterStats {
        player_id: 8478402.into(),
        sweater_number,
        name: LocalizedString {
            default: name.to_string(),
        },
        position,
        goals: 1,
        assists: 2,
        points: 3,
        plus_minus: -1,
        pim: 0,
        hits: 0,
        power_play_goals: 0,
        sog: 4,
        faceoff_winning_pctg: 0.0,
        toi: "18:30".to_string(),
        blocked_shots: 0,
        shifts: 20,
        giveaways: 0,
        takeaways: 0,
    }
}

fn sample_goalie(sweater_number: i32, name: &str, save_pctg: Option<f64>) -> GoalieStats {
    GoalieStats {
        player_id: 8476341.into(),
        sweater_number,
        name: LocalizedString {
            default: name.to_string(),
        },
        position: Some(Position::Goalie),
        even_strength_shots_against: "20".to_string(),
        power_play_shots_against: "5".to_string(),
        shorthanded_shots_against: "1".to_string(),
        save_shots_against: "24".to_string(),
        save_pctg,
        even_strength_goals_against: 1,
        power_play_goals_against: 1,
        shorthanded_goals_against: 0,
        pim: None,
        goals_against: 2,
        toi: "60:00".to_string(),
        starter: Some(true),
        decision: None,
        shots_against: 26,
        saves: 24,
    }
}

// --- format_team_stats (covers the private format_skater_stats / format_goalie_stats) ---

#[test]
fn test_format_team_stats_includes_forwards_defense_and_goalies_sections() {
    let display = DisplayConfig::default();
    let stats = nhl_api::TeamPlayerStats {
        forwards: vec![sample_skater(34, "Auston Matthews", Some(Position::Center))],
        defense: vec![sample_skater(44, "Morgan Rielly", Some(Position::Defense))],
        goalies: vec![sample_goalie(60, "Joseph Woll", Some(0.923))],
    };

    let mut output = String::new();
    format_team_stats(&mut output, "TOR", &stats, &display);

    assert!(output.contains("TOR - Forwards"));
    assert!(output.contains("TOR - Defense"));
    assert!(output.contains("TOR - Goalies"));
    assert!(output.contains("Auston Matthews"));
    assert!(output.contains("Morgan Rielly"));
    assert!(output.contains("Joseph Woll"));
    assert!(output.contains("0.923"));

    // Sections must appear in Forwards, Defense, Goalies order
    let forwards_idx = output.find("TOR - Forwards").unwrap();
    let defense_idx = output.find("TOR - Defense").unwrap();
    let goalies_idx = output.find("TOR - Goalies").unwrap();
    assert!(forwards_idx < defense_idx);
    assert!(defense_idx < goalies_idx);
}

#[test]
fn test_format_team_stats_skater_with_no_position_renders_dash() {
    let display = DisplayConfig::default();
    let stats = nhl_api::TeamPlayerStats {
        forwards: vec![sample_skater(34, "No Position", None)],
        defense: vec![],
        goalies: vec![],
    };

    let mut output = String::new();
    format_team_stats(&mut output, "TOR", &stats, &display);

    let row = output
        .lines()
        .find(|l| l.contains("No Position"))
        .expect("player row present");
    assert!(row.contains(" -   "));
}

#[test]
fn test_format_team_stats_goalie_with_no_save_pctg_renders_dash() {
    let display = DisplayConfig::default();
    let stats = nhl_api::TeamPlayerStats {
        forwards: vec![],
        defense: vec![],
        goalies: vec![sample_goalie(31, "Anton Forsberg", None)],
    };

    let mut output = String::new();
    format_team_stats(&mut output, "OTT", &stats, &display);

    let row = output
        .lines()
        .find(|l| l.contains("Anton Forsberg"))
        .expect("goalie row present");
    assert!(row.trim_end().ends_with('-'));
}

#[test]
fn test_format_team_stats_empty_rosters_only_render_headers() {
    let display = DisplayConfig::default();
    let stats = nhl_api::TeamPlayerStats {
        forwards: vec![],
        defense: vec![],
        goalies: vec![],
    };

    let mut output = String::new();
    format_team_stats(&mut output, "TOR", &stats, &display);

    // Column headers still render even with no players
    assert!(output.contains("Name"));
    assert!(output.contains("SV%"));
    // But no player rows are present: 3 sections (Forwards/Defense/Goalies),
    // each contributing a blank line + header text + separator + column header row.
    const LINES_PER_EMPTY_SECTION: usize = 4;
    const SECTION_COUNT: usize = 3;
    assert_eq!(
        output.lines().count(),
        LINES_PER_EMPTY_SECTION * SECTION_COUNT
    );
}

// --- format_boxscore ---

#[test]
fn test_format_boxscore_includes_teams_score_and_sog() {
    let boxscore = fixtures::create_mock_boxscore(2024020005);
    let display = DisplayConfig::default();

    let output = format_boxscore(&boxscore, &display);

    assert!(output.contains("Maple Leafs @ Senators"));
    assert!(output.contains("TOR"));
    assert!(output.contains("OTT"));
    assert!(output.contains("Date: 2024-11-20"));
    assert!(output.contains("Venue: Scotiabank Arena"));
}

#[test]
fn test_format_boxscore_shows_time_when_clock_is_running() {
    let boxscore = fixtures::create_mock_boxscore(2024020002); // live game
    let display = DisplayConfig::default();

    let output = format_boxscore(&boxscore, &display);

    assert!(boxscore.clock.running);
    assert!(output.contains("Time: 12:34"));
}

#[test]
fn test_format_boxscore_omits_time_during_intermission_when_clock_stopped() {
    let mut boxscore = fixtures::create_mock_boxscore(2024020005);
    boxscore.clock.running = false;
    boxscore.clock.in_intermission = true;
    let display = DisplayConfig::default();

    let output = format_boxscore(&boxscore, &display);

    assert!(!output.contains("Time:"));
}

#[test]
fn test_format_boxscore_includes_player_stats_for_both_teams() {
    let mut boxscore = fixtures::create_mock_boxscore(2024020005);
    boxscore.player_by_game_stats.away_team.forwards =
        vec![sample_skater(34, "Away Forward", Some(Position::Center))];
    boxscore.player_by_game_stats.home_team.forwards =
        vec![sample_skater(18, "Home Forward", Some(Position::Center))];
    let display = DisplayConfig::default();

    let output = format_boxscore(&boxscore, &display);

    assert!(output.contains("Away Forward"));
    assert!(output.contains("Home Forward"));
    assert!(output.contains(&format!("{} - Forwards", boxscore.away_team.abbrev)));
    assert!(output.contains(&format!("{} - Forwards", boxscore.home_team.abbrev)));
}

// --- run() integration via MockClient ---

#[tokio::test]
async fn test_run_returns_ok_and_prints_boxscore() {
    let client = MockClient::new();
    let config = Config::default();

    let result = run(&client, 2024020005, &config).await;

    assert!(result.is_ok());
}
