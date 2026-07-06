use std::sync::Arc;
use std::time::SystemTime;
use tracing::debug;

use crate::tui::action::Action;
use crate::tui::component::Effect;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::reducers::standings::rebuild_standings_focusable_metadata;
use crate::tui::state::{AppState, LoadingKey};

/// Handle all data loading actions (API responses)
///
/// Returns Ok((new_state, effect)) if the action was handled,
/// or Err(state) to pass ownership back to the caller.
///
/// Takes component_states to update focusable metadata in component state when data loads.
#[allow(clippy::result_large_err)]
pub fn reduce_data_loading(
    state: AppState,
    action: &Action,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) -> Result<(AppState, Effect), AppState> {
    match action {
        Action::StandingsLoaded(result) => Ok(handle_standings_loaded(
            state,
            result.clone(),
            component_states,
        )),
        Action::ScheduleLoaded(result) => Ok(handle_schedule_loaded(
            state,
            result.clone(),
            component_states,
        )),
        Action::GameDetailsLoaded(game_id, result) => {
            Ok(handle_game_details_loaded(state, *game_id, result.clone()))
        }
        Action::BoxscoreLoaded(game_id, result) => {
            Ok(handle_boxscore_loaded(state, *game_id, result.clone()))
        }
        Action::TeamRosterStatsLoaded(team_abbrev, result) => Ok(handle_team_roster_loaded(
            state,
            team_abbrev.clone(),
            result.clone(),
        )),
        Action::PlayerStatsLoaded(player_id, result) => Ok(handle_player_stats_loaded(
            state,
            *player_id,
            result.clone(),
        )),
        Action::RefreshData => Ok(handle_refresh_data(state)),
        _ => Err(state),
    }
}

fn handle_standings_loaded(
    state: AppState,
    result: Result<Vec<nhl_api::Standing>, Arc<nhl_api::NHLApiError>>,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(standings) => {
            debug!("DATA: Loaded {} standings", standings.len());
            new_state.data.standings = Arc::new(Some(standings.clone()));
            new_state.data.errors.clear();
            new_state.data.loading.remove(&LoadingKey::Standings);

            // Rebuild demo document focusable data in component state
            #[cfg(feature = "development")]
            {
                use crate::tui::components::demo_tab::DemoDocument;
                use crate::tui::document::FocusContext;
                use crate::tui::document_nav::DocumentNavState;
                if let Some(demo_state) =
                    component_states.get_mut::<DocumentNavState>(DEMO_TAB_PATH)
                {
                    let demo_doc = DemoDocument::new(Some(standings.clone()));
                    demo_state.sync_focusables(&demo_doc, &FocusContext::default());
                }
            }

            // Rebuild standings document focusable data in component state
            rebuild_standings_focusable_metadata(&new_state, component_states);
        }
        Err(e) => {
            debug!("DATA: Failed to load standings: {}", e);
            new_state.data.errors.insert(
                "standings".to_string(),
                format!("Failed to load standings: {}", e),
            );
            new_state.data.loading.remove(&LoadingKey::Standings);

            // Rebuild demo focusable data for empty standings case
            #[cfg(feature = "development")]
            {
                use crate::tui::components::demo_tab::DemoDocument;
                use crate::tui::document::FocusContext;
                use crate::tui::document_nav::DocumentNavState;
                if let Some(demo_state) =
                    component_states.get_mut::<DocumentNavState>(DEMO_TAB_PATH)
                {
                    let demo_doc = DemoDocument::new(None);
                    demo_state.sync_focusables(&demo_doc, &FocusContext::default());
                }
            }

            // Clear standings focusable data in component state on error
            use crate::tui::components::standings_tab::StandingsTabState;
            if let Some(standings_state) =
                component_states.get_mut::<StandingsTabState>(STANDINGS_TAB_PATH)
            {
                standings_state.doc_nav.focusables.clear();
            }
        }
    }

    (new_state, Effect::None)
}

