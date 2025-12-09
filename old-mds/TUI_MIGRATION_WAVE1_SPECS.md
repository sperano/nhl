# TUI Migration - Wave 1 Detailed Specifications

## Overview

These are the 4 tasks that can be executed **in parallel** by Sonnet agents. Each specification is self-contained with all necessary context.

---

## Task 1.2a: Text Rendering Utilities

### Objective
Create buffer-based text rendering utilities that work directly with `ratatui::buffer::Buffer` for efficient text operations.

### Context Files to Read
1. `src/tui/widgets/mod.rs` - RenderableWidget trait
2. `src/tui/widgets/testing.rs` - Testing patterns and utilities
3. `src/formatting.rs` - BoxChars structure

### File to Create
`src/tui/widgets/buffer_utils.rs` (text rendering section)

### Required Functions

```rust
use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::{Line, Span}};

/// Render a single line of text to the buffer at the specified position
///
/// # Arguments
/// * `buf` - The buffer to render to
/// * `x` - Starting x position
/// * `y` - Y position
/// * `text` - The text to render
/// * `style` - Style to apply
/// * `max_width` - Optional maximum width (truncates if exceeded)
///
/// # Example
/// ```rust
/// render_text(buf, 5, 10, "Hello World", Style::default(), Some(10));
/// // Renders "Hello Wor" at position (5, 10)
/// ```
pub fn render_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
    max_width: Option<u16>,
) {
    // Implementation
}

/// Render centered text within a given width
///
/// # Example
/// ```rust
/// render_centered_text(buf, area, "Title", Style::default().bold());
/// // Centers "Title" within the area
/// ```
pub fn render_centered_text(
    buf: &mut Buffer,
    area: Rect,
    text: &str,
    style: Style,
) {
    // Implementation
}

/// Render right-aligned text
///
/// # Example
/// ```rust
/// render_right_aligned_text(buf, area, "Score: 3-2", Style::default());
/// // Right-aligns text within the area
/// ```
pub fn render_right_aligned_text(
    buf: &mut Buffer,
    area: Rect,
    text: &str,
    style: Style,
) {
    // Implementation
}

/// Render text with padding
///
/// # Arguments
/// * `padding` - (left, right, top, bottom) padding
///
/// # Example
/// ```rust
/// render_padded_text(buf, area, "Content", Style::default(), (2, 2, 1, 1));
/// ```
pub fn render_padded_text(
    buf: &mut Buffer,
    area: Rect,
    text: &str,
    style: Style,
    padding: (u16, u16, u16, u16),
) {
    // Implementation
}

/// Fill an area with a repeating character
///
/// # Example
/// ```rust
/// fill_area(buf, area, '─', Style::default());
/// // Fills area with horizontal lines
/// ```
pub fn fill_area(
    buf: &mut Buffer,
    area: Rect,
    ch: char,
    style: Style,
) {
    // Implementation
}

/// Clear an area (fill with spaces)
pub fn clear_area(buf: &mut Buffer, area: Rect) {
    fill_area(buf, area, ' ', Style::default());
}
```

### Test Requirements

Create comprehensive tests in the same file:

```rust
#[cfg(test)]
mod text_tests {
    use super::*;
    use crate::tui::widgets::testing::*;

