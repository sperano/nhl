use super::*;
use nhl_api::{
    AssistSummary, DefendingSide, GameScheduleState, GameSummary, GoalSummary, LocalizedString,
    MatchupTeam, PeriodDescriptor, PeriodScoring, PeriodType, Season,
};

fn make_matchup_team(abbrev: &str, score: i32) -> MatchupTeam {
    MatchupTeam {
        id: 1.into(),
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
        player_id: 1.into(),
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
        home_team_defending_side: Some(DefendingSide::Left),
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
        id: 1.into(),
        season: Season::new(2024),
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
            period_type: Some(period_type),
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
        id: 2024020001.into(),
        game_type: nhl_api::GameType::RegularSeason,
        game_date: Some("2024-11-20".to_string()),
        start_time_utc: "2024-11-21T00:00:00Z".to_string(),
        game_state,
        away_team: nhl_api::ScheduleTeam {
            id: 1.into(),
            abbrev: away_abbrev.to_string(),
            place_name: None,
            logo: String::new(),
            score: away_score,
        },
        home_team: nhl_api::ScheduleTeam {
            id: 2.into(),
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
                        period_type: Some(PeriodType::Regulation),
                        max_regulation_periods: 3,
                    },
                    goals: vec![make_goal(1, false, 1, 0)],
                },
                PeriodScoring {
                    period_descriptor: PeriodDescriptor {
                        number: 3,
                        period_type: Some(PeriodType::Regulation),
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
            shootout: vec![],
            three_stars: vec![],
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
                        period_type: Some(PeriodType::Regulation),
                        max_regulation_periods: 3,
                    },
                    goals: vec![make_goal(1, false, 1, 0)],
                },
                PeriodScoring {
                    period_descriptor: PeriodDescriptor {
                        number: 3,
                        period_type: Some(PeriodType::Regulation),
                        max_regulation_periods: 3,
                    },
                    goals: vec![
                        make_goal(3, true, 1, 1),
                        make_goal(3, true, 1, 2),
                        make_goal(3, true, 1, 3),
                    ],
                },
            ],
            shootout: vec![],
            three_stars: vec![],
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
