use crate::commands::parse_game_date;
use crate::commands::scores_format::extract_period_scores;
use crate::config;
use crate::data_provider::NHLDataProvider;
use anyhow::{Context, Result};
use nhl_api::{GameClock, GameMatchup, GameState, ScheduleGame};

// Layout Constants
/// Width of the game box border (number of horizontal dashes between the corner characters)
const BOX_WIDTH: usize = 88;

/// Width of the box's inner content area, i.e. everything between the two vertical bars
/// including the single space of padding on each side (`BOX_WIDTH` minus those 2 spaces).
const CONTENT_WIDTH: usize = BOX_WIDTH - 2;

/// Width of team abbreviation column in detailed display
const TEAM_ABBREV_WIDTH: usize = 15;

/// Width of period column in score table
const PERIOD_COL_WIDTH: usize = 5;

/// Width of total score column
const TOTAL_COL_WIDTH: usize = 7;

/// Number of spaces between the team abbreviation column and the first period column
const PERIOD_HEADER_GAP: usize = 3;

/// Width of the centered short status label (e.g. "LIVE", "FINAL") on the score summary line
const STATUS_LABEL_WIDTH: usize = 15;

/// Width of header separator line
const HEADER_SEPARATOR_WIDTH: usize = 90;

/// Last period number that is still regulation play
const REGULATION_PERIODS: i32 = 3;

/// Period number of overtime (used to decide whether to show an OT column)
const OVERTIME_PERIOD_NUMBER: i32 = 4;

/// Period number of a shootout (used to decide whether to show an SO column)
const SHOOTOUT_PERIOD_NUMBER: i32 = 5;

/// Index of the overtime score within `PeriodScores::away_periods`/`home_periods`
const OVERTIME_INDEX: usize = 3;

/// Index of the shootout score within `PeriodScores::away_periods`/`home_periods`
const SHOOTOUT_INDEX: usize = 4;

pub async fn run(client: &dyn NHLDataProvider, date: Option<String>) -> Result<()> {
    let game_date = parse_game_date(date)?;
    let config = config::read();

    let schedule = client
        .daily_schedule(Some(game_date))
        .await
        .context("Failed to fetch schedule")?;

    // Display header
    println!("\n{}", "═".repeat(HEADER_SEPARATOR_WIDTH));
    println!("NHL SCORES - {}", schedule.date);
    println!("{}\n", "═".repeat(HEADER_SEPARATOR_WIDTH));

    if schedule.number_of_games == 0 {
        println!("No games scheduled for this date.\n");
        return Ok(());
    }

    // Process each game
    for (i, game) in schedule.games.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let game_started = game.game_state.has_started();

        if game_started {
            // Fetch the game landing data, which includes the period-by-period scoring summary
            match client.landing(game.id).await {
                Ok(matchup) => print!("{}", format_detailed_score(&matchup)),
                Err(_) => print!("{}", format_simple_score(game, &config.time_format)),
            }
        } else {
            print!("{}", format_simple_score(game, &config.time_format));
        }
    }

    println!();

    Ok(())
}

/// Wrap a content string with the box's left/right borders, padding it to `CONTENT_WIDTH`
/// so every line lines up with the top/bottom borders produced by `build_box_border`.
fn content_line(content: &str) -> String {
    format!("│ {:<width$} │\n", content, width = CONTENT_WIDTH)
}

fn build_box_border(style: char) -> String {
    let end_char = match style {
        '┌' => '┐',
        '├' => '┤',
        '└' => '┘',
        _ => '│',
    };
    format!("{}{:─<width$}{}\n", style, "", end_char, width = BOX_WIDTH)
}

/// Short status word shown in the score summary line (e.g. "LIVE" instead of a stale "FINAL").
fn short_status_label(state: GameState) -> &'static str {
    match state {
        GameState::Final | GameState::Off => "FINAL",
        GameState::Live | GameState::Critical => "LIVE",
        GameState::Future | GameState::PreGame => "SCHEDULED",
        GameState::Postponed => "POSTPONED",
        GameState::Suspended => "SUSPENDED",
    }
}

