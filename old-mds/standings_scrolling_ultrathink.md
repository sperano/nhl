# UltraThink: Implementing Viewport Scrolling in Standings - League

## Executive Summary

Implementing viewport scrolling for standings-league view requires changes across **3 layers**:
1. **Reducer layer** - Auto-scroll logic when selection changes
2. **Component layer** - Window the data before passing to widget
3. **Key handler layer** - Add PageUp/PageDown support

**Critical insight**: The `scroll_offset` field already exists in state but is completely unused. We can activate it without breaking changes.

---

## Current Architecture Analysis

### Data Flow
```
User presses Down
  ↓
Key Handler (keys.rs)
  ↓
Action::StandingsAction(MoveSelectionDown)
  ↓
Reducer (reducers/standings.rs)
  - Updates selected_row (with wrapping)
  - Does NOT touch scroll_offset
  ↓
Component (components/standings_tab.rs)
  - Receives selected_row, scroll_offset
  - Passes ALL 32 teams to TableWidget
  - Does NOT window data
  ↓
Widget (components/table.rs)
  - Renders all rows sequentially
  - Stops when y >= area.bottom()
  - Selected rows below screen are invisible!
```

### Existing Patterns We Can Follow

**Pattern 1: Team Detail Panel** (`team_detail_panel.rs:113-167`)
```rust
// Calculate visible window
let available_height = area.height.saturating_sub(10) as usize;
let visible_end = (scroll_offset + available_height).min(total_items);
let show_from = scroll_offset.min(total_items);
let show_to = visible_end;

// Window the data
let windowed_items = &items[show_from..show_to];

// Adjust selection for windowing
let adjusted_selection = selected_index
    .filter(|&idx| idx >= show_from && idx < show_to)
    .map(|idx| idx - show_from);

// Create widget with windowed data
TableWidget::from_data(columns, windowed_items.to_vec())
    .with_selection_opt(adjusted_selection, Some(0))
```

**Pattern 2: Manual Scroll Actions** (`reducers/panels.rs:66-88`)
```rust
fn scroll_panel_down(state: AppState, amount: usize) -> (AppState, Effect) {
    panel.scroll_offset = panel.scroll_offset.saturating_add(amount);
}
```

**Key observation**: Panels have PageUp/PageDown that manually adjust `scroll_offset`, but they DON'T have auto-scroll when selection changes!

---

## Design Decision: Where Should Auto-Scroll Logic Live?

### Option A: In the Reducer (RECOMMENDED)
```rust
fn handle_move_selection_down(state: AppState) -> (AppState, Effect) {
    // Update selected_row
    new_state.ui.standings.selected_row = new_row;

    // Auto-scroll to keep selection visible
    let visible_height = 20; // ??? How do we know?
    if new_row >= scroll_offset + visible_height {
        new_state.ui.standings.scroll_offset = new_row - visible_height + 1;
    }
}
```

**Problem**: Reducer doesn't know `available_height` - that's calculated during rendering!

**Solution**: Use a reasonable constant (like 20-25 lines) or make auto-scroll optional

### Option B: In the Component (COMPLEX)
```rust
fn render_single_column_view(&self, props: &StandingsTabProps) -> Element {
    let available_height = calculate_available_height(area); // But we don't have area yet!

    // Adjust scroll_offset to keep selection visible
    let adjusted_scroll = ensure_selection_visible(
        props.selected_row,
        props.scroll_offset,
        available_height,
    );

    // Window data using adjusted_scroll
    let windowed_teams = &standings[adjusted_scroll..];
}
```

**Problem**: Components are pure - they can't modify props/state. And we don't have `area` during `view()`.

### Option C: Hybrid Approach (PRAGMATIC) ✅

**Reducer**: Implements auto-scroll with a conservative estimate (20 lines)
```rust
const ESTIMATED_VISIBLE_TEAMS: usize = 20;

fn handle_move_selection_down(state: AppState) -> (AppState, Effect) {
    new_state.ui.standings.selected_row = new_row;

    // Auto-scroll conservatively
    if new_row < scroll_offset {
        new_state.ui.standings.scroll_offset = new_row;
    } else if new_row >= scroll_offset + ESTIMATED_VISIBLE_TEAMS {
        new_state.ui.standings.scroll_offset = new_row - ESTIMATED_VISIBLE_TEAMS + 1;
    }
}
```

