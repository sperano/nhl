# Cleanup Plan: Remove Shims + Standardize Naming

**Date**: 2025-11-30

## Overview

Two-part cleanup:
1. Remove legacy `*Action` shims, dispatch `ComponentMessage` directly from `keys.rs`
2. Standardize naming: `DemoTabMessage` → `DemoTabMsg`, clean up phase comments

---

## Part 1: Remove Action Shims

### Changes to `src/tui/action.rs`

**Remove**:
- `ScoresAction` enum entirely
- `StandingsAction` enum entirely
- `Action::ScoresAction(ScoresAction)` variant
- `Action::StandingsAction(StandingsAction)` variant
- Related `Clone` impl arms

**Add**:
- `Action::SelectGame(i64)` - was `ScoresAction::SelectGame`
- `Action::RebuildStandingsFocusable` - was `StandingsAction::RebuildFocusableMetadata`

### Changes to `src/tui/keys.rs`

**Replace all shim dispatches with direct `ComponentMessage`**:

| Old | New |
|-----|-----|
| `Action::ScoresAction(ScoresAction::DateLeft)` | `Action::ComponentMessage { path: SCORES_TAB_PATH, message: Box::new(ScoresTabMsg::NavigateLeft) }` |
| `Action::ScoresAction(ScoresAction::DateRight)` | `Action::ComponentMessage { path: SCORES_TAB_PATH, message: Box::new(ScoresTabMsg::NavigateRight) }` |
| `Action::ScoresAction(ScoresAction::EnterBoxSelection)` | `Action::ComponentMessage { path: SCORES_TAB_PATH, message: Box::new(ScoresTabMsg::EnterBoxSelection) }` |
| `Action::ScoresAction(ScoresAction::ExitBoxSelection)` | `Action::ComponentMessage { path: SCORES_TAB_PATH, message: Box::new(ScoresTabMsg::ExitBoxSelection) }` |
| `Action::ScoresAction(ScoresAction::SelectGame(id))` | `Action::SelectGame(id)` |
| `Action::StandingsAction(StandingsAction::CycleViewLeft)` | `Action::ComponentMessage { path: STANDINGS_TAB_PATH, message: Box::new(StandingsTabMsg::CycleViewLeft) }` |
| `Action::StandingsAction(StandingsAction::CycleViewRight)` | `Action::ComponentMessage { path: STANDINGS_TAB_PATH, message: Box::new(StandingsTabMsg::CycleViewRight) }` |
| `Action::StandingsAction(StandingsAction::EnterBrowseMode)` | `Action::ComponentMessage { path: STANDINGS_TAB_PATH, message: Box::new(StandingsTabMsg::EnterBrowseMode) }` |
| `Action::StandingsAction(StandingsAction::ExitBrowseMode)` | `Action::ComponentMessage { path: STANDINGS_TAB_PATH, message: Box::new(StandingsTabMsg::ExitBrowseMode) }` |

### Changes to `src/tui/reducer.rs`

**Remove**:
- `Action::ScoresAction(scores_action) => reduce_scores(...)` match arm
- `Action::StandingsAction(standings_action) => reduce_standings(...)` match arm

**Add**:
- `Action::SelectGame(game_id) => handle_select_game(state, game_id)`
- `Action::RebuildStandingsFocusable => { rebuild_focusable_metadata(&state, component_states); (state, Effect::None) }`

**Update tests** to use new action variants.

### Changes to `src/tui/reducers/scores.rs`

**Delete entire file** - logic moves to:
- Shim variants: eliminated (direct `ComponentMessage` dispatch)
- `SelectGame`: moves to `reducer.rs` as `handle_select_game()`

### Changes to `src/tui/reducers/standings.rs`

**Keep only**:
- `rebuild_focusable_metadata()` function (used by data loading)

**Remove**:
- `reduce_standings()` function
- All shim match arms
- Tests for shim behavior

### Changes to `src/tui/reducers/mod.rs`

**Remove**:
- `pub use scores::reduce_scores;`
- Update `reduce_standings` export if needed

### Changes to `src/tui/components/standings_tab.rs`

**Update**:
- `StandingsTabMsg::CycleViewLeft` and `CycleViewRight` currently return `Effect::Action(Action::StandingsAction(StandingsAction::RebuildFocusableMetadata))`
- Change to `Effect::Action(Action::RebuildStandingsFocusable)`

### Changes to `src/tui/mod.rs`

**Remove from exports**:
- `ScoresAction`
- `StandingsAction`

### Changes to `benches/performance.rs`

**Update**:
- Uses `Action::StandingsAction(StandingsAction::CycleViewRight)` - change to `ComponentMessage`

---

## Part 2: Standardize Naming

### Rename `DemoTabMessage` → `DemoTabMsg`

**Files**:
- `src/tui/components/demo_tab.rs` - definition and all usages
- `src/tui/keys.rs` - import and usages

### Clean Up Phase Comments

**Action**: Remove "Phase X:" prefixes from comments, keep useful architectural explanations.

**Files** (remove phase references):
- `src/tui/reducer.rs`
- `src/tui/keys.rs`
- `src/tui/state.rs`
- `src/tui/action.rs`
- `src/tui/reducers/navigation.rs`
- `src/tui/reducers/standings.rs`
- `src/tui/reducers/data_loading.rs`
- `src/tui/components/scores_tab.rs`
- `src/tui/components/standings_tab.rs`
- `src/tui/components/settings_tab.rs`
- `src/tui/components/demo_tab.rs`
- `src/tui/components/app.rs`

**Keep** (but reword):
- `src/tui/state.rs` - explanation of why `game_date` is duplicated (remove "PHASE 7 COMPLETE" header)

---

## Execution Order

1. Part 1a: Update `action.rs` - add new variants, keep old ones temporarily
2. Part 1b: Update `keys.rs` - switch to direct `ComponentMessage` dispatch
3. Part 1c: Update `reducer.rs` - add new handlers
4. Part 1d: Update `standings_tab.rs` - use new `RebuildStandingsFocusable` action
5. Part 1e: Update `mod.rs` exports
6. Part 1f: Update `benches/performance.rs`
7. Part 1g: Remove old code - `ScoresAction`, `StandingsAction`, `reducers/scores.rs`, shim code in `reducers/standings.rs`
8. Part 1h: Update/remove tests
9. **Checkpoint**: `cargo test` - all tests pass
10. Part 2a: Rename `DemoTabMessage` → `DemoTabMsg`
11. Part 2b: Clean up phase comments
12. **Final**: `cargo test` - all tests pass

---

## Risk Assessment

**Low risk**:
- Renaming `DemoTabMessage` - compiler catches all usages
- Removing phase comments - no functional change

**Medium risk**:
- Removing shim reducers - need to ensure all call sites updated
- Moving `SelectGame` to top-level - several call sites in `keys.rs`

**Mitigation**:
- Compiler will catch missing match arms
- Run `cargo test` at checkpoint
- Run `cargo build` frequently during changes
