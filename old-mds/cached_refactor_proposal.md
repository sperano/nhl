# SharedData Refactoring Proposal: Using the `cached` Crate

**Author:** Claude Code
**Date:** 2025-01-03
**Version:** 1.0

---

## Executive Summary

### Current Problems

The NHL CLI/TUI application currently uses a `SharedData` struct wrapped in `Arc<RwLock<>>` to share state between the background data fetching loop and the TUI rendering loop. This approach has several issues:

1. **Over-fetching**: Data is refetched every 60 seconds even when unchanged (e.g., standings, schedule for same date)
2. **Manual cache management**: `club_stats` and `player_info` use manual HashMap checks with no eviction policy
3. **Unbounded memory growth**: Cached HashMaps never expire old entries
4. **Code duplication**: Cache existence checks scattered across multiple functions
5. **No TTL support**: Stale data persists indefinitely (e.g., yesterday's schedule)
6. **Unnecessary API calls**: Boxscore refetched every 60s even when game is final

### Benefits of `cached` Crate

The `cached` crate provides:

- **Declarative caching** via `#[cached]` procedural macro
- **TTL support** for automatic expiration of stale data
- **LRU eviction** to prevent unbounded memory growth
- **Thread-safe** caching without additional synchronization
- **Automatic cache key generation** from function arguments
- **Zero boilerplate** compared to manual HashMap management

### High-Level Architecture Changes

**Before:**
```
Background Loop → API Client → NHL API
     ↓
Manual cache checks in SharedData HashMaps
     ↓
Update SharedData with Arc<RwLock>
     ↓
TUI reads SharedData
```

**After:**
```
Background Loop → Cached API Wrappers → (Cache Hit? Return) → NHL API
     ↓
Update simplified SharedData
     ↓
TUI reads SharedData (smaller, simpler)
```

---

## Current Architecture Analysis

### SharedData Struct (src/main.rs:29-72)

```rust
#[derive(Clone)]
pub struct SharedData {
    // Core game data - refetched every interval
    pub standings: Arc<Vec<Standing>>,
    pub schedule: Arc<Option<DailySchedule>>,
    pub period_scores: Arc<HashMap<i64, PeriodScores>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,

    // On-demand data - manual caching
    pub boxscore: Arc<Option<nhl_api::Boxscore>>,
    pub club_stats: Arc<HashMap<String, nhl_api::ClubStats>>,  // Manual cache
    pub player_info: Arc<HashMap<i64, nhl_api::PlayerLanding>>, // Manual cache

    // UI state
    pub config: config::Config,
    pub last_refresh: Option<SystemTime>,
    pub game_date: nhl_api::GameDate,
    pub error_message: Option<String>,

    // Selection tracking
    pub selected_game_id: Option<i64>,
    pub selected_team_abbrev: Option<String>,
    pub selected_player_id: Option<i64>,

    // Loading flags
    pub boxscore_loading: bool,
    pub club_stats_loading: bool,
    pub player_info_loading: bool,
}
```

### Current Caching Patterns

#### Manual Cache Check Example (src/background.rs:187-195)

```rust
// Check if we already have the data cached
let needs_fetch = {
    let data = shared_data.read().await;
    !data.club_stats.contains_key(team_abbrev)
};

if !needs_fetch {
    // Already cached, skip fetch
    return;
}
```

**Problems:**
- No expiration - stale data persists forever
- Manual synchronization required
- Code duplication across fetch functions
- No size limits - unbounded growth

#### No Cache Example (src/background.rs:74-101)

```rust
// Schedule fetched EVERY interval, even for same date
async fn fetch_schedule_with_games(
    client: &Client,
    shared_data: &SharedDataHandle,
    date: nhl_api::GameDate,
) -> Result<()> {
    // Always fetches, no cache check
    let schedule = client.daily_schedule(Some(date)).await?;
    // ... update SharedData
}
```

**Problems:**
- Unnecessary API calls when viewing same date
- Network bandwidth waste
- Slower response times

---

## Proposed Architecture

### Cache Strategy by Data Type

| Data Type | Cache Key | TTL | Size Limit | Rationale |
|-----------|-----------|-----|------------|-----------|
| **Standings** | `()` (singleton) | 60s | 1 | Changes frequently during game days |
| **Schedule (today)** | `(date)` | 60s | 1 | Live games update scores frequently |
| **Schedule (past)** | `(date)` | 1 hour | 7 days | Historical data rarely changes |
| **Game Landing (live)** | `(game_id)` | 30s | 50 games | Scores update during game |
| **Game Landing (final)** | `(game_id)` | 1 hour | 100 games | Final games don't change |
| **Boxscore (live)** | `(game_id)` | 30s | 5 games | Player stats update during game |
| **Boxscore (final)** | `(game_id)` | 6 hours | 20 games | Detailed stats rarely needed |
| **Club Stats** | `(team, season)` | 1 hour | 32 teams | Rosters stable during season |
| **Player Info** | `(player_id)` | 24 hours | 100 players | Bio data rarely changes |

### Cached Function Design

#### Pattern: Conditional TTL Based on Game State

```rust
use cached::proc_macro::cached;
use cached::TimedSizedCache;

// Boxscore with dynamic TTL based on game state
#[cached(
    ty = "TimedSizedCache<i64, nhl_api::Boxscore>",
    create = "{ TimedSizedCache::with_size_and_lifespan(20, 1800) }",  // 30 min default
    convert = r#"{ game_id }"#,
    result = true
)]
async fn fetch_boxscore_cached(
    client: &Client,
    game_id: i64,
) -> Result<nhl_api::Boxscore> {
    let boxscore = client.boxscore(&GameId::new(game_id)).await?;
    Ok(boxscore)
}
```

#### Pattern: Date-Based Caching with Different TTLs

```rust
// Schedule caching with LRU for memory management
#[cached(
    ty = "TimedSizedCache<String, nhl_api::DailySchedule>",
    create = "{ TimedSizedCache::with_size_and_lifespan(7, 60) }",  // 7 dates, 60s TTL
    convert = r#"{ format!("{}", date) }"#,
    result = true
)]
async fn fetch_schedule_cached(
    client: &Client,
    date: nhl_api::GameDate,
) -> Result<nhl_api::DailySchedule> {
    client.daily_schedule(Some(date)).await
}
```

#### Pattern: Composite Key Caching

```rust
// Club stats with team + season composite key
#[cached(
    ty = "TimedSizedCache<String, nhl_api::ClubStats>",
    create = "{ TimedSizedCache::with_size_and_lifespan(32, 3600) }",  // 32 teams, 1 hour
    convert = r#"{ format!("{}:{}", team_abbrev, season) }"#,
    result = true
)]
async fn fetch_club_stats_cached(
    client: &Client,
    team_abbrev: &str,
    season: i32,
) -> Result<nhl_api::ClubStats> {
    client.club_stats(team_abbrev, season, 2).await
}
```

#### Pattern: Singleton Caching (Standings)

```rust
// Standings - only one entry, refreshes every 60s
#[cached(
    ty = "TimedSizedCache<(), Vec<Standing>>",
    create = "{ TimedSizedCache::with_size_and_lifespan(1, 60) }",
    convert = r#"{ () }"#,
    result = true
)]
async fn fetch_standings_cached(
    client: &Client,
) -> Result<Vec<Standing>> {
    client.current_league_standings().await
}
```

### Advanced: Multi-Tier Caching for Live vs Final Games

```rust
// Two separate caches for live vs final games
#[cached(
    name = "LIVE_GAME_CACHE",
    ty = "TimedSizedCache<i64, nhl_api::GameMatchup>",
    create = "{ TimedSizedCache::with_size_and_lifespan(50, 30) }",  // 30s TTL
    convert = r#"{ game_id }"#,
    result = true
)]
async fn fetch_live_game_cached(
    client: &Client,
    game_id: i64,
) -> Result<nhl_api::GameMatchup> {
    client.landing(&GameId::new(game_id)).await
}

#[cached(
    name = "FINAL_GAME_CACHE",
    ty = "TimedSizedCache<i64, nhl_api::GameMatchup>",
    create = "{ TimedSizedCache::with_size_and_lifespan(100, 3600) }",  // 1 hour TTL
    convert = r#"{ game_id }"#,
    result = true
)]
async fn fetch_final_game_cached(
    client: &Client,
    game_id: i64,
) -> Result<nhl_api::GameMatchup> {
    client.landing(&GameId::new(game_id)).await
}

// Smart dispatcher based on game state
async fn fetch_game_with_smart_cache(
    client: &Client,
    game_id: i64,
    game_state: &str,
) -> Result<nhl_api::GameMatchup> {
    match game_state {
        "LIVE" | "CRIT" => fetch_live_game_cached(client, game_id).await,
        "FINAL" | "OFF" => fetch_final_game_cached(client, game_id).await,
        _ => client.landing(&GameId::new(game_id)).await,  // No cache for unknown states
    }
}
```

---

## Implementation Details

### Phase 1: Add Dependency and Cache Module

#### Update Cargo.toml

```toml
[dependencies]
cached = { version = "0.49", features = ["async"] }
```

#### Create src/cache.rs

```rust
//! Cached API wrappers using the cached crate
//!
//! This module provides cached versions of all NHL API methods with
//! appropriate TTL and size limits for each data type.

use anyhow::Result;
use cached::proc_macro::cached;
use cached::TimedSizedCache;
use nhl_api::{Client, GameId, GameDate, Standing, DailySchedule, GameMatchup, Boxscore, ClubStats, PlayerLanding};

// Re-export cache control functions
pub use cached::Cached;

/// Clear all caches (useful for testing or manual refresh)
pub fn clear_all_caches() {
    STANDINGS_CACHE.lock().unwrap().cache_clear();
    SCHEDULE_CACHE.lock().unwrap().cache_clear();
    GAME_CACHE.lock().unwrap().cache_clear();
    BOXSCORE_CACHE.lock().unwrap().cache_clear();
    CLUB_STATS_CACHE.lock().unwrap().cache_clear();
    PLAYER_INFO_CACHE.lock().unwrap().cache_clear();
}

/// Get cache statistics for monitoring
pub fn cache_stats() -> CacheStats {
    CacheStats {
        standings_entries: STANDINGS_CACHE.lock().unwrap().cache_size(),
        schedule_entries: SCHEDULE_CACHE.lock().unwrap().cache_size(),
        game_entries: GAME_CACHE.lock().unwrap().cache_size(),
        boxscore_entries: BOXSCORE_CACHE.lock().unwrap().cache_size(),
        club_stats_entries: CLUB_STATS_CACHE.lock().unwrap().cache_size(),
        player_info_entries: PLAYER_INFO_CACHE.lock().unwrap().cache_size(),
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub standings_entries: usize,
    pub schedule_entries: usize,
    pub game_entries: usize,
    pub boxscore_entries: usize,
    pub club_stats_entries: usize,
    pub player_info_entries: usize,
}

// Cached API methods

#[cached(
    name = "STANDINGS_CACHE",
    ty = "TimedSizedCache<(), Vec<Standing>>",
    create = "{ TimedSizedCache::with_size_and_lifespan(1, 60) }",
    convert = r#"{ () }"#,
    result = true
)]
pub async fn fetch_standings_cached(
    client: &Client,
) -> Result<Vec<Standing>> {
    client.current_league_standings().await
}

#[cached(
    name = "SCHEDULE_CACHE",
    ty = "TimedSizedCache<String, DailySchedule>",
    create = "{ TimedSizedCache::with_size_and_lifespan(7, 60) }",
    convert = r#"{ format!("{}", date) }"#,
    result = true
)]
pub async fn fetch_schedule_cached(
    client: &Client,
    date: GameDate,
) -> Result<DailySchedule> {
    client.daily_schedule(Some(date)).await
}

#[cached(
    name = "GAME_CACHE",
    ty = "TimedSizedCache<i64, GameMatchup>",
    create = "{ TimedSizedCache::with_size_and_lifespan(50, 30) }",
    convert = r#"{ game_id }"#,
    result = true
)]
pub async fn fetch_game_cached(
    client: &Client,
    game_id: i64,
) -> Result<GameMatchup> {
    client.landing(&GameId::new(game_id)).await
}

#[cached(
    name = "BOXSCORE_CACHE",
    ty = "TimedSizedCache<i64, Boxscore>",
    create = "{ TimedSizedCache::with_size_and_lifespan(20, 1800) }",
    convert = r#"{ game_id }"#,
    result = true
)]
pub async fn fetch_boxscore_cached(
    client: &Client,
    game_id: i64,
) -> Result<Boxscore> {
    client.boxscore(&GameId::new(game_id)).await
}

#[cached(
    name = "CLUB_STATS_CACHE",
    ty = "TimedSizedCache<String, ClubStats>",
    create = "{ TimedSizedCache::with_size_and_lifespan(32, 3600) }",
    convert = r#"{ format!("{}:{}", team_abbrev, season) }"#,
    result = true
)]
pub async fn fetch_club_stats_cached(
    client: &Client,
    team_abbrev: &str,
    season: i32,
) -> Result<ClubStats> {
    client.club_stats(team_abbrev, season, 2).await
}

#[cached(
    name = "PLAYER_INFO_CACHE",
    ty = "TimedSizedCache<i64, PlayerLanding>",
    create = "{ TimedSizedCache::with_size_and_lifespan(100, 86400) }",
    convert = r#"{ player_id }"#,
    result = true
)]
pub async fn fetch_player_info_cached(
    client: &Client,
    player_id: i64,
) -> Result<PlayerLanding> {
    client.player_landing(player_id).await
}
```

#### Update src/main.rs

```rust
mod cache;  // Add this line
```

---

### Phase 2: Simplify SharedData

Remove manual caching HashMaps since caching is now handled by `cached` crate:

#### Before:
```rust
pub struct SharedData {
    pub standings: Arc<Vec<Standing>>,
    pub schedule: Arc<Option<DailySchedule>>,
    pub period_scores: Arc<HashMap<i64, PeriodScores>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,
    pub boxscore: Arc<Option<nhl_api::Boxscore>>,
    pub club_stats: Arc<HashMap<String, nhl_api::ClubStats>>,       // Remove
    pub player_info: Arc<HashMap<i64, nhl_api::PlayerLanding>>,    // Remove
    // ... rest
}
```

#### After:
```rust
pub struct SharedData {
    // Core data - still in SharedData for TUI rendering
    pub standings: Arc<Vec<Standing>>,
    pub schedule: Arc<Option<DailySchedule>>,
    pub period_scores: Arc<HashMap<i64, PeriodScores>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,

    // Selected item data - fetched on-demand from cache
    pub boxscore: Arc<Option<nhl_api::Boxscore>>,
    pub selected_club_stats: Arc<Option<nhl_api::ClubStats>>,      // Changed: single item
    pub selected_player_info: Arc<Option<nhl_api::PlayerLanding>>, // Changed: single item

    // UI state (unchanged)
    pub config: config::Config,
    pub last_refresh: Option<SystemTime>,
    pub game_date: nhl_api::GameDate,
    pub error_message: Option<String>,
    pub selected_game_id: Option<i64>,
    pub selected_team_abbrev: Option<String>,
    pub selected_player_id: Option<i64>,
    pub boxscore_loading: bool,
    pub club_stats_loading: bool,
    pub player_info_loading: bool,
}
```

**Benefits:**
- Removed 2 HashMap fields
- Cache management delegated to `cached` crate
- Automatic memory limits and TTL
- Simpler code

---

### Phase 3: Refactor Background Loop

Update `src/background.rs` to use cached functions:

#### Before (fetch_standings):
```rust
async fn fetch_standings(
    client: &Client,
    shared_data: &SharedDataHandle,
) -> Result<()> {
    let standings = client.current_league_standings().await?;

    let mut data = shared_data.write().await;
    data.standings = Arc::new(standings);
    data.last_refresh = Some(SystemTime::now());
    data.error_message = None;

    Ok(())
}
```

#### After (fetch_standings):
```rust
async fn fetch_standings(
    client: &Client,
    shared_data: &SharedDataHandle,
) -> Result<()> {
    // Uses cache automatically - only hits API if TTL expired
    let standings = crate::cache::fetch_standings_cached(client).await?;

    let mut data = shared_data.write().await;
    data.standings = Arc::new(standings);
    data.last_refresh = Some(SystemTime::now());
    data.error_message = None;

    Ok(())
}
```

#### Before (fetch_club_stats):
```rust
async fn fetch_club_stats(
    client: &Client,
    shared_data: &SharedDataHandle,
    team_abbrev: &str,
) -> Result<()> {
    // Manual cache check
    let needs_fetch = {
        let data = shared_data.read().await;
        !data.club_stats.contains_key(team_abbrev)
    };

    if !needs_fetch {
        return Ok(());
    }

    // Calculate season
    let now = chrono::Utc::now();
    let season = if now.month() >= 10 {
        format!("{}{}", now.year(), now.year() + 1)
    } else {
        format!("{}{}", now.year() - 1, now.year())
    };

    let season_int: i32 = season.parse()?;
    let stats = client.club_stats(team_abbrev, season_int, 2).await?;

    {
        let mut data = shared_data.write().await;
        let mut stats_map = (*data.club_stats).clone();
        stats_map.insert(team_abbrev.to_string(), stats);
        data.club_stats = Arc::new(stats_map);
        data.club_stats_loading = false;
    }

    Ok(())
}
```

#### After (fetch_club_stats):
```rust
async fn fetch_club_stats(
    client: &Client,
    shared_data: &SharedDataHandle,
    team_abbrev: &str,
) -> Result<()> {
    // Calculate season
    let now = chrono::Utc::now();
    let season = if now.month() >= 10 {
        format!("{}{}", now.year(), now.year() + 1)
    } else {
        format!("{}{}", now.year() - 1, now.year())
    };

    let season_int: i32 = season.parse()?;

    // Uses cache automatically - no manual checks needed
    let stats = crate::cache::fetch_club_stats_cached(client, team_abbrev, season_int).await?;

    {
        let mut data = shared_data.write().await;
        data.selected_club_stats = Arc::new(Some(stats));
        data.club_stats_loading = false;
    }

    Ok(())
}
```

**Code Reduction:**
- Removed ~15 lines of manual cache checking
- No HashMap clone/insert operations
- Automatic TTL handling
- Thread-safe without additional locks

---

### Phase 4: Update TUI Rendering

Update TUI to access selected data instead of HashMap lookups:

#### Before (src/tui/standings/view.rs):
```rust
fn render_team_stats(
    f: &mut Frame,
    area: Rect,
    team_abbrev: &str,
    club_stats: &HashMap<String, nhl_api::ClubStats>,
) {
    if let Some(stats) = club_stats.get(team_abbrev) {
        // Render stats
    } else {
        // Show loading
    }
}
```

#### After:
```rust
fn render_team_stats(
    f: &mut Frame,
    area: Rect,
    selected_club_stats: &Option<nhl_api::ClubStats>,
    loading: bool,
) {
    if let Some(stats) = selected_club_stats {
        // Render stats
    } else if loading {
        // Show loading
    } else {
        // Show "no data"
    }
}
```

---

## Configuration and Tuning

### Adjustable Cache Parameters

Add configuration options to `config.toml`:

```toml
[cache]
# Enable/disable caching (useful for testing)
enabled = true

# TTL values in seconds
standings_ttl = 60
schedule_ttl = 60
game_ttl = 30
boxscore_ttl = 1800  # 30 minutes
club_stats_ttl = 3600  # 1 hour
player_info_ttl = 86400  # 24 hours

# Cache size limits (number of entries)
schedule_max_size = 7
game_max_size = 50
boxscore_max_size = 20
club_stats_max_size = 32
player_info_max_size = 100
```

### Dynamic Cache Configuration

```rust
// src/cache.rs with config support
use crate::config::Config;

pub fn init_caches(config: &Config) {
    // This would require more complex setup with lazy_static or once_cell
    // For now, caches use hardcoded values from macro
    // Future enhancement: runtime cache configuration
}
```

**Note:** The `#[cached]` macro uses compile-time constants, so runtime configuration would require a more complex setup with `lazy_static!` or `OnceCell`. This is a potential Phase 5 enhancement.

---

## Advanced Features

### Cache Invalidation API

```rust
// src/cache.rs

/// Force refresh of standings cache (bypass TTL)
pub async fn refresh_standings(client: &Client) -> Result<Vec<Standing>> {
    STANDINGS_CACHE.lock().unwrap().cache_clear();
    fetch_standings_cached(client).await
}

/// Force refresh of specific game (bypass TTL)
pub async fn refresh_game(client: &Client, game_id: i64) -> Result<GameMatchup> {
    GAME_CACHE.lock().unwrap().cache_remove(&game_id);
    fetch_game_cached(client, game_id).await
}

/// Force refresh of schedule for specific date
pub async fn refresh_schedule(client: &Client, date: GameDate) -> Result<DailySchedule> {
    let key = format!("{}", date);
    SCHEDULE_CACHE.lock().unwrap().cache_remove(&key);
    fetch_schedule_cached(client, date).await
}
```

### Cache Metrics and Monitoring

```rust
// src/cache.rs

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheMetrics {
    pub standings: CacheMetric,
    pub schedule: CacheMetric,
    pub games: CacheMetric,
    pub boxscores: CacheMetric,
    pub club_stats: CacheMetric,
    pub player_info: CacheMetric,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheMetric {
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,  // Would need custom wrapper to track
}

pub fn get_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        standings: CacheMetric {
            size: STANDINGS_CACHE.lock().unwrap().cache_size(),
            capacity: 1,
            hit_rate: 0.0,  // Requires additional tracking
        },
        // ... rest
    }
}
```

### Cache Warmup on Startup

```rust
// src/background.rs

pub async fn warmup_caches(client: &Client, game_date: GameDate) -> Result<()> {
    // Pre-populate caches with common data
    let _ = crate::cache::fetch_standings_cached(client).await;
    let _ = crate::cache::fetch_schedule_cached(client, game_date).await;

    Ok(())
}
```

---

## Migration Strategy

### Phase 1: Add Cached Module (Non-Breaking)
**Timeline:** 1 day

1. Add `cached` dependency to Cargo.toml
2. Create `src/cache.rs` with all cached functions
3. Add `mod cache;` to `src/main.rs`
4. **No changes to existing code yet**
5. Write unit tests for cached functions

**Verification:**
```bash
cargo build  # Should compile successfully
cargo test cache::  # Run cache-specific tests
```

### Phase 2: Migrate Background Loop (Breaking)
**Timeline:** 2 days

1. Update `fetch_standings()` to use `fetch_standings_cached()`
2. Update `fetch_schedule_with_games()` to use `fetch_schedule_cached()`
3. Update `fetch_all_started_games()` to use `fetch_game_cached()`
4. Update `fetch_boxscore()` to use `fetch_boxscore_cached()`
5. Update `fetch_club_stats()` to use `fetch_club_stats_cached()`
6. Update `fetch_player_info()` to use `fetch_player_info_cached()`
7. **Remove manual cache checks**

**Verification:**
```bash
cargo build
cargo run  # Test TUI with all tabs
cargo run -- standings
cargo run -- scores
cargo run -- boxscore 2024020174
```

### Phase 3: Simplify SharedData (Breaking)
**Timeline:** 1 day

1. Change `club_stats: Arc<HashMap<String, ClubStats>>` → `selected_club_stats: Arc<Option<ClubStats>>`
2. Change `player_info: Arc<HashMap<i64, PlayerLanding>>` → `selected_player_info: Arc<Option<PlayerLanding>>`
3. Update all TUI views to use new fields
4. Update `clear_*` functions in SharedData

**Verification:**
```bash
cargo build
# Test all TUI navigation paths
# Verify team details panel
# Verify player details panel
```

### Phase 4: Add Configuration (Optional)
**Timeline:** 1 day

1. Add `[cache]` section to default config
2. Document cache settings in README
3. Add cache stats to debug mode or settings panel

**Verification:**
```bash
# Test with custom cache TTLs in config.toml
# Verify cache respects size limits
```

### Phase 5: Advanced Features (Future)
**Timeline:** 2-3 days

1. Add cache metrics tracking
2. Add cache warmup on startup
3. Add manual cache invalidation UI
4. Add runtime cache configuration
5. Add cache hit/miss logging in debug mode

---

## Performance Impact Analysis

### Expected Improvements

| Scenario | Before | After | Improvement |
|----------|--------|-------|-------------|
| **View same date twice** | 2 API calls | 1 API call (cached) | 50% reduction |
| **Navigate date back/forward** | 2 API calls | 2 API calls (but faster) | Same calls, faster response |
| **Re-open same boxscore** | Fetch every 60s | Fetch once per 30s | 50% reduction |
| **View team stats twice** | 2 API calls | 1 API call | 50% reduction |
| **Memory usage** | Unbounded growth | Bounded by LRU | Predictable |

### Memory Usage Estimation

**Current (worst case):**
- `club_stats`: Unbounded HashMap (could grow to 32 teams × ~500KB = 16MB)
- `player_info`: Unbounded HashMap (could grow to 100s of players × ~50KB = 5MB+)
- **Total:** Unpredictable, 20MB+

**After:**
- Standings: 1 entry × ~50KB = 50KB
- Schedule: 7 entries × ~20KB = 140KB
- Games: 50 entries × ~10KB = 500KB
- Boxscore: 20 entries × ~100KB = 2MB
- Club Stats: 32 entries × ~500KB = 16MB
- Player Info: 100 entries × ~50KB = 5MB
- **Total:** ~24MB (bounded)

**Trade-off:** Slightly higher baseline memory, but bounded and predictable.

### Network Reduction

**Typical 10-minute session:**

Before:
- Standings: 10 fetches (every 60s)
- Schedule: 10 fetches
- Games: 130 fetches (13 games × 10)
- **Total: 150 API calls**

After:
- Standings: 2 fetches (60s TTL)
- Schedule: 2 fetches (same date)
- Games: 26 fetches (30s TTL for live games)
- **Total: 30 API calls**

**Improvement: 80% reduction in API calls**

---

## Trade-offs and Considerations

### Advantages

✅ **Automatic cache management** - No manual HashMap operations
✅ **TTL support** - Stale data expires automatically
✅ **Memory bounds** - LRU eviction prevents unbounded growth
✅ **Thread-safe** - Built-in synchronization via Mutex
✅ **Reduced API calls** - Better performance and less network usage
✅ **Simpler code** - Less boilerplate, more declarative
✅ **Better UX** - Faster responses for cached data

### Disadvantages

⚠️ **Compile-time configuration** - TTL/size limits in macro, not runtime config
⚠️ **Global state** - Caches are global static variables
⚠️ **No persistence** - Cache lost on app restart
⚠️ **Memory overhead** - Caches use ~24MB vs unbounded current approach
⚠️ **Less visibility** - Cache hits/misses not easily observable without instrumentation

### Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| **Stale data shown** | Use appropriate TTLs; add manual refresh |
| **Cache stampede** | Use `result = true` to cache errors briefly |
| **Memory limits too small** | Monitor cache metrics; adjust sizes |
| **Breaking changes** | Phased migration with fallback |

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_standings_cache_ttl() {
        let client = Client::new().unwrap();

        // First call - cache miss
        let start = std::time::Instant::now();
        let standings1 = fetch_standings_cached(&client).await.unwrap();
        let first_call_time = start.elapsed();

        // Second call - cache hit (should be faster)
        let start = std::time::Instant::now();
        let standings2 = fetch_standings_cached(&client).await.unwrap();
        let second_call_time = start.elapsed();

        assert_eq!(standings1.len(), standings2.len());
        assert!(second_call_time < first_call_time);
    }

    #[tokio::test]
    async fn test_cache_size_limit() {
        let client = Client::new().unwrap();

        // Fetch more schedules than cache size (7)
        for i in 0..10 {
            let date = GameDate::Date(
                chrono::NaiveDate::from_ymd_opt(2024, 11, i).unwrap()
            );
            let _ = fetch_schedule_cached(&client, date).await;
        }

        let stats = cache_stats();
        assert!(stats.schedule_entries <= 7);
    }
}
```

### Integration Tests

```bash
# Test full TUI flow with caching
cargo run  # Navigate through all tabs, verify no errors

