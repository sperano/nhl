# How Scrolling and Navigation Works in Standings - League View

## Overview

The standings navigation system uses a **cursor-based navigation model** rather than traditional scrolling. Currently, there is **no viewport scrolling** implemented in the standings view - all 32 teams are rendered at once.

---

## State Structure

```rust
pub struct StandingsUiState {
    pub view: GroupBy,              // Which view: Wildcard/Division/Conference/League
    pub browse_mode: bool,          // true = navigating teams, false = selecting view
    pub selected_column: usize,     // Which column (0 or 1, for 2-column layouts)
    pub selected_row: usize,        // Which team row (0-31 in League view)
    pub scroll_offset: usize,       // Currently UNUSED (always 0)
    pub layout: Vec<Vec<String>>,   // Cached layout: layout[column][row] = team_abbrev
}
```

**Key insight**: `scroll_offset` exists but **is not used anywhere in rendering**. It's reset to 0 whenever the view changes.

---

## Navigation Modes

### Mode 1: View Selection (browse_mode = false)

**Focus**: Tab bar showing [Wildcard | Division | Conference | League]

**Keys**:
- **Left/Right**: Cycle through views (Division ← → Conference ← → League ← → Wildcard)
- **Down**: Enter browse mode to navigate teams
- **ESC**: Return to main tab bar

**What happens**:
```
User presses Right
  ↓
Action::StandingsAction(StandingsAction::CycleViewRight)
  ↓
Reducer: state.ui.standings.view = view.next()
         rebuild_standings_layout_cache()  // Rebuild cached layout
         reset_standings_selection()        // Reset row=0, col=0, scroll=0
  ↓
Re-render with new view
```

---

### Mode 2: Browse Mode (browse_mode = true)

**Focus**: The standings table itself (team selection)

**Keys**:
- **Up/Down**: Navigate between teams (with wrapping)
- **Left/Right**: Switch columns (only in Conference/Division/Wildcard views with 2 columns)
- **Enter**: Open team detail panel
- **ESC**: Exit browse mode, return to view selection

---

## League View Specifics

### Layout

**League view shows a SINGLE COLUMN** with all 32 teams sorted by points:

```
League
═════════════════════

  Team                    GP   W  L  OT  PTS
─────────────────────────────────────────────
▶ Florida Panthers        82  52 24  6  110    ← selected_row = 0
  Carolina Hurricanes     82  52 23  7  111
  New York Rangers        82  55 23  4  114
  ...
  Chicago Blackhawks      82  23 53  6   52    ← selected_row = 31
```

**Key characteristics**:
1. **Single column**: `selected_column` is always 0
2. **All teams visible**: No windowing/scrolling - all 32 teams rendered
3. **Cursor navigation**: Visual selector (▶) shows current position
4. **Wrapping**: Down at bottom wraps to top, Up at top wraps to bottom

---

## How Navigation Works (League View)

### 1. Moving Down

```rust
fn handle_move_selection_down(state: AppState) -> (AppState, Effect) {
    let team_count = 32;  // In League view, always 32 teams
    let max_row = 31;     // team_count - 1

    if state.ui.standings.selected_row >= max_row {
        // At last team - wrap to first team
        new_state.ui.standings.selected_row = 0;
    } else {
        new_state.ui.standings.selected_row += 1;
    }
}
```

**Example**:
```
selected_row = 0  → Press Down → selected_row = 1
selected_row = 31 → Press Down → selected_row = 0 (wraps)
```

---

### 2. Moving Up

```rust
fn handle_move_selection_up(state: AppState) -> (AppState, Effect) {
    if new_state.ui.standings.selected_row == 0 {
        // At first team - wrap to last team
        new_state.ui.standings.selected_row = 31;
    } else {
        new_state.ui.standings.selected_row -= 1;
    }
}
```

**Example**:
```
selected_row = 1  → Press Up → selected_row = 0
selected_row = 0  → Press Up → selected_row = 31 (wraps)
```

---

### 3. Moving Left/Right

**In League view: LEFT/RIGHT DO NOTHING**

```rust
fn handle_move_selection_left(state: AppState) -> (AppState, Effect) {
    // Only applies to Conference/Division/Wildcard views with 2 columns
    if matches!(view, GroupBy::Conference | GroupBy::Division | GroupBy::Wildcard) {
        // Switch columns...
    }
    // In League view, this is a no-op
}
```

League view has only 1 column, so column navigation is disabled.

---

### 4. Selecting a Team

**When user presses Enter**:

```rust
fn handle_select_team(state: AppState) -> (AppState, Effect) {
    // Look up team at current position using cached layout
    let team_abbrev = state.ui.standings.layout
        .get(selected_column)     // Column 0 in League view
        .and_then(|col| col.get(selected_row))  // Row 0-31
        .cloned();

    if let Some(team_abbrev) = team_abbrev {
        // Push TeamDetail panel onto navigation stack
        let panel = Panel::TeamDetail { abbrev: team_abbrev };
        new_state.navigation.panel_stack.push(PanelState {
            panel,
            scroll_offset: 0,  // Panel gets its own scroll state
            selected_index: None,
        });
    }

    (new_state, Effect::None)
}
```

