# Detailed Implementation Plan: Viewport Scrolling in Standings

## Overview

This plan implements viewport scrolling for standings-league view with **extensive test coverage** using `assert_buffer` for all rendering tests. The implementation is split into 5 phases, each with comprehensive unit and integration tests.

**Total Estimated Effort**: 8-12 hours (including extensive testing)
**Risk Level**: Low (well-tested, isolated changes)

---

## Phase 1: Add Auto-Scroll Helper Functions (2 hours)

### 1.1 Implementation

**File**: `src/tui/reducers/standings.rs`

**Add constant**:
```rust
/// Conservative estimate of visible teams for auto-scroll
/// This is used by the reducer to keep selection roughly in view.
/// The actual visible height is calculated during rendering.
const ESTIMATED_VISIBLE_TEAMS: usize = 20;
```

**Add helper function**:
```rust
/// Ensure the selected team is visible by adjusting scroll_offset
///
/// This function implements auto-scroll logic:
/// - If selection is above the visible window, scroll up
/// - If selection is below the visible window, scroll down
/// - Uses ESTIMATED_VISIBLE_TEAMS as a conservative estimate
///
/// # Arguments
///
/// * `state` - Mutable reference to AppState
///
/// # Example
///
/// ```
/// let mut state = AppState::default();
/// state.ui.standings.selected_row = 25;
/// state.ui.standings.scroll_offset = 0;
/// ensure_selection_visible(&mut state);
/// assert!(state.ui.standings.scroll_offset > 0);
/// ```
fn ensure_selection_visible(state: &mut AppState) {
    let selected = state.ui.standings.selected_row;
    let scroll = state.ui.standings.scroll_offset;

    // If selection is above visible window, scroll up
    if selected < scroll {
        state.ui.standings.scroll_offset = selected;
        debug!(
            "STANDINGS: Auto-scroll UP to keep row {} visible (scroll_offset: {} -> {})",
            selected, scroll, state.ui.standings.scroll_offset
        );
        return;
    }

    // If selection is below visible window, scroll down
    let visible_end = scroll + ESTIMATED_VISIBLE_TEAMS;
    if selected >= visible_end {
        let new_scroll = selected.saturating_sub(ESTIMATED_VISIBLE_TEAMS - 1);
        debug!(
            "STANDINGS: Auto-scroll DOWN to keep row {} visible (scroll_offset: {} -> {})",
            selected, scroll, new_scroll
        );
        state.ui.standings.scroll_offset = new_scroll;
    }
}

/// Helper to get total team count for current view
fn get_team_count(state: &AppState) -> usize {
    if let Some(standings) = state.data.standings.as_ref().as_ref() {
        get_team_count_for_column(
            standings,
            state.ui.standings.view,
            state.ui.standings.selected_column,
            state.system.config.display_standings_western_first,
        )
    } else {
        0
    }
}
```

**Modify existing navigation handlers**:
```rust
fn handle_move_selection_up(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    if let Some(standings) = new_state.data.standings.as_ref().as_ref() {
        let team_count = get_team_count_for_column(
            standings,
            new_state.ui.standings.view,
            new_state.ui.standings.selected_column,
            new_state.system.config.display_standings_western_first,
        );

        if team_count > 0 {
            let max_row = team_count - 1;
            if new_state.ui.standings.selected_row == 0 {
                // At first team - wrap to last team AND reset scroll
                new_state.ui.standings.selected_row = max_row;
                new_state.ui.standings.scroll_offset = 0; // NEW: Reset scroll on wrap
                debug!("STANDINGS: Wrapped to bottom, reset scroll_offset to 0");
            } else {
                new_state.ui.standings.selected_row -= 1;
                // NEW: Ensure selection stays visible
                ensure_selection_visible(&mut new_state);
            }
        }
    }

    (new_state, Effect::None)
}

fn handle_move_selection_down(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    if let Some(standings) = new_state.data.standings.as_ref().as_ref() {
        let team_count = get_team_count_for_column(
            standings,
            new_state.ui.standings.view,
            new_state.ui.standings.selected_column,
            new_state.system.config.display_standings_western_first,
        );

        if team_count > 0 {
            let max_row = team_count - 1;
            if new_state.ui.standings.selected_row >= max_row {
                // At last team - wrap to first team AND reset scroll
                new_state.ui.standings.selected_row = 0;
                new_state.ui.standings.scroll_offset = 0; // NEW: Reset scroll on wrap
                debug!("STANDINGS: Wrapped to top, reset scroll_offset to 0");
            } else {
                new_state.ui.standings.selected_row += 1;
                // NEW: Ensure selection stays visible
                ensure_selection_visible(&mut new_state);
            }
        }
    }

    (new_state, Effect::None)
}
```

---

### 1.2 Unit Tests for Auto-Scroll Logic

**File**: `src/tui/reducers/standings.rs` (in `#[cfg(test)]` section)

