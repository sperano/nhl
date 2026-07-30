use tracing::debug;

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document::handle_stacked_document_key;
use crate::tui::document_nav::DocumentNavState;
use crate::tui::state::{AppState, DocumentStackEntry, LoadingKey};
use crate::tui::types::StackedDocument;

/// Handle all document stack management actions
///
/// Returns Ok((new_state, effect)) if the action was handled,
/// or Err(state) to pass ownership back to the caller.
#[allow(clippy::result_large_err)]
pub fn reduce_document_stack(
    state: AppState,
    action: &Action,
) -> Result<(AppState, Effect), AppState> {
    match action {
        Action::PushDocument(doc) => Ok(push_document(state, doc.clone())),
        Action::PopDocument => Ok(pop_document(state)),
        Action::StackedDocumentKey(key) => Ok(stacked_document_key(state, *key)),
        _ => Err(state),
    }
}

/// Handle key events routed to stacked documents
fn stacked_document_key(state: AppState, key: crossterm::event::KeyEvent) -> (AppState, Effect) {
    // Season cycling for team detail is a stack-level concern (it rewrites
    // the top document's identity), so it is intercepted here rather than in
    // handle_stacked_document_key, which only sees an immutable document.
    match key.code {
        crossterm::event::KeyCode::Char('[') => return cycle_team_detail_season(state, false),
        crossterm::event::KeyCode::Char(']') => return cycle_team_detail_season(state, true),
        _ => {}
    }

    let mut new_state = state;
    let width = new_state.system.terminal_width;

    if let Some(entry) = new_state.navigation.document_stack.last_mut() {
        let effect = handle_stacked_document_key(
            &entry.document,
            key,
            &mut entry.nav,
            &new_state.data,
            width,
        );
        return (new_state, effect);
    }

    (new_state, Effect::None)
}

fn push_document(state: AppState, doc: StackedDocument) -> (AppState, Effect) {
    debug!("DOCUMENT_STACK: Pushing document onto stack: {:?}", doc);
    let mut new_state = state;
    new_state
        .navigation
        .document_stack
        .push(DocumentStackEntry::new(doc.clone()));

    // Return fetch effect directly based on document type
    // This eliminates the need for runtime to compare old/new state
    let fetch_effect = match &doc {
        StackedDocument::Boxscore { game_id, .. } => {
            // Check if we don't already have the data and aren't already loading
            if !new_state.data.boxscores.contains_key(game_id)
                && !new_state
                    .data
                    .loading
                    .contains(&LoadingKey::Boxscore(*game_id))
            {
                debug!(
                    "DOCUMENT_STACK: Requesting boxscore fetch for game_id={}",
                    game_id
                );
                new_state
                    .data
                    .loading
                    .insert(LoadingKey::Boxscore(*game_id));
                Effect::FetchBoxscore(*game_id)
            } else {
                Effect::None
            }
        }
        StackedDocument::TeamDetail { abbrev, season } => {
            // Normalize "latest" locally when the seasons list is already
            // known, so a re-open hits the (abbrev, season) map instead of
            // re-fetching through the un-cached seasons endpoint.
            let season = season.or_else(|| {
                new_state
                    .data
                    .team_seasons
                    .get(abbrev)
                    .and_then(|ids| ids.last().copied())
            });
            if let Some(StackedDocument::TeamDetail { season: s, .. }) = new_state
                .navigation
                .document_stack
                .last_mut()
                .map(|entry| &mut entry.document)
            {
                *s = season;
            }

            let have_data = season.is_some_and(|s| {
                new_state
                    .data
                    .team_roster_stats
                    .contains_key(&(abbrev.clone(), s))
            });
            let key = LoadingKey::TeamRosterStats(abbrev.clone(), season);
            if !have_data && !new_state.data.loading.contains(&key) {
                debug!(
                    "DOCUMENT_STACK: Requesting team roster stats fetch for team={} season={:?}",
                    abbrev, season
                );
                new_state.data.loading.insert(key);
                Effect::FetchTeamRosterStats(abbrev.clone(), season)
            } else {
                Effect::None
            }
        }
        StackedDocument::PlayerDetail { player_id, .. } => {
            if !new_state.data.player_data.contains_key(player_id)
                && !new_state
                    .data
                    .loading
                    .contains(&LoadingKey::PlayerStats(*player_id))
            {
                debug!(
                    "DOCUMENT_STACK: Requesting player stats fetch for player_id={}",
                    player_id
                );
                new_state
                    .data
                    .loading
                    .insert(LoadingKey::PlayerStats(*player_id));
                Effect::FetchPlayerStats(*player_id)
            } else {
                Effect::None
            }
        }
    };

    (new_state, fetch_effect)
}

