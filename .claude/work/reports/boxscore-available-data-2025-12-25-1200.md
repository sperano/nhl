# Boxscore Document: Available vs Displayed Data

**Report Date:** 2025-12-25

## Executive Summary

The boxscore document currently displays detailed player-level statistics but is missing several team-level game summary features. The two specific items you asked about:

| Feature | Available? | Currently Displayed? |
|---------|-----------|---------------------|
| **Shots Total** | Yes - `BoxscoreTeam.sog` | **No** (only individual player SOG in tables) |
| **Scores Per Period** | Requires `GameSummary` from different endpoint | **No** |

---

## Current Boxscore Document Display

**File:** `src/tui/components/boxscore_document.rs`

### What's Rendered Now

1. **Score Header** (BigScore widget with unicode digits or text fallback)
   - Team names
   - Current score
   - Game status (scheduled/live/final with OT/SO indicator)
   - Venue name

2. **Player Statistics Tables** (per team)
   - **Skaters (Forwards & Defense):** #, Player, Pos, G, A, PTS, +/-, SOG, Hits, Blk, PIM, FO%, TOI
   - **Goalies:** #, Player, SA, GA, SV, SV%, TOI, PIM

---

## Available API Data

### From `Boxscore` Endpoint (currently used)

**Team-Level Data Available but NOT Displayed:**

| Field | Type | Location | Description |
|-------|------|----------|-------------|
| `sog` | `i32` | `BoxscoreTeam` | Total shots on goal for team |
| `tv_broadcasts` | `Vec<TvBroadcast>` | `Boxscore` | TV network information |
| `special_event` | `Option<SpecialEvent>` | `Boxscore` | Winter Classic, Stadium Series, etc. |

**Player-Level Data Available but NOT Displayed:**

| Field | Type | Location | Description |
|-------|------|----------|-------------|
| `power_play_goals` | `i32` | `SkaterStats` | Individual PP goals |
| `shifts` | `i32` | `SkaterStats` | Number of shifts |
| `giveaways` | `i32` | `SkaterStats` | Giveaways |
| `takeaways` | `i32` | `SkaterStats` | Takeaways |
| `even_strength_shots_against` | `String` | `GoalieStats` | ES save/shot (e.g., "20/22") |
| `power_play_shots_against` | `String` | `GoalieStats` | PP save/shot |
| `shorthanded_shots_against` | `String` | `GoalieStats` | SH save/shot |
| `starter` | `Option<bool>` | `GoalieStats` | Whether goalie started |
| `decision` | `Option<GoalieDecision>` | `GoalieStats` | W/L/OTL |

### From `GameSummary` (requires `GameMatchup` or `PlayByPlay` endpoint)

**Period-by-Period Scoring is in `GameSummary.scoring`:**

```rust
pub struct GameSummary {
    pub scoring: Vec<PeriodScoring>,      // Goals by period
    pub shootout: Option<Vec<ShootoutAttempt>>,
    pub three_stars: Option<Vec<ThreeStar>>,
    pub penalties: Vec<PeriodPenalties>,   // Penalties by period
}

pub struct PeriodScoring {
    pub period_descriptor: PeriodDescriptor,
    pub goals: Vec<GoalSummary>,  // Detailed goal information
}

pub struct GoalSummary {
    pub player_id: i64,
    pub time_in_period: String,
    pub away_score: i32,          // Running score after this goal
    pub home_score: i32,
    pub shot_type: String,        // "snap", "wrist", etc.
    pub goal_modifier: String,    // "empty-net", "shorthanded", etc.
    pub strength: String,         // "5v5", "5v4", etc.
    pub assists: Vec<AssistSummary>,
    // ... more fields
}
```

**Three Stars:**
```rust
pub struct ThreeStar {
    pub star: i32,           // 1, 2, or 3
    pub player_id: i64,
    pub name: LocalizedString,
    pub goals: Option<i32>,
    pub assists: Option<i32>,
    pub points: Option<i32>,
    // For goalies:
    pub save_pctg: Option<f64>,
}
```

