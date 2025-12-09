use std::sync::Arc;

use nhl_api::GameDate;

use super::action::Action;
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
                    effects.push(self.fetch_game_details(game.id));
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
            Action::StandingsLoaded(result.map_err(|e| e.to_string()))
        }))
    }

    /// Fetch daily schedule for a specific date (with caching)
    pub fn fetch_schedule(&self, date: GameDate) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_schedule_cached(client.as_ref(), date).await;
            Action::ScheduleLoaded(result.map_err(|e| e.to_string()))
        }))
    }

    /// Fetch game details for a specific game (with caching)
    pub fn fetch_game_details(&self, game_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_game_cached(client.as_ref(), game_id).await;
            Action::GameDetailsLoaded(game_id, result.map_err(|e| e.to_string()))
        }))
    }

    /// Fetch team roster stats for a specific team (current season, regular season)
    ///
    /// This method dynamically determines the current season by fetching available seasons
    /// and selecting the most recent one that has regular season data (with caching).
    pub fn fetch_team_roster_stats(&self, team_abbrev: String) -> Effect {
        let client = self.client.clone();
        let abbrev = team_abbrev.clone();
        Effect::Async(Box::pin(async move {
            // First, get available seasons for this team (not cached - small data)
            let seasons_result = client.club_stats_season(&abbrev).await;

            let result = match seasons_result {
                Ok(seasons) => {
                    // Find the most recent season that has regular season data (game_type = 2)
                    let current_season = seasons
                        .iter()
                        .filter(|s| s.game_types.contains(&REGULAR_SEASON))
                        .max_by_key(|s| s.season);

                    match current_season {
                        Some(season_info) => {
                            // Fetch stats for the current season (with caching)
                            cache::fetch_club_stats_cached(
                                client.as_ref(),
                                &abbrev,
                                season_info.season,
                            )
                            .await
                        }
                        None => Err(nhl_api::NHLApiError::ApiError {
                            message: "No regular season data available for team".to_string(),
                            status_code: 404,
                        }),
                    }
                }
                Err(e) => Err(e),
            };

            Action::TeamRosterStatsLoaded(team_abbrev, result.map_err(|e| e.to_string()))
        }))
    }

    /// Fetch player landing data (career stats, season stats, etc.)
    pub fn fetch_player_stats(&self, player_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_player_landing_cached(client.as_ref(), player_id).await;
            Action::PlayerStatsLoaded(player_id, result.map_err(|e| e.to_string()))
        }))
    }

    /// Fetch boxscore for a specific game (with caching)
    pub fn fetch_boxscore(&self, game_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = cache::fetch_boxscore_cached(client.as_ref(), game_id).await;
            Action::BoxscoreLoaded(game_id, result.map_err(|e| e.to_string()))
        }))
    }
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
                content_focused: false,
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
}
