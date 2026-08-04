use std::sync::Arc;
use std::time::SystemTime;
use tracing::debug;

use crate::tui::action::{Action, TeamRosterStatsPayload};
use crate::tui::component::Effect;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::reducers::standings::rebuild_standings_focusable_metadata;
use crate::tui::state::{AppState, LoadingKey};
use crate::tui::types::StackedDocument;

// Error-message prefixes used both to build the user-facing status-bar message
// and to recognize (via `starts_with`) whether a currently-displayed status
// error belongs to this data type. This lets a successful load clear only its
// own error, without clobbering an unrelated error from a different feed that
// is still failing (e.g. a schedule fetch succeeding must not hide a standings
// error).
const STANDINGS_ERROR_PREFIX: &str = "Failed to load standings:";
const SCHEDULE_ERROR_PREFIX: &str = "Failed to load schedule:";
const BOXSCORE_ERROR_PREFIX: &str = "Failed to load boxscore:";
const TEAM_ROSTER_ERROR_PREFIX: &str = "Failed to load team roster:";
const PLAYER_STATS_ERROR_PREFIX: &str = "Failed to load player stats:";

/// Game-detail fetches happen many at once (one per live game, every refresh
/// cycle), so unlike the other feeds this message carries no per-game detail.
/// See `handle_game_details_loaded` for the dedup policy that keeps a batch of
/// failures from spamming the status bar.
const GAME_DETAILS_ERROR_MESSAGE: &str = "Failed to load game details";

/// Clears the status-bar error message, but only if it's currently showing an
/// error that belongs to this data type (matched by prefix). A successful load
/// of one feed should not erase an error still showing for a different feed.
fn clear_error_with_prefix(state: &mut AppState, prefix: &str) {
    if state.system.status_is_error
        && state
            .system
            .status_message
            .as_deref()
            .is_some_and(|message| message.starts_with(prefix))
    {
        state.system.reset_status_message();
    }
}

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
        Action::TeamRosterStatsLoaded {
            abbrev,
            requested_season,
            result,
        } => Ok(handle_team_roster_loaded(
            state,
            abbrev.clone(),
            *requested_season,
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

/// Applies a successful standings load: stores the data, clears any matching error,
/// and rebuilds the demo (dev builds) and standings tabs' focusable metadata.
fn apply_standings_loaded(
    new_state: &mut AppState,
    standings: Vec<nhl_api::Standing>,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) {
    debug!("DATA: Loaded {} standings", standings.len());
    new_state.data.standings = Arc::new(Some(standings.clone()));
    clear_error_with_prefix(new_state, STANDINGS_ERROR_PREFIX);
    new_state.data.loading.remove(&LoadingKey::Standings);

    // Rebuild demo document focusable data in component state
    #[cfg(feature = "development")]
    {
        use crate::tui::components::demo_tab::DemoDocument;
        use crate::tui::document::FocusContext;
        use crate::tui::document_nav::DocumentNavState;
        if let Some(demo_state) = component_states.get_mut::<DocumentNavState>(DEMO_TAB_PATH) {
            let demo_doc = DemoDocument::new(Some(standings.clone()));
            demo_state.sync_focusables(&demo_doc, &FocusContext::default());
        }
    }

    // Rebuild standings document focusable data in component state
    rebuild_standings_focusable_metadata(new_state, component_states);
}

/// Applies a failed standings load: reports the error and clears focusable data in the
/// demo (dev builds) and standings tabs since there's no data to show.
fn apply_standings_error(
    new_state: &mut AppState,
    e: Arc<nhl_api::NHLApiError>,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) {
    debug!("DATA: Failed to load standings: {}", e);
    new_state
        .system
        .set_status_error_message(format!("{} {}", STANDINGS_ERROR_PREFIX, e));
    new_state.data.loading.remove(&LoadingKey::Standings);

    // Rebuild demo focusable data for empty standings case
    #[cfg(feature = "development")]
    {
        use crate::tui::components::demo_tab::DemoDocument;
        use crate::tui::document::FocusContext;
        use crate::tui::document_nav::DocumentNavState;
        if let Some(demo_state) = component_states.get_mut::<DocumentNavState>(DEMO_TAB_PATH) {
            let demo_doc = DemoDocument::new(None);
            demo_state.sync_focusables(&demo_doc, &FocusContext::default());
        }
    }

    // Clear standings focusable data in component state on error
    use crate::tui::components::standings_tab::StandingsTabState;
    if let Some(standings_state) = component_states.get_mut::<StandingsTabState>(STANDINGS_TAB_PATH)
    {
        standings_state.doc_nav.focusables.clear();
    }
}

fn handle_standings_loaded(
    state: AppState,
    result: Result<Vec<nhl_api::Standing>, Arc<nhl_api::NHLApiError>>,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(standings) => apply_standings_loaded(&mut new_state, standings, component_states),
        Err(e) => apply_standings_error(&mut new_state, e, component_states),
    }

    (new_state, Effect::None)
}

/// Rebuilds the Scores tab's focusable metadata from a freshly loaded schedule.
///
/// Single sync call fills position/height/id/row-position/link-target together --
/// link_target is load-bearing for ActivateGame (Enter on a focused box pushes the
/// target's boxscore document), so a sync site can no longer forget it.
fn rebuild_scores_focusable_metadata(
    new_state: &AppState,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
    schedule: &nhl_api::DailySchedule,
) {
    use crate::tui::components::score_boxes_document::ScoreBoxesDocument;
    use crate::tui::components::scores_tab::ScoresTabState;
    use crate::tui::document::FocusContext;

    if let Some(scores_state) = component_states.get_mut::<ScoresTabState>(SCORES_TAB_PATH) {
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

        scores_state
            .doc_nav
            .sync_focusables(&doc, &FocusContext::default());
    }
}

/// Returns fetch effects for every schedule game that has already started, so its
/// per-game detail view has data as soon as it's opened.
fn game_details_fetch_effects(schedule: &nhl_api::DailySchedule) -> Effect {
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
            effects.push(Effect::FetchGameDetails(game.id.into()));
        }
    }

    if effects.is_empty() {
        Effect::None
    } else {
        Effect::Batch(effects)
    }
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
            clear_error_with_prefix(&mut new_state, SCHEDULE_ERROR_PREFIX);
            // TODO: Remove Schedule loading key - needs date string

            rebuild_scores_focusable_metadata(&new_state, component_states, &schedule);
            // Return fetch effects for started games
            // This eliminates the need for runtime to compare old/new state
            let combined_effect = game_details_fetch_effects(&schedule);

            return (new_state, combined_effect);
        }
        Err(e) => {
            debug!("DATA: Failed to load schedule: {}", e);
            new_state
                .system
                .set_status_error_message(format!("{} {}", SCHEDULE_ERROR_PREFIX, e));
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
            clear_error_with_prefix(&mut new_state, GAME_DETAILS_ERROR_MESSAGE);
        }
        Err(e) => {
            debug!("DATA: Failed to load game details for {}: {}", game_id, e);
            new_state
                .data
                .loading
                .remove(&LoadingKey::GameDetails(game_id));

            // A schedule refresh can trigger many of these in one cycle (one per
            // live game). Only surface the first failure so per-game noise
            // doesn't spam-overwrite the status bar every tick, and so a more
            // actionable standings/schedule error already showing isn't hidden.
            if !new_state.system.status_is_error {
                new_state
                    .system
                    .set_status_error_message(GAME_DETAILS_ERROR_MESSAGE.to_string());
            }
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
            // Focusable metadata is populated on-demand by handle_stacked_document_key
            Arc::make_mut(&mut new_state.data.boxscores).insert(game_id, boxscore);
            new_state
                .data
                .loading
                .remove(&LoadingKey::Boxscore(game_id));
            clear_error_with_prefix(&mut new_state, BOXSCORE_ERROR_PREFIX);
        }
        Err(e) => {
            debug!("DATA: Failed to load boxscore for {}: {}", game_id, e);
            new_state
                .system
                .set_status_error_message(format!("{} {}", BOXSCORE_ERROR_PREFIX, e));
            new_state
                .data
                .loading
                .remove(&LoadingKey::Boxscore(game_id));
        }
    }

    (new_state, Effect::None)
}

