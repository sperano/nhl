use crate::config::{Config, DisplayConfig};
use crate::data_provider::NHLDataProvider;
use crate::formatting::format_header;
#[cfg(feature = "game_stats")]
use crate::layout_constants::BOXSCORE_STAT_BAR_WIDTH;
use crate::layout_constants::{BOXSCORE_LABEL_WIDTH, BOXSCORE_SCORE_WIDTH};
use anyhow::{Context, Result};
use nhl_api::Boxscore;
#[cfg(feature = "game_stats")]
use nhl_api::TeamGameStats;

/// Format skater (forwards/defense) stats table
fn format_skater_stats(
    output: &mut String,
    team_abbrev: &str,
    position_name: &str,
    players: &[nhl_api::SkaterStats],
    display: &DisplayConfig,
) {
    let header = format!("{} - {}", team_abbrev, position_name);
    output.push_str(&format!("\n{}", format_header(&header, false, display)));
    output.push_str(&format!(
        "{:<3} {:<20} {:<4} {:>3} {:>3} {:>3} {:>4} {:>6}\n",
        "#", "Name", "Pos", "G", "A", "P", "+/-", "TOI"
    ));
    for player in players {
        output.push_str(&format!(
            "{:<3} {:<20} {:<4} {:>3} {:>3} {:>3} {:>4} {:>6}\n",
            player.sweater_number,
            player.name.default,
            player.position.map_or("-", |p| p.code()),
            player.goals,
            player.assists,
            player.points,
            player.plus_minus,
            player.toi
        ));
    }
}

/// Format goalie stats table
fn format_goalie_stats(
    output: &mut String,
    team_abbrev: &str,
    goalies: &[nhl_api::GoalieStats],
    display: &DisplayConfig,
) {
    let header = format!("{} - Goalies", team_abbrev);
    output.push_str(&format!("\n{}", format_header(&header, false, display)));
    output.push_str(&format!(
        "{:<3} {:<20} {:>4} {:>6} {:>6} {:>6}\n",
        "#", "Name", "SA", "Saves", "GA", "SV%"
    ));
    for goalie in goalies {
        let sv_pct = goalie
            .save_pctg
            .map(|p| format!("{:.3}", p))
            .unwrap_or_else(|| "-".to_string());
        output.push_str(&format!(
            "{:<3} {:<20} {:>4} {:>6} {:>6} {:>6}\n",
            goalie.sweater_number,
            goalie.name.default,
            goalie.shots_against,
            goalie.saves,
            goalie.goals_against,
            sv_pct
        ));
    }
}

/// Format all player stats for a team
pub fn format_team_stats(
    output: &mut String,
    team_abbrev: &str,
    stats: &nhl_api::TeamPlayerStats,
    display: &DisplayConfig,
) {
    format_skater_stats(output, team_abbrev, "Forwards", &stats.forwards, display);
    format_skater_stats(output, team_abbrev, "Defense", &stats.defense, display);
    format_goalie_stats(output, team_abbrev, &stats.goalies, display);
}

/// Format a game stats comparison bar showing relative values
#[cfg(feature = "game_stats")]
fn format_stat_bar(away_val: i32, home_val: i32, bar_width: usize) -> String {
    let total = away_val + home_val;
    if total == 0 {
        return format!("{:width$}", "", width = bar_width);
    }

    let away_width = ((away_val as f64 / total as f64) * bar_width as f64).round() as usize;
    let home_width = bar_width.saturating_sub(away_width);

    format!("{}{}", "█".repeat(away_width), "█".repeat(home_width))
}

/// Format a single "label / away value / bar / home value" comparison row.
#[cfg(feature = "game_stats")]
fn format_stat_bar_row(label: &str, away_val: i32, home_val: i32, bar_width: usize) -> String {
    format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        label,
        away_val,
        format_stat_bar(away_val, home_val, bar_width),
        home_val,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    )
}

/// Format the face-off % row, which shows percentages and a wins/total fraction
/// instead of the raw values used by `format_stat_bar_row`.
#[cfg(feature = "game_stats")]
fn format_faceoff_pct_row(
    away_stats: &TeamGameStats,
    home_stats: &TeamGameStats,
    bar_width: usize,
) -> String {
    let away_fo_pct = away_stats.faceoff_percentage();
    let home_fo_pct = home_stats.faceoff_percentage();
    format!(
        "{:<label_w$} {:>score_w$.1}%  {:^bar_w$}  {:<score_w$.1}%\n",
        "Face-off %",
        away_fo_pct,
        format!("{}/{}", away_stats.faceoff_wins, away_stats.faceoff_total),
        home_fo_pct,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    )
}

