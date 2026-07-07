/// Mock NHL API client for development and testing
use crate::data_provider::NHLDataProvider;
use async_trait::async_trait;
use nhl_api::{
    Boxscore, ClubStats, DailySchedule, Franchise, GameDate, GameId, GameMatchup, GameType,
    NHLApiError, PlayByPlay, PlayerGameLog, PlayerId, PlayerLanding, PlayerSearchResult, Season,
    SeasonGameTypes, Standing,
};
use tracing::info;

use crate::fixtures;

/// Mock client that returns fixture data instead of making real API calls
pub struct MockClient;

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockClient {
    /// Create a new mock client
    pub fn new() -> Self {
        info!("Creating MockClient for development mode");
        Self
    }
}

#[async_trait]
impl NHLDataProvider for MockClient {
    async fn current_league_standings(&self) -> Result<Vec<Standing>, NHLApiError> {
        info!("MockClient: Returning mock standings");
        Ok(fixtures::create_mock_standings())
    }

    async fn daily_schedule(&self, date: Option<GameDate>) -> Result<DailySchedule, NHLApiError> {
        info!("MockClient: Returning mock schedule for date: {:?}", date);
        Ok(fixtures::create_mock_schedule(date))
    }

    async fn landing(&self, game_id: GameId) -> Result<GameMatchup, NHLApiError> {
        info!(
            "MockClient: Returning mock game matchup for game {}",
            game_id
        );
        Ok(fixtures::create_mock_game_matchup(game_id.into()))
    }

    async fn boxscore(&self, game_id: GameId) -> Result<Boxscore, NHLApiError> {
        info!("MockClient: Returning mock boxscore for game {}", game_id);
        Ok(fixtures::create_mock_boxscore(game_id.into()))
    }

    async fn play_by_play(&self, game_id: GameId) -> Result<PlayByPlay, NHLApiError> {
        info!(
            "MockClient: Returning mock play-by-play for game {}",
            game_id
        );
        Ok(fixtures::create_mock_play_by_play(game_id.into()))
    }

    async fn club_stats(
        &self,
        team_abbrev: &str,
        season: i32,
        game_type: GameType,
    ) -> Result<ClubStats, NHLApiError> {
        info!(
            "MockClient: Returning mock club stats for {} {} {}",
            team_abbrev, season, game_type
        );
        Ok(fixtures::create_mock_club_stats(
            team_abbrev,
            season,
            game_type,
        ))
    }

    async fn club_stats_season(
        &self,
        team_abbr: &str,
    ) -> Result<Vec<SeasonGameTypes>, NHLApiError> {
        info!("MockClient: Returning mock seasons for {}", team_abbr);
        Ok(vec![
            SeasonGameTypes {
                season: Season::new(2024),
                game_types: vec![GameType::RegularSeason],
            },
            SeasonGameTypes {
                season: Season::new(2023),
                game_types: vec![GameType::RegularSeason, GameType::Playoffs],
            },
        ])
    }

    async fn player_landing(&self, player_id: PlayerId) -> Result<PlayerLanding, NHLApiError> {
        info!(
            "MockClient: Returning mock player landing for {}",
            player_id
        );
        Ok(fixtures::create_mock_player_landing(player_id.into()))
    }

    async fn franchises(&self) -> Result<Vec<Franchise>, NHLApiError> {
        info!("MockClient: Returning mock franchises");
        Ok(fixtures::create_mock_franchises())
    }

    async fn search_player(
        &self,
        query: &str,
        limit: Option<i32>,
    ) -> Result<Vec<PlayerSearchResult>, NHLApiError> {
        info!(
            "MockClient: Returning mock player search for '{}' (limit: {:?})",
            query, limit
        );
        Ok(fixtures::create_mock_player_search(query, limit))
    }

    async fn player_game_log(
        &self,
        player_id: PlayerId,
        season: i32,
        game_type: GameType,
    ) -> Result<PlayerGameLog, NHLApiError> {
        info!(
            "MockClient: Returning mock player game log for {} {} {}",
            player_id, season, game_type
        );
        Ok(fixtures::create_mock_player_game_log(
            player_id.into(),
            season,
            game_type,
        ))
    }

    async fn league_standings_for_season(
        &self,
        season_id: i64,
    ) -> Result<Vec<Standing>, NHLApiError> {
        info!(
            "MockClient: Returning mock standings for season {}",
            season_id
        );
        // Just return current mock standings for any season
        Ok(fixtures::create_mock_standings())
    }

    async fn league_standings_for_date(
        &self,
        date: &GameDate,
    ) -> Result<Vec<Standing>, NHLApiError> {
        info!(
            "MockClient: Returning mock standings for date {}",
            date.to_api_string()
        );
        // Just return current mock standings for any date
        Ok(fixtures::create_mock_standings())
    }
}
