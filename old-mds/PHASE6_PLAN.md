# Phase 6: Component Enhancement Plan

## Overview

Phase 6 focuses on bringing the React-like components to **feature parity** with the legacy imperative code. Once components match legacy functionality, we can safely remove old code.

## Goals

1. **Feature Parity**: New components match all legacy features
2. **Better UX**: Leverage React patterns for richer interactions
3. **Remove Legacy**: Gradually delete old code as tabs are completed
4. **No Regressions**: Maintain all existing functionality

## Strategy: Tab-by-Tab Migration

### Approach
- ✅ Phase 5: Built foundation (complete)
- 🔄 Phase 6a: Enhance ScoresTab (Week 5 goal)
- 🔄 Phase 6b: Enhance StandingsTab (Week 6 goal)
- 🔄 Phase 6c: Implement remaining tabs (Week 7 goal)
- 🔄 Phase 6d: Remove legacy code (Week 8 goal)

### Risk Mitigation
- Keep experimental mode toggle throughout
- Test each enhancement before moving to next
- Keep legacy code until new version is confirmed working
- Easy rollback: just remove `NHL_EXPERIMENTAL=1`

---

## Phase 6a: Enhance ScoresTab

### Current State
- ✅ Date selector with 5-date window
- ✅ Breadcrumb navigation
- ✅ Simple game list (text only)
- ❌ No rich GameBox widgets
- ❌ No game selection
- ❌ No boxscore drill-down

### Target State (Feature Parity)
- ✅ Date selector with 5-date window (already done)
- ✅ Breadcrumb navigation (already done)
- ✅ Rich GameBox widgets with:
  - Team logos/abbreviations
  - Score display (for live/final games)
  - Period/time display (for live games)
  - Game state indicators (Preview/Live/Final)
  - Visual styling (colors, borders)
- ✅ Game selection with arrow keys
- ✅ Boxscore drill-down on Enter
- ✅ Boxscore view with full details

### Implementation Tasks

#### Task 6a.1: Create GameBox Widget for Component System
**File**: `src/tui/components/widgets/game_box.rs`

Wrap existing `GameBox` widget to work with component system:
```rust
pub struct GameBoxWidget {
    game: nhl_api::Game,
    period_scores: Option<PeriodScores>,
    game_info: Option<GameMatchup>,
    selected: bool,
}

impl RenderableWidget for GameBoxWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Use existing GameBox logic
        let game_box = crate::tui::widgets::GameBox::new(/* ... */);
        game_box.render(area, buf);
    }
}
```

**Success Criteria**:
- GameBoxWidget implements RenderableWidget
- Shows team abbreviations
- Shows scores (if available)
- Shows game state (Preview/Live/Final)
- Visual styling matches legacy

#### Task 6a.2: Enhance GameListWidget with GameBoxes
**File**: `src/tui/components/scores_tab.rs`

Replace simple text list with grid of GameBox widgets:
```rust
struct GameListWidget {
    schedule: Option<DailySchedule>,
    period_scores: HashMap<i64, PeriodScores>,
    game_info: HashMap<i64, GameMatchup>,
    selected_index: Option<usize>,
}

impl RenderableWidget for GameListWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Layout games in grid (2 columns)
        // Render each as GameBoxWidget
        // Highlight selected game
    }
}
```

**Success Criteria**:
- Games displayed in grid layout
- Each game shows as rich GameBox
- Visual match with legacy mode
- Responsive to terminal size

#### Task 6a.3: Add Game Selection to ScoresAction
**File**: `src/tui/framework/action.rs` (extend), `src/tui/framework/reducer.rs`

Add game selection state:
```rust
// In ScoresUiState
pub struct ScoresUiState {
    pub selected_date_index: usize,
    pub game_date: GameDate,
    pub box_selection_active: bool,      // NEW
    pub selected_game_index: Option<usize>, // NEW
}

// New actions
pub enum ScoresAction {
    // ... existing ...
    MoveGameSelectionUp,
    MoveGameSelectionDown,
    MoveGameSelectionLeft,
    MoveGameSelectionRight,
}
```

