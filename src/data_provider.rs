/// Trait for providing NHL data, abstracting over real API clients and mock implementations
use async_trait::async_trait;
use nhl_api::{
    Boxscore, ClubStats, DailySchedule, Franchise, GameDate, GameMatchup, GameType, NHLApiError,
    PlayerLanding, SeasonGameTypes, Standing,
};

/// Trait for NHL data providers, implemented by both real Client and MockClient
#[async_trait]
pub trait NHLDataProvider: Send + Sync {
    /// Get current league standings
    async fn current_league_standings(&self) -> Result<Vec<Standing>, NHLApiError>;

    /// Get daily schedule for a specific date
    async fn daily_schedule(&self, date: Option<GameDate>) -> Result<DailySchedule, NHLApiError>;

    /// Get game landing data (summary with period scores)
    async fn landing(&self, game_id: i64) -> Result<GameMatchup, NHLApiError>;

    /// Get boxscore for a specific game
    async fn boxscore(&self, game_id: i64) -> Result<Boxscore, NHLApiError>;

    /// Get club stats for a team
    async fn club_stats(
        &self,
        team_abbrev: &str,
        season: i32,
        game_type: GameType,
    ) -> Result<ClubStats, NHLApiError>;

    /// Get available seasons for a team
    async fn club_stats_season(&self, team_abbr: &str)
        -> Result<Vec<SeasonGameTypes>, NHLApiError>;

    /// Get player landing data
    async fn player_landing(&self, player_id: i64) -> Result<PlayerLanding, NHLApiError>;

    /// Get all franchises
    async fn franchises(&self) -> Result<Vec<Franchise>, NHLApiError>;

    /// Get league standings for a specific season
    async fn league_standings_for_season(
        &self,
        season_id: i64,
    ) -> Result<Vec<Standing>, NHLApiError>;

    /// Get league standings for a specific date
    async fn league_standings_for_date(
        &self,
        date: &GameDate,
    ) -> Result<Vec<Standing>, NHLApiError>;
}

/// Implement the trait for the real nhl_api::Client
#[async_trait]
impl NHLDataProvider for nhl_api::Client {
    async fn current_league_standings(&self) -> Result<Vec<Standing>, NHLApiError> {
        self.current_league_standings().await
    }

    async fn daily_schedule(&self, date: Option<GameDate>) -> Result<DailySchedule, NHLApiError> {
        self.daily_schedule(date).await
    }

    async fn landing(&self, game_id: i64) -> Result<GameMatchup, NHLApiError> {
        self.landing(game_id).await
    }

    async fn boxscore(&self, game_id: i64) -> Result<Boxscore, NHLApiError> {
        self.boxscore(game_id).await
    }

    async fn club_stats(
        &self,
        team_abbrev: &str,
        season: i32,
        game_type: GameType,
    ) -> Result<ClubStats, NHLApiError> {
        self.club_stats(team_abbrev, season, game_type).await
    }

    async fn club_stats_season(
        &self,
        team_abbr: &str,
    ) -> Result<Vec<SeasonGameTypes>, NHLApiError> {
        self.club_stats_season(team_abbr).await
    }

    async fn player_landing(&self, player_id: i64) -> Result<PlayerLanding, NHLApiError> {
        self.player_landing(player_id).await
    }

    async fn franchises(&self) -> Result<Vec<Franchise>, NHLApiError> {
        self.franchises().await
    }

    async fn league_standings_for_season(
        &self,
        season_id: i64,
    ) -> Result<Vec<Standing>, NHLApiError> {
        self.league_standings_for_season(season_id).await
    }

    async fn league_standings_for_date(
        &self,
        date: &GameDate,
    ) -> Result<Vec<Standing>, NHLApiError> {
        self.league_standings_for_date(date).await
    }
}