    #[test]
    fn test_render_text_basic() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        render_text(&mut buf, 0, 0, "Hello", Style::default(), None);
        assert_eq!(buffer_line(&buf, 0), "Hello               ");
    }

    #[test]
    fn test_render_text_truncation() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        render_text(&mut buf, 0, 0, "Hello World", Style::default(), Some(5));
        assert_eq!(buffer_line(&buf, 0), "Hello               ");
    }

    #[test]
    fn test_render_centered_text() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        render_centered_text(&mut buf, buf.area, "Hi", Style::default());
        assert_eq!(buffer_line(&buf, 0), "    Hi    ");
    }

    #[test]
    fn test_render_right_aligned() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        render_right_aligned_text(&mut buf, buf.area, "End", Style::default());
        assert_eq!(buffer_line(&buf, 0), "       End");
    }

    #[test]
    fn test_fill_area() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));
        fill_area(&mut buf, buf.area, '*', Style::default());
        assert_eq!(buffer_line(&buf, 0), "*****");
        assert_eq!(buffer_line(&buf, 1), "*****");
    }

    #[test]
    fn test_padded_text() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        render_padded_text(&mut buf, buf.area, "X", Style::default(), (2, 2, 1, 1));
        assert_eq!(buffer_line(&buf, 0), "          ");
        assert_eq!(buffer_line(&buf, 1), "  X       ");
        assert_eq!(buffer_line(&buf, 2), "          ");
    }
}
```

### Implementation Notes
- Use `buf.set_string()` for basic text rendering
- Handle Unicode correctly (use `unicode_width` crate if needed)
- Respect area boundaries - never write outside the given Rect
- Truncate text that exceeds max_width with proper Unicode handling
- Preserve existing buffer content outside the target area

---

## Task 1.2b: Border/Box Drawing Utilities

### Objective
Create utilities for drawing boxes and borders using the box drawing characters from DisplayConfig.

### Context Files to Read
1. `src/tui/widgets/mod.rs` - RenderableWidget trait
2. `src/tui/widgets/testing.rs` - Testing patterns
3. `src/formatting.rs` - BoxChars structure (IMPORTANT: Study this carefully)
4. `src/config.rs` - DisplayConfig structure

### File to Create
`src/tui/widgets/buffer_utils.rs` (border drawing section - append to existing file if 1.2a is done)

### Required Functions

```rust
use crate::formatting::BoxChars;
use crate::config::DisplayConfig;

/// Draw a simple box border around an area
///
/// # Example
/// ```rust
/// draw_box(buf, area, &config.box_chars, Style::default());
/// // Draws: ╭───╮
/// //        │   │
/// //        ╰───╯
/// ```
pub fn draw_box(
    buf: &mut Buffer,
    area: Rect,
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation
}

/// Draw a box with a title
///
/// # Example
/// ```rust
/// draw_titled_box(buf, area, "Settings", &config.box_chars, Style::default());
/// // Draws: ╭─Settings─╮
/// //        │          │
/// //        ╰──────────╯
/// ```
pub fn draw_titled_box(
    buf: &mut Buffer,
    area: Rect,
    title: &str,
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation
}

/// Draw a horizontal line
///
/// # Example
/// ```rust
/// draw_horizontal_line(buf, 2, 10, 5, &box_chars, Style::default());
/// // Draws 5 characters starting at (2, 10): ─────
/// ```
pub fn draw_horizontal_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation using box_chars.horizontal
}

/// Draw a vertical line
pub fn draw_vertical_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    height: u16,
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation using box_chars.vertical
}

/// Draw a double horizontal line (for emphasis)
pub fn draw_double_horizontal_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation using box_chars.double_horizontal
}