**Success Criteria**:
- Can navigate between games with arrow keys
- Selected game visually highlighted
- State persists across date changes
- Wraps around at boundaries

#### Task 6a.4: Add Boxscore Drill-Down
**File**: `src/tui/components/scores_tab.rs`, new panel component

Add boxscore panel:
```rust
pub struct BoxscorePanel {
    game_id: i64,
    boxscore: Option<Boxscore>,
}

impl Component for BoxscorePanel {
    fn view(&self, props: &Props, state: &State) -> Element {
        // Render detailed boxscore
        // Period scores, team stats, etc.
    }
}
```

**Success Criteria**:
- Enter key opens boxscore for selected game
- Boxscore shows period-by-period scores
- Shows team statistics
- ESC key closes boxscore
- Visual match with legacy

#### Task 6a.5: Wire Up in BridgeRuntime
**File**: `src/tui/framework/bridge.rs`

Add key mappings for new actions:
```rust
// In key_to_action(), when in box selection mode:
KeyCode::Up => ScoresAction::MoveGameSelectionUp,
KeyCode::Down => ScoresAction::MoveGameSelectionDown,
KeyCode::Enter => ScoresAction::SelectGame,
KeyCode::Esc => ScoresAction::ExitBoxSelection,
```

**Success Criteria**:
- All keys work correctly
- Smooth transitions between modes
- State updates properly
- No crashes or hangs

#### Task 6a.6: Update Props from SharedData
**File**: `src/tui/framework/bridge.rs` (sync_from_shared_data)

Sync period scores and game info:
```rust
pub async fn sync_from_shared_data(&mut self) {
    let data = self.shared_data.read().await;
    let state = self.runtime.state_mut();

    // Sync period scores
    for (game_id, scores) in data.period_scores.iter() {
        state.data.game_details.entry(*game_id)
            .or_insert_with(|| GameInfo { id: *game_id })
            .period_scores = Some(scores.clone());
    }
}
```

**Success Criteria**:
- Period scores flow to components
- Game info updates in real-time
- No data loss during sync
- Performance acceptable

### Testing Checklist (6a)

**Manual Testing**:
- [ ] GameBoxes render correctly
- [ ] Can select games with arrow keys
- [ ] Selected game is highlighted
- [ ] Enter opens boxscore
- [ ] Boxscore shows all data
- [ ] ESC closes boxscore
- [ ] Date navigation still works
- [ ] No visual regressions

**Automated Testing**:
- [ ] GameBoxWidget rendering test
- [ ] Game selection reducer tests
- [ ] Boxscore panel component test
- [ ] Integration test for full flow

---

## Phase 6b: Enhance StandingsTab

### Current State
- ✅ View selector (Division/Conference/League)
- ✅ Simple standings table (first 10 teams)
- ✅ Team selection highlighting
- ❌ No full team list
- ❌ No team detail drill-down
- ❌ No roster view

### Target State (Feature Parity)
- ✅ View selector (already done)
- ✅ Full standings table:
  - All teams in current view
  - Proper column alignment
  - Division headers (for Division view)
  - Conference headers (for Conference view)
  - Scrolling for long lists
- ✅ Team selection with arrow keys:
  - Up/Down: Move within column
  - Left/Right: Switch columns (Division/Conference views)
  - Enter: Open team detail
- ✅ Team detail panel:
  - Team record, stats
  - Recent games
  - Roster preview
- ✅ Roster drill-down:
  - Full team roster
  - Player stats
  - Player selection

### Implementation Tasks

#### Task 6b.1: Enhance StandingsTableWidget
**File**: `src/tui/components/standings_tab.rs`