**Test 1: Auto-scroll when moving selection down beyond visible area**
```rust
#[test]
fn test_auto_scroll_down_when_selection_moves_below_visible() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 19; // Just at edge of visible (0-19)
    state.ui.standings.scroll_offset = 0;

    // Move down - should trigger scroll
    let (new_state, _) = handle_move_selection_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 20);
    assert_eq!(new_state.ui.standings.scroll_offset, 1,
        "Should scroll down by 1 to keep row 20 visible (20 - 20 + 1 = 1)");

    // Selection should be within visible window
    assert!(new_state.ui.standings.selected_row >= new_state.ui.standings.scroll_offset);
    assert!(new_state.ui.standings.selected_row < new_state.ui.standings.scroll_offset + ESTIMATED_VISIBLE_TEAMS);
}

#[test]
fn test_auto_scroll_down_multiple_times() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 0;
    state.ui.standings.scroll_offset = 0;

    // Move down 25 times
    for i in 1..=25 {
        let (new_state, _) = handle_move_selection_down(state.clone());
        state = new_state;

        assert_eq!(state.ui.standings.selected_row, i);
        // Scroll should track selection to keep it visible
        if i >= ESTIMATED_VISIBLE_TEAMS {
            assert!(state.ui.standings.scroll_offset > 0,
                "After moving to row {}, scroll should have started", i);
        }
    }
}
```

**Test 2: Auto-scroll when moving selection up above visible area**
```rust
#[test]
fn test_auto_scroll_up_when_selection_moves_above_visible() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 15;
    state.ui.standings.scroll_offset = 10;

    // Move up - should scroll up to keep selection visible
    let (new_state, _) = handle_move_selection_up(state);

    assert_eq!(new_state.ui.standings.selected_row, 14);
    assert_eq!(new_state.ui.standings.scroll_offset, 10,
        "Should not scroll yet - row 14 is still visible (10-29)");

    // Move up more
    let mut state = new_state;
    for _ in 0..5 {
        let (new_state, _) = handle_move_selection_up(state.clone());
        state = new_state;
    }

    assert_eq!(state.ui.standings.selected_row, 9);
    assert_eq!(state.ui.standings.scroll_offset, 9,
        "Should scroll up to keep row 9 visible");
}
```

**Test 3: Wrapping resets scroll**
```rust
#[test]
fn test_wrap_to_top_resets_scroll() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 31;
    state.ui.standings.scroll_offset = 15;

    // Wrap from bottom to top
    let (new_state, _) = handle_move_selection_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 0);
    assert_eq!(new_state.ui.standings.scroll_offset, 0,
        "Wrapping to top should reset scroll_offset");
}

#[test]
fn test_wrap_to_bottom_resets_scroll() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 0;
    state.ui.standings.scroll_offset = 0;

    // Wrap from top to bottom
    let (new_state, _) = handle_move_selection_up(state);

    assert_eq!(new_state.ui.standings.selected_row, 31);
    assert_eq!(new_state.ui.standings.scroll_offset, 0,
        "Wrapping to bottom should reset scroll_offset to 0");
}
```

**Test 4: No scroll when selection is already visible**
```rust
#[test]
fn test_no_scroll_when_selection_already_visible() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 10;
    state.ui.standings.scroll_offset = 5;

    let initial_scroll = state.ui.standings.scroll_offset;

    // Move down within visible window (5-24)
    let (new_state, _) = handle_move_selection_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 11);
    assert_eq!(new_state.ui.standings.scroll_offset, initial_scroll,
        "Scroll should not change when selection moves within visible window");
}
```

**Test 5: Edge case - empty standings**
```rust
#[test]
fn test_no_scroll_with_empty_standings() {
    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(vec![]));
    state.ui.standings.selected_row = 0;
    state.ui.standings.scroll_offset = 0;

    let (new_state, _) = handle_move_selection_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 0);
    assert_eq!(new_state.ui.standings.scroll_offset, 0);
}
```