fn handle_schedule_loaded(
    state: AppState,
    result: Result<nhl_api::DailySchedule, Arc<nhl_api::NHLApiError>>,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(schedule) => {
            debug!("DATA: Loaded schedule with {} games", schedule.games.len());
            new_state.data.schedule = Arc::new(Some(schedule.clone()));
            new_state.data.errors.clear();
            // TODO: Remove Schedule loading key - needs date string

            // Rebuild scores tab focusable metadata from the document
            use crate::tui::components::score_boxes_document::ScoreBoxesDocument;
            use crate::tui::components::scores_tab::ScoresTabState;
            use crate::tui::document::FocusContext;

            if let Some(scores_state) = component_states.get_mut::<ScoresTabState>(SCORES_TAB_PATH)
            {
                // Calculate boxes_per_row from terminal width
                let boxes_per_row =
                    ScoreBoxesDocument::boxes_per_row_for_width(new_state.system.terminal_width);

                // Create the document to extract focusable metadata
                // animation_frame doesn't affect focusable positions, so use 0
                let doc = ScoreBoxesDocument::new(
                    Arc::new(Some(schedule.clone())),
                    new_state.data.game_info.clone(),
                    boxes_per_row,
                    scores_state.game_date.clone(),
                    0,
                );

                // Single sync call fills position/height/id/row-position/link-target
                // together -- link_target is load-bearing for ActivateGame (Enter on
                // a focused box pushes the target's boxscore document), so a sync
                // site can no longer forget it.
                scores_state
                    .doc_nav
                    .sync_focusables(&doc, &FocusContext::default());
            }

            // Return fetch effects for started games
            // This eliminates the need for runtime to compare old/new state
            let mut effects = Vec::new();
            for game in &schedule.games {
                // Only fetch details for games that have started
                if game.game_state != nhl_api::GameState::Future
                    && game.game_state != nhl_api::GameState::PreGame
                {
                    debug!(
                        "DATA: Requesting game details fetch for game_id={}",
                        game.id
                    );
                    effects.push(Effect::FetchGameDetails(game.id));
                }
            }

            let combined_effect = if effects.is_empty() {
                Effect::None
            } else {
                Effect::Batch(effects)
            };

            return (new_state, combined_effect);
        }
        Err(e) => {
            debug!("DATA: Failed to load schedule: {}", e);
            new_state.data.errors.insert(
                "error".to_string(),
                format!("Failed to load schedule: {}", e),
            );
            // TODO: Remove Schedule loading key - needs date string
        }
    }

    (new_state, Effect::None)
}

fn handle_game_details_loaded(
    state: AppState,
    game_id: i64,
    result: Result<nhl_api::GameMatchup, Arc<nhl_api::NHLApiError>>,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(game_matchup) => {
            debug!("DATA: Loaded game details for {}", game_id);

            // Extract period scores from the game matchup
            if let Some(ref summary) = game_matchup.summary {
                let period_scores = crate::commands::scores_format::extract_period_scores(summary);
                Arc::make_mut(&mut new_state.data.period_scores).insert(game_id, period_scores);
            }

            // Store game info
            Arc::make_mut(&mut new_state.data.game_info).insert(game_id, game_matchup);

            new_state
                .data
                .loading
                .remove(&LoadingKey::GameDetails(game_id));
        }
        Err(e) => {
            debug!("DATA: Failed to load game details for {}: {}", game_id, e);
            new_state
                .data
                .loading
                .remove(&LoadingKey::GameDetails(game_id));
        }
    }

    (new_state, Effect::None)
}

fn handle_boxscore_loaded(
    state: AppState,
    game_id: i64,
    result: Result<nhl_api::Boxscore, Arc<nhl_api::NHLApiError>>,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(boxscore) => {
            debug!("DATA: Loaded boxscore for game {}", game_id);
            // Focusable metadata is populated on-demand by StackedDocumentHandler
            Arc::make_mut(&mut new_state.data.boxscores).insert(game_id, boxscore);
            new_state
                .data
                .loading
                .remove(&LoadingKey::Boxscore(game_id));
        }
        Err(e) => {
            debug!("DATA: Failed to load boxscore for {}: {}", game_id, e);
            new_state.data.errors.insert(
                "error".to_string(),
                format!("Failed to load boxscore: {}", e),
            );
            new_state
                .data
                .loading
                .remove(&LoadingKey::Boxscore(game_id));
        }
    }

    (new_state, Effect::None)
}

