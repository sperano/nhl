# Navigation Behavior Specification

This document specifies the expected navigation behavior from a user's perspective.

## Key Event Flow

```
KeyEvent -> key_to_action(key, state) -> Option<Action>
```

Priority:
1. Global keys (q=Quit, /=CommandPalette)
2. ESC key (priority-based: document → modal → browse mode → content focus)
3. Document stack navigation (when stacked document open)
4. Number keys (1-6 direct tab switching)
5. Tab bar focused: arrows navigate tabs
6. Content focused: delegated to tab-specific handlers

## Focus Hierarchy

1. Tab bar (top level)
2. Content area (subtabs)
3. Item selection (within content)
4. Document stack (drill-down views)

## Main Tabs

- **Left/Right arrows**: Navigate between Scores, Standings, Settings
- **Down arrow**: Enter content focus
- **ESC**: Context-dependent (pop document, close modal, exit content focus, or quit)
- **Number keys 1-6**: Direct tab switching

## Scores Tab Navigation (5-Date Sliding Window)

### Architecture

The window has a **sticky base date** (leftmost date) that only shifts at edges:

- **window_base_date** = game_date - selected_index
- **Window** = `[base, base+1, base+2, base+3, base+4]` (always 5 dates)
- **game_date** = `window_base_date + selected_index`
- **selected_index** = position within window (0-4)

### Navigation Behavior

**Within Window (index 1-3 → 0-4):**
- `selected_index` changes
- `game_date` changes to match
- Window stays the same
- Refresh triggered

**At Left Edge (index = 0, press Left):**
- `selected_index` stays at 0
- `game_date` decrements by 1 day
- Window shifts left by 1 day
- Refresh triggered

**At Right Edge (index = 4, press Right):**
- `selected_index` stays at 4
- `game_date` increments by 1 day
- Window shifts right by 1 day
- Refresh triggered

### Example Sequence

```
Start: game_date=11/02, selected_index=2
  Window: [10/31, 11/01, 11/02, 11/03, 11/04]

Press Left: selected_index=1, game_date=11/01
  Window: [10/31, 11/01, 11/02, 11/03, 11/04] ← Same window

Press Left: selected_index=0, game_date=10/31
  Window: [10/31, 11/01, 11/02, 11/03, 11/04] ← Same window

Press Left at edge: game_date=10/30, selected_index=0
  Window: [10/30, 10/31, 11/01, 11/02, 11/03] ← Window shifted

Press Right: game_date=10/31, selected_index=1
  Window: [10/30, 10/31, 11/01, 11/02, 11/03] ← Same window
```

### Implementation

**Date Window Calculation (`scores_tab.rs`):**
```rust
fn calculate_date_window(game_date: &GameDate, selected_index: usize) -> [GameDate; 5] {
    let window_base_date = game_date.add_days(-(selected_index as i64));
    [
        window_base_date.add_days(0),
        window_base_date.add_days(1),
        window_base_date.add_days(2),
        window_base_date.add_days(3),
        window_base_date.add_days(4),
    ]
}
```

### Critical Rules

1. **NEVER** change the window calculation formula without updating this spec
2. **ALWAYS** ensure game_date updates on every arrow key press
3. **ALWAYS** maintain `window_base = game_date - selected_index`
4. **ALWAYS** run tests after navigation changes
5. **REFRESH** must trigger on every date change

### Common Mistakes

❌ Making game_date always at center (index 2)
❌ Not updating game_date when navigating within window
❌ Forgetting to trigger refresh on within-window navigation
❌ Shifting window when navigating within it

✅ Window base is sticky until edge is reached
✅ game_date moves within the window
✅ Refresh triggers on every date change
✅ Window only shifts at edges (index 0 or 4)

## Standings Tab Navigation

### View Selection Mode (Initial State)

User starts focused on subtab bar:
- **Division** (default)
- **Conference**
- **League**

**Navigation keys:**
- **Left/Right**: Cycle through views
- **Down**: Enter team selection mode
- **ESC**: Exit to main tabs

### Team Selection Mode

**Visual indicator**: Selected team highlighted in `selection_fg` color.

**Navigation keys:**
- **Up**: Move up; if at first team, exit to view selection mode
- **Down**: Move down; if at last team, stay
- **Left/Right**: Switch columns (Conference/Division views only)
- **ESC**: Exit to view selection mode
- **Enter**: Activate team (opens TeamDetail)
- **PageUp/PageDown/Home/End**: Scroll viewport

### Column Behavior by View

**League View:**
- Single column, all 32 teams
- Sorted by points
- Left/Right have no effect

**Conference View:**
- Two columns
- Order depends on `display_standings_western_first` config
- Left/Right switch columns, preserving row position

**Division View:**
- Two columns (Eastern/Western divisions)
- Order depends on `display_standings_western_first` config
- Teams grouped by division, sorted by points within
- Left/Right switch columns, preserving row position

### Auto-scrolling

When navigating with Up/Down:
- Viewport scrolls to keep selected team visible
- Scrolling is immediate during navigation

**Manual scrolling** (PageUp/PageDown/Home/End):
- Available in both view selection and team selection modes
- Does not change team selection
- PageUp/PageDown: Scroll by 10 lines
- Home/End: Scroll to top/bottom
