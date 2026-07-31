use super::*;
use crate::tui::document_nav::DocumentNavState;

fn make_entry(document: StackedDocument, focus_index: Option<usize>) -> DocumentStackEntry {
    DocumentStackEntry {
        document,
        nav: DocumentNavState {
            focus_index,
            ..Default::default()
        },
    }
}

#[test]
fn test_push_document() {
    let state = AppState::default();
    let panel = StackedDocument::TeamDetail {
        abbrev: "BOS".to_string(),
        season: None,
    };

    let (new_state, effect) = push_document(state, panel.clone());

    assert_eq!(new_state.navigation.document_stack.len(), 1);
    assert_eq!(
        new_state.navigation.document_stack[0].nav.focus_index,
        Some(0)
    );
    // Should return fetch effect since we don't have the data
    assert!(matches!(
        effect,
        Effect::FetchTeamRosterStats(ref abbrev, None) if abbrev == "BOS"
    ));
}

fn test_boxscore(game_id: i64) -> StackedDocument {
    StackedDocument::Boxscore {
        game_id,
        away_abbrev: "TOR".to_string(),
        home_abbrev: "BOS".to_string(),
        away_score: 0,
        home_score: 0,
        game_date: "12/24".to_string(),
    }
}

#[test]
fn test_push_document_boxscore_returns_fetch_effect() {
    let state = AppState::default();
    let game_id = 2024020001;
    let panel = test_boxscore(game_id);

    let (new_state, effect) = push_document(state, panel);

    assert_eq!(new_state.navigation.document_stack.len(), 1);
    assert!(matches!(effect, Effect::FetchBoxscore(id) if id == game_id));
    // The dispatched fetch must mark the key as loading, or the loading
    // animation never fires and a rapid re-push would re-fetch.
    assert!(new_state
        .data
        .loading
        .contains(&LoadingKey::Boxscore(game_id)));
}

#[test]
fn test_push_document_team_detail_marks_loading() {
    let state = AppState::default();
    let panel = StackedDocument::TeamDetail {
        abbrev: "BOS".to_string(),
        season: None,
    };

    let (new_state, effect) = push_document(state, panel);

    assert!(matches!(
        effect,
        Effect::FetchTeamRosterStats(ref abbrev, None) if abbrev == "BOS"
    ));
    assert!(new_state
        .data
        .loading
        .contains(&LoadingKey::TeamRosterStats("BOS".to_string(), None)));
}

#[test]
fn test_push_document_player_detail_marks_loading() {
    let state = AppState::default();
    let player_id = 8478402;
    let panel = StackedDocument::PlayerDetail {
        player_id,
        sweater_number: Some(87),
        last_name: "Crosby".to_string(),
    };

    let (new_state, effect) = push_document(state, panel);

    assert!(matches!(effect, Effect::FetchPlayerStats(id) if id == player_id));
    assert!(new_state
        .data
        .loading
        .contains(&LoadingKey::PlayerStats(player_id)));
}

#[test]
fn test_push_document_no_fetch_if_already_loading() {
    let mut state = AppState::default();
    let game_id = 2024020001;

    // Mark as already loading
    state.data.loading.insert(LoadingKey::Boxscore(game_id));

    let panel = test_boxscore(game_id);
    let (_new_state, effect) = push_document(state, panel);

    // Should NOT return fetch effect since we're already loading
    assert!(matches!(effect, Effect::None));
}

#[test]
fn test_pop_document_clears_loading_state() {
    let mut state = AppState::default();
    let game_id = 2024020001;

    // Push a boxscore panel and add loading state
    state
        .navigation
        .document_stack
        .push(make_entry(test_boxscore(game_id), None));
    state.data.loading.insert(LoadingKey::Boxscore(game_id));

    let (new_state, _) = pop_document(state);

    assert!(new_state.navigation.document_stack.is_empty());
    assert!(!new_state
        .data
        .loading
        .contains(&LoadingKey::Boxscore(game_id)));
}

// ========================================================================
// Season-aware push, cycling, and pop cleanup
// ========================================================================

use std::collections::HashMap;
use std::sync::Arc;

fn club_stats(season: i32) -> nhl_api::ClubStats {
    crate::fixtures::create_mock_club_stats("BOS", season, nhl_api::GameType::RegularSeason)
}

/// State with BOS seasons [20222023, 20232024, 20242025] known and stats
/// loaded for the given seasons.
fn state_with_seasons(loaded: &[i32]) -> AppState {
    let mut state = AppState::default();
    let mut seasons = HashMap::new();
    seasons.insert("BOS".to_string(), vec![20222023, 20232024, 20242025]);
    state.data.team_seasons = Arc::new(seasons);
    let mut stats = HashMap::new();
    for &s in loaded {
        stats.insert(("BOS".to_string(), s), club_stats(s));
    }
    state.data.team_roster_stats = Arc::new(stats);
    state
}

fn push_team_detail(state: AppState, season: Option<i32>) -> (AppState, Effect) {
    push_document(
        state,
        StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season,
        },
    )
}

fn top_season(state: &AppState) -> Option<i32> {
    match &state.navigation.document_stack.last().unwrap().document {
        StackedDocument::TeamDetail { season, .. } => *season,
        other => panic!("expected TeamDetail on top, got {other:?}"),
    }
}

fn cycle(state: AppState, key_char: char) -> (AppState, Effect) {
    stacked_document_key(
        state,
        crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char(key_char)),
    )
}

