use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::tui::document::FocusContext;
use nhl_api::{ClubGoalieStats, ClubSkaterStats, LocalizedString, Position, Season};
use ratatui::{buffer::Buffer, layout::Rect};

/// `stat_line` is `(games_played, goals, assists, points)`, grouped to keep the
/// argument count down since these four numbers are always supplied together.
fn create_test_skater(
    player_id: i64,
    first_name: &str,
    last_name: &str,
    position: Position,
    stat_line: (i32, i32, i32, i32),
) -> ClubSkaterStats {
    let (gp, goals, assists, points) = stat_line;
    ClubSkaterStats {
        player_id: player_id.into(),
        headshot: String::new(),
        first_name: LocalizedString {
            default: first_name.to_string(),
        },
        last_name: LocalizedString {
            default: last_name.to_string(),
        },
        position: Some(position),
        games_played: gp,
        goals,
        assists,
        points,
        plus_minus: 5,
        penalty_minutes: 10,
        power_play_goals: 2,
        shorthanded_goals: 0,
        game_winning_goals: 1,
        overtime_goals: 0,
        shots: 50,
        shooting_pctg: 0.15,
        avg_time_on_ice_per_game: 18.5,
        avg_shifts_per_game: 22.0,
        faceoff_win_pctg: 0.52,
    }
}

fn create_test_goalie(
    player_id: i64,
    first_name: &str,
    last_name: &str,
    gp: i32,
    wins: i32,
) -> ClubGoalieStats {
    ClubGoalieStats {
        player_id: player_id.into(),
        headshot: String::new(),
        first_name: LocalizedString {
            default: first_name.to_string(),
        },
        last_name: LocalizedString {
            default: last_name.to_string(),
        },
        games_played: gp,
        games_started: gp,
        wins,
        losses: 5,
        overtime_losses: 2,
        goals_against_average: 2.50,
        save_percentage: 0.915,
        shots_against: 500,
        saves: 457,
        goals_against: 43,
        shutouts: 2,
        goals: 0,
        assists: 1,
        points: 1,
        penalty_minutes: 0,
        time_on_ice: 1500,
    }
}

fn create_test_standing() -> Standing {
    Standing {
        conference_abbrev: Some("Eastern".to_string()),
        conference_name: Some("Eastern".to_string()),
        division_abbrev: "Atlantic".to_string(),
        division_name: "Atlantic".to_string(),
        team_name: LocalizedString {
            default: "Test Team".to_string(),
        },
        team_common_name: LocalizedString {
            default: "Test".to_string(),
        },
        team_abbrev: LocalizedString {
            default: "TST".to_string(),
        },
        team_logo: String::new(),
        wins: 10,
        losses: 5,
        ot_losses: 2,
        points: 22,
    }
}

fn create_test_club_stats() -> ClubStats {
    let skaters = vec![
        create_test_skater(1, "John", "Doe", Position::Center, (20, 10, 15, 25)),
        create_test_skater(2, "Jane", "Smith", Position::LeftWing, (18, 8, 12, 20)),
    ];
    let goalies = vec![create_test_goalie(3, "Bob", "Johnson", 15, 8)];

    ClubStats {
        season: Season::new(2024),
        game_type: nhl_api::GameType::RegularSeason,
        skaters,
        goalies,
    }
}

#[test]
fn test_document_builds_with_data() {
    let standing = create_test_standing();
    let club_stats = create_test_club_stats();

    let doc =
        TeamDetailDocumentContent::new("TST".to_string(), Some(standing), Some(club_stats), true);

    let elements = doc.build(&FocusContext::default());

    // Should have: heading, text (record), blank, skaters table, blank, goalies table
    assert!(elements.len() >= 4);
}

#[test]
fn test_document_metadata() {
    let standing = create_test_standing();

    let doc = TeamDetailDocumentContent::new("TST".to_string(), Some(standing), None, true);

    assert_eq!(doc.title(), "Test Team Test");
    // No club stats loaded yet: season component falls back to 0
    assert_eq!(doc.id(), "team_detail_TST_0");
}

#[test]
fn test_document_without_standing() {
    let doc = TeamDetailDocumentContent::new("TST".to_string(), None, None, true);

    assert_eq!(doc.title(), "TST");
    assert_eq!(doc.id(), "team_detail_TST_0");
}

