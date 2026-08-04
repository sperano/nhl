use std::sync::Arc;

use nhl_api::GameDate;

use super::action::{Action, TeamRosterStatsPayload};
use super::component::Effect;
use super::state::AppState;
use crate::cache;
use crate::data_provider::NHLDataProvider;

/// Regular season game type identifier
const REGULAR_SEASON: nhl_api::GameType = nhl_api::GameType::RegularSeason;

/// Effect handler for data fetching operations
///
/// This handles all async data fetching from the NHL API.
/// Each method returns an Effect that will dispatch the appropriate
/// *Loaded action when complete.
pub struct DataEffects {
    client: Arc<dyn NHLDataProvider>,
}

impl DataEffects {
    /// Create a new DataEffects handler with an NHL data provider
    pub fn new(client: Arc<dyn NHLDataProvider>) -> Self {
        Self { client }
    }

    /// Handle a refresh request - fetches all necessary data based on current state
    pub fn handle_refresh(&self, state: &AppState) -> Effect {
        let mut effects = vec![
            self.fetch_standings(),
            self.fetch_schedule(state.ui.scores.game_date.clone()),
        ];

        // Add game detail fetches for started games
        if let Some(schedule) = state.data.schedule.as_ref().as_ref() {
            for game in &schedule.games {
                // Only fetch details for games that have started
                if game.game_state != nhl_api::GameState::Future
                    && game.game_state != nhl_api::GameState::PreGame
                {
                    effects.push(self.fetch_game_details(game.id.into()));
                }
            }
        }

        Effect::Batch(effects)
    }

    /// Handle a schedule refresh for a specific date
    pub fn handle_refresh_schedule(&self, date: nhl_api::GameDate) -> Effect {
        self.fetch_schedule(date)
    }

    /// Fetch current league standings (with caching)
    pub fn fetch_standings(&self) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_standings_cached(client.as_ref()).await;
            Action::StandingsLoaded(result.map_err(Arc::new))
        }))
    }

    /// Fetch daily schedule for a specific date (with caching)
    pub fn fetch_schedule(&self, date: GameDate) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_schedule_cached(client.as_ref(), date).await;
            Action::ScheduleLoaded(result.map_err(Arc::new))
        }))
    }

    /// Fetch game details for a specific game (with caching)
    pub fn fetch_game_details(&self, game_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_game_cached(client.as_ref(), game_id.into()).await;
            Action::GameDetailsLoaded(game_id, result.map_err(Arc::new))
        }))
    }

    /// Fetch team roster stats for a specific team and season (regular season).
    ///
    /// With `season: Some(id)`, fetches that season's stats directly (with
    /// caching); `fetch_seasons` additionally fetches the team's available
    /// seasons so the reducer can store them (needed when opening a team
    /// directly at a specific season before its list is known). With
    /// `season: None`, first fetches the available seasons, resolves
    /// "latest" to the most recent one with regular season data, and always
    /// includes the season list in the payload.
    pub fn fetch_team_roster_stats(
        &self,
        team_abbrev: String,
        season: Option<i32>,
        fetch_seasons: bool,
    ) -> Effect {
        let client = self.client.clone();
        let abbrev = team_abbrev.clone();
        Effect::Async(Box::pin(async move {
            let result = match season {
                Some(id) => {
                    // Best-effort: a failed season-list fetch degrades to
                    // seasons staying unknown, not a roster load failure.
                    let seasons = if fetch_seasons {
                        fetch_regular_season_ids(client.as_ref(), &abbrev)
                            .await
                            .ok()
                    } else {
                        None
                    };
                    cache::fetch_club_stats_cached(client.as_ref(), &abbrev, id)
                        .await
                        .map(|stats| TeamRosterStatsPayload { seasons, stats })
                }
                None => match fetch_regular_season_ids(client.as_ref(), &abbrev).await {
                    Ok(ids) => match ids.last().copied() {
                        Some(latest) => {
                            cache::fetch_club_stats_cached(client.as_ref(), &abbrev, latest)
                                .await
                                .map(|stats| TeamRosterStatsPayload {
                                    seasons: Some(ids),
                                    stats,
                                })
                        }
                        None => Err(nhl_api::NHLApiError::ApiError {
                            message: "No regular season data available for team".to_string(),
                            status_code: 404,
                        }),
                    },
                    Err(e) => Err(e),
                },
            };

            Action::TeamRosterStatsLoaded {
                abbrev: team_abbrev,
                requested_season: season,
                result: result.map_err(Arc::new),
            }
        }))
    }

    /// Fetch player landing data (career stats, season stats, etc.)
    pub fn fetch_player_stats(&self, player_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result =
                cache::fetch_player_landing_cached(client.as_ref(), player_id.into()).await;
            Action::PlayerStatsLoaded(player_id, result.map_err(Arc::new))
        }))
    }

    /// Fetch boxscore for a specific game (with caching)
    pub fn fetch_boxscore(&self, game_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_boxscore_cached(client.as_ref(), game_id.into()).await;
            Action::BoxscoreLoaded(game_id, result.map_err(Arc::new))
        }))
    }
}

