# Implementation Plan: Game Details Navigation

## Architecture Overview

The implementation will follow the existing modular navigation patterns, creating a multi-level navigation system:
1. **Scores Tab** → **Game List** → **Game Details** → **Player Details**
2. Navigation will use the same state/view/handler pattern used by other tabs

## Phase 1: Game Details State Structure

**File: `src/tui/scores/state.rs`**
- Add new fields to track game details navigation:
  ```rust
  pub struct State {
      // ... existing fields ...

      // Game details navigation
      pub game_details_active: bool,       // Are we viewing game details?
      pub player_selection_active: bool,   // Are we selecting players?
      pub selected_player_section: PlayerSection,  // Which section is selected
      pub selected_player_index: usize,    // Index within the section
      pub game_details_scrollable: Scrollable, // For game details scrolling
  }

  #[derive(Clone, Copy, PartialEq)]
  pub enum PlayerSection {
      AwayForwards,
      AwayDefense,
      AwayGoalies,
      HomeForwards,
      HomeDefense,
      HomeGoalies,
      ScoringSummary(usize), // Index in scoring plays
  }
  ```

## Phase 2: Navigation Hierarchy

**Navigation Flow:**
```
Scores Tab
  └─ Date Selection (existing)
      └─ Game Box Selection (existing)
          └─ Game Details View (NEW)
              ├─ Scoring Summary (selectable player names)
              ├─ Away Team Stats
              │   ├─ Forwards (selectable)
              │   ├─ Defense (selectable)
              │   └─ Goalies (selectable)
              └─ Home Team Stats
                  ├─ Forwards (selectable)
                  ├─ Defense (selectable)
                  └─ Goalies (selectable)
```

## Phase 3: Game Details Module Structure

Create a new submodule for game details:

**Files to create:**
- `src/tui/scores/game_details/mod.rs`
- `src/tui/scores/game_details/state.rs`
- `src/tui/scores/game_details/view.rs`
- `src/tui/scores/game_details/handler.rs`

**`src/tui/scores/game_details/state.rs`:**
```rust
pub struct GameDetailsState {
    pub player_selection_active: bool,
    pub selected_section: PlayerSection,
    pub selected_index: usize,
    pub scrollable: Scrollable,
}
```

## Phase 4: View Rendering Implementation

**`src/tui/scores/game_details/view.rs`:**
- Modify the existing `render_boxscore_content` to support player highlighting
- Add visual indicators for selectable players:
  - Highlight current selection with configured `selection_fg` color
  - Use arrow indicators (▶) for the currently selected player
  - Add section headers that change color when their section is focused

**Key rendering changes:**
1. Make player names in scoring summary selectable
2. Make player names in team stats tables selectable
3. Add visual breadcrumb: `Scores > Game > [Team] Players`
4. Show keyboard hints: `↑↓ Navigate • Enter Select • Esc Back`

## Phase 5: Navigation Handler Implementation

**`src/tui/scores/game_details/handler.rs`:**
```rust
pub async fn handle_key(
    key: KeyEvent,
    state: &mut GameDetailsState,
    boxscore: &Boxscore,
    shared_data: &SharedDataHandle,
    refresh_tx: &mpsc::Sender<()>,
) -> bool {
    match key.code {
        KeyCode::Down => navigate_to_next_player(state, boxscore),
        KeyCode::Up => navigate_to_previous_player(state, boxscore),
        KeyCode::Enter => select_current_player(state, boxscore, shared_data, refresh_tx).await,
        KeyCode::Esc => exit_game_details(state),
        KeyCode::Tab => jump_to_next_section(state, boxscore),
        KeyCode::BackTab => jump_to_previous_section(state, boxscore),
        _ => handle_scrolling(key, &mut state.scrollable),
    }
}
```

**Navigation logic:**
- **Up/Down**: Navigate within current section, wrap to next/previous section at boundaries
- **Tab/Shift+Tab**: Jump between sections (Away Forwards → Away Defense → Away Goalies → Home Forwards → etc.)
- **Enter**: Select player and transition to player details view
- **Esc**: Exit player selection mode or exit game details

## Phase 6: Player Details Integration

