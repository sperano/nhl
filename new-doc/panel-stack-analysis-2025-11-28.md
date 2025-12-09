# Panel Stack Implementation Analysis (2025-11-28)

## Executive Summary

The NHL TUI currently implements a **stack-based drill-down navigation system** using a `Vec<PanelState>` stored in global `AppState.navigation.panel_stack`. This document provides a comprehensive analysis of how panels work, their current implementation, and what would need to change to migrate to a document-based system.

**Key Statistics:**
- 3 panel types: Boxscore, TeamDetail, PlayerDetail
- 1 main reducer file: `src/tui/reducers/panels.rs` (1434 lines)
- 3 panel component files
- 4 related action types
- 17 comprehensive unit tests
- Full nested navigation support

---

## 1. Panel Type Definition

**Location:** `src/tui/types.rs`

```rust
#[derive(Debug, Clone)]
pub enum Panel {
    Boxscore { game_id: i64 },
    TeamDetail { abbrev: String },
    PlayerDetail { player_id: i64 },
}
```

### Panel Descriptions

| Panel Type | Purpose | Data Source | Key Feature |
|-----------|---------|-------------|------------|
| **Boxscore** | Detailed game statistics for a single game | `state.data.boxscores[game_id]` | Displays away/home teams' forwards, defense, goalies with stats |
| **TeamDetail** | Team roster with season stats | `state.data.team_roster_stats[abbrev]` | Sorts skaters by points, goalies by GP; enables player selection |
| **PlayerDetail** | Player career statistics | `state.data.player_data[player_id]` | Shows season totals; each season selectable to navigate to team |

---

## 2. State Management Architecture

### 2.1 Panel Stack Storage

**Location:** `src/tui/state.rs`

```rust
#[derive(Debug, Clone)]
pub struct NavigationState {
    pub current_tab: Tab,
    pub panel_stack: Vec<PanelState>,  // Core: vector of open panels
    pub content_focused: bool,
}

#[derive(Debug, Clone)]
pub struct PanelState {
    pub panel: Panel,
    pub selected_index: Option<usize>,  // Selection within current panel
}
```

### 2.2 Key Design Decisions

**Stack semantics:**
- Last element in `panel_stack` is the visible/active panel
- Multiple panels can be stacked (nested navigation)
- `selected_index` only relevant for current (top) panel
- Empty stack means normal tab content is visible

**Data location:**
- Global state in `AppState.navigation.panel_stack`
- Not in component-local state
- Accessible from everywhere (keys, reducers, effects)

---

## 3. Action System

**Location:** `src/tui/action.rs`

```rust
pub enum Action {
    PushPanel(Panel),           // Open a new panel
    PopPanel,                   // Close current panel
    PanelSelectNext,            // Move selection down (Up arrow)
    PanelSelectPrevious,        // Move selection up (Down arrow)
    PanelSelectItem,            // Activate selected item (Enter key)
}
```

### Action Flow

```
Key Event → key_to_action() → Action → reduce() → (new AppState, Effect)
```

**Reducer:** `src/tui/reducers/panels.rs::reduce_panels()`

---

## 4. Complete Lifecycle

### 4.1 Opening a Panel

**Trigger:** User presses Enter on an item (game, team, or player)

**Example: User selects a game in Scores tab**

**File:** `src/tui/keys.rs` (lines 164-178)

```rust
KeyCode::Enter => {
    if let Some(scores_state) = component_states.get::<ScoresTabState>("app/scores_tab") {
        if let Some(selected_index) = scores_state.doc_nav.focus_index {
            if let Some(schedule) = state.data.schedule.as_ref().as_ref() {
                if let Some(game) = schedule.games.get(selected_index) {
                    return Some(Action::ScoresAction(ScoresAction::SelectGame(game.id)));
                }
            }
        }
    }
    None
}
```

