use super::*;
use crate::fixtures;
use crate::tui::types::StackedDocument;
use nhl_api::{GameState as ApiGameState, ScheduleGame, ScheduleTeam};

fn create_test_game(id: i64, away: &str, home: &str) -> ScheduleGame {
    ScheduleGame {
        id: id.into(),
        game_type: nhl_api::GameType::RegularSeason,
        game_date: Some("2024-01-15".to_string()),
        start_time_utc: "2024-01-15T20:00:00Z".to_string(),
        game_state: ApiGameState::Final,
        game_schedule_state: nhl_api::GameScheduleState::Ok,
        away_team: ScheduleTeam {
            id: 1.into(),
            abbrev: away.to_string(),
            score: Some(2),
            logo: String::new(),
            place_name: None,
        },
        home_team: ScheduleTeam {
            id: 2.into(),
            abbrev: home.to_string(),
            score: Some(3),
            logo: String::new(),
            place_name: None,
        },
    }
}

#[test]
fn test_empty_schedule_returns_loading_animation() {
    let doc = ScoreBoxesDocument::new(
        Arc::new(None),
        Arc::new(HashMap::new()),
        2,
        GameDate::today(),
        0,
    );

    let focus = FocusContext::default();
    let elements = doc.build(&focus);

    // Should return a text element with loading animation
    assert_eq!(elements.len(), 1);
}

#[test]
fn test_single_game_layout() {
    let game = create_test_game(1, "TOR", "MTL");
    let schedule = DailySchedule {
        date: "2024-01-15".to_string(),
        games: vec![game],
        next_start_date: None,
        previous_start_date: None,
        number_of_games: 1,
    };

    let doc = ScoreBoxesDocument::new(
        Arc::new(Some(schedule)),
        Arc::new(HashMap::new()),
        2,
        GameDate::today(),
        0,
    );

    let focus = FocusContext::default();
    let elements = doc.build(&focus);

    // Should have 1 spacer + 1 row with 1 game = 2 elements
    assert_eq!(elements.len(), 2);
}

#[test]
fn test_two_games_in_row() {
    let game1 = create_test_game(1, "TOR", "MTL");
    let game2 = create_test_game(2, "BOS", "NYR");
    let schedule = DailySchedule {
        date: "2024-01-15".to_string(),
        games: vec![game1, game2],
        next_start_date: None,
        previous_start_date: None,
        number_of_games: 2,
    };

    let doc = ScoreBoxesDocument::new(
        Arc::new(Some(schedule)),
        Arc::new(HashMap::new()),
        2, // 2 boxes per row
        GameDate::today(),
        0,
    );

    let focus = FocusContext::default();
    let elements = doc.build(&focus);

    // Should have 1 spacer + 1 row with 2 games = 2 elements
    assert_eq!(elements.len(), 2);
}

#[test]
fn test_three_games_two_rows() {
    let game1 = create_test_game(1, "TOR", "MTL");
    let game2 = create_test_game(2, "BOS", "NYR");
    let game3 = create_test_game(3, "EDM", "VAN");
    let schedule = DailySchedule {
        date: "2024-01-15".to_string(),
        games: vec![game1, game2, game3],
        next_start_date: None,
        previous_start_date: None,
        number_of_games: 3,
    };

    let doc = ScoreBoxesDocument::new(
        Arc::new(Some(schedule)),
        Arc::new(HashMap::new()),
        2, // 2 boxes per row
        GameDate::today(),
        0,
    );

    let focus = FocusContext::default();
    let elements = doc.build(&focus);

    // Should have 2 spacers + 2 rows = 4 elements
    assert_eq!(elements.len(), 4);
}