---

## Recommendations

### ~~1. Display Team Shots Total (Easy - No new API call needed)~~ ✓ DONE

~~The `sog` field already exists on `BoxscoreTeam`. Could be added to the score header:~~

```
Devils 3    Sabres 2
SOG: 30     SOG: 25
```

~~**Implementation:** Modify `build_score()` in `boxscore_document.rs` to include team SOG from `boxscore.away_team.sog` and `boxscore.home_team.sog`.~~

**Completed 2025-12-25:** Added SOG display to BigScore widget as a centered line below the score digits. Shows "SOG: X - Y" format.

### 2. Display Period-by-Period Scoring (Requires additional data)

**Option A: Fetch `GameMatchup` which includes `GameSummary`**
- The `GameMatchup` endpoint already includes `summary: Option<GameSummary>`
- Would require either:
  - Fetching `GameMatchup` in addition to `Boxscore`
  - Or replacing `Boxscore` with `GameMatchup` if it has all needed data

**Option B: Fetch `PlayByPlay` for most detailed data**
- Contains full game event timeline
- Has `summary: Option<GameSummary>`
- Most granular but larger response

### 3. Period Scoring Box Example

```
         1st  2nd  3rd  OT   Total
Devils    1    1    1   -     3
Sabres    0    1    1   -     2
```

Would need to calculate from `GameSummary.scoring[].goals` by iterating through goals and extracting `away_score`/`home_score` at period boundaries.

### 4. Other Potential Additions (already have data)

| Feature | Source | Effort |
|---------|--------|--------|
| Team shots total | `BoxscoreTeam.sog` | Low |
| TV broadcast info | `Boxscore.tv_broadcasts` | Low |
| Goalie decision (W/L) | `GoalieStats.decision` | Low |
| Special event badge | `Boxscore.special_event` | Low |
| Three stars | `GameSummary.three_stars` | Medium (new endpoint) |
| Goal timeline | `GameSummary.scoring` | Medium (new endpoint) |
| Penalty summary | `GameSummary.penalties` | Medium (new endpoint) |

---

## API Endpoint Reference

| Endpoint | Returns | Has Period Scoring? |
|----------|---------|-------------------|
| `/gamecenter/{id}/boxscore` | `Boxscore` | No |
| `/gamecenter/{id}/landing` | `GameMatchup` | Yes (via `summary`) |
| `/gamecenter/{id}/play-by-play` | `PlayByPlay` | Yes (via `summary`) |

---

## Files to Modify

To add team shots:
- `src/tui/components/boxscore_document.rs:76-101` - `build_score()` method

To add period scoring (requires new data):
1. `src/tui/reducers/data.rs` - Add action to fetch GameMatchup
2. `src/tui/state.rs` - Add field for GameSummary data
3. `src/tui/components/boxscore_document.rs` - Add period scoring section
4. `src/tui/document/elements.rs` - Possibly new DocumentElement type for period table

---

## Changes Made (2025-12-25)

### Skater Columns Added
| Column | Width | Field |
|--------|-------|-------|
| PPG | 3 | `power_play_goals` |
| GA | 2 | `giveaways` |
| TA | 2 | `takeaways` |
| SH | 3 | `shifts` |

### Goalie Columns Added
| Column | Width | Field |
|--------|-------|-------|
| DEC | 3 | `decision` (W/L/OTL) |
| S | 1 | `starter` (✓ for starter) |
| ES | 6 | `even_strength_shots_against` |
| PP | 4 | `power_play_shots_against` |
| SH | 4 | `shorthanded_shots_against` |

### Width Update
- `TEAM_BOXSCORE_WIDTH` updated from 89 to 105
- `TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH` now 214 (105 * 2 + 4)

### Other Changes
- Added `checkmark` field to `BoxChars` struct (✓ unicode, * ascii)
- Added `box_chars` field to `FocusContext` for passing box chars to column builders