/// Fetch the team's season ids that have regular-season data, sorted
/// ascending (not cached - small data).
async fn fetch_regular_season_ids(
    client: &dyn NHLDataProvider,
    abbrev: &str,
) -> Result<Vec<i32>, nhl_api::NHLApiError> {
    let seasons = client.club_stats_season(abbrev).await?;
    let mut ids: Vec<i32> = seasons
        .iter()
        .filter(|s| s.game_types.contains(&REGULAR_SEASON))
        .map(|s| s.season.id())
        .collect();
    ids.sort_unstable();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::{NavigationState, SystemState, UiState};
    use crate::tui::types::Tab;

    fn create_test_state() -> AppState {
        AppState {
            navigation: NavigationState {
                current_tab: Tab::Scores,
                document_stack: Vec::new(),
                focus_in_content: false,
            },
            data: Default::default(),
            ui: UiState::default(),
            system: SystemState::default(),
        }
    }

    #[test]
    fn test_fetch_standings_returns_async_effect() {
        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);

        let effect = effects.fetch_standings();

        // Verify it returns an Async effect
        assert!(matches!(effect, Effect::Async(_)));
    }

    #[test]
    fn test_fetch_schedule_returns_async_effect() {
        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);

        let date = GameDate::default();
        let effect = effects.fetch_schedule(date);

        // Verify it returns an Async effect
        assert!(matches!(effect, Effect::Async(_)));
    }

    #[test]
    fn test_fetch_game_details_returns_async_effect() {
        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);

        let effect = effects.fetch_game_details(2024020001);

        // Verify it returns an Async effect
        assert!(matches!(effect, Effect::Async(_)));
    }

    #[test]
    fn test_handle_refresh_returns_batch_effect() {
        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);
        let state = create_test_state();

        let effect = effects.handle_refresh(&state);

        // Verify it returns a Batch effect
        assert!(matches!(effect, Effect::Batch(_)));
    }

    #[test]
    fn test_handle_refresh_includes_standings_and_schedule() {
        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);
        let state = create_test_state();

        let effect = effects.handle_refresh(&state);

        // Verify batch contains at least 2 effects (standings + schedule)
        if let Effect::Batch(effects) = effect {
            assert!(effects.len() >= 2);
        } else {
            panic!("Expected Batch effect");
        }
    }

    #[tokio::test]
    #[ignore] // Integration test - requires network access
    async fn test_cache_integration_standings() {
        use crate::cache;

        // Clear cache before test
        cache::clear_all_caches().await;

        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);

        // First fetch - should hit the API and cache
        let effect1 = effects.fetch_standings();
        if let Effect::Async(future) = effect1 {
            let _action = future.await;
        }

        // Verify cache has entry
        let stats = cache::cache_stats().await;
        assert_eq!(stats.standings_entries, 1);

        // Second fetch - should hit the cache
        let effect2 = effects.fetch_standings();
        if let Effect::Async(future) = effect2 {
            let _action = future.await;
        }

        // Cache should still have 1 entry
        let stats = cache::cache_stats().await;
        assert_eq!(stats.standings_entries, 1);
    }

    #[tokio::test]
    #[ignore] // Integration test - requires network access
    async fn test_cache_integration_schedule() {
        use crate::cache;

        // Clear cache before test
        cache::clear_all_caches().await;

        let client = crate::tui::testing::create_client();
        let effects = DataEffects::new(client);

        let date = GameDate::Now;

        // First fetch - should hit the API and cache
        let effect1 = effects.fetch_schedule(date.clone());
        if let Effect::Async(future) = effect1 {
            let _action = future.await;
        }

        // Verify cache has entry
        let stats = cache::cache_stats().await;
        assert_eq!(stats.schedule_entries, 1);

        // Second fetch - should hit the cache
        let effect2 = effects.fetch_schedule(date);
        if let Effect::Async(future) = effect2 {
            let _action = future.await;
        }

        // Cache should still have 1 entry
        let stats = cache::cache_stats().await;
        assert_eq!(stats.schedule_entries, 1);
    }

    #[tokio::test]
    async fn test_fetch_team_roster_stats_latest_resolves_seasons_via_mock() {
        let effects = DataEffects::new(Arc::new(crate::dev::mock_client::MockClient::new()));

        let effect = effects.fetch_team_roster_stats("TOR".to_string(), None, false);
        let Effect::Async(future) = effect else {
            panic!("expected Async effect");
        };

        match future.await {
            Action::TeamRosterStatsLoaded {
                abbrev,
                requested_season,
                result,
            } => {
                assert_eq!(abbrev, "TOR");
                assert_eq!(requested_season, None);
                let payload = result.expect("mock fetch succeeds");
                // Mock serves seasons 2023-24 and 2024-25; latest wins
                assert_eq!(payload.seasons, Some(vec![20232024, 20242025]));
                assert_eq!(payload.stats.season.id(), 20242025);
            }
            other => panic!("expected TeamRosterStatsLoaded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_fetch_team_roster_stats_specific_season_skips_season_list() {
        let effects = DataEffects::new(Arc::new(crate::dev::mock_client::MockClient::new()));

        let effect = effects.fetch_team_roster_stats("TOR".to_string(), Some(20232024), false);
        let Effect::Async(future) = effect else {
            panic!("expected Async effect");
        };

        match future.await {
            Action::TeamRosterStatsLoaded {
                requested_season,
                result,
                ..
            } => {
                assert_eq!(requested_season, Some(20232024));
                let payload = result.expect("mock fetch succeeds");
                assert_eq!(payload.seasons, None);
                assert_eq!(payload.stats.season.id(), 20232024);
            }
            other => panic!("expected TeamRosterStatsLoaded, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_fetch_team_roster_stats_specific_season_with_fetch_seasons_includes_list() {
        let effects = DataEffects::new(Arc::new(crate::dev::mock_client::MockClient::new()));

        // Opening a team directly at a historical season (e.g. from a
        // player's past-season row) must also deliver the season list so
        // cycling and the current-season flag work.
        let effect = effects.fetch_team_roster_stats("TOR".to_string(), Some(20232024), true);
        let Effect::Async(future) = effect else {
            panic!("expected Async effect");
        };

        match future.await {
            Action::TeamRosterStatsLoaded {
                requested_season,
                result,
                ..
            } => {
                assert_eq!(requested_season, Some(20232024));
                let payload = result.expect("mock fetch succeeds");
                assert_eq!(payload.seasons, Some(vec![20232024, 20242025]));
                assert_eq!(payload.stats.season.id(), 20232024);
            }
            other => panic!("expected TeamRosterStatsLoaded, got {other:?}"),
        }
    }
}