fn format_detailed_score(matchup: &GameMatchup) -> String {
    let away_abbrev = &matchup.away_team.abbrev;
    let home_abbrev = &matchup.home_team.abbrev;
    let away_score = matchup.away_team.score;
    let home_score = matchup.home_team.score;
    let max_period = matchup.period_descriptor.number;

    let mut output = String::new();
    output.push_str(&build_box_border('┌'));

    output.push_str(&content_line(&format_score_summary_line(matchup)));

    let status_text = format_game_status(
        matchup.game_state,
        matchup.period_descriptor.number,
        matchup.clock.as_ref(),
    );
    output.push_str(&content_line(&status_text));

    output.push_str(&build_box_border('├'));

    output.push_str(&content_line(&build_period_header(max_period)));
    output.push_str(&content_line(&build_period_header_separator(max_period)));

    output.push_str(&format_period_score_rows(
        matchup,
        away_abbrev,
        home_abbrev,
        away_score,
        home_score,
        max_period,
    ));

    output.push_str(&build_box_border('└'));

    output
}

/// Build the "AWAY  score STATUS score  HOME" summary line shown at the top of the box.
fn format_score_summary_line(matchup: &GameMatchup) -> String {
    format!(
        "{:<team_width$} {:>2}{:^status_width$}{:>2}  {:<team_width$}",
        matchup.away_team.abbrev,
        matchup.away_team.score,
        short_status_label(matchup.game_state),
        matchup.home_team.score,
        matchup.home_team.abbrev,
        team_width = TEAM_ABBREV_WIDTH,
        status_width = STATUS_LABEL_WIDTH
    )
}

/// Build both teams' period-by-period score rows (each wrapped in `content_line`).
///
/// Real per-period scores come from the game summary's scoring-by-period breakdown.
/// If the summary isn't available yet (e.g. moments after puck drop), falls back to
/// showing dashes for every period rather than fabricating data.
fn format_period_score_rows(
    matchup: &GameMatchup,
    away_abbrev: &str,
    home_abbrev: &str,
    away_score: i32,
    home_score: i32,
    max_period: i32,
) -> String {
    let mut output = String::new();
    match &matchup.summary {
        Some(summary) => {
            let period_scores = extract_period_scores(summary);
            output.push_str(&content_line(&format_period_row(
                away_abbrev,
                &period_scores.away_periods,
                true,
                away_score,
                max_period,
            )));
            output.push_str(&content_line(&format_period_row(
                home_abbrev,
                &period_scores.home_periods,
                true,
                home_score,
                max_period,
            )));
        }
        None => {
            output.push_str(&content_line(&format_period_row(
                away_abbrev,
                &[],
                false,
                away_score,
                max_period,
            )));
            output.push_str(&content_line(&format_period_row(
                home_abbrev,
                &[],
                false,
                home_score,
                max_period,
            )));
        }
    }
    output
}

/// Build the period-column header content (e.g. `TEAM   1  2  3  OT SO  T`).
fn build_period_header(max_period: i32) -> String {
    let mut header = format!(
        "{:<width$}{}",
        "",
        " ".repeat(PERIOD_HEADER_GAP),
        width = TEAM_ABBREV_WIDTH
    );
    header.push_str(&format!("{:^width$}", "1", width = PERIOD_COL_WIDTH));
    header.push_str(&format!("{:^width$}", "2", width = PERIOD_COL_WIDTH));
    header.push_str(&format!("{:^width$}", "3", width = PERIOD_COL_WIDTH));

    if max_period > REGULATION_PERIODS {
        header.push_str(&format!("{:^width$}", "OT", width = PERIOD_COL_WIDTH));
    }
    if max_period > OVERTIME_PERIOD_NUMBER {
        header.push_str(&format!("{:^width$}", "SO", width = PERIOD_COL_WIDTH));
    }

    header.push_str(&format!("{:^width$}", "T", width = TOTAL_COL_WIDTH));
    header
}

/// Build the dashed separator line shown directly under the period-column header.
fn build_period_header_separator(max_period: i32) -> String {
    let mut separator = format!(
        "{:<width$}{}",
        "",
        " ".repeat(PERIOD_HEADER_GAP),
        width = TEAM_ABBREV_WIDTH
    );
    separator.push_str(&"─".repeat(PERIOD_COL_WIDTH)); // Period 1
    separator.push_str(&"─".repeat(PERIOD_COL_WIDTH)); // Period 2
    separator.push_str(&"─".repeat(PERIOD_COL_WIDTH)); // Period 3

    if max_period > REGULATION_PERIODS {
        separator.push_str(&"─".repeat(PERIOD_COL_WIDTH));
    }
    if max_period > OVERTIME_PERIOD_NUMBER {
        separator.push_str(&"─".repeat(PERIOD_COL_WIDTH));
    }

    separator.push_str(&"─".repeat(TOTAL_COL_WIDTH));
    separator
}