**Component**: Does windowing based on actual available height
```rust
fn render_single_column_view(&self, props: &StandingsTabProps, area: Rect) -> Element {
    let available_height = area.height.saturating_sub(8) as usize; // Headers, chrome
    let visible_end = (props.scroll_offset + available_height).min(standings.len());

    let windowed_teams = &standings[props.scroll_offset..visible_end];

    // Adjust selection
    let adjusted_row = props.selected_row
        .checked_sub(props.scroll_offset)
        .filter(|&r| r < windowed_teams.len());

    TableWidget::from_data(columns, windowed_teams.to_vec())
        .with_selection_opt(adjusted_row, Some(0))
}
```

**Why this works**:
- Reducer ensures selection is roughly visible (within ±20 rows of scroll_offset)
- Component fine-tunes with actual screen dimensions
- If reducer's estimate is off, component still renders correctly (just might not be perfectly centered)

---

## Edge Cases and Challenges

### Challenge 1: Component Signature Mismatch

**Current**:
```rust
fn render_single_column_view(&self, props: &StandingsTabProps, standings: &[Standing])
    -> Element
```

**Needed**:
```rust
fn render_single_column_view(&self, props: &StandingsTabProps, standings: &[Standing], area: Rect)
    -> Element
```

**Problem**: Components build `Element` trees BEFORE knowing the final `Rect`. The `area` is only known during `RenderableWidget::render()`.

**Solution**: Create a wrapper widget that does the windowing:

```rust
#[derive(Clone)]
struct ScrollableTableWidget {
    all_teams: Vec<Standing>,
    columns: &'static Vec<ColumnDef<Standing>>,
    selected_row: usize,
    scroll_offset: usize,
    focused: bool,
}

impl RenderableWidget for ScrollableTableWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // NOW we have area!
        let available_height = area.height.saturating_sub(5) as usize;
        let visible_end = (self.scroll_offset + available_height).min(self.all_teams.len());

        let windowed_teams = &self.all_teams[self.scroll_offset..visible_end];
        let adjusted_row = self.selected_row
            .checked_sub(self.scroll_offset)
            .filter(|&r| r < windowed_teams.len());

        let table = TableWidget::from_data(self.columns, windowed_teams.to_vec())
            .with_selection_opt(adjusted_row, Some(0))
            .with_focused(self.focused);

        table.render(area, buf, config);
    }
}
```

This is **cleaner** because windowing happens at the right layer!

### Challenge 2: Two-Column Views (Conference/Division)

**League view**: 1 column × 32 teams
**Conference view**: 2 columns × 16 teams each

**Question**: Should `scroll_offset` be global or per-column?

**Analysis**:
- Current state has `selected_column` and `selected_row`
- When switching columns, row is preserved
- This implies a **single shared scroll_offset**

**Recommendation**: Use single `scroll_offset` but apply it independently to each column

```rust
fn render_conference_view(&self, props: &StandingsTabProps) -> Element {
    // Each column independently windowed with same scroll_offset
    let left_windowed = window_column(&left_teams, props.scroll_offset, available_height);
    let right_windowed = window_column(&right_teams, props.scroll_offset, available_height);

    // Adjust selection based on which column is selected
    let left_selected = if props.selected_column == 0 {
        adjust_selection(props.selected_row, props.scroll_offset)
    } else {
        None
    };

    let right_selected = if props.selected_column == 1 {
        adjust_selection(props.selected_row, props.scroll_offset)
    } else {
        None
    };
}
```

### Challenge 3: Wrapping Behavior

**Current**: Down at row 31 wraps to row 0

**With scrolling**:
- Wrapping should reset `scroll_offset` to 0 (top)
- Or should it preserve scroll? (confusing UX)

**Recommendation**: Reset scroll on wrap