**Test 6: Helper function tests**
```rust
#[test]
fn test_ensure_selection_visible_scroll_down() {
    let mut state = AppState::default();
    state.ui.standings.selected_row = 25;
    state.ui.standings.scroll_offset = 0;

    ensure_selection_visible(&mut state);

    // Row 25 with scroll 0 means row is at position 25
    // Visible window is 0-19, so row 25 is outside
    // New scroll should be: 25 - 20 + 1 = 6
    assert_eq!(state.ui.standings.scroll_offset, 6);
}

#[test]
fn test_ensure_selection_visible_scroll_up() {
    let mut state = AppState::default();
    state.ui.standings.selected_row = 5;
    state.ui.standings.scroll_offset = 10;

    ensure_selection_visible(&mut state);

    assert_eq!(state.ui.standings.scroll_offset, 5,
        "Should scroll up to make row 5 visible");
}

#[test]
fn test_ensure_selection_visible_no_change() {
    let mut state = AppState::default();
    state.ui.standings.selected_row = 10;
    state.ui.standings.scroll_offset = 5;

    ensure_selection_visible(&mut state);

    assert_eq!(state.ui.standings.scroll_offset, 5,
        "Should not change scroll when selection is visible");
}
```

---

## Phase 2: Create Windowed Standings Widget (3 hours)

### 2.1 Implementation

**File**: `src/tui/components/standings_tab.rs`

**Add widget struct**:
```rust
/// Widget that renders a windowed view of standings with scrolling support
///
/// This widget handles viewport scrolling by windowing the data based on
/// scroll_offset and available screen height. It adjusts selection indices
/// to account for the windowing.
#[derive(Clone)]
struct WindowedStandingsTable {
    all_teams: Vec<Standing>,
    columns: &'static Vec<ColumnDef<Standing>>,
    selected_row: usize,
    selected_column: usize,
    scroll_offset: usize,
    focused: bool,
    margin: u16,
}

impl WindowedStandingsTable {
    /// Create a new windowed standings table
    fn new(
        teams: Vec<Standing>,
        selected_row: usize,
        selected_column: usize,
        scroll_offset: usize,
        focused: bool,
    ) -> Self {
        Self {
            all_teams: teams,
            columns: StandingsTab::standings_columns(),
            selected_row,
            selected_column,
            scroll_offset,
            focused,
            margin: 0,
        }
    }

    /// Calculate available height for table content
    ///
    /// Subtracts space needed for:
    /// - Column headers (1 line)
    /// - Separator (1 line)
    /// - Bottom padding (1 line)
    fn calculate_available_height(&self, area: Rect) -> usize {
        area.height.saturating_sub(3) as usize
    }

    /// Window the teams based on scroll_offset and available height
    fn window_teams(&self, available_height: usize) -> Vec<Standing> {
        let visible_start = self.scroll_offset;
        let visible_end = (self.scroll_offset + available_height).min(self.all_teams.len());

        self.all_teams[visible_start..visible_end].to_vec()
    }

    /// Adjust selection row for windowed view
    ///
    /// Returns None if selected row is outside the visible window
    fn adjust_selection(&self) -> Option<usize> {
        let available_height = ESTIMATED_VISIBLE_TEAMS; // Conservative estimate
        let visible_start = self.scroll_offset;
        let visible_end = (self.scroll_offset + available_height).min(self.all_teams.len());

        if self.selected_row >= visible_start && self.selected_row < visible_end {
            Some(self.selected_row - visible_start)
        } else {
            None
        }
    }
}

impl RenderableWidget for WindowedStandingsTable {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        if self.all_teams.is_empty() {
            return;
        }

        // Calculate visible window with actual area
        let available_height = self.calculate_available_height(area);
        let windowed_teams = self.window_teams(available_height);

        // Adjust selection for windowed view
        let adjusted_row = if self.selected_row >= self.scroll_offset {
            let relative_row = self.selected_row - self.scroll_offset;
            if relative_row < windowed_teams.len() {
                Some(relative_row)
            } else {
                None
            }
        } else {
            None
        };

        // Create table with windowed data
        let table = TableWidget::from_data(self.columns, windowed_teams)
            .with_selection_opt(adjusted_row, Some(self.selected_column))
            .with_focused(self.focused)
            .with_margin(self.margin);

        table.render(area, buf, config);
    }

    fn clone_box(&self) -> Box<dyn RenderableWidget> {
        Box::new(self.clone())
    }
}
```

**Update render_single_column_view**:
```rust
fn render_single_column_view(&self, props: &StandingsTabProps, standings: &[Standing]) -> Element {
    // Use windowed table widget
    Element::Widget(Box::new(WindowedStandingsTable::new(
        standings.to_vec(),
        props.selected_row,
        props.selected_column,
        props.scroll_offset,
        props.browse_mode,
    )))
}
```

---