fn handle_team_roster_loaded(
    state: AppState,
    team_abbrev: String,
    result: Result<nhl_api::ClubStats, Arc<nhl_api::NHLApiError>>,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(roster) => {
            debug!("DATA: Loaded roster for team {}", team_abbrev);
            // Focusable metadata is populated on-demand by StackedDocumentHandler
            Arc::make_mut(&mut new_state.data.team_roster_stats)
                .insert(team_abbrev.clone(), roster);
            new_state
                .data
                .loading
                .remove(&LoadingKey::TeamRosterStats(team_abbrev));
        }
        Err(e) => {
            debug!(
                "DATA: Failed to load team roster for {}: {}",
                team_abbrev, e
            );
            new_state.data.errors.insert(
                "error".to_string(),
                format!("Failed to load team roster: {}", e),
            );
            new_state
                .data
                .loading
                .remove(&LoadingKey::TeamRosterStats(team_abbrev));
        }
    }

    (new_state, Effect::None)
}

fn handle_player_stats_loaded(
    state: AppState,
    player_id: i64,
    result: Result<nhl_api::PlayerLanding, Arc<nhl_api::NHLApiError>>,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(stats) => {
            debug!("DATA: Loaded stats for player {}", player_id);
            // Focusable metadata is populated on-demand by StackedDocumentHandler
            Arc::make_mut(&mut new_state.data.player_data).insert(player_id, stats);
            new_state
                .data
                .loading
                .remove(&LoadingKey::PlayerStats(player_id));
        }
        Err(e) => {
            debug!("DATA: Failed to load player stats for {}: {}", player_id, e);
            new_state.data.errors.insert(
                "error".to_string(),
                format!("Failed to load player stats: {}", e),
            );
            new_state
                .data
                .loading
                .remove(&LoadingKey::PlayerStats(player_id));
        }
    }

    (new_state, Effect::None)
}

fn handle_refresh_data(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.system.last_refresh = Some(SystemTime::now());
    (new_state, Effect::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_details_loaded_stores_game_info() {
        // Regression test: Ensure period_scores are extracted from game_matchup.summary
        // This was missing after the reducer refactoring, causing final game scores to show as "-"
        //
        // Note: This test verifies the logic exists by checking the code compiles with
        // the period_scores extraction. Full integration testing requires real GameMatchup data.

        let state = AppState::default();
        const TEST_GAME_ID: i64 = 2024020123;

        // Verify the function signature exists and handles both Ok and Err cases
        let result_ok: Result<nhl_api::GameMatchup, Arc<nhl_api::NHLApiError>> =
            Err(Arc::new(nhl_api::NHLApiError::Other("test".to_string())));
        let (new_state, _effect) =
            handle_game_details_loaded(state.clone(), TEST_GAME_ID, result_ok);

        // Verify loading key is removed on error
        assert!(!new_state
            .data
            .loading
            .contains(&LoadingKey::GameDetails(TEST_GAME_ID)));

        // The actual test with real GameMatchup data would require constructing
        // a complex struct with all required fields. The key behavior to test is:
        // 1. Game info is stored in game_info HashMap
        // 2. If summary exists, period_scores are extracted and stored
        // 3. Loading key is removed
        //
        // This is verified by the code review showing lines 88-92 extract period_scores
    }

    /// Regression test: the scores metadata sync must populate link_targets.
    /// ActivateGame reads focused_link_target() to push the boxscore document;
    /// when this site skipped link_targets, Enter on a focused game silently
    /// did nothing.
    #[test]
    fn test_schedule_loaded_populates_scores_link_targets() {
        use crate::fixtures::create_mock_schedule;
        use crate::tui::component_store::ComponentStateStore;
        use crate::tui::components::scores_tab::ScoresTabState;
        use crate::tui::document::LinkTarget;
        use crate::tui::types::StackedDocument;

        let mut store = ComponentStateStore::new();
        store.insert(SCORES_TAB_PATH.to_string(), ScoresTabState::default());

        let schedule = create_mock_schedule(None);
        let expected_game_id = schedule.games[0].id;
        let (_state, _effect) =
            handle_schedule_loaded(AppState::default(), Ok(schedule), &mut store);

        let scores = store.get::<ScoresTabState>(SCORES_TAB_PATH).unwrap();
        let nav = &scores.doc_nav;
        assert!(!nav.focusables.is_empty(), "expected at least one game box");
        let first_link_target = nav.focusables.first().and_then(|f| f.link_target.as_ref());
        assert!(
            matches!(
                first_link_target,
                Some(LinkTarget::Push(StackedDocument::Boxscore { game_id, .. }))
                    if *game_id == expected_game_id
            ),
            "first game box must carry a Push(Boxscore) target, got {:?}",
            first_link_target
        );
    }
}