```rust
fn handle_move_selection_down(state: AppState) -> (AppState, Effect) {
    if selected_row >= max_row {
        // Wrap to top AND reset scroll
        new_state.ui.standings.selected_row = 0;
        new_state.ui.standings.scroll_offset = 0;
    } else {
        new_state.ui.standings.selected_row += 1;
        // Auto-scroll if needed
        adjust_scroll_to_keep_visible(&mut new_state);
    }
}
```

### Challenge 4: Cached Layout Invalidation

**Current**: Layout is cached and rebuilt when view changes

**With scrolling**: Layout still maps `[column][row]` to team_abbrev, but now row indices need adjustment

```rust
// Cached layout still uses absolute indices
layout[0][5] = "TOR"  // Row 5 in the full list

// When selecting a team in windowed view
let windowed_row = 2;  // User sees row 2 on screen
let absolute_row = scroll_offset + windowed_row;  // = 13 if scroll_offset = 11
let team = layout[selected_column][absolute_row];  // "TOR"
```

**No changes needed** - the layout cache still works! We just need to translate windowed row → absolute row when looking up.

### Challenge 5: Initial Scroll Position

**Scenario**: User enters browse mode with selection at row 0

**Question**: Should scroll start at 0 or center the selection?

**Current behavior**: Starts at row 0, scroll 0 (top of list)

**Recommendation**: Keep current behavior (simplest)

---

## Implementation Strategy

### Phase 1: Add Auto-Scroll to Reducer (Low Risk)

**File**: `src/tui/reducers/standings.rs`

**Changes**:
```rust
const ESTIMATED_VISIBLE_TEAMS: usize = 20;

fn handle_move_selection_up(state: AppState) -> (AppState, Effect) {
    // ... existing selection logic ...

    // NEW: Auto-scroll to keep selection visible
    ensure_selection_visible(&mut new_state);

    (new_state, Effect::None)
}

fn handle_move_selection_down(state: AppState) -> (AppState, Effect) {
    // ... existing selection logic ...

    // NEW: Auto-scroll to keep selection visible
    ensure_selection_visible(&mut new_state);

    (new_state, Effect::None)
}

fn ensure_selection_visible(state: &mut AppState) {
    let selected = state.ui.standings.selected_row;
    let scroll = state.ui.standings.scroll_offset;

    // If selection is above visible window, scroll up
    if selected < scroll {
        state.ui.standings.scroll_offset = selected;
    }

    // If selection is below visible window, scroll down
    else if selected >= scroll + ESTIMATED_VISIBLE_TEAMS {
        state.ui.standings.scroll_offset = selected.saturating_sub(ESTIMATED_VISIBLE_TEAMS - 1);
    }
}
```

**Testing**: Can test without changing component - scroll_offset will update but won't affect rendering yet

### Phase 2: Add Windowing to Component (Medium Risk)

**File**: `src/tui/components/standings_tab.rs`

**Option A**: Modify existing TableWidget usage (simpler but less flexible)
```rust
fn render_single_column_view(&self, props: &StandingsTabProps, standings: &[Standing])
    -> Element
{
    // Return a custom windowing widget
    Element::Widget(Box::new(WindowedStandingsTable {
        teams: standings.to_vec(),
        selected_row: props.selected_row,
        scroll_offset: props.scroll_offset,
        focused: props.browse_mode,
    }))
}
```

**Option B**: Create reusable ScrollableTableWidget (cleaner architecture)
```rust
// In src/tui/components/scrollable_table.rs
pub struct ScrollableTableWidget<T: Clone> {
    all_items: Vec<T>,
    columns: Vec<ColumnDef<T>>,
    selected_row: usize,
    scroll_offset: usize,
    focused: bool,
}

impl<T: Clone> RenderableWidget for ScrollableTableWidget<T> {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        let available_height = self.calculate_available_height(area);
        let windowed_items = self.window_items(available_height);
        let adjusted_selection = self.adjust_selection();

        let table = TableWidget::from_data(&self.columns, windowed_items)
            .with_selection_opt(adjusted_selection, Some(0))
            .with_focused(self.focused);

        table.render(area, buf, config);
    }
}
```