/// Build a single team's period-by-period score row.
///
/// `periods` holds real per-period scores (indexed by `PERIOD_*_INDEX`/`OVERTIME_INDEX`/
/// `SHOOTOUT_INDEX`, matching `PeriodScores`'s layout) and `has_data` indicates whether that
/// data is available at all. Columns for periods beyond `max_period` (i.e. periods that
/// haven't happened yet) always show a dash, since showing "0" there would misleadingly
/// imply the period is over.
/// Value (or "-" placeholder) for one period's score cell.
///
/// Columns for periods beyond `max_period` (i.e. periods that haven't happened yet)
/// always show a dash, since showing "0" there would misleadingly imply the period
/// is over.
fn period_row_value(
    periods: &[i32],
    has_data: bool,
    period_num: i32,
    index: usize,
    max_period: i32,
) -> String {
    if has_data && period_num <= max_period {
        periods
            .get(index)
            .map(|score| score.to_string())
            .unwrap_or_else(|| "-".to_string())
    } else {
        "-".to_string()
    }
}

fn format_period_row(
    team_abbrev: &str,
    periods: &[i32],
    has_data: bool,
    total_score: i32,
    max_period: i32,
) -> String {
    let mut row = format!(
        "{:<width$}{}",
        team_abbrev,
        " ".repeat(PERIOD_HEADER_GAP),
        width = TEAM_ABBREV_WIDTH
    );

    let cell = |period_num: i32, index: usize| {
        format!(
            "{:^width$}",
            period_row_value(periods, has_data, period_num, index, max_period),
            width = PERIOD_COL_WIDTH
        )
    };

    row.push_str(&cell(1, 0));
    row.push_str(&cell(2, 1));
    row.push_str(&cell(3, 2));

    if max_period > REGULATION_PERIODS {
        row.push_str(&cell(OVERTIME_PERIOD_NUMBER, OVERTIME_INDEX));
    }
    if max_period > OVERTIME_PERIOD_NUMBER {
        row.push_str(&cell(SHOOTOUT_PERIOD_NUMBER, SHOOTOUT_INDEX));
    }

    row.push_str(&format!("{:^width$}", total_score, width = TOTAL_COL_WIDTH));
    row
}

fn format_simple_score(game: &ScheduleGame, time_format: &str) -> String {
    let mut output = String::new();
    output.push_str(&build_box_border('┌'));

    if let (Some(away_score), Some(home_score)) = (game.away_team.score, game.home_team.score) {
        let score_summary = format!(
            "{:<width$} {:>2}  -  {:>2}  {:<width$}",
            game.away_team.abbrev,
            away_score,
            home_score,
            game.home_team.abbrev,
            width = TEAM_ABBREV_WIDTH
        );
        output.push_str(&content_line(&score_summary));
    } else {
        let matchup_summary = format!(
            "{:<width$}  @  {:<width$}",
            game.away_team.abbrev,
            game.home_team.abbrev,
            width = TEAM_ABBREV_WIDTH
        );
        output.push_str(&content_line(&matchup_summary));
    }

    let status = if game.game_state.is_scheduled() {
        format!(
            "Scheduled: {}",
            crate::commands::format_local_time(&game.start_time_utc, time_format)
        )
    } else {
        format!("Status: {}", game.game_state)
    };
    output.push_str(&content_line(&status));

    output.push_str(&build_box_border('└'));

    output
}

fn format_game_status(state: GameState, period: i32, clock: Option<&GameClock>) -> String {
    match state {
        GameState::Final | GameState::Off => "FINAL".to_string(),
        GameState::Live | GameState::Critical => {
            let period_str = match period {
                1 => "1st",
                2 => "2nd",
                3 => "3rd",
                _ => "OT",
            };

            match clock {
                Some(clock) if clock.in_intermission => {
                    format!("{} Period - Intermission", period_str)
                }
                Some(clock) => format!("{} Period - {}", period_str, clock.time_remaining),
                None => format!("{} Period", period_str),
            }
        }
        GameState::Future | GameState::PreGame => "Scheduled".to_string(),
        GameState::Postponed => "Postponed".to_string(),
        GameState::Suspended => "Suspended".to_string(),
    }
}

#[cfg(test)]
#[path = "scores_tests.rs"]
mod tests;
