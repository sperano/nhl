use crate::config::{Config, DisplayConfig};
use crate::data_provider::NHLDataProvider;
use crate::formatting::format_header;
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
            player.position,
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

    // Shots on Goal
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        "Shots On Goal",
        away_stats.shots_on_goal,
        format_stat_bar(
            away_stats.shots_on_goal,
            home_stats.shots_on_goal,
            bar_width
        ),
        home_stats.shots_on_goal,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Face-off %
    let away_fo_pct = away_stats.faceoff_percentage();
    let home_fo_pct = home_stats.faceoff_percentage();
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$.1}%  {:^bar_w$}  {:<score_w$.1}%\n",
        "Face-off %",
        away_fo_pct,
        format!("{}/{}", away_stats.faceoff_wins, away_stats.faceoff_total),
        home_fo_pct,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Power Play %
    let away_pp_pct = away_stats.power_play_percentage();
    let home_pp_pct = home_stats.power_play_percentage();
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$.1}%  {:^bar_w$}  {:<score_w$.1}%\n",
        "Power Play %",
        away_pp_pct,
        format!(
            "{}/{}",
            away_stats.power_play_goals, away_stats.power_play_opportunities
        ),
        home_pp_pct,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Penalty Minutes
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        "Penalty Minutes",
        away_stats.penalty_minutes,
        format_stat_bar(
            away_stats.penalty_minutes,
            home_stats.penalty_minutes,
            bar_width
        ),
        home_stats.penalty_minutes,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Hits
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        "Hits",
        away_stats.hits,
        format_stat_bar(away_stats.hits, home_stats.hits, bar_width),
        home_stats.hits,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Blocked Shots
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        "Blocked Shots",
        away_stats.blocked_shots,
        format_stat_bar(
            away_stats.blocked_shots,
            home_stats.blocked_shots,
            bar_width
        ),
        home_stats.blocked_shots,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Giveaways
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        "Giveaways",
        away_stats.giveaways,
        format_stat_bar(away_stats.giveaways, home_stats.giveaways, bar_width),
        home_stats.giveaways,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
    ));

    // Takeaways
    output.push_str(&format!(
        "{:<label_w$} {:>score_w$}  {:^bar_w$}  {:<score_w$}\n",
        "Takeaways",
        away_stats.takeaways,
        format_stat_bar(away_stats.takeaways, home_stats.takeaways, bar_width),
        home_stats.takeaways,
        label_w = BOXSCORE_LABEL_WIDTH,
        score_w = BOXSCORE_SCORE_WIDTH,
        bar_w = bar_width
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
        .boxscore(game_id)
        .await
        .context("Failed to fetch boxscore")?;
    print!("{}", format_boxscore(&boxscore, &config.display));

    Ok(())
}