**Reducer chain:**
1. `ScoresAction::SelectGame(game_id)` in `reducers/scores.rs`
2. Pushes `Action::PushPanel(Panel::Boxscore { game_id })`
3. `reduce_data_loading()` in `reducers/data_loading.rs` intercepts
4. Adds loading key: `LoadingKey::Boxscore(game_id)`
5. Dispatches `DataEffects::fetch_boxscore(game_id)` → async Effect
6. When data arrives: `Action::BoxscoreLoaded(game_id, Ok(data))`
7. Boxscore stored in `state.data.boxscores[game_id]`

### 4.2 Selection and Navigation Within Panel

**Key handlers:** `src/tui/keys.rs` (lines 93-110)

```rust
fn handle_panel_navigation(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Up => Some(Action::PanelSelectPrevious),
        KeyCode::Down => Some(Action::PanelSelectNext),
        KeyCode::Enter => Some(Action::PanelSelectItem),
        _ => None,
    }
}
```

**Reducer:** `src/tui/reducers/panels.rs`

```rust
fn panel_select_next(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;
    if let Some(panel) = new_state.navigation.panel_stack.last_mut() {
        if let Some(idx) = panel.selected_index {
            panel.selected_index = Some(idx.saturating_add(1));
        } else {
            panel.selected_index = Some(0);
        }
    }
    (new_state, Effect::None)
}
```

**Visual feedback:** Selection index determines which player row is highlighted in boxscore/team detail.

### 4.3 Activating Items (Nested Navigation)

**Trigger:** User presses Enter on selected item

**Reducer:** `src/tui/reducers/panels.rs` (lines 280-306)

This calls panel-specific handlers based on the current panel type.

#### From Boxscore Panel

**Handler:** `handle_boxscore_selection()` (lines 171-232)

```rust
fn handle_boxscore_selection(
    state: AppState,
    game_id: i64,
    selected_index: usize,
) -> Option<(AppState, Effect)> {
    if let Some(boxscore) = state.data.boxscores.get(&game_id) {
        // Determine which player was selected based on index
        let player_id = if selected_index < away_forwards_count {
            // Away forward
        } else if selected_index < away_forwards_count + away_defense_count {
            // Away defense
        } else if selected_index < away_total {
            // Away goalie
        } else if selected_index < away_total + home_forwards_count {
            // Home forward
        } else if selected_index < away_total + home_forwards_count + home_defense_count {
            // Home defense
        } else {
            // Home goalie
        };

        // Push PlayerDetail panel
        let mut new_state = state;
        new_state.navigation.panel_stack.push(PanelState {
            panel: Panel::PlayerDetail { player_id },
            selected_index: None,
        });
        Some((new_state, Effect::None))
    }
}
```

This is a **critical piece of logic**: the index mapping between visual position and player ID must account for section boundaries.

#### From TeamDetail Panel

**Handler:** `handle_team_roster_selection()` (lines 109-168)

```rust
fn handle_team_roster_selection(
    state: AppState,
    abbrev: &str,
    selected_index: usize,
) -> Option<(AppState, Effect)> {
    if let Some(roster) = state.data.team_roster_stats.get(abbrev) {
        // CRITICAL: Must sort the same way as team_detail_panel.rs does
        let mut sorted_skaters = roster.skaters.clone();
        sorted_skaters.sort_by_points_desc();

        let mut sorted_goalies = roster.goalies.clone();
        sorted_goalies.sort_by_games_played_desc();

        let num_skaters = sorted_skaters.len();

        // Check if selecting a skater
        if selected_index < num_skaters {
            if let Some(player) = sorted_skaters.get(selected_index) {
                // Push PlayerDetail
                let mut new_state = state;
                new_state.navigation.panel_stack.push(PanelState {
                    panel: Panel::PlayerDetail { player_id: player.player_id },
                    selected_index: None,
                });
                return Some((new_state, Effect::None));
            }
        } else {
            // Goalie index calculation
            let goalie_idx = selected_index - num_skaters;
            if let Some(goalie) = sorted_goalies.get(goalie_idx) {
                // Push PlayerDetail
                // ...
            }
        }
    }
}
```