#[test]
fn test_game_box_link_target_pushes_matching_boxscore() {
    let game = create_test_game(42, "TOR", "MTL");
    let schedule = DailySchedule {
        date: "2024-01-15".to_string(),
        games: vec![game],
        next_start_date: None,
        previous_start_date: None,
        number_of_games: 1,
    };
    let game_date = GameDate::from_ymd(2024, 1, 15).unwrap();

    let doc = ScoreBoxesDocument::new(
        Arc::new(Some(schedule)),
        Arc::new(HashMap::new()),
        2,
        game_date,
        0,
    );

    let targets: Vec<_> = doc
        .focusables(&FocusContext::default())
        .into_iter()
        .map(|f| f.link_target)
        .collect();
    assert_eq!(targets.len(), 1);
    match &targets[0] {
        Some(LinkTarget::Push(StackedDocument::Boxscore {
            game_id,
            away_abbrev,
            home_abbrev,
            away_score,
            home_score,
            game_date,
        })) => {
            assert_eq!(*game_id, 42);
            assert_eq!(away_abbrev, "TOR");
            assert_eq!(home_abbrev, "MTL");
            // create_test_game fixture scores (away 2, home 3).
            assert_eq!(*away_score, 2);
            assert_eq!(*home_score, 3);
            assert_eq!(game_date, "01/15");
        }
        other => panic!("expected Push(Boxscore), got {other:?}"),
    }
}

fn document_with_matchup(
    game: &ScheduleGame,
    away_score: Option<i32>,
    home_score: Option<i32>,
) -> ScoreBoxesDocument {
    let mut matchup = fixtures::create_mock_game_matchup(game.id.into());
    matchup.away_team.score = away_score;
    matchup.home_team.score = home_score;
    let game_info = HashMap::from([(i64::from(game.id), matchup)]);

    ScoreBoxesDocument::new(
        Arc::new(None),
        Arc::new(game_info),
        2,
        GameDate::from_ymd(2024, 1, 15).unwrap(),
        0,
    )
}

fn boxscore_scores(target: LinkTarget) -> (i32, i32) {
    match target {
        LinkTarget::Push(StackedDocument::Boxscore {
            away_score,
            home_score,
            ..
        }) => (away_score, home_score),
        other => panic!("expected Push(Boxscore), got {other:?}"),
    }
}

#[test]
fn test_game_info_scores_override_schedule_scores_in_link_and_box() {
    // The schedule fixture says 2-3; the (fresher) matchup says 4-1.
    let game = create_test_game(42, "TOR", "MTL");
    let doc = document_with_matchup(&game, Some(4), Some(1));

    assert_eq!(boxscore_scores(doc.build_link_target(&game)), (4, 1));
    let score_box = doc.create_score_box(&game);
    assert_eq!(score_box.away_score, Some(4));
    assert_eq!(score_box.home_score, Some(1));
}

#[test]
fn test_game_info_without_scores_links_zero_and_shows_no_score() {
    let game = create_test_game(42, "TOR", "MTL");
    let doc = document_with_matchup(&game, None, None);

    assert_eq!(boxscore_scores(doc.build_link_target(&game)), (0, 0));
    let score_box = doc.create_score_box(&game);
    assert_eq!(score_box.away_score, None);
    assert_eq!(score_box.home_score, None);
}

#[test]
fn test_title_and_id() {
    let doc = ScoreBoxesDocument::new(
        Arc::new(None),
        Arc::new(HashMap::new()),
        2,
        GameDate::today(),
        0,
    );

    assert!(doc.title().contains("Scores for"));
    assert!(doc.id().starts_with("scoreboxes_"));
}

#[test]
fn test_boxes_per_row_for_width() {
    // Gap is 8 chars, so each additional box needs 33 chars (25 + 8)

    // Width 25 = 1 box (exactly fits)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(25), 1);

    // Width 32 = 1 box (not enough for second, need 33 more)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(32), 1);

    // Width 58 = 2 boxes (25 + 8 gap + 25)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(58), 2);

    // Width 90 = 2 boxes (25 + 33 + 32 remaining, not enough for third)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(90), 2);

    // Width 91 = 3 boxes (25 + 33 + 33)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(91), 3);

    // Width 100 = 3 boxes
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(100), 3);

    // Width 200 = 6 boxes (25 + 5*33 = 190, next would be 223)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(200), 6);

    // Width 20 = 1 box (minimum)
    assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(20), 1);
}
