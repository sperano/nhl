use crate::data_provider::NHLDataProvider;
use cached::proc_macro::cached;
use nhl_api::{
    DailySchedule, GameDate, GameId, GameMatchup, NHLApiError, PlayerId, Season, Standing,
};

pub use cached::Cached;

#[cfg(test)]
pub async fn clear_all_caches() {
    STANDINGS_CACHE.lock().await.cache_clear();
    SCHEDULE_CACHE.lock().await.cache_clear();
    GAME_CACHE.lock().await.cache_clear();
    BOXSCORE_CACHE.lock().await.cache_clear();
    CLUB_STATS_CACHE.lock().await.cache_clear();
    PLAYER_INFO_CACHE.lock().await.cache_clear();
}

#[cfg(test)]
#[derive(Debug)]
pub struct CacheStats {
    pub standings_entries: usize,
    pub schedule_entries: usize,
    pub game_entries: usize,
    pub boxscore_entries: usize,
    pub club_stats_entries: usize,
    pub player_info_entries: usize,
}

#[cfg(test)]
pub async fn cache_stats() -> CacheStats {
    CacheStats {
        standings_entries: STANDINGS_CACHE.lock().await.cache_size(),
        schedule_entries: SCHEDULE_CACHE.lock().await.cache_size(),
        game_entries: GAME_CACHE.lock().await.cache_size(),
        boxscore_entries: BOXSCORE_CACHE.lock().await.cache_size(),
        club_stats_entries: CLUB_STATS_CACHE.lock().await.cache_size(),
        player_info_entries: PLAYER_INFO_CACHE.lock().await.cache_size(),
    }
}

#[allow(clippy::unused_unit)]
#[cached(
    name = "STANDINGS_CACHE",
    type = "cached::TimedSizedCache<(), Vec<Standing>>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(1, 60) }",
    convert = r#"{ () }"#,
    result = true
)]
pub async fn fetch_standings_cached(
    client: &dyn NHLDataProvider,
) -> Result<Vec<Standing>, NHLApiError> {
    client.current_league_standings().await
}

#[cached(
    name = "SCHEDULE_CACHE",
    type = "cached::TimedSizedCache<String, DailySchedule>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(14, 60) }",
    convert = r#"{ format!("{}", date) }"#,
    result = true
)]
pub async fn fetch_schedule_cached(
    client: &dyn NHLDataProvider,
    date: GameDate,
) -> Result<DailySchedule, NHLApiError> {
    client.daily_schedule(Some(date)).await
}

#[cached(
    name = "GAME_CACHE",
    type = "cached::TimedSizedCache<GameId, GameMatchup>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(100, 30) }",
    convert = r#"{ game_id }"#,
    result = true
)]
pub async fn fetch_game_cached(
    client: &dyn NHLDataProvider,
    game_id: GameId,
) -> Result<GameMatchup, NHLApiError> {
    client.landing(game_id).await
}

#[cached(
    name = "BOXSCORE_CACHE",
    type = "cached::TimedSizedCache<GameId, nhl_api::Boxscore>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(40, 1800) }",
    convert = r#"{ game_id }"#,
    result = true
)]
pub async fn fetch_boxscore_cached(
    client: &dyn NHLDataProvider,
    game_id: GameId,
) -> Result<nhl_api::Boxscore, NHLApiError> {
    client.boxscore(game_id).await
}

#[cached(
    name = "CLUB_STATS_CACHE",
    type = "cached::TimedSizedCache<String, nhl_api::ClubStats>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(32, 3600) }",
    convert = r#"{ format!("{}:{}", team_abbrev, season) }"#,
    result = true
)]
pub async fn fetch_club_stats_cached(
    client: &dyn NHLDataProvider,
    team_abbrev: &str,
    season: i32,
) -> Result<nhl_api::ClubStats, NHLApiError> {
    let season = Season::try_from(season).map_err(|e| NHLApiError::Other(e.to_string()))?;
    client
        .club_stats(team_abbrev, season, nhl_api::GameType::RegularSeason)
        .await
}

#[cached(
    name = "PLAYER_INFO_CACHE",
    type = "cached::TimedSizedCache<PlayerId, nhl_api::PlayerLanding>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(100, 86400) }",
    convert = r#"{ player_id }"#,
    result = true
)]
pub async fn fetch_player_landing_cached(
    client: &dyn NHLDataProvider,
    player_id: PlayerId,
) -> Result<nhl_api::PlayerLanding, NHLApiError> {
    client.player_landing(player_id).await
}

pub async fn refresh_standings(client: &dyn NHLDataProvider) -> Result<Vec<Standing>, NHLApiError> {
    STANDINGS_CACHE.lock().await.cache_clear();
    fetch_standings_cached(client).await
}

pub async fn refresh_game(
    client: &dyn NHLDataProvider,
    game_id: GameId,
) -> Result<GameMatchup, NHLApiError> {
    GAME_CACHE.lock().await.cache_remove(&game_id);
    fetch_game_cached(client, game_id).await
}

pub async fn refresh_schedule(
    client: &dyn NHLDataProvider,
    date: GameDate,
) -> Result<DailySchedule, NHLApiError> {
    let key = date.to_string();
    SCHEDULE_CACHE.lock().await.cache_remove(&key);
    fetch_schedule_cached(client, date).await
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