**Critical Detail:** Sorting must match rendering! The test `test_panel_select_item_uses_sorted_roster` (lines 726-847) explicitly validates this.

#### From PlayerDetail Panel

**Handler:** `handle_player_season_selection()` (lines 235-278)

```rust
fn handle_player_season_selection(
    state: AppState,
    player_id: i64,
    selected_index: usize,
) -> Option<(AppState, Effect)> {
    if let Some(player) = state.data.player_data.get(&player_id) {
        if let Some(seasons) = &player.season_totals {
            // Filter to NHL regular season only
            let mut nhl_seasons: Vec<_> = seasons
                .iter()
                .filter(|s| {
                    s.game_type == nhl_api::GameType::RegularSeason && s.league_abbrev == "NHL"
                })
                .collect();
            nhl_seasons.sort_by_season_desc();

            if let Some(season) = nhl_seasons.get(selected_index) {
                // Extract team abbreviation and push TeamDetail
                let mut new_state = state;
                new_state.navigation.panel_stack.push(PanelState {
                    panel: Panel::TeamDetail {
                        abbrev: abbrev.to_string(),
                    },
                    selected_index: Some(0),
                });
                return Some((new_state, Effect::None));
            }
        }
    }
}
```

This enables **circular navigation**: Boxscore → Player → Team → back to Boxscore → etc.

### 4.4 Closing Panels (ESC Key)

**Location:** `src/tui/keys.rs` (lines 52-91)

```rust
fn handle_esc_key(state: &AppState, component_states: &ComponentStateStore) -> Option<Action> {
    // Priority 1: If there's a panel open, close it
    if !state.navigation.panel_stack.is_empty() {
        debug!("KEY: ESC pressed with panel open - popping panel");
        return Some(Action::PopPanel);
    }
    // ... other ESC handlers for modals, etc.
}
```

**Reducer:** `src/tui/reducers/panels.rs` (lines 31-70)

```rust
fn pop_panel(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    if let Some(panel_state) = new_state.navigation.panel_stack.pop() {
        // Clear the loading state for the panel being popped
        match &panel_state.panel {
            Panel::Boxscore { game_id } => {
                new_state.data.loading.remove(&LoadingKey::Boxscore(*game_id));
            }
            Panel::TeamDetail { abbrev } => {
                new_state.data.loading.remove(&LoadingKey::TeamRosterStats(abbrev.clone()));
            }
            Panel::PlayerDetail { player_id } => {
                new_state.data.loading.remove(&LoadingKey::PlayerStats(*player_id));
            }
        }

        debug!(
            "PANEL: Popped panel, {} remaining",
            new_state.navigation.panel_stack.len()
        );
    }

    (new_state, Effect::None)
}
```

**Important:** Closing a panel clears its loading state, preventing stale data.

---

## 5. Rendering Architecture

### 5.1 Panel Overlay Rendering

**Location:** `src/tui/components/app.rs` (lines 67-128)

When panels are open, they replace normal tab content:

```rust
if let Some(panel_state) = state.navigation.panel_stack.last() {
    // Panel is open
    let panel_element = self.render_panel(state, panel_state);
    let breadcrumb_element = self.render_breadcrumb(state);

    let content_with_breadcrumb = vertical(
        [
            Constraint::Length(1),      // Breadcrumb
            Constraint::Min(0),          // Panel content
        ],
        vec![breadcrumb_element, panel_element],
    );

    // Replace tab content with panel overlay
    match state.navigation.current_tab {
        Tab::Scores => (content_with_breadcrumb, Element::None, Element::None),
        Tab::Standings => (Element::None, content_with_breadcrumb, Element::None),
        Tab::Settings => (Element::None, Element::None, content_with_breadcrumb),
        _ => (Element::None, Element::None, Element::None),
    }
} else {
    // Normal tab rendering
    (
        self.render_scores_tab(state),
        self.render_standings_tab(state),
        self.render_settings_tab(state),
    )
}
```

### 5.2 Breadcrumb Navigation Display

**Location:** `src/tui/components/breadcrumb.rs`

