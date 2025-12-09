# Game Details Navigation - User Guide

## Overview

The NHL TUI now supports full multi-level navigation from game scores to individual player details! This feature allows you to:
- View game boxscores
- Navigate through player lists
- View detailed player information and career statistics
- Navigate back with breadcrumb trails

## Navigation Flow

```
Scores Tab
    ↓ [Enter on a game]
Boxscore View
    ↓ [Down arrow]
Player Selection Mode
    ↓ [Arrow keys to navigate]
    ↓ [Enter on a player]
Player Details View
    ↓ [Esc to go back]
```

## Keyboard Controls

### Scores Tab (Game List)
- **Left/Right**: Change date (5-day sliding window)
- **Down**: Enter game selection mode
- **Enter**: View selected game's boxscore

### Boxscore View
- **Down**: Enter player selection mode
- **PageUp/PageDown**: Scroll boxscore
- **Home/End**: Jump to top/bottom
- **S**: Save boxscore to file
- **Esc**: Return to game list

### Player Selection Mode
- **Up/Down**: Navigate between players
- **Tab**: Jump to next section (forwards → defense → goalies)
- **Shift+Tab**: Jump to previous section
- **Enter**: View selected player's details
- **Esc**: Exit player selection mode
- **PageUp/PageDown**: Scroll boxscore

### Player Details View
- **PageUp/PageDown**: Scroll through player info
- **Home/End**: Jump to top/bottom
- **Esc**: Return to boxscore

## Player Sections

When in player selection mode, you can navigate through 6 sections:

### Away Team:
1. **Forwards** - Away team forwards
2. **Defense** - Away team defensemen
3. **Goalies** - Away team goaltenders

### Home Team:
4. **Forwards** - Home team forwards
5. **Defense** - Home team defensemen
6. **Goalies** - Home team goaltenders

Use **Tab** to quickly jump between sections, or use **Up/Down** to navigate naturally through all players.

## Visual Indicators

### Breadcrumbs
When viewing a player, the subtab area shows breadcrumbs:
```
Player Name
```

### Loading States
- "Loading boxscore..." - Fetching game data
- "Loading player information..." - Fetching player details

### Player Details Display
When viewing a player, you'll see:
- **Player Bio Card**: Position, jersey number, height, weight, shoots, birthplace, etc.
- **Career Statistics**: Season-by-season NHL stats (games, goals, assists, points, etc.)

## Example Usage Session

1. **Start the TUI**:
   ```bash
   cargo run
   ```

2. **Navigate to Scores tab** (if not already there)

3. **Select today's date** and browse games

4. **Press Enter** on a started/completed game to view boxscore

5. **Press Down** to enter player selection mode

6. **Use arrow keys** to find a player of interest

7. **Press Enter** on the player to view their details

8. **Review their career stats** (scroll with PageUp/PageDown)

9. **Press Esc** to return to the boxscore

10. **Press Esc again** to return to the game list

## Tips

- **Tab navigation is fastest**: Use Tab/Shift+Tab to quickly jump between team sections
- **Scrolling works everywhere**: PageUp/PageDown works in boxscore and player details
- **Breadcrumbs show context**: The subtab area always shows where you are
- **ESC is your friend**: Press ESC to go back one level at any time

## Technical Details

### Player Data Source
- Player information is fetched from the NHL API
- Career statistics include only NHL seasons
- Data is cached to avoid redundant API calls

### Performance
- Player selection uses efficient bounds checking
- Empty sections are automatically skipped
- Scrolling is optimized with viewport management

### Error Handling
- Missing player data shows loading state
- API errors are displayed in status bar
- Navigation remains functional even with errors

## Architecture

The implementation follows a modular design:

```
scores/
├── state.rs           - Navigation state management
├── handler.rs         - Key event handling
├── view.rs           - Rendering (game list, boxscore, player details)
├── panel.rs          - Panel definitions
└── game_details/
    ├── state.rs      - Player selection state
    ├── handler.rs    - Player navigation logic
    ├── view.rs       - Boxscore rendering
    └── players.rs    - Player extraction from boxscore
```

### Navigation Stack
Uses a stack-based navigation system:
- Each "level" (game, player) is pushed onto the stack
- ESC pops from the stack
- Breadcrumbs show the full trail

## Future Enhancements (Optional)

Potential improvements that could be added:
- Visual highlighting of selected player in boxscore
- Configuration options (highlight colors, scroll speeds)
- Additional player stats (season splits, game logs)
- Team roster navigation
- Player comparison views

## Troubleshooting

### "Loading player information..." never resolves
- Check your internet connection
- The NHL API may be temporarily unavailable
- Press ESC and try again

### Player selection doesn't work
- Ensure the game has started (future games have no player data)
- Make sure you're in player selection mode (press Down first)

### Breadcrumbs don't appear
- Breadcrumbs only show when viewing a player detail panel
- In boxscore view, date tabs are shown instead

## Integration

This feature integrates seamlessly with existing TUI functionality:
- Uses the same scrolling system as standings
- Shares player detail widgets with standings tab
- Follows the same keyboard conventions
- Maintains consistent visual style

---

**Enjoy exploring player statistics!** 🏒
