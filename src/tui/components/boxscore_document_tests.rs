use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::tui::document::FocusContext;
use nhl_api::{
    Boxscore, BoxscoreTeam, GameClock, GameScheduleState, GameState, GoalieDecision, GoalieStats,
    LocalizedString, PeriodDescriptor, PeriodType, PlayerByGameStats, Position, Season,
    SkaterStats, TeamPlayerStats,
};

/// Create a test skater with minimal data
fn create_test_skater(name: &str, sweater_number: i32, position: Position) -> SkaterStats {
    SkaterStats {
        player_id: (sweater_number as i64).into(),
        name: LocalizedString {
            default: name.to_string(),
        },
        sweater_number,
        position: Some(position),
        goals: 1,
        assists: 2,
        points: 3,
        plus_minus: 1,
        pim: 2,
        hits: 3,
        power_play_goals: 0,
        sog: 4,
        faceoff_winning_pctg: 0.5,
        toi: "15:30".to_string(),
        blocked_shots: 1,
        shifts: 20,
        giveaways: 1,
        takeaways: 2,
    }
}

/// Create a test goalie with minimal data
fn create_test_goalie(name: &str, sweater_number: i32) -> GoalieStats {
    GoalieStats {
        player_id: (sweater_number as i64).into(),
        name: LocalizedString {
            default: name.to_string(),
        },
        sweater_number,
        position: Some(Position::Goalie),
        even_strength_shots_against: "20".to_string(),
        power_play_shots_against: "5".to_string(),
        shorthanded_shots_against: "0".to_string(),
        save_shots_against: "25".to_string(),
        save_pctg: Some(0.920),
        even_strength_goals_against: 1,
        power_play_goals_against: 1,
        shorthanded_goals_against: 0,
        pim: Some(0),
        goals_against: 2,
        toi: "60:00".to_string(),
        starter: Some(true),
        decision: Some(GoalieDecision::Win),
        shots_against: 25,
        saves: 23,
    }
}

fn create_test_boxscore() -> Boxscore {
    let away_forwards = vec![
        create_test_skater("A. Forward1", 10, Position::Center),
        create_test_skater("A. Forward2", 11, Position::LeftWing),
    ];
    let away_defense = vec![create_test_skater("A. Defense1", 20, Position::Defense)];
    let away_goalies = vec![create_test_goalie("A. Goalie", 30)];

    let home_forwards = vec![
        create_test_skater("H. Forward1", 12, Position::Center),
        create_test_skater("H. Forward2", 13, Position::RightWing),
    ];
    let home_defense = vec![create_test_skater("H. Defense1", 21, Position::Defense)];
    let home_goalies = vec![create_test_goalie("H. Goalie", 31)];

    Boxscore {
        id: 2024020001.into(),
        season: Season::new(2024),
        game_type: nhl_api::GameType::RegularSeason,
        limited_scoring: false,
        game_date: "2024-10-04".to_string(),
        venue: LocalizedString {
            default: "Test Arena".to_string(),
        },
        venue_location: LocalizedString {
            default: "Test City".to_string(),
        },
        start_time_utc: "2024-10-04T19:00:00Z".to_string(),
        eastern_utc_offset: "-04:00".to_string(),
        venue_utc_offset: "-04:00".to_string(),
        tv_broadcasts: vec![],
        game_state: GameState::Final,
        game_schedule_state: GameScheduleState::Ok,
        period_descriptor: PeriodDescriptor {
            number: 3,
            period_type: Some(PeriodType::Regulation),
            max_regulation_periods: 3,
        },
        special_event: None,
        away_team: BoxscoreTeam {
            id: 1.into(),
            common_name: LocalizedString {
                default: "Devils".to_string(),
            },
            abbrev: "NJD".to_string(),
            score: 3,
            sog: 30,
            logo: String::new(),
            dark_logo: String::new(),
            place_name: LocalizedString {
                default: "New Jersey".to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: "New Jersey".to_string(),
            },
        },
        home_team: BoxscoreTeam {
            id: 7.into(),
            common_name: LocalizedString {
                default: "Sabres".to_string(),
            },
            abbrev: "BUF".to_string(),
            score: 2,
            sog: 25,
            logo: String::new(),
            dark_logo: String::new(),
            place_name: LocalizedString {
                default: "Buffalo".to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: "Buffalo".to_string(),
            },
        },
        clock: GameClock {
            time_remaining: "00:00".to_string(),
            seconds_remaining: 0,
            running: false,
            in_intermission: false,
        },
        player_by_game_stats: PlayerByGameStats {
            away_team: TeamPlayerStats {
                forwards: away_forwards,
                defense: away_defense,
                goalies: away_goalies,
            },
            home_team: TeamPlayerStats {
                forwards: home_forwards,
                defense: home_defense,
                goalies: home_goalies,
            },
        },
    }
}