```rust
pub struct BreadcrumbWidget {
    pub current_tab: Tab,
    pub panel_stack: Vec<PanelState>,
}

fn build_breadcrumb_text(&self) -> Vec<Span<'_>> {
    let mut spans = Vec::new();

    // Start with tab name
    spans.push(Span::styled(
        tab_name,
        Style::default().add_modifier(Modifier::BOLD),
    ));

    // Add each panel
    for panel_state in &self.panel_stack {
        spans.push(Span::raw(" > "));

        let panel_text = match &panel_state.panel {
            Panel::Boxscore { game_id } => format!("Boxscore: Game {}", game_id),
            Panel::TeamDetail { abbrev } => format!("Team: {}", abbrev),
            Panel::PlayerDetail { player_id } => format!("Player: {}", player_id),
        };

        spans.push(Span::raw(panel_text));
    }

    spans
}
```

**Example breadcrumbs:**
- `Scores` (no panels)
- `Scores > Boxscore: Game 2024020001` (one panel)
- `Scores > Boxscore: Game 2024020001 > Player: 8478402` (nested)
- `Scores > Boxscore: Game 2024020001 > Player: 8478402 > Team: TOR` (deeply nested)

### 5.3 Individual Panel Components

Each panel type has its own component with props passed from global state.

#### BoxscorePanel

**Location:** `src/tui/components/boxscore_panel.rs`

```rust
#[derive(Clone)]
pub struct BoxscorePanelProps {
    pub game_id: i64,
    pub boxscore: Option<Boxscore>,
    pub loading: bool,
    pub selected_index: Option<usize>,  // Highlights which player row
    pub focused: bool,
}
```

Renders:
- Away team players (forwards, defense, goalies)
- Home team players (forwards, defense, goalies)
- Each player's stats
- Highlighted selection row

#### TeamDetailPanel

**Location:** `src/tui/components/team_detail_panel.rs`

```rust
#[derive(Clone)]
pub struct TeamDetailPanelProps {
    pub team_abbrev: String,
    pub club_stats: Option<ClubStats>,
    pub loading: bool,
    pub selected_index: Option<usize>,  // Highlights which player row
}
```

Renders:
- Team header (name, record, division)
- Skaters table (sorted by points descending)
- Goalies table (sorted by GP descending)
- Highlighted selection row

#### PlayerDetailPanel

**Location:** `src/tui/components/player_detail_panel.rs`

```rust
#[derive(Clone)]
pub struct PlayerDetailPanelProps {
    pub player_id: i64,
    pub player_data: Option<PlayerLanding>,
    pub loading: bool,
    pub selected_index: Option<usize>,  // Highlights which season row
}
```

Renders:
- Player header (name, position, team)
- Career stats
- Season totals table (sorted by season descending)
- Highlighted selection row

---

## 6. Data Loading Mechanism

### 6.1 Effect Dispatch

**Location:** `src/tui/reducers/data_loading.rs`

When `PushPanel` is processed:

```rust
Action::PushPanel(panel) => {
    new_state.data.loading.insert(...);  // Mark as loading

    let effect = match panel {
        Panel::Boxscore { game_id } => {
            DataEffects::new(client.clone()).fetch_boxscore(*game_id)
        }
        Panel::TeamDetail { abbrev } => {
            DataEffects::new(client.clone()).fetch_team_roster_stats(abbrev.clone())
        }
        Panel::PlayerDetail { player_id } => {
            DataEffects::new(client.clone()).fetch_player_stats(*player_id)
        }
    };

    (new_state, effect)
}
```

### 6.2 Loading State

**Location:** `src/tui/state.rs`

```rust
pub enum LoadingKey {
    Boxscore(i64),
    TeamRosterStats(String),
    PlayerStats(i64),
}

pub struct DataState {
    pub boxscores: Arc<HashMap<i64, Boxscore>>,
    pub team_roster_stats: Arc<HashMap<String, ClubStats>>,
    pub player_data: Arc<HashMap<i64, PlayerLanding>>,
    pub loading: HashSet<LoadingKey>,
}
```