/// Draw corner characters
pub fn draw_corner(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    corner_type: CornerType,
    box_chars: &BoxChars,
    style: Style,
) {
    let ch = match corner_type {
        CornerType::TopLeft => &box_chars.top_left,
        CornerType::TopRight => &box_chars.top_right,
        CornerType::BottomLeft => &box_chars.bottom_left,
        CornerType::BottomRight => &box_chars.bottom_right,
    };
    buf.set_string(x, y, ch, style);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerType {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Draw a junction (where lines meet)
pub fn draw_junction(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    junction_type: JunctionType,
    box_chars: &BoxChars,
    style: Style,
) {
    let ch = match junction_type {
        JunctionType::Top => &box_chars.top_junction,
        JunctionType::Bottom => &box_chars.bottom_junction,
        JunctionType::Left => &box_chars.left_junction,
        JunctionType::Right => &box_chars.right_junction,
        JunctionType::Cross => &box_chars.cross,
    };
    buf.set_string(x, y, ch, style);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JunctionType {
    Top,    // ┬
    Bottom, // ┴
    Left,   // ├
    Right,  // ┤
    Cross,  // ┼
}
```

### Test Requirements

```rust
#[cfg(test)]
mod border_tests {
    use super::*;
    use crate::tui::widgets::testing::*;

    #[test]
    fn test_draw_box_unicode() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 3));
        let box_chars = BoxChars::unicode();
        draw_box(&mut buf, buf.area, &box_chars, Style::default());

        assert_eq!(buffer_line(&buf, 0), "╭───╮");
        assert_eq!(buffer_line(&buf, 1), "│   │");
        assert_eq!(buffer_line(&buf, 2), "╰───╯");
    }

    #[test]
    fn test_draw_box_ascii() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 3));
        let box_chars = BoxChars::ascii();
        draw_box(&mut buf, buf.area, &box_chars, Style::default());

        assert_eq!(buffer_line(&buf, 0), "+---+");
        assert_eq!(buffer_line(&buf, 1), "|   |");
        assert_eq!(buffer_line(&buf, 2), "+---+");
    }

    #[test]
    fn test_draw_titled_box() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        let box_chars = BoxChars::unicode();
        draw_titled_box(&mut buf, buf.area, "Test", &box_chars, Style::default());

        assert_eq!(buffer_line(&buf, 0), "╭─Test───╮");
        assert_eq!(buffer_line(&buf, 1), "│        │");
        assert_eq!(buffer_line(&buf, 2), "╰────────╯");
    }

    #[test]
    fn test_lines_and_junctions() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 5));
        let box_chars = BoxChars::unicode();

        // Draw a cross pattern
        draw_horizontal_line(&mut buf, 0, 2, 5, &box_chars, Style::default());
        draw_vertical_line(&mut buf, 2, 0, 5, &box_chars, Style::default());
        draw_junction(&mut buf, 2, 2, JunctionType::Cross, &box_chars, Style::default());

        assert_eq!(buffer_line(&buf, 2), "──┼──");
        // Check vertical line exists at x=2 for all rows
    }
}
```

### Implementation Notes
- BoxChars provides all the characters you need
- Support both ASCII and Unicode modes (test both)
- Ensure boxes fit within the given area
- For titled boxes, center the title in the top border
- Handle edge cases like area too small for box

---

## Task 1.2c: Table/Grid Layout Utilities

### Objective
Create utilities for table and grid layouts, including column management and dividers.

### Context Files to Read
1. `src/tui/widgets/mod.rs` - RenderableWidget trait
2. `src/tui/widgets/testing.rs` - Testing patterns
3. `src/formatting.rs` - BoxChars structure
4. Look at `src/tui/standings/view.rs` for table layout patterns

### File to Create
`src/tui/widgets/buffer_utils.rs` (table/grid section - append to existing file)

### Required Functions

```rust
/// Column definition for tables
#[derive(Debug, Clone)]
pub struct Column {
    pub header: String,
    pub width: u16,
    pub alignment: Alignment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// Draw a table header row
///
/// # Example
/// ```rust
/// let columns = vec![
///     Column { header: "Team".into(), width: 10, alignment: Alignment::Left },
///     Column { header: "W".into(), width: 3, alignment: Alignment::Right },
///     Column { header: "L".into(), width: 3, alignment: Alignment::Right },
/// ];
/// draw_table_header(buf, area, &columns, &box_chars, Style::default().bold());
/// // Draws: Team       W  L
/// //        ─────────────────
/// ```
pub fn draw_table_header(
    buf: &mut Buffer,
    area: Rect,
    columns: &[Column],
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation
}

/// Draw a table row
///
/// # Example
/// ```rust
/// let values = vec!["Maple Leafs", "28", "16"];
/// draw_table_row(buf, area, &columns, &values, y, Style::default());
/// ```
pub fn draw_table_row(
    buf: &mut Buffer,
    area: Rect,
    columns: &[Column],
    values: &[&str],
    y: u16,
    style: Style,
) {
    // Implementation
}

/// Draw a divider line between sections
///
/// # Example
/// ```rust
/// draw_divider(buf, area, 5, DividerStyle::Single, &box_chars, Style::default());
/// ```
pub fn draw_divider(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    divider_style: DividerStyle,
    box_chars: &BoxChars,
    style: Style,
) {
    // Implementation
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DividerStyle {
    Single,  // ─────
    Double,  // ═════
    Dashed,  // ╌╌╌╌╌
}

/// Calculate column positions for a grid layout
///
/// # Example
/// ```rust
/// let positions = calculate_grid_columns(80, 3, 2);
/// // Returns x positions for 3 columns with 2 char spacing in 80 char width
/// // e.g., [0, 27, 54]
/// ```
pub fn calculate_grid_columns(
    total_width: u16,
    num_columns: u16,
    spacing: u16,
) -> Vec<u16> {
    // Implementation
}

/// Draw a grid cell (useful for game boxes)
///
/// # Example
/// ```rust
/// draw_grid_cell(buf, Rect::new(x, y, 20, 5), "Content", &box_chars, Style::default());
/// ```
pub fn draw_grid_cell(
    buf: &mut Buffer,
    area: Rect,
    content: &str,
    box_chars: &BoxChars,
    style: Style,
) {
    draw_box(buf, area, box_chars, style);
    // Center content inside
}

/// Create a layout for multiple columns with equal widths
pub fn create_column_layout(
    area: Rect,
    num_columns: usize,
    spacing: u16,
) -> Vec<Rect> {
    // Implementation - return vec of Rect for each column
}
```

### Test Requirements

```rust
#[cfg(test)]
mod table_tests {
    use super::*;
    use crate::tui::widgets::testing::*;

    #[test]
    fn test_table_header() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 2));
        let columns = vec![
            Column { header: "Name".into(), width: 10, alignment: Alignment::Left },
            Column { header: "Score".into(), width: 5, alignment: Alignment::Right },
        ];
        let box_chars = BoxChars::unicode();

