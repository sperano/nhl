# Plan: Eliminate Unnecessary AppState Clones in dispatch()

## Step 0: Collect Baseline Metrics (BEFORE refactor)

Run these commands and record the results:

```bash
# Count state.clone() in dispatch method
grep -n "\.clone()" src/tui/runtime.rs | head -20

# Count state.clone() in all reducers
grep -c "state\.clone()" src/tui/reducers/*.rs src/tui/runtime.rs

# Count lines in dispatch() and check_for_*_fetch methods
sed -n '72,152p' src/tui/runtime.rs | wc -l
sed -n '154,265p' src/tui/runtime.rs | wc -l
```

Record in this table:

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| `state.clone()` in runtime.rs | ___ | ___ | ___% |
| `state.clone()` in reducers/ | ___ | ___ | ___% |
| Lines in dispatch() | ___ | ___ | ___% |
| Lines in check_for_*_fetch methods | ___ | ___ | -100% (deleted) |
| Total clones per action (worst case) | ___ | 0 | ___% |

## Step FINAL: Collect After Metrics

After all changes, re-run the same commands and fill in the "After" column.
Calculate improvement percentages.

---

## Problem

`runtime.rs:dispatch()` clones `AppState` multiple times per action:

1. **Line 81**: `reduce(self.state.clone(), action.clone(), ...)` for `RefreshData`
2. **Line 90**: `self.state.clone()` for `RefreshSchedule`
3. **Line 102**: `reduce(self.state.clone(), action, ...)` for normal actions

Additionally, sub-reducers like `reduce_navigation` take `&AppState` but then immediately clone it internally (lines 11-19), creating double clones.

## Current Flow

```
dispatch() clones state
    → reduce() takes ownership
        → sub-reducers clone again internally
```

## Solution: Take Ownership at Runtime, Borrow in Reducers

Change the architecture so the runtime gives up ownership of `self.state` temporarily, and reducers take a mutable reference:

### Option A: Swap Pattern (Recommended)

```rust
pub fn dispatch(&mut self, action: Action) {
    // Take ownership temporarily
    let state = std::mem::take(&mut self.state);

    // Reduce without cloning
    let (new_state, effect) = reduce(state, action, &mut self.component_states);

    // Put it back
    self.state = new_state;
}
```

This eliminates all clones in the happy path. The `reduce` function already takes `AppState` by value.

### Changes Required

**1. `runtime.rs` - dispatch() method**

Replace clones with `std::mem::take`:

```rust
pub fn dispatch(&mut self, action: Action) {
    let effect = if matches!(action, Action::RefreshData) {
        // Take ownership, reduce, put back
        let state = std::mem::take(&mut self.state);
        let (new_state, _) = reduce(state, action.clone(), &mut self.component_states);
        self.state = new_state;
        self.data_effects.handle_refresh(&self.state)
    } else if let Action::RefreshSchedule(date) = &action {
        // Modify in place - no clone needed
        self.state.ui.scores.game_date = date.clone();
        self.state.data.schedule = Arc::new(None);
        Arc::make_mut(&mut self.state.data.game_info).clear();
        Arc::make_mut(&mut self.state.data.period_scores).clear();
        self.data_effects.handle_refresh_schedule(date.clone())
    } else {
        // Normal path - swap pattern
        let old_state = self.state.clone(); // Need old for comparison
        let state = std::mem::take(&mut self.state);
        let (new_state, reducer_effect) = reduce(state, action, &mut self.component_states);

        // Check effects before replacing state
        let boxscore_effect = self.check_for_boxscore_fetch(&old_state, &new_state);
        // ... other checks ...

        self.state = new_state;
        // combine effects...
    };
    // ...
}
```

**Issue**: The `check_for_*_fetch` methods need both old and new state for comparison. This still requires one clone of the old state.

**2. Better approach for RefreshSchedule**

For `RefreshSchedule`, we don't need to clone at all - just mutate in place:

```rust
} else if let Action::RefreshSchedule(date) = &action {
    self.state.ui.scores.game_date = date.clone();
    self.state.data.schedule = Arc::new(None);
    Arc::make_mut(&mut self.state.data.game_info).clear();
    Arc::make_mut(&mut self.state.data.period_scores).clear();
    self.data_effects.handle_refresh_schedule(date.clone())
}
```

This is already almost the current code, but avoids the initial clone.

**3. Sub-reducers - eliminate internal clones**

Change sub-reducers to take `AppState` by value instead of `&AppState`:

```rust
// Before
pub fn reduce_navigation(state: &AppState, action: &Action) -> Option<(AppState, Effect)> {
    match action {
        Action::NavigateTab(tab) => Some(navigate_to_tab(state.clone(), *tab)),
        // ^^^ clones every time

// After
pub fn reduce_navigation(state: AppState, action: &Action) -> Option<(AppState, Effect)> {
    match action {
        Action::NavigateTab(tab) => Some(navigate_to_tab(state, *tab)),
        // ^^^ no clone, ownership transferred
```

**Issue**: If a sub-reducer returns `None` (action not handled), we lose ownership.

**Solution**: Return `Result<(AppState, Effect), AppState>` or use `Cow<AppState>`.

## Recommended Implementation

Given the complexity of the full refactor, a phased approach:

### Phase 1: Quick Win - RefreshSchedule (0 clones → already at 0)

The `RefreshSchedule` branch already mutates in place after cloning. Just remove the clone:

```rust
} else if let Action::RefreshSchedule(date) = &action {
    // No clone needed - mutate directly
    self.state.ui.scores.game_date = date.clone();
    self.state.data.schedule = Arc::new(None);
    Arc::make_mut(&mut self.state.data.game_info).clear();
    Arc::make_mut(&mut self.state.data.period_scores).clear();
    self.data_effects.handle_refresh_schedule(date.clone())
}
```

### Phase 2: Main Path - Use mem::take

For the main `else` branch, we need old_state for comparisons. Keep one clone but use swap:

```rust
} else {
    let old_state = self.state.clone(); // Needed for effect detection
    let state = std::mem::take(&mut self.state);
    let (new_state, reducer_effect) = reduce(state, action, &mut self.component_states);
    // ... effect checks using old_state and new_state ...
    self.state = new_state;
}
```

This changes from 1 clone + reduce takes ownership to: 1 clone for comparison only.

## Option 3: Move Effect Detection into Reducers (Recommended)

### Current Problem

The `check_for_*_fetch` methods in `runtime.rs` compare old and new state to detect:
1. Document stack grew → fetch data for new document (boxscore, team, player)
2. Schedule loaded → fetch game details for started games

This comparison requires cloning `self.state` before calling `reduce()`.

### Solution: Reducers Return Fetch Effects Directly

Instead of `dispatch()` detecting what changed, the reducer that makes the change returns the appropriate effect.

**Example: PushDocument**

```rust
// reducers/document_stack.rs - BEFORE
fn push_document(state: AppState, doc: StackedDocument) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.navigation.document_stack.push(DocumentStackEntry::new(doc));
    (new_state, Effect::None)  // Runtime detects this and fetches data
}

// AFTER
fn push_document(state: AppState, doc: StackedDocument) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.navigation.document_stack.push(DocumentStackEntry::new(doc.clone()));

    // Reducer returns fetch effect directly
    let fetch_effect = match &doc {
        StackedDocument::Boxscore { game_id } => {
            if !new_state.data.boxscores.contains_key(game_id)
                && !new_state.data.loading.contains(&LoadingKey::Boxscore(*game_id))
            {
                Effect::FetchBoxscore(*game_id)  // New effect variant
            } else {
                Effect::None
            }
        }
        StackedDocument::TeamDetail { abbrev } => {
            if !new_state.data.team_roster_stats.contains_key(abbrev)
                && !new_state.data.loading.contains(&LoadingKey::TeamRosterStats(abbrev.clone()))
            {
                Effect::FetchTeamRosterStats(abbrev.clone())
            } else {
                Effect::None
            }
        }
        StackedDocument::PlayerDetail { player_id } => {
            if !new_state.data.player_data.contains_key(player_id)
                && !new_state.data.loading.contains(&LoadingKey::PlayerStats(*player_id))
            {
                Effect::FetchPlayerStats(*player_id)
            } else {
                Effect::None
            }
        }
    };

    (new_state, fetch_effect)
}
```