### 6.3 Data Arrival

When async fetch completes, action is dispatched:

```rust
Action::BoxscoreLoaded(game_id, result) => {
    // Remove loading key
    new_state.data.loading.remove(&LoadingKey::Boxscore(game_id));

    // Store data
    if let Ok(boxscore) = result {
        Arc::make_mut(&mut new_state.data.boxscores)
            .insert(game_id, boxscore);
    }
}
```

---

## 7. Test Coverage

**Location:** `src/tui/reducers/panels.rs` (lines 308-1434)

17 comprehensive tests covering:

```
test_push_panel                                      (basic push)
test_pop_panel_clears_loading_state                 (cleanup on pop)
test_panel_selection                                (up/down navigation)
test_panel_select_item_skater                       (select skater from team)
test_panel_select_item_goalie                       (select goalie from team)
test_panel_select_item_second_goalie                (index mapping for 2nd goalie)
test_panel_select_item_uses_sorted_roster           (CRITICAL: sorting invariant)
test_panel_select_item_sorted_second_position       (2nd position in sorted list)
test_panel_select_item_goalies_sorted_by_games_played (CRITICAL: goalie sorting)
test_panel_select_item_boxscore_away_forward        (select away forward from box)
test_panel_select_item_boxscore_home_forward        (select home forward from box)
test_panel_select_item_boxscore_away_defense        (select away defense from box)
test_panel_select_item_player_detail_season         (navigate to team from season)
test_panel_select_item_sorted_second_position       (2nd item in sorted list)
test_panel_select_item_player_detail_season         (season selection)
```

**Key test:** `test_panel_select_item_uses_sorted_roster` (lines 726-847)

This test creates a roster where visual order ≠ data order, verifying that selection mapping respects sorted order, not data order. This is the **sorting invariant** that keeps visual and logical state synchronized.

---

## 8. Navigation Helper Utilities

**Location:** `src/tui/navigation.rs`

```rust
pub fn breadcrumb_trail(panel_stack: &[PanelState]) -> Vec<String>
pub fn breadcrumb_string(panel_stack: &[PanelState], separator: &str) -> String
pub fn is_at_root(panel_stack: &[PanelState]) -> bool
pub fn current_panel(panel_stack: &[PanelState]) -> Option<&Panel>
pub fn stack_depth(panel_stack: &[PanelState]) -> usize
```

These are utility functions for common panel stack operations.

---

## 9. Complete Navigation Example

### Scenario: Navigate Scores → Boxscore → Player → Team

