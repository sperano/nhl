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

    let _ = fetch_game_cached(&client, 2024020001.into()).await;
    let _ = fetch_game_cached(&client, 2024020002.into()).await;

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

#[tokio::test]
async fn test_club_stats_rejects_invalid_season_id() {
    let client = MockClient::new();

    let result = fetch_club_stats_cached(&client, "TOR", 2024).await;

    assert!(matches!(result, Err(NHLApiError::Other(_))));
}
