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

    let score_summary = format!(
        "{:<team_width$} {:>2}{:^status_width$}{:>2}  {:<team_width$}",
        away_abbrev,
        away_score,
        short_status_label(matchup.game_state),
        home_score,
        home_abbrev,
        team_width = TEAM_ABBREV_WIDTH,
        status_width = STATUS_LABEL_WIDTH
    );
    output.push_str(&content_line(&score_summary));

    let status_text = format_game_status(
        matchup.game_state,
        matchup.period_descriptor.number,
        matchup.clock.as_ref(),
    );
    output.push_str(&content_line(&status_text));

    output.push_str(&build_box_border('├'));

    output.push_str(&content_line(&build_period_header(max_period)));
    output.push_str(&content_line(&build_period_header_separator(max_period)));

    // Real per-period scores come from the game summary's scoring-by-period breakdown.
    // If the summary isn't available yet (e.g. moments after puck drop), fall back to
    // showing dashes for every period rather than fabricating data.
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

    output.push_str(&build_box_border('└'));

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

    let period_value = |period_num: i32, index: usize| -> String {
        if has_data && period_num <= max_period {
            periods
                .get(index)
                .map(|score| score.to_string())
                .unwrap_or_else(|| "-".to_string())
        } else {
            "-".to_string()
        }
    };

    row.push_str(&format!(
        "{:^width$}",
        period_value(1, 0),
        width = PERIOD_COL_WIDTH
    ));
    row.push_str(&format!(
        "{:^width$}",
        period_value(2, 1),
        width = PERIOD_COL_WIDTH
    ));
    row.push_str(&format!(
        "{:^width$}",
        period_value(3, 2),
        width = PERIOD_COL_WIDTH
    ));

    if max_period > REGULATION_PERIODS {
        row.push_str(&format!(
            "{:^width$}",
            period_value(OVERTIME_PERIOD_NUMBER, OVERTIME_INDEX),
            width = PERIOD_COL_WIDTH
        ));
    }
    if max_period > OVERTIME_PERIOD_NUMBER {
        row.push_str(&format!(
            "{:^width$}",
            period_value(SHOOTOUT_PERIOD_NUMBER, SHOOTOUT_INDEX),
            width = PERIOD_COL_WIDTH
        ));
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
mod tests {
    use super::*;
    use nhl_api::{
        AssistSummary, DefendingSide, GameScheduleState, GameSummary, GoalSummary, LocalizedString,
        MatchupTeam, PeriodDescriptor, PeriodScoring, PeriodType,
    };

    fn make_matchup_team(abbrev: &str, score: i32) -> MatchupTeam {
        MatchupTeam {
            id: 1,
            common_name: LocalizedString {
                default: abbrev.to_string(),
            },
            abbrev: abbrev.to_string(),
            place_name: LocalizedString {
                default: abbrev.to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: abbrev.to_string(),
            },
            score,
            sog: 0,
            logo: String::new(),
            dark_logo: String::new(),
        }
    }

    fn make_goal(_period_num: i32, is_home: bool, away_score: i32, home_score: i32) -> GoalSummary {
        GoalSummary {
            situation_code: "1551".to_string(),
            event_id: 1,
            strength: "ev".to_string(),
            player_id: 1,
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: "Player".to_string(),
            },
            name: LocalizedString {
                default: "Test Player".to_string(),
            },
            team_abbrev: LocalizedString {
                default: "TST".to_string(),
            },
            headshot: String::new(),
            highlight_clip_sharing_url: None,
            highlight_clip: None,
            discrete_clip: None,
            goals_to_date: None,
            away_score,
            home_score,
            leading_team_abbrev: None,
            time_in_period: "05:00".to_string(),
            shot_type: "wrist".to_string(),
            goal_modifier: "none".to_string(),
            assists: Vec::<AssistSummary>::new(),
            home_team_defending_side: DefendingSide::Left,
            is_home,
        }
    }

    fn make_matchup(
        game_state: GameState,
        period_number: i32,
        period_type: PeriodType,
        away_score: i32,
        home_score: i32,
        summary: Option<GameSummary>,
        clock: Option<GameClock>,
    ) -> GameMatchup {
        GameMatchup {
            id: 1,
            season: 20242025,
            game_type: nhl_api::GameType::RegularSeason,
            limited_scoring: false,
            game_date: "2024-11-20".to_string(),
            venue: LocalizedString {
                default: "Test Arena".to_string(),
            },
            venue_location: LocalizedString {
                default: "Test City".to_string(),
            },
            start_time_utc: "2024-11-21T00:00:00Z".to_string(),
            eastern_utc_offset: "-05:00".to_string(),
            venue_utc_offset: "-05:00".to_string(),
            venue_timezone: "America/New_York".to_string(),
            period_descriptor: PeriodDescriptor {
                number: period_number,
                period_type,
                max_regulation_periods: 3,
            },
            tv_broadcasts: vec![],
            game_state,
            game_schedule_state: GameScheduleState::Ok,
            special_event: None,
            away_team: make_matchup_team("AWY", away_score),
            home_team: make_matchup_team("HME", home_score),
            shootout_in_use: true,
            max_periods: 5,
            reg_periods: 3,
            ot_in_use: true,
            ties_in_use: false,
            summary,
            clock,
        }
    }

    fn make_schedule_game(
        away_abbrev: &str,
        home_abbrev: &str,
        game_state: GameState,
        away_score: Option<i32>,
        home_score: Option<i32>,
    ) -> ScheduleGame {
        ScheduleGame {
            id: 2024020001,
            game_type: nhl_api::GameType::RegularSeason,
            game_date: Some("2024-11-20".to_string()),
            start_time_utc: "2024-11-21T00:00:00Z".to_string(),
            game_state,
            away_team: nhl_api::ScheduleTeam {
                id: 1,
                abbrev: away_abbrev.to_string(),
                place_name: None,
                logo: String::new(),
                score: away_score,
            },
            home_team: nhl_api::ScheduleTeam {
                id: 2,
                abbrev: home_abbrev.to_string(),
                place_name: None,
                logo: String::new(),
                score: home_score,
            },
        }
    }

    #[test]
    fn test_content_line_matches_border_width() {
        let border = build_box_border('┌');
        let content = content_line("hello");
        // Both lines (including trailing newline) should have identical character counts,
        // proving the box's left/right edges stay aligned.
        assert_eq!(border.chars().count(), content.chars().count());
    }

    #[test]
    fn test_build_box_border_variants() {
        assert!(build_box_border('┌').starts_with('┌'));
        assert!(build_box_border('┌').trim_end().ends_with('┐'));
        assert!(build_box_border('├').starts_with('├'));
        assert!(build_box_border('├').trim_end().ends_with('┤'));
        assert!(build_box_border('└').starts_with('└'));
        assert!(build_box_border('└').trim_end().ends_with('┘'));
    }

    #[test]
    fn test_short_status_label() {
        assert_eq!(short_status_label(GameState::Final), "FINAL");
        assert_eq!(short_status_label(GameState::Off), "FINAL");
        assert_eq!(short_status_label(GameState::Live), "LIVE");
        assert_eq!(short_status_label(GameState::Critical), "LIVE");
        assert_eq!(short_status_label(GameState::Future), "SCHEDULED");
    }

    #[test]
    fn test_format_game_status_live_with_clock() {
        let clock = GameClock {
            time_remaining: "12:34".to_string(),
            seconds_remaining: 754,
            running: true,
            in_intermission: false,
        };
        let status = format_game_status(GameState::Live, 1, Some(&clock));
        assert_eq!(status, "1st Period - 12:34");
    }

    #[test]
    fn test_format_game_status_intermission() {
        let clock = GameClock {
            time_remaining: "00:00".to_string(),
            seconds_remaining: 0,
            running: false,
            in_intermission: true,
        };
        let status = format_game_status(GameState::Live, 2, Some(&clock));
        assert_eq!(status, "2nd Period - Intermission");
    }

    #[test]
    fn test_format_game_status_no_clock_available() {
        let status = format_game_status(GameState::Live, 3, None);
        assert_eq!(status, "3rd Period");
    }

    #[test]
    fn test_format_game_status_final() {
        assert_eq!(format_game_status(GameState::Final, 3, None), "FINAL");
    }

    #[test]
    fn test_format_game_status_scheduled() {
        assert_eq!(format_game_status(GameState::Future, 0, None), "Scheduled");
    }

    #[test]
    fn test_build_period_header_regulation_only() {
        let header = build_period_header(3);
        assert!(header.contains('1'));
        assert!(header.contains('2'));
        assert!(header.contains('3'));
        assert!(header.contains('T'));
        assert!(!header.contains("OT"));
        assert!(!header.contains("SO"));
    }

    #[test]
    fn test_build_period_header_with_overtime() {
        let header = build_period_header(4);
        assert!(header.contains("OT"));
        assert!(!header.contains("SO"));
    }

    #[test]
    fn test_build_period_header_with_shootout() {
        let header = build_period_header(5);
        assert!(header.contains("OT"));
        assert!(header.contains("SO"));
    }

    #[test]
    fn test_format_period_row_shows_real_scores_up_to_current_period() {
        // Real per-period data: 1 goal in P1, 0 in P2, 2 in P3
        let periods = vec![1, 0, 2];
        let row = format_period_row("TOR", &periods, true, 3, 3);

        // No more placeholder dashes for periods that have already happened.
        assert!(row.contains('1'));
        assert!(row.contains('0'));
        assert!(row.contains('2'));
    }

    #[test]
    fn test_format_period_row_dashes_for_future_periods() {
        // Currently in period 1; periods 2 and 3 haven't happened yet.
        let periods = vec![1, 0, 0];
        let row = format_period_row("TOR", &periods, true, 1, 1);

        // Column layout: team(15) + gap(3) + P1(5) + P2(5) + P3(5) + T(7)
        let p2_start = TEAM_ABBREV_WIDTH + PERIOD_HEADER_GAP + PERIOD_COL_WIDTH;
        let p2_column = &row[p2_start..p2_start + PERIOD_COL_WIDTH];
        assert!(
            p2_column.contains('-'),
            "P2 column should be a dash, got: {p2_column:?}"
        );
    }

    #[test]
    fn test_format_period_row_no_data_shows_all_dashes() {
        let row = format_period_row("TOR", &[], false, 3, 3);
        let team_and_gap = TEAM_ABBREV_WIDTH + PERIOD_HEADER_GAP;
        let period_columns = &row[team_and_gap..team_and_gap + PERIOD_COL_WIDTH * 3];
        assert!(!period_columns.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_format_simple_score_scheduled_game_uses_configured_time_format() {
        let game = make_schedule_game("BOS", "MTL", GameState::Future, None, None);
        let output = format_simple_score(&game, "%H:%M:%S");

        assert!(output.contains("BOS"));
        assert!(output.contains("MTL"));
        assert!(output.contains("Scheduled:"));
        assert!(!output.contains("AM") && !output.contains("PM"));
    }

    #[test]
    fn test_format_simple_score_with_scores() {
        let game = make_schedule_game("BOS", "MTL", GameState::Final, Some(4), Some(2));
        let output = format_simple_score(&game, "%H:%M:%S");

        assert!(output.contains("BOS"));
        assert!(output.contains('4'));
        assert!(output.contains('2'));
        assert!(output.contains("MTL"));
    }

    #[test]
    fn test_format_simple_score_lines_align_with_border() {
        let game = make_schedule_game("BOS", "MTL", GameState::Future, None, None);
        let output = format_simple_score(&game, "%H:%M:%S");
        let widths: Vec<usize> = output.lines().map(|line| line.chars().count()).collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "all lines should be the same width, got: {widths:?}"
        );
    }

    #[test]
    fn test_format_detailed_score_lines_align_with_border() {
        let matchup = make_matchup(
            GameState::Final,
            3,
            PeriodType::Regulation,
            2,
            4,
            Some(GameSummary {
                scoring: vec![
                    PeriodScoring {
                        period_descriptor: PeriodDescriptor {
                            number: 1,
                            period_type: PeriodType::Regulation,
                            max_regulation_periods: 3,
                        },
                        goals: vec![make_goal(1, false, 1, 0)],
                    },
                    PeriodScoring {
                        period_descriptor: PeriodDescriptor {
                            number: 3,
                            period_type: PeriodType::Regulation,
                            max_regulation_periods: 3,
                        },
                        goals: vec![
                            make_goal(3, true, 1, 1),
                            make_goal(3, true, 1, 2),
                            make_goal(3, false, 2, 2),
                            make_goal(3, true, 2, 3),
                            make_goal(3, true, 2, 4),
                        ],
                    },
                ],
                shootout: None,
                three_stars: None,
                penalties: vec![],
            }),
            None,
        );

        let output = format_detailed_score(&matchup);
        let widths: Vec<usize> = output.lines().map(|line| line.chars().count()).collect();
        assert!(
            widths.windows(2).all(|w| w[0] == w[1]),
            "all lines should be the same width, got: {widths:?}"
        );
    }

    #[test]
    fn test_format_detailed_score_wires_real_period_data() {
        // Away team scores once in P1; home team scores 3 times in P3.
        let matchup = make_matchup(
            GameState::Final,
            3,
            PeriodType::Regulation,
            1,
            3,
            Some(GameSummary {
                scoring: vec![
                    PeriodScoring {
                        period_descriptor: PeriodDescriptor {
                            number: 1,
                            period_type: PeriodType::Regulation,
                            max_regulation_periods: 3,
                        },
                        goals: vec![make_goal(1, false, 1, 0)],
                    },
                    PeriodScoring {
                        period_descriptor: PeriodDescriptor {
                            number: 3,
                            period_type: PeriodType::Regulation,
                            max_regulation_periods: 3,
                        },
                        goals: vec![
                            make_goal(3, true, 1, 1),
                            make_goal(3, true, 1, 2),
                            make_goal(3, true, 1, 3),
                        ],
                    },
                ],
                shootout: None,
                three_stars: None,
                penalties: vec![],
            }),
            None,
        );

        let output = format_detailed_score(&matchup);
        let lines: Vec<&str> = output.lines().collect();
        // Layout: 0 top border, 1 score summary, 2 status, 3 mid border, 4 period header,
        // 5 header separator, 6 away period row, 7 home period row, 8 bottom border.
        // The score summary line (index 1) also contains "HME", so we must address the
        // home team's period row by its known position rather than searching for the
        // abbreviation. Use char-based indexing (not byte slicing) since the box border is
        // drawn with multi-byte box-drawing characters.
        let home_row = lines[7];
        let border_prefix_chars = 2; // "│ "
        let team_and_gap = TEAM_ABBREV_WIDTH + PERIOD_HEADER_GAP;
        let period_columns: String = home_row
            .chars()
            .skip(border_prefix_chars + team_and_gap)
            .take(PERIOD_COL_WIDTH * 3)
            .collect();
        assert!(
            period_columns.contains('3'),
            "expected a real P3 score of 3, got columns: {period_columns:?}"
        );
    }

    #[test]
    fn test_format_detailed_score_no_summary_shows_dashes_not_zeros() {
        let matchup = make_matchup(
            GameState::Live,
            1,
            PeriodType::Regulation,
            0,
            0,
            None,
            Some(GameClock {
                time_remaining: "15:00".to_string(),
                seconds_remaining: 900,
                running: true,
                in_intermission: false,
            }),
        );

        let output = format_detailed_score(&matchup);
        assert!(output.contains("1st Period - 15:00"));
        // Without summary data we must not fabricate zeros for periods. The score summary
        // line (index 1) also contains both abbreviations, so address the two actual period
        // rows (indices 6 and 7) directly rather than searching by abbreviation.
        let lines: Vec<&str> = output.lines().collect();
        for line in [lines[6], lines[7]] {
            assert!(line.contains('-'), "expected dashes, got: {line:?}");
        }
    }
}