**Recommendation**: Start with Option A (simpler), refactor to Option B later if needed elsewhere

### Phase 3: Add PageUp/PageDown (Low Risk)

**File**: `src/tui/keys.rs`

```rust
fn handle_standings_tab_keys(key_code: KeyCode, state: &AppState) -> Option<Action> {
    if state.ui.standings.browse_mode {
        match key_code {
            KeyCode::Down => Some(Action::StandingsAction(StandingsAction::MoveSelectionDown)),
            KeyCode::Up => Some(Action::StandingsAction(StandingsAction::MoveSelectionUp)),

            // NEW: Page navigation
            KeyCode::PageDown => Some(Action::StandingsAction(StandingsAction::PageDown)),
            KeyCode::PageUp => Some(Action::StandingsAction(StandingsAction::PageUp)),
            KeyCode::Home => Some(Action::StandingsAction(StandingsAction::GoToTop)),
            KeyCode::End => Some(Action::StandingsAction(StandingsAction::GoToBottom)),

            KeyCode::Left => Some(Action::StandingsAction(StandingsAction::MoveSelectionLeft)),
            KeyCode::Right => Some(Action::StandingsAction(StandingsAction::MoveSelectionRight)),
            KeyCode::Enter => Some(Action::StandingsAction(StandingsAction::SelectTeam)),
            _ => None,
        }
    } else {
        // ... view selection mode ...
    }
}
```

**File**: `src/tui/action.rs`

```rust
pub enum StandingsAction {
    // ... existing actions ...
    PageDown,
    PageUp,
    GoToTop,
    GoToBottom,
}
```

**File**: `src/tui/reducers/standings.rs`

```rust
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
    const PAGE_SIZE: usize = 10;
    let mut new_state = state;

    // Get team count
    let team_count = get_team_count(&new_state);
    if team_count == 0 { return (new_state, Effect::None); }

    // Move selection down by PAGE_SIZE
    let new_row = (new_state.ui.standings.selected_row + PAGE_SIZE).min(team_count - 1);
    new_state.ui.standings.selected_row = new_row;

    // Auto-scroll will handle scroll_offset
    ensure_selection_visible(&mut new_state);

    (new_state, Effect::None)
}

fn handle_go_to_top(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.ui.standings.selected_row = 0;
    new_state.ui.standings.scroll_offset = 0;
    (new_state, Effect::None)
}

fn handle_go_to_bottom(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;
    let team_count = get_team_count(&new_state);
    if team_count > 0 {
        new_state.ui.standings.selected_row = team_count - 1;
        ensure_selection_visible(&mut new_state);
    }
    (new_state, Effect::None)
}
```

---

## Risk Assessment

### Low Risk Changes
✅ Auto-scroll in reducer - Pure state transformation, easily testable
✅ PageUp/PageDown actions - New functionality, no existing behavior changes
✅ Tests - Can add comprehensive test coverage before implementing

### Medium Risk Changes
⚠️ Windowing in component - Changes rendering logic, need careful testing
⚠️ Selection index adjustment - Off-by-one errors possible

### High Risk Changes
❌ None! This is an additive feature.

---

## Testing Strategy

### Unit Tests (Reducer)

```rust
#[test]
fn test_auto_scroll_when_selection_moves_below_visible() {
    let mut state = AppState::default();
    state.ui.standings.selected_row = 25;
    state.ui.standings.scroll_offset = 0;

    // Move down - should trigger scroll
    let (new_state, _) = handle_move_selection_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 26);
    assert!(new_state.ui.standings.scroll_offset > 0);
    // Selection should be within visible window
    assert!(new_state.ui.standings.selected_row >= new_state.ui.standings.scroll_offset);
    assert!(new_state.ui.standings.selected_row < new_state.ui.standings.scroll_offset + 20);
}

#[test]
fn test_auto_scroll_when_selection_moves_above_visible() {
    let mut state = AppState::default();
    state.ui.standings.selected_row = 5;
    state.ui.standings.scroll_offset = 10;

    let (new_state, _) = handle_move_selection_up(state);

    assert_eq!(new_state.ui.standings.selected_row, 4);
    assert_eq!(new_state.ui.standings.scroll_offset, 4); // Scrolled up
}

#[test]
fn test_wrap_to_top_resets_scroll() {
    let mut state = AppState::default();
    state.ui.standings.selected_row = 31;
    state.ui.standings.scroll_offset = 15;

    let (new_state, _) = handle_move_selection_down(state);

    assert_eq!(new_state.ui.standings.selected_row, 0);
    assert_eq!(new_state.ui.standings.scroll_offset, 0);
}
```