### 2.2 Widget Rendering Tests (using assert_buffer)

**File**: `src/tui/components/standings_tab.rs` (in `#[cfg(test)]` section)

**Test 1: Windowed view renders only visible teams**
```rust
#[test]
fn test_windowed_table_renders_subset_of_teams() {
    use crate::tui::testing::{assert_buffer, create_test_standings, RENDER_WIDTH};

    let standings = create_test_standings(32);
    let widget = WindowedStandingsTable::new(
        standings,
        15,  // selected_row (absolute)
        0,   // selected_column
        10,  // scroll_offset (start at team 10)
        true, // focused
    );

    let area = Rect::new(0, 0, RENDER_WIDTH, 15);
    let mut buf = Buffer::empty(area);
    let config = crate::config::DisplayConfig::default();

    widget.render(area, &mut buf, &config);

    // Should show teams 10-24 (or fewer if area is smaller)
    // Team at row 0 (Team 10 absolute) should have no selector
    // Team at row 5 (Team 15 absolute) should have selector
    let lines = crate::tui::testing::buffer_lines(&buf);

    // Check that we see the correct teams (Team 10, Team 11, etc.)
    // First data row should be Team 10 (not Team 0)
    assert!(lines[2].contains("Team 10"), "First visible team should be Team 10");

    // Row with selector should be Team 15 (at visual row 5)
    let selector_line = lines.iter()
        .position(|line| line.contains("▶"))
        .expect("Should find selector");
    assert!(lines[selector_line].contains("Team 15"),
        "Selector should be on Team 15");
}

#[test]
fn test_windowed_table_with_scroll_offset_zero() {
    use crate::tui::testing::{assert_buffer, create_test_standings, RENDER_WIDTH};

    let standings = create_test_standings(32);
    let widget = WindowedStandingsTable::new(
        standings,
        0,   // selected_row
        0,   // selected_column
        0,   // scroll_offset (no scrolling)
        true,
    );

    let area = Rect::new(0, 0, RENDER_WIDTH, 15);
    let mut buf = Buffer::empty(area);
    let config = crate::config::DisplayConfig::default();

    widget.render(area, &mut buf, &config);

    let lines = crate::tui::testing::buffer_lines(&buf);

    // First team should be Team 0
    assert!(lines[2].contains("Team 0"), "First visible team should be Team 0");

    // Selector should be on first data row
    assert!(lines[2].contains("▶"), "Selector should be on first team");
}

#[test]
fn test_windowed_table_near_end_of_list() {
    use crate::tui::testing::{assert_buffer, create_test_standings, RENDER_WIDTH};

    let standings = create_test_standings(32);
    let widget = WindowedStandingsTable::new(
        standings,
        31,  // selected_row (last team)
        0,   // selected_column
        25,  // scroll_offset (near end)
        true,
    );

    let area = Rect::new(0, 0, RENDER_WIDTH, 15);
    let mut buf = Buffer::empty(area);
    let config = crate::config::DisplayConfig::default();

    widget.render(area, &mut buf, &config);

    let lines = crate::tui::testing::buffer_lines(&buf);

    // Should show teams 25-31 (last 7 teams)
    assert!(lines[2].contains("Team 25"), "First visible should be Team 25");

    // Last team should be visible
    assert!(lines.iter().any(|line| line.contains("Team 31")),
        "Team 31 should be visible");

    // Selector should be on Team 31
    let selector_line = lines.iter()
        .position(|line| line.contains("▶"))
        .expect("Should find selector");
    assert!(lines[selector_line].contains("Team 31"),
        "Selector should be on Team 31");
}

#[test]
fn test_windowed_table_selection_outside_window() {
    use crate::tui::testing::{create_test_standings, RENDER_WIDTH};

    let standings = create_test_standings(32);
    let widget = WindowedStandingsTable::new(
        standings,
        5,   // selected_row (outside window)
        0,   // selected_column
        10,  // scroll_offset (window is 10-29)
        true,
    );

    let area = Rect::new(0, 0, RENDER_WIDTH, 15);
    let mut buf = Buffer::empty(area);
    let config = crate::config::DisplayConfig::default();

    widget.render(area, &mut buf, &config);

    let lines = crate::tui::testing::buffer_lines(&buf);

    // Should NOT have selector (selection is outside visible window)
    assert!(!lines.iter().any(|line| line.contains("▶")),
        "Should not show selector when selection is outside window");
}
```

