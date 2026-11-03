use super::*;
use crate::dev::mock_client::MockClient;
use crate::fixtures;
use crate::formatting::MISSING_STAT;
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
        toi: Some("18:30".to_string()),
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
        even_strength_shots_against: Some("20".to_string()),
        power_play_shots_against: Some("5".to_string()),
        shorthanded_shots_against: Some("1".to_string()),
        save_shots_against: Some("24".to_string()),
        save_pctg,
        even_strength_goals_against: Some(1),
        power_play_goals_against: Some(1),
        shorthanded_goals_against: Some(0),
        pim: None,
        goals_against: Some(2),
        toi: Some("60:00".to_string()),
        starter: Some(true),
        decision: None,
        shots_against: Some(26),
        saves: Some(24),
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
fn test_format_team_stats_missing_goalie_and_skater_stats_render_dashes() {
    // Older games omit a backup goalie's stat line and skaters' TOI.
    let display = DisplayConfig::default();
    let mut skater = sample_skater(9, "Old Timer", Some(Position::Center));
    skater.toi = None;
    let mut goalie = sample_goalie(1, "Backup Goalie", None);
    goalie.shots_against = None;
    goalie.saves = None;
    goalie.goals_against = None;
    let stats = nhl_api::TeamPlayerStats {
        forwards: vec![skater],
        defense: vec![],
        goalies: vec![goalie],
    };

    let mut output = String::new();
    format_team_stats(&mut output, "MTL", &stats, &display);

    let skater_row = output.lines().find(|l| l.contains("Old Timer")).unwrap();
    assert!(skater_row.trim_end().ends_with(MISSING_STAT));
    let goalie_row = output
        .lines()
        .find(|l| l.contains("Backup Goalie"))
        .unwrap();
    let goalie_stats: Vec<&str> = goalie_row.split_whitespace().skip(3).collect();
    assert_eq!(goalie_stats, [MISSING_STAT; 4]);
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

    assert!(boxscore.clock.as_ref().unwrap().running);
    assert!(output.contains("Time: 12:34"));
}

#[test]
fn test_format_boxscore_omits_time_during_intermission_when_clock_stopped() {
    let mut boxscore = fixtures::create_mock_boxscore(2024020005);
    let clock = boxscore.clock.as_mut().unwrap();
    clock.running = false;
    clock.in_intermission = true;
    let display = DisplayConfig::default();

    let output = format_boxscore(&boxscore, &display);

    assert!(!output.contains("Time:"));
}

#[test]
fn test_format_boxscore_scheduled_game_without_clock_or_score() {
    // Before puck drop the API sends no clock, score or SOG.
    let mut boxscore = fixtures::create_mock_boxscore(2024020005);
    boxscore.clock = None;
    boxscore.away_team.score = None;
    boxscore.away_team.sog = None;
    boxscore.home_team.score = None;
    boxscore.home_team.sog = None;
    let display = DisplayConfig::default();

    let output = format_boxscore(&boxscore, &display);

    assert!(!output.contains("Time:"));
    let away_line = format!(
        "{:<label_w$} {:>score_w$}",
        boxscore.away_team.abbrev,
        MISSING_STAT,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    );
    assert_eq!(output.matches(&away_line).count(), 2, "{output}");
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

#[cfg(feature = "game_stats")]
#[test]
fn test_format_faceoff_pct_row_shows_percentages_and_fraction() {
    let away = TeamGameStats::default().with_faceoffs(nhl_api::FaceoffTotals {
        wins: 13,
        total: 35,
    });
    let home = TeamGameStats::default().with_faceoffs(nhl_api::FaceoffTotals {
        wins: 22,
        total: 35,
    });

    let row = format_faceoff_pct_row(&away, &home, BOXSCORE_STAT_BAR_WIDTH);

    assert!(row.contains("37.1%"), "{row}");
    assert!(row.contains("13/35"), "{row}");
    assert!(row.contains("62.9%"), "{row}");
}

#[cfg(feature = "game_stats")]
#[test]
fn test_format_faceoff_pct_row_without_faceoffs_renders_dashes() {
    // The boxscore carries no faceoff counts, so the totals are absent.
    let stats = TeamGameStats::default();

    let row = format_faceoff_pct_row(&stats, &stats, BOXSCORE_STAT_BAR_WIDTH);

    let values: Vec<&str> = row.split_whitespace().skip(2).collect();
    assert_eq!(values, [MISSING_STAT; 3]);
}