**Leverage existing player panel from standings:**
- Reuse `StandingsPanel::PlayerDetail` structure
- Extract player ID from selected player in boxscore
- Trigger player data fetch through `SharedData.selected_player_id`
- Display player career stats using existing rendering

**Navigation context:**
```rust
pub enum ScoresNavigationPanel {
    GameList,
    GameDetails { game_id: i64 },
    PlayerDetail {
        player_id: i64,
        player_name: String,
        from_game_id: i64,
    },
}
```

## Phase 7: Integration Points

**`src/tui/scores/handler.rs` modifications:**
1. When `boxscore_view_active` is true, delegate to game_details handler
2. Track navigation depth to handle Esc properly
3. Maintain breadcrumb trail for user orientation

**`src/shared_data.rs` additions:**
```rust
pub struct SharedData {
    // ... existing fields ...
    pub game_details_player_focus: Option<(i64, String)>, // (player_id, name)
}
```

## Phase 8: Visual Design

**Selection indicators:**
```
┌─ NYR @ BOS ─ Game Details ──────────────────────────┐
│ Scoring Summary                                      │
│ ────────────────                                     │
│ 1st 05:23  ▶ C. McAvoy (2)  Assists: D. Pastrnak   │
│ 1st 12:45    A. Fox (5)     Assists: M. Zibanejad  │
│                                                      │
│ BOS - Forwards                                      │
│ ────────────────                                     │
│ # Name                Pos  G  A  P  +/-  TOI       │
│ 63 B. Marchand        LW   1  2  3   2   18:42     │
│ 88 ▶ D. Pastrnak     RW   0  1  1   1   17:23     │
│                                                      │
│ [↑↓] Navigate [Tab] Next Section [Enter] View Player│
└──────────────────────────────────────────────────────┘
```

## Phase 9: Testing Strategy

**Test cases to implement:**
1. **Navigation flow**: Scores → Game → Player → Back to Game
2. **Section transitions**: Verify Tab/Shift+Tab cycles through all sections
3. **Player selection**: Ensure correct player ID is selected
4. **Boundary conditions**: First/last player in sections
5. **Empty sections**: Handle teams with no goalies dressed
6. **Scoring summary**: Select scorer or assist player names
7. **Memory management**: Ensure navigation stack doesn't leak
8. **Error handling**: Missing player data, API failures

## Phase 10: Configuration

**Add to `Config` struct:**
```rust
pub struct DisplayConfig {
    // ... existing ...
    pub game_details_auto_scroll: bool,  // Auto-scroll to keep selection visible
    pub highlight_player_stats: bool,    // Highlight row of selected player
}
```

## Implementation Order

1. **Foundation** (Phase 1-3): Create module structure and basic state
2. **Navigation** (Phase 4-5): Implement player selection within game details
3. **Player View** (Phase 6): Connect to existing player details panel
4. **Polish** (Phase 7-10): Integration, testing, and configuration

## Key Design Decisions

1. **Reuse existing patterns**: Follow the modular state/view/handler pattern
2. **Leverage player panel**: Reuse StandingsPanel::PlayerDetail for consistency
3. **Section-based navigation**: Group players by team and position for easier navigation
4. **Smart wrapping**: Up from first player goes to scoring summary, down from last wraps to top
5. **Visual consistency**: Use same selection colors and indicators as standings tab
6. **Breadcrumb navigation**: Always show user where they are in the hierarchy

## Benefits

This plan ensures a seamless, intuitive navigation experience that matches the existing UI patterns while adding powerful new functionality for exploring game and player details.

## Technical Considerations

### State Management
- Maintain clear separation between UI state (selection, scrolling) and data state (player info)
- Use Arc<RwLock> pattern for shared data consistency
- Ensure navigation state is properly cleaned up on exit

### Performance
- Lazy load player details only when selected
- Cache player data to avoid redundant API calls
- Use efficient rendering with ratatui's stateful widgets

### Error Handling
- Gracefully handle missing player data
- Show loading states during data fetches
- Provide clear error messages for API failures

### Accessibility
- Clear visual indicators for current selection
- Consistent keyboard shortcuts across all navigation levels
- Breadcrumb trail for orientation