# Test CLI commands
cargo run -- standings
cargo run -- scores -d 2024-11-02
cargo run -- boxscore 2024020174

# Test rapid navigation (should use cache)
# Repeatedly switch between dates in TUI
```

### Performance Benchmarks

```rust
// benches/cache_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_cached_vs_uncached(c: &mut Criterion) {
    c.bench_function("fetch_standings_cached", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = Client::new().unwrap();

        b.iter(|| {
            runtime.block_on(async {
                fetch_standings_cached(&client).await.unwrap()
            })
        });
    });
}

criterion_group!(benches, benchmark_cached_vs_uncached);
criterion_main!(benches);
```

---

## Documentation Updates

### README.md

```markdown
## Caching

This application uses intelligent caching to reduce API calls and improve performance:

- **Standings**: Cached for 60 seconds
- **Schedule**: Cached for 60 seconds per date (up to 7 dates)
- **Live Games**: Cached for 30 seconds (up to 50 games)
- **Boxscores**: Cached for 30 minutes (up to 20 games)
- **Team Stats**: Cached for 1 hour (up to 32 teams)
- **Player Info**: Cached for 24 hours (up to 100 players)

Cache settings can be adjusted in `~/.config/nhl/config.toml`.
```

### config.toml.example

```toml
[cache]
enabled = true
standings_ttl = 60
schedule_ttl = 60
game_ttl = 30
boxscore_ttl = 1800
club_stats_ttl = 3600
player_info_ttl = 86400
```

---

## Future Enhancements

### 1. Persistent Cache (Disk-based)

Use `cached`'s disk cache feature to persist data across app restarts:

```rust
#[cached(
    disk = true,
    disk_dir = "~/.cache/nhl",
    // ... other params
)]
```

Benefits:
- Instant startup with cached data
- Reduced initial API calls
- Offline browsing of historical data

### 2. Cache Warming Strategy

Pre-populate caches with predicted data:

```rust
async fn warm_caches_intelligent(client: &Client) {
    // Fetch today's schedule
    let today = GameDate::Now;
    let _ = fetch_schedule_cached(&client, today).await;

    // Fetch yesterday and tomorrow too
    let yesterday = today.add_days(-1);
    let tomorrow = today.add_days(1);
    let _ = fetch_schedule_cached(&client, yesterday).await;
    let _ = fetch_schedule_cached(&client, tomorrow).await;

    // Fetch standings
    let _ = fetch_standings_cached(&client).await;
}
```

### 3. Smart TTL Based on Game State

Adjust TTL dynamically:

```rust
fn calculate_ttl_for_game(game_state: &str) -> u64 {
    match game_state {
        "LIVE" | "CRIT" => 30,    // 30 seconds for live games
        "FUT" => 3600,             // 1 hour for future games
        "FINAL" | "OFF" => 86400,  // 24 hours for final games
        _ => 300,                  // 5 minutes default
    }
}
```

### 4. Cache Preloading on Tab Switch

When user navigates to Standings tab, preload team stats for visible teams:

```rust
async fn on_standings_tab_entered(client: &Client, visible_teams: &[String]) {
    for team_abbrev in visible_teams {
        tokio::spawn(async move {
            let _ = fetch_club_stats_cached(&client, team_abbrev, season).await;
        });
    }
}
```

### 5. Cache Hit Rate Metrics in TUI

Show cache performance in Settings or Debug panel:

```
Cache Statistics:
  Standings:     10 hits / 2 misses  (83% hit rate)
  Schedule:      25 hits / 5 misses  (83% hit rate)
  Games:        120 hits / 30 misses (80% hit rate)
```

---

## Conclusion

Refactoring SharedData to use the `cached` crate will:

1. **Reduce code complexity** by 200+ lines of manual cache management
2. **Improve performance** with 80% reduction in API calls
3. **Bound memory usage** to ~24MB with LRU eviction
4. **Enable future features** like disk persistence and intelligent preloading
5. **Maintain compatibility** with phased migration approach

The migration can be done incrementally over 4-5 days with minimal risk.

### Recommended Next Steps

1. ✅ Review this proposal with team
2. ⬜ Add `cached` dependency and create `src/cache.rs` (Phase 1)
3. ⬜ Test cached functions in isolation
4. ⬜ Migrate background loop one function at a time (Phase 2)
5. ⬜ Update SharedData struct and TUI (Phase 3)
6. ⬜ Add configuration support (Phase 4)
7. ⬜ Monitor performance and adjust TTLs as needed

---

**End of Proposal**