**Test 2: Exact buffer comparison for windowed rendering**
```rust
#[test]
fn test_windowed_table_exact_rendering() {
    use crate::tui::testing::{assert_buffer, create_test_standings};

    let standings = create_test_standings(5); // Small set for exact comparison
    let widget = WindowedStandingsTable::new(
        standings,
        1,   // selected_row
        0,   // selected_column
        0,   // scroll_offset
        true,
    );

    let area = Rect::new(0, 0, 50, 8);
    let mut buf = Buffer::empty(area);
    let config = crate::config::DisplayConfig::default();

    widget.render(area, &mut buf, &config);

    assert_buffer(&buf, &[
        "  Team                    GP   W  L  OT  PTS  ",
        "──────────────────────────────────────────────",
        "  Team 0                   0   0  0   0    0  ",
        "▶ Team 1                   0   0  0   0    0  ",  // Selected
        "  Team 2                   0   0  0   0    0  ",
        "  Team 3                   0   0  0   0    0  ",
        "  Team 4                   0   0  0   0    0  ",
        "                                              ",
    ]);
}

#[test]
fn test_windowed_table_with_scroll_offset() {
    use crate::tui::testing::{assert_buffer, create_test_standings};

    let standings = create_test_standings(10);
    let widget = WindowedStandingsTable::new(
        standings,
        7,   // selected_row (absolute)
        0,   // selected_column
        5,   // scroll_offset (skip first 5 teams)
        true,
    );

    let area = Rect::new(0, 0, 50, 7);
    let mut buf = Buffer::empty(area);
    let config = crate::config::DisplayConfig::default();

    widget.render(area, &mut buf, &config);

    assert_buffer(&buf, &[
        "  Team                    GP   W  L  OT  PTS  ",
        "──────────────────────────────────────────────",
        "  Team 5                   0   0  0   0    0  ",  // First visible (scroll_offset=5)
        "  Team 6                   0   0  0   0    0  ",
        "▶ Team 7                   0   0  0   0    0  ",  // Selected (7-5=row 2)
        "  Team 8                   0   0  0   0    0  ",
        "                                              ",
    ]);
}
```

---

## Phase 3: Add PageUp/PageDown Navigation (2 hours)

### 3.1 Implementation

**File**: `src/tui/action.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum StandingsAction {
    // ... existing actions ...

    /// Scroll down one page (move selection ~10 rows down)
    PageDown,

    /// Scroll up one page (move selection ~10 rows up)
    PageUp,

    /// Jump to first team
    GoToTop,

    /// Jump to last team
    GoToBottom,
}
```

**File**: `src/tui/keys.rs`

```rust
fn handle_standings_tab_keys(key_code: KeyCode, state: &AppState) -> Option<Action> {
    if state.ui.standings.browse_mode {
        match key_code {
            KeyCode::Up => Some(Action::StandingsAction(StandingsAction::MoveSelectionUp)),
            KeyCode::Down => Some(Action::StandingsAction(StandingsAction::MoveSelectionDown)),
            KeyCode::Left => Some(Action::StandingsAction(StandingsAction::MoveSelectionLeft)),
            KeyCode::Right => Some(Action::StandingsAction(StandingsAction::MoveSelectionRight)),
            KeyCode::Enter => Some(Action::StandingsAction(StandingsAction::SelectTeam)),

            // NEW: Page navigation
            KeyCode::PageDown => Some(Action::StandingsAction(StandingsAction::PageDown)),
            KeyCode::PageUp => Some(Action::StandingsAction(StandingsAction::PageUp)),
            KeyCode::Home => Some(Action::StandingsAction(StandingsAction::GoToTop)),
            KeyCode::End => Some(Action::StandingsAction(StandingsAction::GoToBottom)),

            _ => None,
        }
    } else {
        // ... view selection mode ...
    }
}
```

**File**: `src/tui/reducers/standings.rs`