```
STEP 1: User in Scores tab, presses Enter on a game
─────────────────────────────────────────────────────
- Key event: KeyCode::Enter
- key_to_action() → Action::ScoresAction(SelectGame(2024020001))
- reduce_scores() → Effect::Action(PushPanel(...))
- reduce() processes PushPanel
- reduce_data_loading() matches PushPanel
  - Adds LoadingKey::Boxscore(2024020001) to loading set
  - Returns Effect::Async(fetch_boxscore(2024020001))
- Async fetch begins
- State: panel_stack = []
         loading = {Boxscore(2024020001)}
         boxscores = {}

STEP 2: Boxscore data arrives
─────────────────────────────
- Effect completes → Action::BoxscoreLoaded(2024020001, Ok(data))
- reduce_data_loading() matches BoxscoreLoaded
  - Removes Boxscore(2024020001) from loading
  - Inserts data into boxscores HashMap
  - Actually pushes the panel now (odd, see below)
- State: panel_stack = [PanelState { Boxscore { 2024020001 }, selected_index: Some(0) }]
         loading = {}
         boxscores = {2024020001: Boxscore{...}}

STEP 3: App renders Boxscore panel
──────────────────────────────────
- App component checks panel_stack.last()
- Renders BreadcrumbWidget("Scores > Boxscore: Game 2024020001")
- Renders BoxscorePanelWidget with selected_index=0
  - Highlights first player (away forward 0)

STEP 4: User presses Down arrow
────────────────────────────────
- Key event: KeyCode::Down
- handle_panel_navigation() → Action::PanelSelectNext
- reduce_panels() processes PanelSelectNext
  - Updates panel.selected_index from 0 to 1
- State: panel_stack[0].selected_index = 1

STEP 5: App re-renders
──────────────────────
- BoxscorePanelWidget re-renders with selected_index=1
- Highlights second player (away forward 1 or first away defenseman)

STEP 6: User presses Enter on selected player
──────────────────────────────────────────────
- Key event: KeyCode::Enter
- handle_panel_navigation() → Action::PanelSelectItem
- reduce_panels() processes PanelSelectItem
  - Gets current panel (Boxscore) and selected_index (1)
  - Calls handle_boxscore_selection(state, 2024020001, 1)
  - Determines player_id (e.g., 8478402)
  - Pushes PlayerDetail panel to stack
  - Returns Effect::Action(PushPanel(PlayerDetail { 8478402 }))
- State: panel_stack = [
           PanelState { Boxscore { 2024020001 }, selected_index: 1 },
           PanelState { PlayerDetail { 8478402 }, selected_index: None }
         ]

STEP 7: Effect processes nested PushPanel
──────────────────────────────────────────
- reduce_data_loading() matches PushPanel(PlayerDetail)
  - Adds LoadingKey::PlayerStats(8478402) to loading
  - Returns Effect::Async(fetch_player_stats(8478402))
- State: loading = {PlayerStats(8478402)}
         player_data = {}

STEP 8: Player data arrives
────────────────────────────
- Effect completes → Action::PlayerStatsLoaded(8478402, Ok(data))
- reduce_data_loading() stores data in player_data HashMap
- State: loading = {}
         player_data = {8478402: PlayerLanding{...}}

STEP 9: App renders nested panels
──────────────────────────────────
- Breadcrumb: "Scores > Boxscore: Game 2024020001 > Player: 8478402"
- Panel content: PlayerDetailPanelWidget
  - Shows player info + career stats
  - selected_index: None (no selection yet)

STEP 10: User presses Down, then Enter on a season
────────────────────────────────────────────────────
- Down/Up/Enter actions navigate within player panel
- selected_index updated to point to a season
- Enter triggers handle_player_season_selection()
  - Finds season from player_data[8478402].season_totals
  - Extracts team abbrev (e.g., "TOR")
  - Pushes TeamDetail { "TOR" } panel
- State: panel_stack = [
           Boxscore { 2024020001 },
           PlayerDetail { 8478402 },
           TeamDetail { "TOR" }
         ]

STEP 11: Effect fetches team roster stats
───────────────────────────────────────────
- Async fetch begins
- DataEffects::fetch_team_roster_stats("TOR")
  - Queries available seasons, finds current season
  - Fetches ClubStats for current season
- When complete: Action::TeamRosterStatsLoaded("TOR", Ok(stats))
- State updated: team_roster_stats["TOR"] = ClubStats{...}

STEP 12: App renders three-deep nested navigation
──────────────────────────────────────────────────
- Breadcrumb: "Scores > Boxscore: Game 2024020001 > Player: 8478402 > Team: TOR"
- Panel content: TeamDetailPanelWidget
  - Shows TOR roster (skaters + goalies)
  - selected_index: 0 (first skater, from PanelState initialization)

STEP 13: User presses ESC
─────────────────────────
- Key event: KeyCode::Escape
- handle_esc_key() → Action::PopPanel
- reduce_panels() processes PopPanel
  - Pops TeamDetail panel
  - Clears LoadingKey::TeamRosterStats("TOR") from loading
- State: panel_stack = [
           Boxscore { 2024020001 },
           PlayerDetail { 8478402 }
         ]

STEP 14: App re-renders with two panels
────────────────────────────────────────
- Breadcrumb: "Scores > Boxscore: Game 2024020001 > Player: 8478402"
- Back to player detail view

STEP 15: User presses ESC again
────────────────────────────────
- Action::PopPanel
- reduce_panels() pops PlayerDetail
  - Clears LoadingKey::PlayerStats(8478402)
- State: panel_stack = [Boxscore { 2024020001 }]

STEP 16: User presses ESC again
────────────────────────────────
- Action::PopPanel
- reduce_panels() pops Boxscore
  - Clears LoadingKey::Boxscore(2024020001)
- State: panel_stack = [] (empty - back to normal Scores tab)
```