/// Applies a successful team roster stats load: stores the roster and (if resolved from
/// a "latest" request) the season list, resolves any pending same-team stack entries
/// whose season was still unknown, and clears any matching error.
fn apply_team_roster_loaded(
    new_state: &mut AppState,
    team_abbrev: String,
    requested_season: Option<i32>,
    payload: TeamRosterStatsPayload,
) {
    let resolved_season = payload.stats.season.id();
    debug!(
        "DATA: Loaded roster for team {} season {}",
        team_abbrev, resolved_season
    );
    if let Some(seasons) = payload.seasons {
        Arc::make_mut(&mut new_state.data.team_seasons).insert(team_abbrev.clone(), seasons);
    }
    // Focusable metadata is populated on-demand by handle_stacked_document_key
    Arc::make_mut(&mut new_state.data.team_roster_stats)
        .insert((team_abbrev.clone(), resolved_season), payload.stats);
    new_state.data.loading.remove(&LoadingKey::TeamRosterStats(
        team_abbrev.clone(),
        requested_season,
    ));
    // A "latest" request has now resolved to a concrete season: give every
    // pending TeamDetail entry for this team its identity, so season cycling
    // and the loading-key lookup have a real id.
    for entry in &mut new_state.navigation.document_stack {
        if let StackedDocument::TeamDetail { abbrev, season } = &mut entry.document {
            if *abbrev == team_abbrev && season.is_none() {
                *season = Some(resolved_season);
            }
        }
    }
    clear_error_with_prefix(new_state, TEAM_ROSTER_ERROR_PREFIX);
}

fn handle_team_roster_loaded(
    state: AppState,
    team_abbrev: String,
    requested_season: Option<i32>,
    result: Result<TeamRosterStatsPayload, Arc<nhl_api::NHLApiError>>,
) -> (AppState, Effect) {
    let mut new_state = state;

    match result {
        Ok(payload) => {
            apply_team_roster_loaded(&mut new_state, team_abbrev, requested_season, payload)
        }
        Err(e) => {
            debug!(
                "DATA: Failed to load team roster for {}: {}",
                team_abbrev, e
            );
            new_state
                .system
                .set_status_error_message(format!("{} {}", TEAM_ROSTER_ERROR_PREFIX, e));
            new_state
                .data
                .loading
                .remove(&LoadingKey::TeamRosterStats(team_abbrev, requested_season));
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
            // Focusable metadata is populated on-demand by handle_stacked_document_key
            Arc::make_mut(&mut new_state.data.player_data).insert(player_id, stats);
            new_state
                .data
                .loading
                .remove(&LoadingKey::PlayerStats(player_id));
            clear_error_with_prefix(&mut new_state, PLAYER_STATS_ERROR_PREFIX);
        }
        Err(e) => {
            debug!("DATA: Failed to load player stats for {}: {}", player_id, e);
            new_state
                .system
                .set_status_error_message(format!("{} {}", PLAYER_STATS_ERROR_PREFIX, e));
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
#[path = "data_loading_tests.rs"]
mod tests;