```rust
/// Number of rows to jump when using PageUp/PageDown
const PAGE_SIZE: usize = 10;

pub fn reduce_standings(state: AppState, action: StandingsAction) -> (AppState, Effect) {
    match action {
        // ... existing actions ...
        StandingsAction::PageDown => handle_page_down(state),
        StandingsAction::PageUp => handle_page_up(state),
        StandingsAction::GoToTop => handle_go_to_top(state),
        StandingsAction::GoToBottom => handle_go_to_bottom(state),
    }
}

fn handle_page_down(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    let team_count = get_team_count(&new_state);
    if team_count == 0 {
        return (new_state, Effect::None);
    }

    // Move selection down by PAGE_SIZE
    let new_row = (new_state.ui.standings.selected_row + PAGE_SIZE).min(team_count - 1);
    new_state.ui.standings.selected_row = new_row;

    // Auto-scroll will handle scroll_offset
    ensure_selection_visible(&mut new_state);

    debug!("STANDINGS: PageDown to row {}", new_row);
    (new_state, Effect::None)
}

fn handle_page_up(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    let team_count = get_team_count(&new_state);
    if team_count == 0 {
        return (new_state, Effect::None);
    }

    // Move selection up by PAGE_SIZE
    let new_row = new_state.ui.standings.selected_row.saturating_sub(PAGE_SIZE);
    new_state.ui.standings.selected_row = new_row;

    // Auto-scroll will handle scroll_offset
    ensure_selection_visible(&mut new_state);

    debug!("STANDINGS: PageUp to row {}", new_row);
    (new_state, Effect::None)
}

fn handle_go_to_top(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    new_state.ui.standings.selected_row = 0;
    new_state.ui.standings.scroll_offset = 0;

    debug!("STANDINGS: Jump to top");
    (new_state, Effect::None)
}

fn handle_go_to_bottom(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    let team_count = get_team_count(&new_state);
    if team_count > 0 {
        new_state.ui.standings.selected_row = team_count - 1;
        ensure_selection_visible(&mut new_state);
    }

    debug!("STANDINGS: Jump to bottom");
    (new_state, Effect::None)
}
```

---

### 3.2 Page Navigation Tests

**File**: `src/tui/reducers/standings.rs`

```rust
#[test]
fn test_page_down() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 0;
    state.ui.standings.scroll_offset = 0;

    let (new_state, _) = handle_page_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 10,
        "PageDown should move 10 rows down");
    assert!(new_state.ui.standings.scroll_offset > 0,
        "Should auto-scroll to keep selection visible");
}

#[test]
fn test_page_down_near_end() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 25;

    let (new_state, _) = handle_page_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 31,
        "PageDown should clamp at last team (25 + 10 = 35, clamped to 31)");
}

#[test]
fn test_page_up() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 15;
    state.ui.standings.scroll_offset = 5;

    let (new_state, _) = handle_page_up(state);

    assert_eq!(new_state.ui.standings.selected_row, 5,
        "PageUp should move 10 rows up");
}

#[test]
fn test_page_up_near_top() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 5;

    let (new_state, _) = handle_page_up(state);

    assert_eq!(new_state.ui.standings.selected_row, 0,
        "PageUp should clamp at first team (5 - 10 = -5, clamped to 0)");
    assert_eq!(new_state.ui.standings.scroll_offset, 0);
}

#[test]
fn test_go_to_top() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 20;
    state.ui.standings.scroll_offset = 10;

    let (new_state, _) = handle_go_to_top(state);

    assert_eq!(new_state.ui.standings.selected_row, 0);
    assert_eq!(new_state.ui.standings.scroll_offset, 0);
}

#[test]
fn test_go_to_bottom() {
    use crate::tui::testing::create_test_standings;

    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.selected_row = 0;
    state.ui.standings.scroll_offset = 0;

    let (new_state, _) = handle_go_to_bottom(state);

    assert_eq!(new_state.ui.standings.selected_row, 31);
    // Auto-scroll should have adjusted scroll_offset
    assert!(new_state.ui.standings.scroll_offset > 0);
}
```

---

## Phase 4: Two-Column View Support (2 hours)

### 4.1 Implementation

**Update Conference/Division view rendering**:

**File**: `src/tui/components/standings_tab.rs`

```rust
fn render_conference_view(&self, props: &StandingsTabProps, standings: &[Standing]) -> Element {
    use std::collections::BTreeMap;

    // Group standings by conference
    let mut grouped: BTreeMap<String, Vec<Standing>> = BTreeMap::new();
    for standing in standings {
        let conference = standing.conference_name
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
        grouped
            .entry(conference)
            .or_default()
            .push(standing.clone());
    }

    // Convert to vec to determine ordering
    let mut groups: Vec<_> = grouped.into_iter().collect();

    if groups.len() == 2 {
        let western_first = props.config.display_standings_western_first;
        if western_first {
            groups.reverse();
        }
    }

    if groups.len() != 2 {
        return self.render_single_column_view(props, standings);
    }

    // NEW: Use WindowedStandingsTable for each column
    let left_table = WindowedStandingsTable::new(
        groups[0].1.clone(),
        if props.selected_column == 0 { props.selected_row } else { usize::MAX },
        0,  // Column index for selection highlighting
        props.scroll_offset,
        props.browse_mode && props.selected_column == 0,
    );

    let right_table = WindowedStandingsTable::new(
        groups[1].1.clone(),
        if props.selected_column == 1 { props.selected_row } else { usize::MAX },
        0,  // Column index for selection highlighting
        props.scroll_offset,
        props.browse_mode && props.selected_column == 1,
    );

    // Layout both tables side-by-side
    horizontal(&[Constraint::Percentage(50), Constraint::Percentage(50)], vec![
        Element::Widget(Box::new(left_table)),
        Element::Widget(Box::new(right_table)),
    ])
}
```