        draw_table_header(&mut buf, buf.area, &columns, &box_chars, Style::default());

        assert_eq!(buffer_line(&buf, 0), "Name      Score     ");
        assert_eq!(buffer_line(&buf, 1), "────────────────────");
    }

    #[test]
    fn test_table_row_alignment() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let columns = vec![
            Column { header: "Left".into(), width: 8, alignment: Alignment::Left },
            Column { header: "Center".into(), width: 6, alignment: Alignment::Center },
            Column { header: "Right".into(), width: 6, alignment: Alignment::Right },
        ];

        let values = vec!["AAA", "BB", "C"];
        draw_table_row(&mut buf, buf.area, &columns, &values, 0, Style::default());

        // Check that alignment is correct
        let line = buffer_line(&buf, 0);
        assert!(line.starts_with("AAA     ")); // Left aligned
        assert!(line.contains("  BB  "));      // Centered
        assert!(line.ends_with("     C"));     // Right aligned
    }

    #[test]
    fn test_grid_columns_calculation() {
        let positions = calculate_grid_columns(80, 3, 2);
        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0], 0);
        // Check reasonable spacing
        assert!(positions[1] > 20);
        assert!(positions[2] > 50);
    }

    #[test]
    fn test_column_layout() {
        let area = Rect::new(0, 0, 30, 10);
        let layouts = create_column_layout(area, 3, 1);

        assert_eq!(layouts.len(), 3);
        assert_eq!(layouts[0].width, 9); // (30 - 2 spaces) / 3
        assert_eq!(layouts[1].x, 10);    // First column width + spacing
        assert_eq!(layouts[2].x, 20);    // Two columns + spacing
    }

    #[test]
    fn test_divider_styles() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let box_chars = BoxChars::unicode();

        draw_divider(&mut buf, buf.area, 0, DividerStyle::Single, &box_chars, Style::default());
        assert_eq!(buffer_line(&buf, 0), "──────────");
    }
}
```

### Implementation Notes
- Column alignment is critical - test all three types
- Grid calculations must handle terminal resize gracefully
- Support variable column widths
- Dividers should span the full width of the area
- Consider spacing between columns in calculations

---

## Task 4.1: TeamRow Widget

### Objective
Create a TeamRow widget that extracts the existing team row rendering logic into a reusable widget.

### Context Files to Read
1. `src/tui/widgets/mod.rs` - RenderableWidget trait
2. `src/tui/widgets/testing.rs` - Testing patterns
3. `src/tui/standings/view.rs` - Look for `render_team_row` function (line ~283)
4. `src/config.rs` - DisplayConfig
5. Review the NHL API Standing struct in the nhl_api crate

### File to Create
`src/tui/widgets/team_row.rs`

### Current Implementation to Extract

From `src/tui/standings/view.rs`:
```rust
fn render_team_row(team: &nhl_api::Standing, is_selected: bool, selection_fg: Color, margin: usize) -> Line<'static> {
    let team_name = &team.team_common_name.default;

    // Format team name and stats
    let team_part = format!("{:<width$}", team_name, width = TEAM_NAME_COL_WIDTH);
    let stats_part = format!(
        " {:>gp_width$} {:>w_width$} {:>l_width$} {:>ot_width$} {:>pts_width$}",
        team.games_played,
        team.wins,
        team.losses,
        team.ot_losses.unwrap_or(0) + team.ties.unwrap_or(0),
        team.points,
        // ... width constants
    );

    // Apply selection highlighting
    // Return Line with spans
}
```

### Widget Implementation

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Modifier},
};
use crate::config::DisplayConfig;
use crate::tui::widgets::RenderableWidget;

/// A widget that renders a single team row in the standings table
#[derive(Debug, Clone)]
pub struct TeamRow {
    /// Team name to display
    pub team_name: String,
    /// Games played
    pub games_played: i32,
    /// Wins
    pub wins: i32,
    /// Losses
    pub losses: i32,
    /// Overtime losses (includes ties)
    pub ot_losses: i32,
    /// Points
    pub points: i32,
    /// Whether this row is selected
    pub is_selected: bool,
    /// Left margin in characters
    pub margin: u16,
}

impl TeamRow {
    /// Create from an NHL API Standing
    pub fn from_standing(team: &nhl_api::Standing, is_selected: bool, margin: u16) -> Self {
        Self {
            team_name: team.team_common_name.default.clone(),
            games_played: team.games_played,
            wins: team.wins,
            losses: team.losses,
            ot_losses: team.ot_losses.unwrap_or(0) + team.ties.unwrap_or(0),
            points: team.points,
            is_selected,
            margin,
        }
    }
}

impl RenderableWidget for TeamRow {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Constants (match existing code)
        const TEAM_NAME_COL_WIDTH: usize = 13;
        const STATS_COL_WIDTH: usize = 3;

        // Build the row text
        let mut x = area.x + self.margin;
        let y = area.y;

        // Determine style based on selection
        let style = if self.is_selected {
            Style::default().fg(config.selection_fg).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Render team name (left aligned)
        let team_text = format!("{:<width$}", self.team_name, width = TEAM_NAME_COL_WIDTH);
        buf.set_string(x, y, &team_text, style);
        x += TEAM_NAME_COL_WIDTH as u16;

        // Render stats (right aligned)
        buf.set_string(x, y, " ", style);
        x += 1;

        // GP
        let gp_text = format!("{:>3}", self.games_played);
        buf.set_string(x, y, &gp_text, style);
        x += 4;

        // W
        let w_text = format!("{:>3}", self.wins);
        buf.set_string(x, y, &w_text, style);
        x += 4;

        // L
        let l_text = format!("{:>3}", self.losses);
        buf.set_string(x, y, &l_text, style);
        x += 4;

        // OT
        let ot_text = format!("{:>3}", self.ot_losses);
        buf.set_string(x, y, &ot_text, style);
        x += 4;

        // PTS
        let pts_text = format!("{:>3}", self.points);
        buf.set_string(x, y, &pts_text, style);
    }

    fn preferred_height(&self) -> Option<u16> {
        Some(1) // Team row is always 1 line tall
    }

    fn preferred_width(&self) -> Option<u16> {
        Some(self.margin + 13 + 1 + (4 * 5)) // margin + team + space + stats
    }
}
```

