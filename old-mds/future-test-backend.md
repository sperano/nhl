# TUI Components for TestBackend Testing

This document identifies TUI components that would benefit from testing with ratatui's `TestBackend`.

## Currently Tested ✅

- **`src/tui/common/status_bar.rs`** - Has TestBackend tests for loading state, countdown, error messages, etc.

## High Priority - Visual Components 🎯

### 1. **Tab Bar** (`src/tui/common/tab_bar.rs`)
- **Why**: Core navigation component used across all tabs
- **What to test**:
  - Tab separator rendering with box characters
  - Selected vs unselected tab styling
  - Focused vs unfocused state colors
  - Proper spacing between tabs
  - Unicode vs ASCII box character rendering

### 2. **Scoring Summary Tables** (`src/tui/scores/view.rs:371-548`)
- **Why**: Just implemented complex table formatting with dynamic widths
- **What to test**:
  - Table borders render correctly with box chars
  - Column alignment (team, description, score, time, shot type)
  - "Unassisted" vs assists rendering
  - Different score widths (1-0 vs 10-10)
  - Multi-goal period formatting

### 3. **Date Subtabs** (`src/tui/scores/view.rs:73-106`)
- **Why**: 5-date sliding window with complex navigation
- **What to test**:
  - 5-date window rendering
  - Selected date highlighting
  - Separator line with connectors at correct positions
  - Date format consistency (MM/DD)

### 4. **Standings View Subtabs** (`src/tui/standings/view.rs:30-88`)
- **Why**: Division/Conference/League selector
- **What to test**:
  - View option rendering (Division | Conference | League)
  - Selection highlighting
  - Breadcrumb rendering when navigating into team details
  - Separator line positioning

## Medium Priority - Layout Components 🎨

### 5. **Standings Table** (`src/tui/standings/view.rs:90-450`)
- **Why**: Complex multi-column layout with team selection
- **What to test**:
  - 1-column vs 2-column layouts
  - Division headers in correct positions
  - Team name alignment and truncation
  - Stats columns (GP, W, L, OT, PTS) alignment
  - Selected team highlighting
  - Scroll offset calculations

### 6. **Scores Grid Layout** (`src/tui/scores/view.rs:114-196`)
- **Why**: Responsive grid (1/2/3 columns) based on terminal width
- **What to test**:
  - 1-column layout (width < 76)
  - 2-column layout (76 ≤ width < 115)
  - 3-column layout (width ≥ 115)
  - Game box highlighting
  - Empty schedule message

### 7. **Breadcrumb Trail** (`src/tui/common/breadcrumb.rs`)
- **Why**: Navigation context display
- **What to test**:
  - Breadcrumb separator rendering
  - Multiple levels (Root > Team > Player)
  - Truncation for long names
  - Box character connectors

### 8. **Separator Lines** (`src/tui/common/separator.rs`)
- **Why**: Reusable component for tab separators
- **What to test**:
  - Connector positioning under tab text
  - Width calculations
  - Unicode vs ASCII mode
  - Padding calculations

## Lower Priority - Complex Integration 🔧

### 9. **Boxscore View** (`src/tui/scores/view.rs:565-586`)
- **Why**: Scrollable content with period boxes
- **What to test**:
  - Period score box rendering
  - Shots table rendering
  - Side-by-side table alignment
  - Scroll behavior

### 10. **Game Score Boxes** (`src/commands/scores_format.rs`)
- **Why**: Individual game box with periods
- **What to test**:
  - Box width consistency (37 chars)
  - Period columns (1, 2, 3, OT, SO)
  - Team abbreviation alignment
  - Total score column
  - Current period highlighting

## Example Test Structure

Here's how a test for the tab bar could look:

```rust
#[test]
fn test_tab_bar_selection_highlighting() {
    let mut terminal = Terminal::new(TestBackend::new(80, 2)).unwrap();
    let theme = Arc::new(DisplayConfig::default());

    terminal.draw(|f| {
        let area = Rect::new(0, 0, 80, 2);
        render(f, area, &["Scores", "Standings", "Settings"], 1, true, &theme);
    }).unwrap();

    let buffer = terminal.backend().buffer();

    // Verify "Standings" is highlighted with selection_fg color
    // Verify separators are present between tabs
    // Verify connector line aligns under tab text
}
```