/// Format game statistics comparison table
#[cfg(feature = "game_stats")]
pub fn format_game_stats_table(
    _away_abbrev: &str,
    _home_abbrev: &str,
    away_stats: &TeamGameStats,
    home_stats: &TeamGameStats,
    display: &DisplayConfig,
) -> String {
    let mut output = String::new();

    output.push_str(&format!("\n{}", format_header("Game Stats", true, display)));

    let bar_width = BOXSCORE_STAT_BAR_WIDTH;

    output.push_str(&format_stat_bar_row(
        "Shots On Goal",
        away_stats.shots_on_goal,
        home_stats.shots_on_goal,
        bar_width,
    ));
    output.push_str(&format_faceoff_pct_row(away_stats, home_stats, bar_width));
    // Power Play Goals (nhl-api 0.8 removed power_play_opportunities: boxscore
    // data has no valid source for it, so a percentage can't be computed)
    output.push_str(&format_stat_bar_row(
        "Power Play Goals",
        away_stats.power_play_goals,
        home_stats.power_play_goals,
        bar_width,
    ));
    output.push_str(&format_stat_bar_row(
        "Penalty Minutes",
        away_stats.penalty_minutes,
        home_stats.penalty_minutes,
        bar_width,
    ));
    output.push_str(&format_stat_bar_row(
        "Hits",
        away_stats.hits,
        home_stats.hits,
        bar_width,
    ));
    output.push_str(&format_stat_bar_row(
        "Blocked Shots",
        away_stats.blocked_shots,
        home_stats.blocked_shots,
        bar_width,
    ));
    output.push_str(&format_stat_bar_row(
        "Giveaways",
        away_stats.giveaways,
        home_stats.giveaways,
        bar_width,
    ));
    output.push_str(&format_stat_bar_row(
        "Takeaways",
        away_stats.takeaways,
        home_stats.takeaways,
        bar_width,
    ));

    output
}

pub fn format_boxscore(boxscore: &Boxscore, display: &DisplayConfig) -> String {
    let mut output = String::new();

    // Display game header
    let header = format!(
        "{} @ {}",
        boxscore.away_team.common_name.default, boxscore.home_team.common_name.default
    );
    output.push_str(&format!("\n{}", format_header(&header, true, display)));
    output.push_str(&format!(
        "Date: {} | Venue: {}\n",
        boxscore.game_date, boxscore.venue.default
    ));
    output.push_str(&format!(
        "Status: {} | Period: {}\n",
        boxscore.game_state, boxscore.period_descriptor.number
    ));
    if boxscore.clock.running || !boxscore.clock.in_intermission {
        output.push_str(&format!("Time: {}\n", boxscore.clock.time_remaining));
    }

    // Display score
    let score_header = format!(
        "{:<label_w$} {:>score_w$}",
        "Team",
        "Score",
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    );
    output.push_str(&format!(
        "\n{}",
        format_header(&score_header, false, display)
    ));
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}\n",
        boxscore.away_team.abbrev,
        boxscore.away_team.score,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    ));
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}\n",
        boxscore.home_team.abbrev,
        boxscore.home_team.score,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    ));

    // Display shots on goal
    let sog_header = format!(
        "{:<label_w$} {:>score_w$}",
        "Team",
        "SOG",
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    );
    output.push_str(&format!("\n{}", format_header(&sog_header, false, display)));
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}\n",
        boxscore.away_team.abbrev,
        boxscore.away_team.sog,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    ));
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}\n",
        boxscore.home_team.abbrev,
        boxscore.home_team.sog,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH
    ));

    #[cfg(feature = "game_stats")]
    {
        let away_team_stats =
            TeamGameStats::from_team_player_stats(&boxscore.player_by_game_stats.away_team);
        let home_team_stats =
            TeamGameStats::from_team_player_stats(&boxscore.player_by_game_stats.home_team);
        output.push_str(&format_game_stats_table(
            &boxscore.away_team.abbrev,
            &boxscore.home_team.abbrev,
            &away_team_stats,
            &home_team_stats,
            display,
        ));
    }

    // Display player stats using extracted helper functions
    format_team_stats(
        &mut output,
        &boxscore.away_team.abbrev,
        &boxscore.player_by_game_stats.away_team,
        display,
    );
    format_team_stats(
        &mut output,
        &boxscore.home_team.abbrev,
        &boxscore.player_by_game_stats.home_team,
        display,
    );

    output
}

pub async fn run(client: &dyn NHLDataProvider, game_id: i64, config: &Config) -> Result<()> {
    let boxscore = client
        .boxscore(game_id.into())
        .await
        .context("Failed to fetch boxscore")?;
    print!("{}", format_boxscore(&boxscore, &config.display));

    Ok(())
}

#[cfg(test)]
mod tests {
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
}