### Test Requirements

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::widgets::testing::*;
    use ratatui::style::Color;

    #[test]
    fn test_team_row_basic() {
        let row = TeamRow {
            team_name: "Maple Leafs".to_string(),
            games_played: 44,
            wins: 28,
            losses: 16,
            ot_losses: 0,
            points: 56,
            is_selected: false,
            margin: 2,
        };

        let buf = render_widget(&row, 40, 1);
        let line = buffer_line(&buf, 0);

        // Check formatting
        assert!(line.contains("Maple Leafs"));
        assert!(line.contains(" 44"));
        assert!(line.contains(" 28"));
        assert!(line.contains(" 16"));
        assert!(line.contains("  0"));
        assert!(line.contains(" 56"));
    }

    #[test]
    fn test_team_row_selection() {
        let config = test_config();
        let row = TeamRow {
            team_name: "Oilers".to_string(),
            games_played: 43,
            wins: 29,
            losses: 13,
            ot_losses: 1,
            points: 59,
            is_selected: true,
            margin: 0,
        };

        let buf = render_widget_with_config(&row, 40, 1, &config);

        // Check that selection color is applied
        let cell = &buf[(0, 0)];
        assert_eq!(cell.fg, config.selection_fg);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_team_row_from_standing() {
        // Mock Standing data
        let standing = nhl_api::Standing {
            team_common_name: nhl_api::TeamCommonName {
                default: "Bruins".to_string(),
            },
            games_played: 45,
            wins: 27,
            losses: 15,
            ot_losses: Some(3),
            ties: None,
            points: 57,
            // ... other fields with defaults
            ..Default::default()
        };

        let row = TeamRow::from_standing(&standing, false, 4);

        assert_eq!(row.team_name, "Bruins");
        assert_eq!(row.games_played, 45);
        assert_eq!(row.wins, 27);
        assert_eq!(row.losses, 15);
        assert_eq!(row.ot_losses, 3);
        assert_eq!(row.points, 57);
        assert_eq!(row.margin, 4);
    }

    #[test]
    fn test_team_row_long_name_truncation() {
        let row = TeamRow {
            team_name: "Really Long Team Name".to_string(),
            games_played: 10,
            wins: 5,
            losses: 5,
            ot_losses: 0,
            points: 10,
            is_selected: false,
            margin: 0,
        };

        let buf = render_widget(&row, 40, 1);
        let line = buffer_line(&buf, 0);

        // Name should be truncated to fit in 13 chars
        assert!(line.starts_with("Really Long T"));
    }

    #[test]
    fn test_preferred_dimensions() {
        let row = TeamRow {
            team_name: "Test".to_string(),
            games_played: 0,
            wins: 0,
            losses: 0,
            ot_losses: 0,
            points: 0,
            is_selected: false,
            margin: 2,
        };

        assert_eq!(row.preferred_height(), Some(1));
        assert_eq!(row.preferred_width(), Some(2 + 13 + 1 + 20)); // margin + team + space + stats
    }
}
```

### Update widgets/mod.rs

Add to `src/tui/widgets/mod.rs`:
```rust
pub mod team_row;
pub use team_row::TeamRow;
```

### Implementation Notes
- Match the exact spacing and alignment from the original
- The team name is 13 characters wide, left-aligned
- All stats columns are 3 characters wide, right-aligned
- Support selection highlighting with bold modifier
- Handle long team names by truncating
- The margin parameter adds space to the left of the entire row

---

## Execution Instructions for Agents

### For Each Agent:

1. **Start by reading the context files** listed in your task specification
2. **Create your implementation file** with all required functions
3. **Write comprehensive tests** achieving 90%+ coverage
4. **Run tests locally**:
   ```bash
   cargo test --bin nhl [your_module]::tests
   ```
5. **Run clippy** to ensure code quality:
   ```bash
   cargo clippy
   ```
6. **Document all public functions** with examples

### Success Criteria:
- ✅ All tests pass
- ✅ No clippy warnings
- ✅ 90%+ test coverage
- ✅ Functions work with both ASCII and Unicode (where applicable)
- ✅ Respects buffer boundaries (never writes outside given Rect)
- ✅ Follows existing code patterns

### Coordination Notes:
- Tasks 1.2a, 1.2b, and 1.2c will all contribute to the same file (`buffer_utils.rs`)
- Each section should be clearly commented and independent
- Task 4.1 (TeamRow) is completely independent and can be done in parallel

---

## Questions for Agents to Consider:

1. **For text utilities**: How should we handle Unicode width? Should we use the `unicode-width` crate?
2. **For border utilities**: Should titled boxes truncate long titles or expand the box?
3. **For table utilities**: How should we handle columns that don't fit in the available width?
4. **For TeamRow**: Should we expose column width constants as configurable parameters?

Document your decisions in comments for review.