## Benefits of TestBackend Testing

1. **Visual Regression Detection** - Catch layout/alignment issues
2. **Width Calculations** - Verify responsive layouts at different terminal sizes
3. **Color/Style Verification** - Ensure selection highlighting works
4. **Box Character Rendering** - Test Unicode vs ASCII mode
5. **No Manual Testing** - Automated verification of visual output

## Recommended Starting Points

The **tab bar** and **scoring summary tables** would give you the best ROI for testing effort, as they're core visual components used throughout the app.

### Tab Bar Test Example

```rust
// src/tui/common/tab_bar.rs

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::Arc;
    use crate::config::DisplayConfig;

    #[test]
    fn test_tab_bar_basic_rendering() {
        let mut terminal = Terminal::new(TestBackend::new(80, 2)).unwrap();
        let theme = Arc::new(DisplayConfig::default());

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 2);
            render(f, area, &["Scores", "Standings", "Settings"], 0, true, &theme);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        let first_line = buffer.content().iter()
            .take(80)
            .map(|c| c.symbol())
            .collect::<String>();

        // Verify tabs are present
        assert!(first_line.contains("Scores"));
        assert!(first_line.contains("Standings"));
        assert!(first_line.contains("Settings"));
    }

    #[test]
    fn test_tab_bar_focused_vs_unfocused() {
        let theme = Arc::new(DisplayConfig::default());

        // Test focused state
        let mut terminal_focused = Terminal::new(TestBackend::new(80, 2)).unwrap();
        terminal_focused.draw(|f| {
            let area = Rect::new(0, 0, 80, 2);
            render(f, area, &["Scores", "Standings"], 0, true, &theme);
        }).unwrap();

        // Test unfocused state
        let mut terminal_unfocused = Terminal::new(TestBackend::new(80, 2)).unwrap();
        terminal_unfocused.draw(|f| {
            let area = Rect::new(0, 0, 80, 2);
            render(f, area, &["Scores", "Standings"], 0, false, &theme);
        }).unwrap();

        // Colors should differ between focused/unfocused
        let focused_buffer = terminal_focused.backend().buffer();
        let unfocused_buffer = terminal_unfocused.backend().buffer();

        // Find "Scores" text and verify styling differs
        assert_ne!(
            focused_buffer.get(0, 0).fg,
            unfocused_buffer.get(0, 0).fg
        );
    }
}
```

### Scoring Summary Test Example

```rust
// src/tui/scores/view.rs - add to existing tests module

#[test]
fn test_scoring_summary_renders_correctly() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let goal = create_test_goal(
        "OTT",
        "M. Amadio",
        4,
        vec![("S. Pinto", 5)],
        1,
        0,
        "5:42",
        "Snap",
    );

    let bc = crate::formatting::BoxChars::ascii();
    let widths = ScoringColumnWidths {
        team: 5,
        description: 20,
        score: 5,
        time: 7,
        shot_type: 7,
    };

    let result = format_goal_table(&goal, &widths, &bc);

    // Verify it renders in a terminal
    let mut terminal = Terminal::new(TestBackend::new(60, 4)).unwrap();
    terminal.draw(|f| {
        let area = Rect::new(0, 0, 60, 4);
        let widget = Paragraph::new(result.clone());
        f.render_widget(widget, area);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = (0..4)
        .map(|y| {
            (0..60)
                .map(|x| buffer.get(x, y).symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Verify box chars present
    assert!(rendered.contains('+'));
    assert!(rendered.contains('-'));
    assert!(rendered.contains('|'));

    // Verify content
    assert!(rendered.contains("OTT"));
    assert!(rendered.contains("M. Amadio"));
    assert!(rendered.contains("5:42"));
    assert!(rendered.contains("Snap"));
}
```

## Testing Strategy

1. **Start Small**: Add tests for `tab_bar.rs` first
2. **Test Variations**: Different widths, focused/unfocused, selected indices
3. **Verify Layout**: Check character positions for alignment
4. **Color Verification**: Test selection highlighting
5. **Expand Coverage**: Move to scoring summary, then other components
6. **CI Integration**: Run tests on every commit to catch regressions
