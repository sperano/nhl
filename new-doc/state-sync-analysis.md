# State Sync Elimination Analysis

**Created**: 2025-11-26
**Goal**: Eliminate duplicate state between global `UiState` and component states

## Current State Duplication

### ScoresTab State

**Component State** (`ScoresTabState`):
- `selected_date_index: usize`
- `game_date: GameDate`
- `box_selection_active: bool`
- `selected_game_index: Option<usize>`
- `boxes_per_row: u16`

**Global State** (`state.ui.scores`):
- `selected_date_index: usize`
- `game_date: GameDate`
- `box_selection_active: bool`
- `selected_game_index: Option<usize>`
- `boxes_per_row: u16`

**Reads from Global State**:
1. `effects.rs:33` - `state.ui.scores.game_date` - used to fetch schedule
2. `scores.rs:88` - `state.ui.scores.selected_game_index` - used to open boxscore panel
3. `navigation.rs:94,142` - `state.ui.scores.box_selection_active` - set/clear on panel pop/content focus
4. `reducer.rs:452` - Test assertion

**Writes to Global State**:
1. `data_loading.rs:278` - `state.ui.scores.game_date = date` - on SetGameDate action
2. `navigation.rs:94,142` - `state.ui.scores.box_selection_active` - on PopPanel/EnterContentFocus
3. `scores.rs` - All score actions update global state via reducer

### StandingsTab State

**Component State** (`StandingsTabState`):
- `view: GroupBy`
- `browse_mode: bool`
- `doc_nav: DocumentNavState`
- `focusable_ids: Vec<FocusableId>`

**Global State** (`state.ui.standings`):
- `view: GroupBy`
- `browse_mode: bool`

**Reads from Global State**:
1. `app.rs:318,337` - `view` and `browse_mode` - used to create StandingsTabProps
2. `navigation.rs:143,149` - `browse_mode` - test assertions
3. `testing.rs:72,368,372` - `view` - test code

**Writes to Global State**:
1. `navigation.rs:95` - `browse_mode = false` - on PopPanel
2. `standings.rs` - All standings actions update global state via reducer

## Usage Analysis

### Category 1: Data Effects (READ)
**Location**: `effects.rs:33`
```rust
self.fetch_schedule(state.ui.scores.game_date.clone())
```
**Purpose**: Needs `game_date` to know which schedule to fetch
**Current Flow**: DataEffects → RefreshData → reads global state → fetches data

### Category 2: Cross-Tab Actions (READ/WRITE)
**Location**: `navigation.rs:94,142` (PopPanel, EnterContentFocus)
```rust
new_state.ui.scores.box_selection_active = false;
```
**Purpose**: When popping panel or entering content, clear box selection mode
**Current Flow**: Navigation action → updates global state → component picks up on next render

### Category 3: Action Handlers (READ)
**Location**: `scores.rs:88` (SelectGame)
```rust
if let Some(selected_index) = new_state.ui.scores.selected_game_index {
    // Push boxscore panel for this game
}
```
**Purpose**: Need to know which game is selected to open its panel
**Current Flow**: SelectGame action → reads global state → pushes panel

### Category 4: Props Creation (READ)
**Location**: `app.rs:318,337`
```rust
view: state.ui.standings.view,
browse_mode: state.ui.standings.browse_mode,
```
**Purpose**: Pass data to component as props
**Current Flow**: Global state → props → component

### Category 5: State Initialization (WRITE)
**Location**: `data_loading.rs:278`
```rust
new_state.ui.scores.game_date = date;
```
**Purpose**: SetGameDate action updates the date
**Current Flow**: Action → global state → component state (via sync?)

## Strategic Options

### Option A: Pass ComponentStates to Effects
**Pros**:
- Effects can read directly from component state
- Eliminates need for global state in Category 1

**Cons**:
- Effects now depend on component state store
- Need to know component paths ("app/scores_tab")
- Breaks encapsulation - effects know about component internals

**Changes Required**:
- `DataEffects::refresh_data(&self, state: &AppState, component_states: &ComponentStateStore)`
- Effect needs to extract game_date from component state

### Option B: Actions Carry Necessary Data
**Pros**:
- Clean separation - actions are self-contained
- No dependencies on global or component state

**Cons**:
- Actions become larger (carry more data)
- Whoever dispatches must provide the data

**Changes Required**:
- `Action::RefreshData` → `Action::RefreshData { game_date: GameDate }`
- `Action::SelectGame` → `Action::SelectGame { game_id: i64 }` (don't need index)

### Option C: Component Messages for Everything
**Pros**:
- Fully component-based - no global UI state at all
- Each component handles its own actions

**Cons**:
- Cross-component communication becomes harder
- Navigation actions (PopPanel) need to message multiple components

**Changes Required**:
- Navigation actions dispatch ComponentMessage to clear modes
- Effects somehow trigger component messages?

### Option D: Hybrid - Keep Minimal Global State as "Intent"
**Pros**:
- Global state becomes write-only "commands"
- Component state is still source of truth for rendering
- Simple for cross-cutting concerns

**Cons**:
- Still have duplication (but with clear semantics)
- Need clear contract about what global state means

**Pattern**:
- Global `box_selection_active` means "user wants to be in box selection"
- Component state is "current rendering state"
- Component reads global on mount/update and syncs

## Recommendation

**Option B: Actions Carry Data** for most cases, with **Option C: Component Messages** for pure UI state.

### Strategy:

1. **RefreshData Action**: Make it carry `game_date`
   - Dispatcher (component) provides current game_date
   - Effect uses the date directly
   - Remove `game_date` from global state

2. **SelectGame Action**: Make it carry `game_id`
   - Instead of reading `selected_game_index` from global state
   - Component dispatches `SelectGame { game_id }` with the actual game ID
   - Remove `selected_game_index` from global state

3. **Box Selection Mode**: Keep as component-only state
   - Remove from global state
   - Navigation actions dispatch ComponentMessage to clear it

4. **Browse Mode**: Keep as component-only state
   - Remove from global state
   - PopPanel dispatches ComponentMessage to StandingsTab to clear it

5. **View State**: Keep in global state TEMPORARILY
   - Used for props creation
   - Can be removed later when props initialization is refactored

### Implementation Order:

1. ✅ Make RefreshData carry game_date
2. ✅ Make SelectGame carry game_id (not index)
3. ✅ Move box_selection_active to component-only
4. ✅ Move browse_mode to component-only
5. 🔄 Remove global state fields
6. 🔄 Update tests

## Open Questions

1. **What about SetGameDate action?**
   - Currently writes to global state
   - Should it dispatch ComponentMessage to ScoresTab instead?
   - Or should date changes always come from component (user clicking arrows)?

2. **How does PopPanel clear browse_mode?**
   - Option A: PopPanel dispatches batch of ComponentMessages
   - Option B: Components listen to panel_stack changes
   - Option C: Keep browse_mode in global state as "exception"

3. **What about boxes_per_row?**
   - Currently calculated in render loop and dispatched
   - Could just be local to component, recalculated each render
   - No need to persist between renders

## Next Steps

1. Start with easiest: `boxes_per_row` - make it component-local only
2. Then: `SelectGame` action - make it carry game_id
3. Then: `RefreshData` - make it carry game_date (or remove entirely)
4. Finally: Cross-tab state clearing (browse_mode, box_selection_active)