---

## 10. Critical Implementation Details

### 10.1 Sorting Invariant

**CRITICAL REQUIREMENT:** Data displayed must be sorted identically in both rendering and selection handling.

**Violation example from history:**
- UI displays roster sorted by points (highest first)
- Reducer uses unsorted data array
- Selecting position 0 picks wrong player

**Current code guards (lines 114-124 in panels.rs):**

```rust
// CRITICAL: Must sort the same way as team_detail_panel.rs does for display
// Otherwise visual position won't match data array index

// Sort skaters by points descending (matching team_detail_panel.rs:103)
let mut sorted_skaters = roster.skaters.clone();
sorted_skaters.sort_by_points_desc();

// Sort goalies by games played descending (matching team_detail_panel.rs:107)
let mut sorted_goalies = roster.goalies.clone();
sorted_goalies.sort_by_games_played_desc();
```

**Test validation:** `test_panel_select_item_uses_sorted_roster` (lines 726-847)

Creates test data where data order ≠ visual order, verifies selection matches visual, not data.

### 10.2 Section Boundary Calculations

In boxscore panels, multiple sections exist (away forwards/defense/goalies, home forwards/defense/goalies). Selection index must correctly map to the right section.

**Example calculation (from handle_boxscore_selection, lines 180-213):**

```rust
let away_forwards_count = away_stats.forwards.len();
let away_defense_count = away_stats.defense.len();
let away_goalies_count = away_stats.goalies.len();

let away_total = away_forwards_count + away_defense_count + away_goalies_count;
let home_forwards_count = home_stats.forwards.len();
let home_defense_count = home_stats.defense.len();

// Determine which player was selected
let player_id = if selected_index < away_forwards_count {
    // Away forward
    away_stats.forwards.get(selected_index).map(|p| p.player_id)
} else if selected_index < away_forwards_count + away_defense_count {
    // Away defense
    let defense_idx = selected_index - away_forwards_count;
    away_stats.defense.get(defense_idx).map(|p| p.player_id)
} else if selected_index < away_total {
    // Away goalie
    let goalie_idx = selected_index - away_forwards_count - away_defense_count;
    away_stats.goalies.get(goalie_idx).map(|p| p.player_id)
} else if selected_index < away_total + home_forwards_count {
    // Home forward
    let forward_idx = selected_index - away_total;
    home_stats.forwards.get(forward_idx).map(|p| p.player_id)
} else if selected_index < away_total + home_forwards_count + home_defense_count {
    // Home defense
    let defense_idx = selected_index - away_total - home_forwards_count;
    home_stats.defense.get(defense_idx).map(|p| p.player_id)
} else {
    // Home goalie
    let goalie_idx = selected_index - away_total - home_forwards_count - home_defense_count;
    home_stats.goalies.get(goalie_idx).map(|p| p.player_id)
};
```

This **must** match boxscore_panel.rs rendering order exactly.

### 10.3 Loading State Lifecycle

```
User opens panel
  → Panel added to stack
  → LoadingKey added to loading set
  → Effect dispatched for async fetch

Data arrives
  → LoadingKey removed from loading set
  → Data stored in data HashMap
  → Component renders with data

User closes panel
  → Panel removed from stack
  → LoadingKey removed from loading set (cleanup)
  → Prevents stale data from being used if reopened later
```

---

## 11. Affected Files: Complete Mapping

### Core Type/State Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/tui/types.rs` | 42 | Panel enum definition |
| `src/tui/state.rs` | 269 | NavigationState, PanelState, panel_stack |
| `src/tui/action.rs` | 158 | Panel-related actions (PushPanel, PopPanel, etc) |