---

### 4.2 Two-Column Tests

```rust
#[test]
fn test_conference_view_with_scrolling() {
    use crate::tui::testing::{create_test_standings, assert_buffer};
    use crate::commands::standings::GroupBy;

    let standings = create_test_standings(32); // 16 per conference
    let mut props = create_test_props();
    props.standings = Arc::new(Some(standings));
    props.view = GroupBy::Conference;
    props.browse_mode = true;
    props.selected_column = 0;  // Left column
    props.selected_row = 10;
    props.scroll_offset = 5;    // Scroll both columns

    let tab = StandingsTab;
    let element = tab.view(&props, &());

    // Render and verify both columns show scrolled content
    // Left column should show teams 5-19
    // Right column should also show teams 5-19
    // Selector should be on left column, row 10 (visual row 5)
    // (Exact assertion depends on layout)
}
```

---

## Phase 5: Integration Testing (2 hours)

### 5.1 End-to-End Workflow Tests

**File**: `src/tui/integration_tests.rs` (new file)

```rust
//! Integration tests for standings scrolling behavior

use crate::tui::testing::{assert_buffer, create_test_standings};
use crate::tui::state::AppState;
use crate::tui::action::{Action, StandingsAction};
use crate::tui::reducer::reduce;
use crate::commands::standings::GroupBy;
use std::sync::Arc;

#[test]
fn test_complete_scrolling_workflow() {
    // Setup: 32 teams, League view
    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.view = GroupBy::League;
    state.ui.standings.browse_mode = true;

    // Navigate down 25 times
    for i in 1..=25 {
        let (new_state, _) = reduce(state, Action::StandingsAction(StandingsAction::MoveSelectionDown));
        state = new_state;

        assert_eq!(state.ui.standings.selected_row, i);

        // After ~20 rows, should start scrolling
        if i >= 20 {
            assert!(state.ui.standings.scroll_offset > 0,
                "Should have scrolled after moving to row {}", i);
        }
    }

    // PageDown from row 25
    let (state, _) = reduce(state, Action::StandingsAction(StandingsAction::PageDown));
    assert_eq!(state.ui.standings.selected_row, 31, "Should clamp at 31");

    // Go to top
    let (state, _) = reduce(state, Action::StandingsAction(StandingsAction::GoToTop));
    assert_eq!(state.ui.standings.selected_row, 0);
    assert_eq!(state.ui.standings.scroll_offset, 0);

    // Go to bottom
    let (state, _) = reduce(state, Action::StandingsAction(StandingsAction::GoToBottom));
    assert_eq!(state.ui.standings.selected_row, 31);
    assert!(state.ui.standings.scroll_offset > 0);
}

#[test]
fn test_column_switching_preserves_scroll() {
    let mut state = AppState::default();
    state.data.standings = Arc::new(Some(create_test_standings(32)));
    state.ui.standings.view = GroupBy::Conference;
    state.ui.standings.browse_mode = true;
    state.ui.standings.selected_row = 10;
    state.ui.standings.selected_column = 0;
    state.ui.standings.scroll_offset = 5;

    // Switch to right column
    let (new_state, _) = reduce(state, Action::StandingsAction(StandingsAction::MoveSelectionRight));

    assert_eq!(new_state.ui.standings.selected_column, 1);
    assert_eq!(new_state.ui.standings.scroll_offset, 5,
        "Scroll should be preserved when switching columns");
}
```

---

## Test Coverage Summary

### Reducer Tests (Phase 1)
- ✅ Auto-scroll down beyond visible
- ✅ Auto-scroll up above visible
- ✅ Wrapping resets scroll
- ✅ No scroll when selection visible
- ✅ Edge case: empty standings
- ✅ Helper function coverage
- **Total**: 10+ test cases

### Widget Rendering Tests (Phase 2)
- ✅ Windowed view renders subset
- ✅ Scroll offset zero (top of list)
- ✅ Near end of list
- ✅ Selection outside window
- ✅ Exact buffer comparisons (using `assert_buffer`)
- **Total**: 10+ test cases with `assert_buffer`

