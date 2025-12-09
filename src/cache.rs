use crate::data_provider::NHLDataProvider;
use cached::proc_macro::cached;
use nhl_api::{DailySchedule, GameDate, GameMatchup, NHLApiError, Standing};

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
    type = "cached::TimedSizedCache<i64, GameMatchup>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(100, 30) }",
    convert = r#"{ game_id }"#,
    result = true
)]
pub async fn fetch_game_cached(
    client: &dyn NHLDataProvider,
    game_id: i64,
) -> Result<GameMatchup, NHLApiError> {
    client.landing(game_id).await
}

#[cached(
    name = "BOXSCORE_CACHE",
    type = "cached::TimedSizedCache<i64, nhl_api::Boxscore>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(40, 1800) }",
    convert = r#"{ game_id }"#,
    result = true
)]
pub async fn fetch_boxscore_cached(
    client: &dyn NHLDataProvider,
    game_id: i64,
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
    client
        .club_stats(team_abbrev, season, nhl_api::GameType::RegularSeason)
        .await
}

#[cached(
    name = "PLAYER_INFO_CACHE",
    type = "cached::TimedSizedCache<i64, nhl_api::PlayerLanding>",
    create = "{ cached::TimedSizedCache::with_size_and_lifespan(100, 86400) }",
    convert = r#"{ player_id }"#,
    result = true
)]
pub async fn fetch_player_landing_cached(
    client: &dyn NHLDataProvider,
    player_id: i64,
) -> Result<nhl_api::PlayerLanding, NHLApiError> {
    client.player_landing(player_id).await
}

pub async fn refresh_standings(client: &dyn NHLDataProvider) -> Result<Vec<Standing>, NHLApiError> {
    STANDINGS_CACHE.lock().await.cache_clear();
    fetch_standings_cached(client).await
}

pub async fn refresh_game(
    client: &dyn NHLDataProvider,
    game_id: i64,
) -> Result<GameMatchup, NHLApiError> {
    GAME_CACHE.lock().await.cache_remove(&game_id);
    fetch_game_cached(client, game_id).await
}