### Reducer Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/tui/reducers/panels.rs` | 1434 | ALL panel state mutations + tests |
| `src/tui/reducers/data_loading.rs` | ? | Intercepts PushPanel, dispatches effects |
| `src/tui/reducer.rs` | 158 | Routes panel actions to sub-reducers |

### Key Handling Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/tui/keys.rs` | 400+ | Panel navigation key handling |
| `src/tui/navigation.rs` | 180 | Panel stack utilities |

### Rendering Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/tui/components/app.rs` | 300+ | Renders panel overlay |
| `src/tui/components/breadcrumb.rs` | 269 | Breadcrumb widget |
| `src/tui/components/boxscore_panel.rs` | 200+ | Boxscore rendering |
| `src/tui/components/team_detail_panel.rs` | 200+ | TeamDetail rendering |
| `src/tui/components/player_detail_panel.rs` | 200+ | PlayerDetail rendering |

### Support Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/tui/effects.rs` | 200+ | DataEffects for async fetch |
| `src/tui/component.rs` | 100+ | Component trait, Element types |

---

## 12. Current Limitations (Comparison to Document System)

### Panel Stack Strengths

- Simple: just a vector of enums + index
- Stateless rendering: components don't maintain state
- Stack semantics clear and understandable
- Good test coverage
- Breadcrumb generation straightforward

### Panel Stack Limitations

- **Only vertical navigation** (Up/Down selection)
- **No support for multi-column layouts** (like Standings view)
- **No scrolling support** (content larger than viewport)
- **Selection is just a number** (loses semantic meaning)
- **Tight coupling** between rendering sort and selection handling
- **No cross-column navigation** (Left/Right within rows)
- **Not composable** (panels are monolithic, can't build from smaller pieces)

### Document System Advantages (Why Migrate)

- **Arbitrary element hierarchies** (headings, text, tables, rows, links)
- **Scrolling/viewport support** (handle tall content)
- **Left/Right navigation** (within Row elements)
- **Focusable IDs** (semantic, not just indices)
- **Declarative building** (DocumentBuilder)
- **Composable** (nest elements easily)
- **Supports Standings use case** (multi-column with side-by-side navigation)

---

## 13. Refactoring Road Map (If Migrating)

### What Would Change

1. **Panel enum replaced** with Document implementations
2. **PanelState.selected_index** → DocumentState with focus_index, scroll_offset
3. **PushPanel/PopPanel** → PushDocument/PopDocument (similar semantics)
4. **Panel components** → Document element tree builders
5. **Breadcrumb** → Uses document.title() instead of panel.label()

### What Could Stay the Same

1. **Stack-based structure** (Vec of document states)
2. **Effect dispatch** for data loading
3. **Key-to-action mapping** logic
4. **Loading state tracking**
5. **Nested navigation** (push/pop semantics)
6. **Global state** architecture (still in AppState.navigation)

### Key Migration Points

| Current | Future |
|---------|--------|
| `Panel` enum (3 variants) | `Box<dyn Document>` trait objects |
| `PanelState.selected_index` | `DocumentNavState` with focus_index + scroll_offset |
| Component-based rendering | Element tree building |
| Index-based selection | Focusable ID-based selection |
| Props from global state | Document methods for data access |

---

## Summary

The **panel stack** is a well-designed, tested system for drill-down navigation with:

- **3 panel types** supporting Boxscore, Team, and Player detail views
- **Stack-based nesting** for multiple drill-down levels
- **Props-based stateless rendering** of component panels
- **Index-based selection** within panel content
- **Async data loading** via effect dispatch
- **Complete test coverage** with critical sorting invariants validated
- **Breadcrumb navigation** showing the user's path

It is **not suitable** for:
- Multi-column layouts (Standings view)
- Scrollable content larger than viewport
- Semantic left/right navigation
- Composable element hierarchies

A migration to the **document system** would enable more powerful navigation patterns but would require significant refactoring of the core navigation model.

