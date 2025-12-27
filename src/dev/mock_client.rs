/// Mock NHL API client for development and testing
use crate::data_provider::NHLDataProvider;
use async_trait::async_trait;
use nhl_api::{
    Boxscore, ClubStats, DailySchedule, Franchise, GameDate, GameMatchup, GameType, NHLApiError,
    PlayerLanding, SeasonGameTypes, Standing,
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

    async fn landing(&self, game_id: i64) -> Result<GameMatchup, NHLApiError> {
        info!(
            "MockClient: Returning mock game matchup for game {}",
            game_id
        );
        Ok(fixtures::create_mock_game_matchup(game_id))
    }

    async fn boxscore(&self, game_id: i64) -> Result<Boxscore, NHLApiError> {
        info!("MockClient: Returning mock boxscore for game {}", game_id);
        Ok(fixtures::create_mock_boxscore(game_id))
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
                season: 20242025,
                game_types: vec![GameType::RegularSeason],
            },
            SeasonGameTypes {
                season: 20232024,
                game_types: vec![GameType::RegularSeason, GameType::Playoffs],
            },
        ])
    }

    async fn player_landing(&self, player_id: i64) -> Result<PlayerLanding, NHLApiError> {
        info!(
            "MockClient: Returning mock player landing for {}",
            player_id
        );
        Ok(fixtures::create_mock_player_landing(player_id))
    }

    async fn franchises(&self) -> Result<Vec<Franchise>, NHLApiError> {
        info!("MockClient: Returning mock franchises");
        Ok(fixtures::create_mock_franchises())
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