/// Cycle the top team-detail document to the previous (`next == false`) or
/// next (`next == true`) season in the team's regular-season list.
///
/// No-ops (returning the state unchanged) when the top document is not a
/// team detail, its season hasn't resolved yet, the team's season list is
/// unknown, or the season is already at the requested end (clamped, no wrap).
fn cycle_team_detail_season(state: AppState, next: bool) -> (AppState, Effect) {
    let mut new_state = state;

    let Some(entry) = new_state.navigation.document_stack.last_mut() else {
        return (new_state, Effect::None);
    };
    let StackedDocument::TeamDetail {
        abbrev,
        season: Some(current),
    } = &entry.document
    else {
        return (new_state, Effect::None);
    };
    let (abbrev, current) = (abbrev.clone(), *current);

    let Some(ids) = new_state.data.team_seasons.get(&abbrev) else {
        return (new_state, Effect::None);
    };
    let Some(pos) = ids.iter().position(|s| *s == current) else {
        return (new_state, Effect::None);
    };
    let new_pos = if next {
        (pos + 1).min(ids.len() - 1)
    } else {
        pos.saturating_sub(1)
    };
    if new_pos == pos {
        return (new_state, Effect::None);
    }
    let new_season = ids[new_pos];

    debug!(
        "DOCUMENT_STACK: Cycling team detail season for {}: {} -> {}",
        abbrev, current, new_season
    );
    if let StackedDocument::TeamDetail { season, .. } = &mut entry.document {
        *season = Some(new_season);
    }
    // The roster changes entirely: reset focus and scroll, keep the viewport
    // height (it only changes on terminal resize). Focusables re-sync on the
    // next key or render.
    entry.nav = DocumentNavState {
        focus_index: Some(0),
        viewport_height: entry.nav.viewport_height,
        ..Default::default()
    };

    let have_data = new_state
        .data
        .team_roster_stats
        .contains_key(&(abbrev.clone(), new_season));
    let key = LoadingKey::TeamRosterStats(abbrev.clone(), Some(new_season));
    if !have_data && !new_state.data.loading.contains(&key) {
        new_state.data.loading.insert(key);
        return (
            new_state,
            Effect::FetchTeamRosterStats(abbrev, Some(new_season)),
        );
    }

    (new_state, Effect::None)
}

fn pop_document(state: AppState) -> (AppState, Effect) {
    debug!("DOCUMENT_STACK: Popping document from stack");
    let mut new_state = state;

    if let Some(doc_entry) = new_state.navigation.document_stack.pop() {
        // Clear the loading state for the document being popped
        match &doc_entry.document {
            StackedDocument::Boxscore { game_id, .. } => {
                new_state
                    .data
                    .loading
                    .remove(&LoadingKey::Boxscore(*game_id));
            }
            StackedDocument::TeamDetail { abbrev, .. } => {
                // Remove every season's key for this team: cycling away from
                // a season with its fetch still in flight and then popping
                // must not strand a key (it would keep the animation hot).
                new_state
                    .data
                    .loading
                    .retain(|k| !matches!(k, LoadingKey::TeamRosterStats(a, _) if a == abbrev));
            }
            StackedDocument::PlayerDetail { player_id, .. } => {
                new_state
                    .data
                    .loading
                    .remove(&LoadingKey::PlayerStats(*player_id));
            }
        }

        debug!(
            "DOCUMENT_STACK: Popped document, {} remaining",
            new_state.navigation.document_stack.len()
        );
    }

    // If no documents left, return focus to content
    if new_state.navigation.document_stack.is_empty() {
        debug!("DOCUMENT_STACK: Document stack empty, returning focus to content");
    }

    (new_state, Effect::None)
}

#[cfg(test)]
mod tests {
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
}