#[test]
fn test_document_builds_with_data() {
    let boxscore = create_test_boxscore();
    let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

    let elements = doc.build(&FocusContext::default());

    // Should have header, score, and player stats sections
    assert!(!elements.is_empty());
}

#[test]
fn test_document_metadata() {
    let boxscore = create_test_boxscore();
    let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

    assert_eq!(doc.title(), "NJD @ BUF - Game 2024020001");
    assert_eq!(doc.id(), "boxscore_2024020001");
}

/// Count TeamBoxscore elements anywhere in the built tree (they sit
/// inside a Row at side-by-side widths, at the top level otherwise).
fn count_team_boxscores(elements: &[DocumentElement]) -> usize {
    elements
        .iter()
        .map(|e| match e {
            DocumentElement::TeamBoxscore { .. } => 1,
            DocumentElement::Row { children, .. } => count_team_boxscores(children),
            _ => 0,
        })
        .sum()
}

#[test]
fn test_build_omits_team_with_no_player_stats() {
    // A team without any player stats (e.g. a game that hasn't started)
    // must not render an empty bordered shell.
    let mut boxscore = create_test_boxscore();
    boxscore.player_by_game_stats.away_team = TeamPlayerStats {
        forwards: vec![],
        defense: vec![],
        goalies: vec![],
    };
    let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

    let elements = doc.build(&FocusContext::default());
    assert_eq!(count_team_boxscores(&elements), 1);
}

#[test]
fn test_build_omits_all_boxscores_when_no_player_stats() {
    let mut boxscore = create_test_boxscore();
    let empty = || TeamPlayerStats {
        forwards: vec![],
        defense: vec![],
        goalies: vec![],
    };
    boxscore.player_by_game_stats.away_team = empty();
    boxscore.player_by_game_stats.home_team = empty();
    let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

    let elements = doc.build(&FocusContext::default());
    assert_eq!(count_team_boxscores(&elements), 0);
    assert_eq!(doc.focusables(&FocusContext::default()).len(), 0);
}

#[test]
fn test_mock_boxscore_document_renders_player_tables() {
    // End-to-end: the mock-mode boxscore document must show actual
    // player rows, not collapsed borders.
    let boxscore = crate::fixtures::create_mock_boxscore(2024020001);
    let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

    let display_config = crate::config::DisplayConfig::default();
    let ctx = RenderContext::focused(&display_config);
    let (buf, _height) = doc.render_full(120, &ctx, &FocusContext::default());

    let text = crate::tui::testing::buffer_lines(&buf).join("\n");
    for name in [
        "Matthews", "Rielly", "Woll", "Stutzle", "Chabot", "Forsberg",
    ] {
        assert!(text.contains(name), "expected player {name} in:\n{text}");
    }
}

#[test]
fn test_mock_boxscore_has_player_stats_for_both_teams() {
    // Mock mode exists to demo the UI; an empty player_by_game_stats
    // renders the boxscore document as bare border lines (regression:
    // 2026-07-08 manual smoke pass).
    let boxscore = crate::fixtures::create_mock_boxscore(2024020001);
    for team in [
        &boxscore.player_by_game_stats.away_team,
        &boxscore.player_by_game_stats.home_team,
    ] {
        assert!(!team.forwards.is_empty());
        assert!(!team.defense.is_empty());
        assert!(!team.goalies.is_empty());
    }
}

#[test]
fn test_focusable_positions() {
    let boxscore = create_test_boxscore();
    let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

    let positions = doc.focusables(&FocusContext::default());

    // Should have focusable positions for all players
    // Away: 2 forwards + 1 defense + 1 goalie = 4
    // Home: 2 forwards + 1 defense + 1 goalie = 4
    // Total = 8
    assert_eq!(positions.len(), 8);
}

#[test]
fn test_loading_state_renders() {
    let widget = BoxscoreDocumentWidget {
        document: None,
        loading: true,
        focused_id: None,
        scroll_offset: 0,
        focused: true,
        animation_frame: 0,
    };

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_no_boxscore_renders() {
    let widget = BoxscoreDocumentWidget {
        document: None,
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        focused: true,
        animation_frame: 0,
    };

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_boxscore_renders() {
    let boxscore = create_test_boxscore();
    let document: Arc<dyn Document> = Arc::new(BoxscoreDocumentContent::new(
        2024020001,
        boxscore,
        TeamView::Away,
    ));
    let widget = BoxscoreDocumentWidget {
        document: Some(document),
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        focused: true,
        animation_frame: 0,
    };

    let area = Rect::new(0, 0, 100, 50);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render without panic
    assert_eq!(*buf.area(), area);
}