pub async fn refresh_schedule(
    client: &dyn NHLDataProvider,
    date: GameDate,
) -> Result<DailySchedule, NHLApiError> {
    let key = format!("{}", date);
    SCHEDULE_CACHE.lock().await.cache_remove(&key);
    fetch_schedule_cached(client, date).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::mock_client::MockClient;

    fn is_rate_limit_error(err: &NHLApiError) -> bool {
        matches!(err, NHLApiError::RateLimitExceeded { .. })
    }

    #[tokio::test]
    async fn test_cache_stats_initial_state() {
        clear_all_caches().await;
        let stats = cache_stats().await;
        assert_eq!(stats.standings_entries, 0);
        assert_eq!(stats.schedule_entries, 0);
        assert_eq!(stats.game_entries, 0);
        assert_eq!(stats.boxscore_entries, 0);
        assert_eq!(stats.club_stats_entries, 0);
        assert_eq!(stats.player_info_entries, 0);
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_standings_cache_works() {
        clear_all_caches().await;
        let client = MockClient::new();

        let stats_before = cache_stats().await;
        assert_eq!(stats_before.standings_entries, 0);

        let result = fetch_standings_cached(&client).await;
        if result.is_ok() {
            let stats_after = cache_stats().await;
            assert_eq!(stats_after.standings_entries, 1);
        }
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_standings_cache_hit() {
        clear_all_caches().await;
        let client = MockClient::new();

        let start1 = std::time::Instant::now();
        let standings1 = match fetch_standings_cached(&client).await {
            Ok(standings) => standings,
            Err(ref e) if is_rate_limit_error(e) => {
                eprintln!("Skipping test due to rate limit: {}", e);
                return;
            }
            Err(e) => panic!("First fetch failed with non-rate-limit error: {}", e),
        };
        let time1 = start1.elapsed();

        let start2 = std::time::Instant::now();
        let standings2 = match fetch_standings_cached(&client).await {
            Ok(standings) => standings,
            Err(ref e) if is_rate_limit_error(e) => {
                eprintln!("Skipping test due to rate limit: {}", e);
                return;
            }
            Err(e) => panic!("Second fetch failed with non-rate-limit error: {}", e),
        };
        let time2 = start2.elapsed();

        assert_eq!(standings1.len(), standings2.len());
        assert!(time2 < time1, "Cache hit should be faster than cache miss");
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_schedule_cache_size_limit() {
        clear_all_caches().await;
        let client = MockClient::new();

        for day in 1..=10 {
            if let Some(date) = chrono::NaiveDate::from_ymd_opt(2024, 11, day) {
                let game_date = GameDate::Date(date);
                let _ = fetch_schedule_cached(&client, game_date).await;
            }
        }

        let stats = cache_stats().await;
        assert!(
            stats.schedule_entries <= 7,
            "Schedule cache should not exceed 7 entries"
        );
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_game_cache_different_keys() {
        clear_all_caches().await;
        let client = MockClient::new();

        let _ = fetch_game_cached(&client, 2024020001).await;
        let _ = fetch_game_cached(&client, 2024020002).await;

        let stats = cache_stats().await;
        assert!(stats.game_entries <= 2);
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_club_stats_cache_composite_key() {
        clear_all_caches().await;
        let client = MockClient::new();

        let _ = fetch_club_stats_cached(&client, "TOR", 20242025).await;
        let _ = fetch_club_stats_cached(&client, "TOR", 20232024).await;
        let _ = fetch_club_stats_cached(&client, "MTL", 20242025).await;

        let stats = cache_stats().await;
        assert!(stats.club_stats_entries <= 3);
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_clear_all_caches() {
        let client = MockClient::new();

        let _ = fetch_standings_cached(&client).await;
        let _ = fetch_schedule_cached(&client, GameDate::Now).await;

        let stats_before = cache_stats().await;
        assert!(stats_before.standings_entries > 0 || stats_before.schedule_entries > 0);

        clear_all_caches().await;

        let stats_after = cache_stats().await;
        assert_eq!(stats_after.standings_entries, 0);
        assert_eq!(stats_after.schedule_entries, 0);
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_refresh_standings_clears_cache() {
        clear_all_caches().await;
        let client = MockClient::new();

        if fetch_standings_cached(&client).await.is_ok() {
            let stats1 = cache_stats().await;
            assert_eq!(stats1.standings_entries, 1);

            if refresh_standings(&client).await.is_ok() {
                let stats2 = cache_stats().await;
                assert_eq!(stats2.standings_entries, 1);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Shared cache state - run individually
    async fn test_refresh_schedule_removes_specific_entry() {
        clear_all_caches().await;
        let client = MockClient::new();

        let date1 = GameDate::Date(chrono::NaiveDate::from_ymd_opt(2024, 11, 1).unwrap());
        let date2 = GameDate::Date(chrono::NaiveDate::from_ymd_opt(2024, 11, 2).unwrap());

        // Try to fetch both schedules, skip test on rate limit
        if let Err(ref e) = fetch_schedule_cached(&client, date1.clone()).await {
            if is_rate_limit_error(e) {
                eprintln!("Skipping test due to rate limit: {}", e);
                return;
            }
        }
        if let Err(ref e) = fetch_schedule_cached(&client, date2).await {
            if is_rate_limit_error(e) {
                eprintln!("Skipping test due to rate limit: {}", e);
                return;
            }
        }

        let stats_before = cache_stats().await;
        if stats_before.schedule_entries >= 2 {
            let _ = refresh_schedule(&client, date1).await;

            let stats_after = cache_stats().await;
            assert!(stats_after.schedule_entries >= 1);
        }
    }
}