#[test]
fn test_focusable_positions() {
    let standing = create_test_standing();
    let club_stats = create_test_club_stats();

    let doc =
        TeamDetailDocumentContent::new("TST".to_string(), Some(standing), Some(club_stats), true);

    let positions = doc.focusables(&FocusContext::default());

    // Should have 3 focusable positions: 2 skaters + 1 goalie
    assert_eq!(positions.len(), 3);
}

#[test]
fn test_focusable_ids() {
    let standing = create_test_standing();
    let club_stats = create_test_club_stats();

    let doc =
        TeamDetailDocumentContent::new("TST".to_string(), Some(standing), Some(club_stats), true);

    let ids = doc.focusables(&FocusContext::default());

    // Should have 3 focusable IDs: 2 skaters + 1 goalie
    assert_eq!(ids.len(), 3);
}

/// Regression test for buffer overflow when rendering tables with limited height.
#[test]
fn test_rendering_with_limited_height_does_not_panic() {
    let mut skaters = vec![];
    for i in 0..30 {
        skaters.push(create_test_skater(
            i,
            "Test",
            &format!("Player{}", i),
            Position::Center,
            (20, 10, 15, 25),
        ));
    }

    let mut goalies = vec![];
    for i in 0..5 {
        goalies.push(create_test_goalie(
            i + 100,
            "Test",
            &format!("Goalie{}", i),
            15,
            8,
        ));
    }

    let club_stats = ClubStats {
        season: Season::new(2024),
        game_type: nhl_api::GameType::RegularSeason,
        skaters,
        goalies,
    };

    let standing = create_test_standing();
    let document: Arc<dyn Document> = Arc::new(TeamDetailDocumentContent::new(
        "TST".to_string(),
        Some(standing),
        Some(club_stats),
        true,
    ));

    let widget = TeamDetailDocumentWidget {
        document: Some(document),
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    // Create a small area that is definitely smaller than the preferred height
    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // This should NOT panic
    widget.render(area, &mut buf, &ctx);

    assert_eq!(*buf.area(), area);
}

#[test]
fn test_rendering_at_exact_boundary_height() {
    let skaters = vec![create_test_skater(
        1,
        "John",
        "Doe",
        Position::Center,
        (20, 10, 15, 25),
    )];
    let goalies = vec![create_test_goalie(2, "Jane", "Smith", 15, 8)];

    let club_stats = ClubStats {
        season: Season::new(2024),
        game_type: nhl_api::GameType::RegularSeason,
        skaters,
        goalies,
    };

    let standing = create_test_standing();
    let document: Arc<dyn Document> = Arc::new(TeamDetailDocumentContent::new(
        "TST".to_string(),
        Some(standing),
        Some(club_stats),
        true,
    ));

    let widget = TeamDetailDocumentWidget {
        document: Some(document),
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 15);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    assert_eq!(*buf.area(), area);
}

#[test]
fn test_loading_state_renders() {
    let widget = TeamDetailDocumentWidget {
        document: None,
        loading: true,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render loading message without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_no_stats_renders() {
    let widget = TeamDetailDocumentWidget {
        document: None,
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render "no stats" message without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_season_line_shows_viewed_season() {
    let club_stats = create_test_club_stats();
    let doc = TeamDetailDocumentContent::new(
        "TST".to_string(),
        Some(create_test_standing()),
        Some(club_stats),
        true,
    );

    let has_season_line = doc.build(&FocusContext::default()).iter().any(|e| {
        matches!(e, DocumentElement::Text { content, .. }
            if content.starts_with("Season:") && content.contains("[ / ]"))
    });
    assert!(has_season_line, "season selector line must render");
}

#[test]
fn test_historical_season_hides_record_line() {
    let club_stats = create_test_club_stats();
    let doc = TeamDetailDocumentContent::new(
        "TST".to_string(),
        Some(create_test_standing()),
        Some(club_stats),
        false,
    );

    let has_record = doc.build(&FocusContext::default()).iter().any(
        |e| matches!(e, DocumentElement::Text { content, .. } if content.starts_with("Record:")),
    );
    assert!(!has_record, "record line describes the current season only");
}

#[test]
fn test_id_embeds_season() {
    let club_stats = create_test_club_stats();
    let doc = TeamDetailDocumentContent::new("TST".to_string(), None, Some(club_stats), true);

    let expected = format!("team_detail_TST_{}", create_test_club_stats().season.id());
    assert_eq!(doc.id(), expected);
}