Show all teams with proper layout:
```rust
struct StandingsTableWidget {
    standings: Vec<Standing>,
    view: GroupBy,
    selected_column: usize,
    selected_row: usize,
    team_mode: bool,
    scroll_offset: usize,
}

impl RenderableWidget for StandingsTableWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Group standings by view
        // Render in columns (1 or 2)
        // Add division/conference headers
        // Highlight selected team
        // Support scrolling
    }
}
```

**Success Criteria**:
- All teams visible (with scrolling)
- Proper column layout
- Headers for divisions/conferences
- Selected team highlighted
- Visual match with legacy

#### Task 6b.2: Add Team Selection State
**File**: `src/tui/framework/state.rs`, `src/tui/framework/reducer.rs`

Enhance standings state:
```rust
pub struct StandingsUiState {
    pub view: GroupBy,
    pub team_mode: bool,
    pub selected_column: usize,
    pub selected_row: usize,
    pub scroll_offset: usize,
}
```

Implement navigation reducers:
```rust
StandingsAction::MoveSelectionUp => {
    // Move up, handle wraparound
}
StandingsAction::MoveSelectionDown => {
    // Move down, handle wraparound
}
```

**Success Criteria**:
- Selection state persists
- Navigation wraps correctly
- Scroll offset updates
- State transitions are smooth

#### Task 6b.3: Create TeamDetailPanel
**File**: `src/tui/components/team_detail_panel.rs`

Show team details:
```rust
pub struct TeamDetailPanel;

#[derive(Clone)]
pub struct TeamDetailProps {
    pub team_abbrev: String,
    pub club_stats: Option<ClubStats>,
    pub recent_games: Vec<Game>,
}

impl Component for TeamDetailPanel {
    fn view(&self, props: &Props, state: &State) -> Element {
        vertical([
            Constraint::Length(5),  // Header
            Constraint::Min(0),     // Stats
            Constraint::Length(10), // Recent games
        ], vec![
            render_header(props),
            render_stats(props),
            render_recent_games(props),
        ])
    }
}
```

**Success Criteria**:
- Shows team name, record
- Shows team statistics
- Shows recent games
- ESC returns to standings
- Visual match with legacy

#### Task 6b.4: Add Panel Navigation to Reducer
**File**: `src/tui/framework/reducer.rs`

Handle panel stack:
```rust
StandingsAction::SelectTeam => {
    let panel = Panel::TeamDetail {
        abbrev: get_selected_team_abbrev(state)
    };
    let mut new_state = state.clone();
    new_state.navigation.panel_stack.push(PanelState {
        panel,
        scroll_offset: 0,
    });

    // Trigger data fetch effect
    let effect = Effect::Async(fetch_team_stats(abbrev));
    (new_state, effect)
}

Action::PopPanel => {
    let mut new_state = state.clone();
    new_state.navigation.panel_stack.pop();
    (new_state, Effect::None)
}
```

**Success Criteria**:
- Panel stack grows/shrinks correctly
- Data fetched when panel opens
- State restored when panel closes
- No memory leaks

#### Task 6b.5: Wire Up in App Component
**File**: `src/tui/components/app.rs`

Route to panel when stack not empty:
```rust
Tab::Standings => {
    if !state.navigation.panel_stack.is_empty() {
        // Render panel instead of standings table
        self.render_panel(state)
    } else {
        // Render standings table
        StandingsTab.view(&props, &())
    }
}
```

**Success Criteria**:
- Panel overlays standings
- Can navigate back
- State persists
- Visual match with legacy

### Testing Checklist (6b)

**Manual Testing**:
- [ ] All teams visible in standings
- [ ] Can navigate with arrow keys
- [ ] Selection wraps correctly
- [ ] Enter opens team detail
- [ ] Team detail shows stats
- [ ] ESC closes team detail
- [ ] View cycling still works
- [ ] No visual regressions

**Automated Testing**:
- [ ] StandingsTableWidget rendering test
- [ ] Team selection reducer tests
- [ ] TeamDetailPanel component test
- [ ] Panel navigation integration test