#[test]
fn test_push_normalizes_latest_when_seasons_known_and_data_cached() {
    let state = state_with_seasons(&[20242025]);

    let (new_state, effect) = push_team_detail(state, None);

    assert_eq!(top_season(&new_state), Some(20242025));
    assert!(matches!(effect, Effect::None), "cached data: no fetch");
    assert!(new_state.data.loading.is_empty());
}

#[test]
fn test_push_normalizes_latest_and_fetches_when_data_missing() {
    let state = state_with_seasons(&[]);

    let (new_state, effect) = push_team_detail(state, None);

    assert_eq!(top_season(&new_state), Some(20242025));
    assert!(matches!(
        effect,
        Effect::FetchTeamRosterStats(ref a, Some(20242025)) if a == "BOS"
    ));
    assert!(new_state
        .data
        .loading
        .contains(&LoadingKey::TeamRosterStats(
            "BOS".to_string(),
            Some(20242025)
        )));
}

#[test]
fn test_cycle_prev_moves_season_resets_nav_and_fetches() {
    let state = state_with_seasons(&[20242025]);
    let (mut state, _) = push_team_detail(state, None);
    // Simulate the user having navigated within the document
    let entry = state.navigation.document_stack.last_mut().unwrap();
    entry.nav.focus_index = Some(3);
    entry.nav.scroll_offset = 7;
    entry.nav.viewport_height = 24;

    let (new_state, effect) = cycle(state, '[');

    assert_eq!(top_season(&new_state), Some(20232024));
    let nav = &new_state.navigation.document_stack.last().unwrap().nav;
    assert_eq!(nav.focus_index, Some(0));
    assert_eq!(nav.scroll_offset, 0);
    assert_eq!(nav.viewport_height, 24, "viewport height survives");
    assert!(matches!(
        effect,
        Effect::FetchTeamRosterStats(ref a, Some(20232024)) if a == "BOS"
    ));
    assert!(new_state
        .data
        .loading
        .contains(&LoadingKey::TeamRosterStats(
            "BOS".to_string(),
            Some(20232024)
        )));
}

#[test]
fn test_cycle_to_cached_season_does_not_fetch() {
    let state = state_with_seasons(&[20232024, 20242025]);
    let (state, _) = push_team_detail(state, None);

    let (new_state, effect) = cycle(state, '[');

    assert_eq!(top_season(&new_state), Some(20232024));
    assert!(matches!(effect, Effect::None));
    assert!(new_state.data.loading.is_empty());
}

#[test]
fn test_cycle_clamps_at_both_ends() {
    let state = state_with_seasons(&[20222023, 20232024, 20242025]);
    let (state, _) = push_team_detail(state, None);

    // `]` at the latest season: no-op
    let (state, effect) = cycle(state, ']');
    assert!(matches!(effect, Effect::None));
    assert_eq!(top_season(&state), Some(20242025));

    // Walk to the oldest, then `[` again: no-op
    let (state, _) = cycle(state, '[');
    let (state, _) = cycle(state, '[');
    assert_eq!(top_season(&state), Some(20222023));
    let (state, effect) = cycle(state, '[');
    assert!(matches!(effect, Effect::None));
    assert_eq!(top_season(&state), Some(20222023));
}

#[test]
fn test_cycle_noop_when_season_unresolved_or_seasons_unknown() {
    // Season still None (first fetch in flight), seasons list unknown
    let mut state = AppState::default();
    state
        .navigation
        .document_stack
        .push(DocumentStackEntry::new(StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season: None,
        }));

    let (state, effect) = cycle(state, '[');
    assert!(matches!(effect, Effect::None));
    assert_eq!(top_season(&state), None);

    // Season resolved but seasons list unknown for the team
    let mut state = AppState::default();
    state
        .navigation
        .document_stack
        .push(DocumentStackEntry::new(StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season: Some(20242025),
        }));
    let (state, effect) = cycle(state, '[');
    assert!(matches!(effect, Effect::None));
    assert_eq!(top_season(&state), Some(20242025));
}

#[test]
fn test_cycle_noop_when_top_document_is_not_team_detail() {
    let mut state = AppState::default();
    state
        .navigation
        .document_stack
        .push(DocumentStackEntry::new(test_boxscore(1)));

    let (state, effect) = cycle(state, ']');
    assert!(matches!(effect, Effect::None));
    assert!(matches!(
        state.navigation.document_stack[0].document,
        StackedDocument::Boxscore { .. }
    ));
}

#[test]
fn test_pop_document_removes_all_roster_keys_for_team() {
    let mut state = AppState::default();
    state
        .navigation
        .document_stack
        .push(DocumentStackEntry::new(StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season: Some(20242025),
        }));
    // Keys from a "latest" fetch and from cycling to another season
    state
        .data
        .loading
        .insert(LoadingKey::TeamRosterStats("BOS".to_string(), None));
    state.data.loading.insert(LoadingKey::TeamRosterStats(
        "BOS".to_string(),
        Some(20232024),
    ));
    // A different team's key must survive
    state
        .data
        .loading
        .insert(LoadingKey::TeamRosterStats("TOR".to_string(), None));

    let (new_state, _) = pop_document(state);

    assert!(new_state.navigation.document_stack.is_empty());
    assert_eq!(new_state.data.loading.len(), 1);
    assert!(new_state
        .data
        .loading
        .contains(&LoadingKey::TeamRosterStats("TOR".to_string(), None)));
}