**Flow**:
```
selected_row = 5, selected_column = 0
  ↓
layout[0][5] → "TOR" (Toronto Maple Leafs)
  ↓
Push Panel::TeamDetail { abbrev: "TOR" }
  ↓
Render team detail panel (overlays standings)
```

---

## Rendering Flow (League View)

```rust
// In StandingsTab component
fn render_single_column_view(&self, props: &StandingsTabProps, standings: &[Standing])
    -> Element
{
    // Create table widget with ALL 32 teams
    let table = TableWidget::from_data(
        Self::standings_columns(),
        standings.to_vec()  // ← ALL teams, no windowing!
    )
    .with_selection(props.selected_row, props.selected_column)
    .with_focused(props.browse_mode)
    .with_margin(0);

    Element::Widget(Box::new(table))
}
```

**TableWidget rendering**:
```rust
impl RenderableWidget for TableWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Render header
        // Render column headers
        // Render ALL rows (no windowing):
        for (row_idx, row_cells) in self.cell_data.iter().enumerate() {
            if y >= area.bottom() {
                break;  // ← Only stop when running out of screen space
            }

            // Render selector if this row is selected
            let is_row_selected = self.selected_row == Some(row_idx) && self.focused;
            let selector = if is_row_selected { "▶ " } else { "  " };

            // Render all cells in this row...
        }
    }
}
```

**Key point**: The widget renders rows sequentially until it runs out of vertical space (`y >= area.bottom()`). It doesn't use `scroll_offset` for windowing.

---

## What DOESN'T Happen (But Could)

### No Viewport Scrolling

Currently, if you're at team #31 (bottom of the list) and it's off-screen, **you can't see it**.

**What SHOULD happen** (not implemented):
```rust
// Calculate visible window
let visible_start = scroll_offset;
let visible_end = (scroll_offset + available_height).min(total_teams);

// Window the data
let visible_teams = &standings[visible_start..visible_end];

// Render only visible teams
let table = TableWidget::from_data(columns, visible_teams.to_vec())
```

**Auto-scroll to keep selection visible** (not implemented):
```rust
// When selection changes
if selected_row < scroll_offset {
    scroll_offset = selected_row;  // Scroll up
}
if selected_row >= scroll_offset + visible_height {
    scroll_offset = selected_row - visible_height + 1;  // Scroll down
}
```

### No PageUp/PageDown Support

The CLAUDE.md mentions PageUp/PageDown, but:
- **Not implemented** in `handle_standings_tab_keys()`
- Only Up/Down/Left/Right/Enter are handled
- `scroll_offset` exists but is unused

---

## Comparison: Conference/Division Views

These views have **2 columns** (e.g., Eastern vs Western):

```
Conference
══════════════════════════════════════════════════════════

Eastern                            Western
─────────────────────────          ─────────────────────────
▶ Florida Panthers                 Dallas Stars
  Carolina Hurricanes              Colorado Avalanche
  ...                              ...
```

**Navigation differences**:
- `selected_column` can be 0 or 1
- Left/Right arrows **DO work** (switch between columns)
- When switching columns, row is preserved (with clamping if new column has fewer teams)

```rust
fn handle_move_selection_right(state: AppState) -> (AppState, Effect) {
    if matches!(view, GroupBy::Conference | GroupBy::Division | GroupBy::Wildcard) {
        // Wrap: 1 → 0, 0 → 1
        new_state.ui.standings.selected_column = if column == 1 { 0 } else { 1 };

        // Clamp row to new column's max teams
        clamp_row_to_column_bounds(&mut new_state);
    }
}
```

---

## Summary

**Current Standings Navigation System**:

1. ✅ **Cursor-based navigation**: Visual selector (▶) shows current position
2. ✅ **Wrapping**: Up/Down wrap at boundaries
3. ✅ **Multi-column support**: Left/Right work in Conference/Division views
4. ✅ **Cached layout**: `layout[column][row]` maps position to team_abbrev
5. ❌ **No scrolling**: All teams rendered at once, no viewport windowing
6. ❌ **No PageUp/PageDown**: Not implemented despite being in docs
7. ❌ **scroll_offset unused**: Field exists but always 0

**Why no scrolling?**
- With ~32 teams and typical terminal height (40+ lines), all teams usually fit on screen
- Cursor navigation with wrapping is sufficient for this scale
- Scrolling would add complexity for minimal benefit

**If you wanted to add scrolling**, you would need to:
1. Implement PageUp/PageDown handlers in `handle_standings_tab_keys()`
2. Add auto-scroll logic to keep `selected_row` visible
3. Window the data in `render_single_column_view()` using `scroll_offset`
4. Update `TableWidget` to accept pre-windowed data with adjusted row indices

The current design prioritizes simplicity over features, which is reasonable given the data scale.
