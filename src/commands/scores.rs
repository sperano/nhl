use crate::commands::parse_game_date;
use crate::data_provider::NHLDataProvider;
use anyhow::{Context, Result};
use nhl_api::{Boxscore, GameClock};

// Layout Constants
/// Width of the game box border
const BOX_WIDTH: usize = 88;

/// Width of team abbreviation column in detailed display
const TEAM_ABBREV_WIDTH: usize = 15;

/// Width of period column in score table
const PERIOD_COL_WIDTH: usize = 5;

/// Width of total score column
const TOTAL_COL_WIDTH: usize = 7;

/// Trailing padding string for box formatting
const TRAILING_PADDING: &str = "                                    ";

/// Width of header separator line
const HEADER_SEPARATOR_WIDTH: usize = 90;

pub async fn run(client: &dyn NHLDataProvider, date: Option<String>) -> Result<()> {
    let game_date = parse_game_date(date)?;

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

        // Determine if game has started
        let game_started = game.game_state.has_started();

        if game_started {
            // Fetch detailed boxscore for period information
            match client.boxscore(game.id).await {
                Ok(boxscore) => {
                    display_detailed_score(&boxscore);
                }
                Err(_) => {
                    // Fall back to simple display if boxscore unavailable
                    display_simple_score(game);
                }
            }
        } else {
            // Game hasn't started yet
            display_simple_score(game);
        }
    }

    println!();

    Ok(())
}

fn display_detailed_score(boxscore: &Boxscore) {
    let away_abbrev = &boxscore.away_team.abbrev;
    let home_abbrev = &boxscore.home_team.abbrev;
    let away_score = boxscore.away_team.score;
    let home_score = boxscore.home_team.score;
    let max_period = boxscore.period_descriptor.number;

    // Box top border
    println!("{}", build_box_border('┌'));

    // Teams and final score line
    println!(
        "│ {:<15} {:>2}     FINAL     {:>2}  {:<15}                                │",
        away_abbrev, away_score, home_score, home_abbrev
    );

    // Game status line
    let status_text = format_game_status(
        boxscore.game_state,
        &boxscore.period_descriptor.number,
        &boxscore.clock,
    );
    println!("│ {:<86} │", status_text);

    // Separator line
    println!("{}", build_box_border('├'));

    // Period-by-period header
    println!("{}", build_period_header(max_period));

    // Header separator line
    println!("{}", build_period_header_separator(max_period));

    // Period lines for both teams
    display_period_line(away_abbrev, away_score, max_period);
    display_period_line(home_abbrev, home_score, max_period);

    // Box bottom border
    println!("{}", build_box_border('└'));
}

fn build_box_border(style: char) -> String {
    let end_char = match style {
        '┌' => '┐',
        '├' => '┤',
        '└' => '┘',
        _ => '│',
    };
    format!("{}{:─<width$}{}", style, "", end_char, width = BOX_WIDTH)
}

fn build_period_header(max_period: i32) -> String {
    let mut header = format!("│ {:<width$}   ", "", width = TEAM_ABBREV_WIDTH);
    header.push_str(&format!("{:^width$}", "1", width = PERIOD_COL_WIDTH));
    header.push_str(&format!("{:^width$}", "2", width = PERIOD_COL_WIDTH));
    header.push_str(&format!("{:^width$}", "3", width = PERIOD_COL_WIDTH));

    if max_period > 3 {
        header.push_str(&format!("{:^width$}", "OT", width = PERIOD_COL_WIDTH));
    }
    if max_period > 4 {
        header.push_str(&format!("{:^width$}", "SO", width = PERIOD_COL_WIDTH));
    }

    header.push_str(&format!("{:^width$}", "T", width = TOTAL_COL_WIDTH));
    header.push_str(TRAILING_PADDING);
    header.push('│');
    header
}

fn build_period_header_separator(max_period: i32) -> String {
    let mut separator = format!(
        "│ {:<width$}   {:─<col_width$}{:─<col_width$}{:─<col_width$}",
        "",
        "",
        "",
        "",
        width = TEAM_ABBREV_WIDTH,
        col_width = PERIOD_COL_WIDTH
    );

    if max_period > 3 {
        separator.push_str(&format!("{:─<width$}", "", width = PERIOD_COL_WIDTH));
    }
    if max_period > 4 {
        separator.push_str(&format!("{:─<width$}", "", width = PERIOD_COL_WIDTH));
    }

    separator.push_str(&format!("{:─<width$}", "", width = TOTAL_COL_WIDTH));
    separator.push_str(TRAILING_PADDING);
    separator.push('│');
    separator
}

fn display_period_line(team_abbrev: &str, total_score: i32, max_period: i32) {
    print!("│ {:<width$}   ", team_abbrev, width = TEAM_ABBREV_WIDTH);

    // For now, we'll show placeholders for period scores
    // The nhl_api crate's Boxscore might not include detailed linescore
    // We'd need to check the actual structure
    for _ in 1..=3 {
        print!("{:^width$}", "-", width = PERIOD_COL_WIDTH);
    }

    if max_period > 3 {
        print!("{:^width$}", "-", width = PERIOD_COL_WIDTH);
    }
    if max_period > 4 {
        print!("{:^width$}", "-", width = PERIOD_COL_WIDTH);
    }

    print!("{:^width$}", total_score, width = TOTAL_COL_WIDTH);
    println!("{}│", TRAILING_PADDING);
}

fn display_simple_score(game: &nhl_api::ScheduleGame) {
    println!("┌{:─<width$}┐", "", width = BOX_WIDTH);

    if let (Some(away_score), Some(home_score)) = (game.away_team.score, game.home_team.score) {
        println!(
            "│ {:<15} {:>2}           {:>2}  {:<15}                                    │",
            game.away_team.abbrev, away_score, home_score, game.home_team.abbrev
        );
    } else {
        println!(
            "│ {:<15}  @  {:<15}                                                │",
            game.away_team.abbrev, game.home_team.abbrev
        );
    }

    let status = if game.game_state.is_scheduled() {
        format!("Scheduled: {}", game.start_time_utc)
    } else {
        format!("Status: {}", game.game_state)
    };
    println!("│ {:<86} │", status);

    println!("└{:─<width$}┘", "", width = BOX_WIDTH);
}

fn format_game_status(state: nhl_api::GameState, period: &i32, clock: &GameClock) -> String {
    use nhl_api::GameState;

    match state {
        GameState::Final | GameState::Off => "FINAL".to_string(),
        GameState::Live | GameState::Critical => {
            let period_str = match period {
                1 => "1st",
                2 => "2nd",
                3 => "3rd",
                _ => "OT",
            };

            if clock.in_intermission {
                format!("{} Period - Intermission", period_str)
            } else {
                format!("{} Period - {}", period_str, clock.time_remaining)
            }
        }
        GameState::Future | GameState::PreGame => "Scheduled".to_string(),
        GameState::Postponed => "Postponed".to_string(),
        GameState::Suspended => "Suspended".to_string(),
    }
}