### New Effect Variants

Add fetch effect variants to `Effect` enum:

```rust
pub enum Effect {
    None,
    Handled,
    Action(Action),
    Batch(Vec<Effect>),
    // New fetch variants
    FetchBoxscore(i64),
    FetchTeamRosterStats(String),
    FetchPlayerStats(i64),
    FetchGameDetails(i64),
    FetchSchedule(GameDate),
    FetchAll,  // For RefreshData
}
```

### Updated Runtime dispatch()

```rust
pub fn dispatch(&mut self, action: Action) {
    // No clone needed - just swap
    let state = std::mem::take(&mut self.state);
    let (new_state, effect) = reduce(state, action, &mut self.component_states);
    self.state = new_state;

    // Execute the effect (which may be a fetch)
    self.execute_effect(effect);
}

fn execute_effect(&self, effect: Effect) {
    match effect {
        Effect::None => {}
        Effect::Handled => {}
        Effect::Action(action) => { /* queue action */ }
        Effect::Batch(effects) => {
            for e in effects {
                self.execute_effect(e);
            }
        }
        Effect::FetchBoxscore(game_id) => {
            let fetch = self.data_effects.fetch_boxscore(game_id);
            let _ = self.effect_tx.send(fetch);
        }
        Effect::FetchTeamRosterStats(abbrev) => {
            let fetch = self.data_effects.fetch_team_roster_stats(abbrev);
            let _ = self.effect_tx.send(fetch);
        }
        // ... etc
    }
}
```

### Schedule Loading Case

For schedule loading, the `reduce_data_loading` reducer handles `ScheduleLoaded`:

```rust
// reducers/data_loading.rs
Action::ScheduleLoaded(schedule) => {
    new_state.data.schedule = Arc::new(Some(schedule.clone()));

    // Return fetch effects for started games
    let mut effects = Vec::new();
    for game in &schedule.games {
        if game.game_state != GameState::Future && game.game_state != GameState::PreGame {
            effects.push(Effect::FetchGameDetails(game.id));
        }
    }

    let combined = if effects.is_empty() {
        Effect::None
    } else {
        Effect::Batch(effects)
    };

    Some((new_state, combined))
}
```

### Performance Improvement

| Path | Before | After |
|------|--------|-------|
| RefreshData | 1 clone | 0 clones (mem::take) |
| RefreshSchedule | 1 clone | 0 clones (mem::take) |
| Normal actions | 1 clone (for comparison) | 0 clones (mem::take) |
| PushDocument | comparison in runtime | reducer returns effect |
| ScheduleLoaded | comparison in runtime | reducer returns effect |

**Total clones per action: 0** (down from 1-2)

### Files to Modify

1. `src/tui/component.rs` - Add new `Effect` variants
2. `src/tui/reducers/document_stack.rs` - Return fetch effects from `push_document`
3. `src/tui/reducers/data_loading.rs` - Return fetch effects from `ScheduleLoaded`
4. `src/tui/runtime.rs`:
   - Remove `check_for_*_fetch` methods
   - Simplify `dispatch()` to use `mem::take`
   - Add `execute_effect()` method to handle new effect types

### Benefits

1. **Zero clones** - No AppState clones in dispatch() hot path
2. **Colocation** - Fetch logic lives where state changes happen
3. **Explicit** - Effects are declarative, not detected via comparison
4. **Testable** - Can unit test that reducers return correct effects
5. **Simpler runtime** - dispatch() becomes trivial swap + reduce + execute