---

## Phase 6c: Implement Remaining Tabs

### Stats Tab
**Purpose**: Show league leaders, statistics
**Priority**: Medium
**Complexity**: Low

**Tasks**:
- Create StatsTab component
- Add stats data to AppState
- Implement stats display widgets
- Add refresh action for stats

### Players Tab
**Purpose**: Browse players, search
**Priority**: Medium
**Complexity**: Medium

**Tasks**:
- Create PlayersTab component
- Add player search state
- Implement player list widget
- Add player detail panel

### Browser Tab
**Purpose**: Browse teams, divisions
**Priority**: Low
**Complexity**: Low

**Tasks**:
- Create BrowserTab component
- Reuse existing navigation patterns
- Implement tree navigation
- Add breadcrumb support

### Settings Tab
**Purpose**: Configure application
**Priority**: High
**Complexity**: Low

**Tasks**:
- Create proper SettingsTab component
- Show config options
- Add edit mode for settings
- Persist changes to config file

---

## Phase 6d: Remove Legacy Code

### Strategy
Remove old code only after new version is confirmed working.

### Order of Removal
1. **Week 7**: Remove `src/tui/scores/` (after 6a complete)
2. **Week 7**: Remove `src/tui/standings/` (after 6b complete)
3. **Week 8**: Remove `src/tui/stats/`, `players/`, `settings/`
4. **Week 8**: Remove `src/tui/mod.rs` old rendering code
5. **Week 8**: Remove SharedData dependency
6. **Week 8**: Make experimental mode the default

### Verification Steps
For each removal:
1. ✅ New version feature-complete
2. ✅ All tests passing
3. ✅ Manual testing successful
4. ✅ No regressions found
5. ✅ Performance acceptable
6. ➡️ Delete old code
7. ➡️ Update imports/exports
8. ➡️ Run full test suite
9. ➡️ Verify build succeeds

---

## Timeline Estimate

| Phase | Tasks | Estimate | Cumulative |
|-------|-------|----------|------------|
| 6a: ScoresTab | 6 tasks | 2-3 days | 2-3 days |
| 6b: StandingsTab | 5 tasks | 2-3 days | 4-6 days |
| 6c: Remaining Tabs | 4 tabs | 2-3 days | 6-9 days |
| 6d: Remove Legacy | Cleanup | 1-2 days | 7-11 days |

**Total**: ~1.5-2.5 weeks of focused work

---

## Success Criteria (Phase 6)

### Functional
- [ ] All tabs implemented
- [ ] Feature parity with legacy code
- [ ] All navigation working
- [ ] All drill-down views working
- [ ] Data display matches legacy

### Quality
- [ ] All tests passing
- [ ] No regressions
- [ ] Performance acceptable
- [ ] No memory leaks
- [ ] Clean code (no warnings)

### User Experience
- [ ] Smooth navigation
- [ ] Responsive UI
- [ ] Clear visual feedback
- [ ] Intuitive interactions
- [ ] Helpful error messages

### Technical
- [ ] Legacy code removed
- [ ] SharedData deprecated
- [ ] AppState sole source of truth
- [ ] Effects system for data fetching
- [ ] Clean architecture

---

## Risk Assessment

### Low Risk
- ✅ Foundation is solid (Phase 5 complete)
- ✅ Can test incrementally
- ✅ Easy rollback mechanism
- ✅ Legacy code as reference

### Medium Risk
- ⚠️ GameBox widget integration
- ⚠️ Panel navigation complexity
- ⚠️ Data sync performance
- ⚠️ Edge cases in navigation

### Mitigation
- Incremental testing at each step
- Keep experimental mode toggle
- Extensive manual testing
- Peer review/user testing
- Performance profiling

---

## Next Action

Start with **Task 6a.1**: Create GameBox widget wrapper for component system.

This is the foundation for enhancing ScoresTab and will validate our approach before proceeding with more complex features.