### Integration Tests (Widget Rendering)

```rust
#[test]
fn test_windowed_table_renders_only_visible_teams() {
    let standings = create_test_standings(32);
    let widget = WindowedStandingsTable {
        teams: standings,
        selected_row: 15,
        scroll_offset: 10,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 25);
    let mut buf = Buffer::empty(area);

    widget.render(area, &mut buf, &default_config());

    // Should show teams 10-34 (or fewer if area is smaller)
    // Team at absolute row 15 should be at visual row 5
    // Should have selector at visual row 5
}
```

### Manual Testing Checklist

- [ ] Navigate to League view
- [ ] Enter browse mode
- [ ] Press Down 20+ times - should scroll automatically
- [ ] Press Up back to top - should scroll back
- [ ] Press PageDown - should jump ~10 teams
- [ ] Press End - should jump to last team
- [ ] Press Home - should jump to first team
- [ ] Wrap from bottom to top - scroll should reset
- [ ] Switch to Conference view - scrolling should work with 2 columns
- [ ] Switch columns - scroll should be preserved
- [ ] Select a team - team detail should open correctly

---

## Rollout Plan

### Step 1: Foundation (Can merge independently)
1. Add auto-scroll helper function to reducers/standings.rs
2. Add comprehensive unit tests
3. Merge (no visible changes yet since windowing not implemented)

### Step 2: Core Scrolling (Breaking changes, needs careful testing)
1. Create WindowedStandingsTable widget
2. Update render_single_column_view to use it
3. Update render_conference_view to use it
4. Update render_division_view to use it
5. Test thoroughly (especially selection index math)
6. Merge after manual testing

### Step 3: Keyboard Navigation (Additive, low risk)
1. Add PageUp/PageDown/Home/End actions
2. Add key handlers
3. Add reducer handlers
4. Test
5. Merge

### Step 4: Polish (Optional enhancements)
1. Add visual scrollbar indicator
2. Add smooth scroll animation (probably overkill)
3. Add configuration option to disable auto-scroll

---

## Alternative: Don't Implement Scrolling

**Devil's advocate**: Should we even add scrolling?

**Reasons to skip**:
- 32 teams typically fit on most terminals (need 35-40 lines)
- Adds complexity for minimal benefit
- Current wrapping behavior works fine
- No user complaints about lack of scrolling

**Reasons to implement**:
- Completeness - `scroll_offset` field exists and is documented
- Consistency - Panels have scrolling
- Better UX on small terminals (laptop screens)
- Future-proofing if we add more data (expanded standings, playoffs)
- Intellectual satisfaction 😄

**Recommendation**: Implement it. The code is clean, well-tested, and doesn't introduce technical debt. The scrolling pattern is already established in panels, so we're just applying it consistently.

---

## Conclusion

**Implementing viewport scrolling is straightforward**:

1. ✅ **Architecture**: Use existing patterns from team_detail_panel
2. ✅ **State**: `scroll_offset` already exists, just activate it
3. ✅ **Logic**: Auto-scroll in reducer, windowing in widget
4. ✅ **Risk**: Low - additive feature, well-isolated changes
5. ✅ **Testing**: Unit + integration tests ensure correctness

**Estimated effort**: 4-6 hours for complete implementation + testing

**Recommendation**: Implement using the hybrid approach (auto-scroll in reducer with conservative estimate, precise windowing in widget). This gives best UX with cleanest architecture.