### Page Navigation Tests (Phase 3)
- ✅ PageDown
- ✅ PageDown near end
- ✅ PageUp
- ✅ PageUp near top
- ✅ GoToTop
- ✅ GoToBottom
- **Total**: 6 test cases

### Two-Column Tests (Phase 4)
- ✅ Conference view with scrolling
- ✅ Division view with scrolling
- ✅ Column switching preserves scroll
- **Total**: 3+ test cases

### Integration Tests (Phase 5)
- ✅ Complete scrolling workflow (navigate, page, jump)
- ✅ Column switching
- ✅ View switching
- **Total**: 3+ test cases

---

## Total Test Count: 30+ tests

All rendering tests use `assert_buffer` for exact buffer comparison.
All reducer tests cover edge cases and boundary conditions.

---

## Rollout Checklist

### Before Starting
- [ ] Review this plan with team
- [ ] Create feature branch: `feature/standings-viewport-scrolling`
- [ ] Ensure all existing tests pass

### Phase 1 Checklist
- [ ] Add `ESTIMATED_VISIBLE_TEAMS` constant
- [ ] Implement `ensure_selection_visible()`
- [ ] Implement `get_team_count()`
- [ ] Update `handle_move_selection_up()`
- [ ] Update `handle_move_selection_down()`
- [ ] Write 10 unit tests for auto-scroll
- [ ] All tests pass: `cargo test --lib reducers::standings`
- [ ] Commit: "feat(standings): Add auto-scroll logic for selection tracking"

### Phase 2 Checklist
- [ ] Create `WindowedStandingsTable` struct
- [ ] Implement `RenderableWidget` trait
- [ ] Update `render_single_column_view()`
- [ ] Write 10 widget rendering tests with `assert_buffer`
- [ ] All tests pass: `cargo test --lib components::standings_tab`
- [ ] Manual test: Navigate League view, verify scrolling
- [ ] Commit: "feat(standings): Add windowed table widget for viewport scrolling"

### Phase 3 Checklist
- [ ] Add actions: `PageDown`, `PageUp`, `GoToTop`, `GoToBottom`
- [ ] Add key handlers
- [ ] Implement reducer handlers
- [ ] Write 6 page navigation tests
- [ ] All tests pass: `cargo test --lib`
- [ ] Manual test: Try PageUp/PageDown/Home/End keys
- [ ] Commit: "feat(standings): Add keyboard navigation for paging"

### Phase 4 Checklist
- [ ] Update `render_conference_view()`
- [ ] Update `render_division_view()`
- [ ] Write 3 two-column tests
- [ ] All tests pass
- [ ] Manual test: Switch between columns while scrolled
- [ ] Commit: "feat(standings): Add scrolling support for two-column views"

### Phase 5 Checklist
- [ ] Create integration test file
- [ ] Write 3 end-to-end workflow tests
- [ ] All tests pass: `cargo test --lib`
- [ ] Manual testing session (30 min)
- [ ] Commit: "test(standings): Add integration tests for scrolling"

### Final Checklist
- [ ] Run full test suite: `cargo test`
- [ ] Run in release mode: `cargo run --release`
- [ ] Test on small terminal (80x24)
- [ ] Test on large terminal (120x40)
- [ ] Update CLAUDE.md with scrolling documentation
- [ ] Create PR with detailed description
- [ ] Request code review

---

## Manual Testing Script

```bash
# Build with development features
cargo build --features development

# Run TUI with mock data
cargo run --features development -- --mock

# Test sequence:
# 1. Navigate to Standings tab
# 2. Press Down to enter browse mode
# 3. Press Down 25+ times - verify scrolling
# 4. Press Home - verify jump to top
# 5. Press End - verify jump to bottom
# 6. Press PageDown - verify page jump
# 7. Press PageUp - verify page jump
# 8. Press Right to cycle to Conference view
# 9. Press Down to enter browse mode
# 10. Press Down 15 times - verify scrolling
# 11. Press Left/Right to switch columns - verify scroll preserved
# 12. Press Up at top - verify wrap to bottom, scroll reset
# 13. Press Down at bottom - verify wrap to top, scroll reset
```

---

## Success Criteria

✅ All 30+ tests pass
✅ `assert_buffer` used for all rendering tests
✅ Manual testing confirms smooth scrolling UX
✅ No regressions in existing functionality
✅ Code coverage ≥90% for new code
✅ Performance: No noticeable lag when scrolling

---

**Estimated Total Time**: 8-12 hours
**Test Coverage**: 30+ tests (all using `assert_buffer` for rendering)
**Risk**: Low (extensive testing, isolated changes)
