//! Generic Table component for displaying data with mixed cell types
//!
//! This component provides a reusable table that supports:
//! - Mixed cell types (Text, PlayerLink, TeamLink)
//! - Column-based layout with customizable alignment
//! - Selection highlighting (focused and unfocused states)
//! - Keyboard navigation (via parent component actions)
//!
//! # Architecture
//!
//! The table follows the current (React-like) framework pattern:
//! - **TableWidget**: Implements `StandaloneWidget` for actual rendering
//! - **CellValue**: Type-safe enum for Text, PlayerLink, or TeamLink
//! - **ColumnDef**: Defines column header, width, alignment, and cell extraction
//! - **Navigation helpers**: Methods to find next/previous link columns
//!
//! # State Management
//!
//! Following the Redux pattern, table selection state lives in AppState (not in the widget):
//! - Parent component stores `selected_row`, `selected_col` in its UiState
//! - Parent component dispatches actions on arrow key presses
//! - Reducer updates selection state
//! - Table widget receives new props and re-renders
//!
//! # Navigation Pattern
//!
//! Left/Right arrow keys should navigate only between link columns, skipping Text columns:
//!
//! ```ignore
//! // In your tab's key handler:
//! KeyCode::Right => {
//!     if let Some(new_col) = table.find_next_link_column(current_col) {
//!         // Dispatch action to update selected_col to new_col
//!         Action::TableAction(TableAction::SelectCell { row: current_row, col: new_col })
//!     }
//! }
//! ```
//!
//! # Usage Example - Player Statistics Table
//!
//! ```ignore
//! use nhl::tui::components::TableWidget;
//! use nhl::tui::{CellValue, ColumnDef, Alignment, Element};
//! use nhl_api::PlayerStats;
//!
//! // 1. Define your row data type (or use existing nhl_api types)
//! struct PlayerRow {
//!     name: String,
//!     id: i64,
//!     games: i32,
//!     goals: i32,
//!     assists: i32,
//! }
//!
//! // 2. Create column definitions
//! let columns = vec![
//!     ColumnDef::new("Player", 25, Alignment::Left, |p: &PlayerRow| {
//!         CellValue::PlayerLink {
//!             display: p.name.clone(),
//!             player_id: p.id,
//!         }
//!     }),
//!     ColumnDef::new("GP", 4, Alignment::Right, |p: &PlayerRow| {
//!         CellValue::Text(p.games.to_string())
//!     }),
//!     ColumnDef::new("G", 4, Alignment::Right, |p: &PlayerRow| {
//!         CellValue::Text(p.goals.to_string())
//!     }),
//!     ColumnDef::new("A", 4, Alignment::Right, |p: &PlayerRow| {
//!         CellValue::Text(p.assists.to_string())
//!     }),
//!     ColumnDef::new("PTS", 5, Alignment::Right, |p: &PlayerRow| {
//!         CellValue::Text((p.goals + p.assists).to_string())
//!     }),
//! ];
//!
//! // 3. Get row data from props
//! let rows: Vec<PlayerRow> = props.player_stats.clone();
//!
//! // 4. Create table widget
//! let table = TableWidget::from_data(&columns, rows)
//!     .with_selection(props.selected_row.unwrap_or(0), props.selected_col.unwrap_or(0))
//!     .with_focused(props.table_focused)
//!     ;
//!
//! // 5. Wrap in Element::Widget for component tree
//! Element::Widget(Box::new(table))
//! ```
//!
//! # Usage Example - Standings Table with Team Links
//!
//! ```ignore
//! let columns = vec![
//!     ColumnDef::new("Team", 25, Alignment::Left, |s: &Standing| {
//!         CellValue::TeamLink {
//!             display: s.team_common_name.default.clone(),
//!             team_abbrev: s.team_abbrev.default.clone(),
//!             season: None,
//!         }
//!     }),
//!     ColumnDef::new("GP", 4, Alignment::Right, |s: &Standing| {
//!         CellValue::Text((s.wins + s.losses + s.ot_losses).to_string())
//!     }),
//!     ColumnDef::new("W", 4, Alignment::Right, |s: &Standing| {
//!         CellValue::Text(s.wins.to_string())
//!     }),
//!     ColumnDef::new("L", 4, Alignment::Right, |s: &Standing| {
//!         CellValue::Text(s.losses.to_string())
//!     }),
//!     ColumnDef::new("PTS", 5, Alignment::Right, |s: &Standing| {
//!         CellValue::Text(s.points.to_string())
//!     }),
//! ];
//!
//! let table = TableWidget::from_data(&columns, standings)
//!     .with_selection(selected_row, selected_col)
//!     .with_focused(focused)
//!     ;
//! ```
//!
//! # Link Activation
//!
//! When Enter is pressed on a link cell, the parent component should:
//!
//! 1. Get the cell value using `table.get_cell_value(row, col)`
//! 2. Check if it's a link using `cell_value.is_link()`
//! 3. Log the link info using `cell_value.link_info()` (for now)
//! 4. Later: Dispatch NavigationAction to navigate to player/team detail
//!
//! ```ignore
//! KeyCode::Enter => {
//!     if let Some(cell) = table.get_cell_value(row, col) {
//!         if cell.is_link() {
//!             println!("Link activated: {}", cell.link_info());
//!             // Future: dispatch Action::Navigate(...)
//!         }
//!     }
//! }
//! ```
//!
//! # Visual States
//!
//! - **Focused selection**: Uses `config.selection_fg` (bright color)
//! - **Unfocused selection**: Uses `config.unfocused_selection_fg()` (dim color)
//! - **Unselected cells**: No special styling (Text and Link look identical)
//! - **Column headers**: Bold + underlined
//!
//! # Navigation Helpers
//!
//! The TableWidget provides helper methods for navigation:
//!
//! - `find_next_link_column(current_col)` - Find next focusable column (skips Text)
//! - `find_prev_link_column(current_col)` - Find previous focusable column
//! - `find_first_link_column()` - Find first focusable column
//! - `get_cell_value(row, col)` - Get CellValue at position
//! - `row_count()` / `column_count()` - Get table dimensions

mod rendering;

use crate::config::RenderContext;
use crate::tui::component::ElementWidget;
use crate::tui::{Alignment, CellValue, ColumnDef, Component, Element};
use ratatui::{buffer::Buffer, layout::Rect};

/// Table component
///
/// Renders a table with mixed cell types (Text and Links).
/// Focus/selection state is provided externally at render time.
pub struct Table;

impl Component for Table {
    type Props = ();
    type State = ();
    type Message = ();

    fn view(&self, _props: &Self::Props, _state: &Self::State) -> Element {
        // This is a marker component - actual usage is via direct TableWidget rendering
        Element::None
    }
}

/// Width of the selector indicator space (e.g., "▶ " or "  ")
const SELECTOR_WIDTH: usize = 2;

/// The actual table widget that implements rendering
///
/// This widget is created directly by parent components that want to render a table.
/// Focus is provided at construction time via `with_focused_row()`.
///
/// Cell data is extracted upfront when creating the widget, making it cloneable.
#[derive(Clone)]
pub struct TableWidget {
    pub(super) column_headers: Vec<String>,
    pub(super) column_widths: Vec<usize>,
    pub(super) column_aligns: Vec<Alignment>,
    pub(super) cell_data: Vec<Vec<CellValue>>,
    /// Which row is focused (externally managed)
    pub(super) focused_row: Option<usize>,
}

impl TableWidget {
    /// Create a table widget with builder pattern
    /// Extracts all cell data upfront from the rows using column definitions
    pub fn from_data<T: Send + Sync>(columns: &[ColumnDef<T>], rows: Vec<T>) -> Self {
        // Extract cell data upfront
        let cell_data: Vec<Vec<CellValue>> = rows
            .iter()
            .map(|row| columns.iter().map(|col| (col.cell_fn)(row)).collect())
            .collect();

        // Extract column metadata
        let column_headers = columns.iter().map(|c| c.header.clone()).collect();
        let column_widths = columns.iter().map(|c| c.width).collect();
        let column_aligns = columns.iter().map(|c| c.align).collect();

        Self {
            column_headers,
            column_widths,
            column_aligns,
            cell_data,
            focused_row: None,
        }
    }

    /// Set which row is focused (externally managed)
    pub fn with_focused_row(mut self, row: Option<usize>) -> Self {
        self.focused_row = row;
        self
    }

    /// Format a cell with alignment
    pub(super) fn format_cell(&self, text: &str, width: usize, align: Alignment) -> String {
        let text_len = text.chars().count(); // Unicode-aware length
        if text_len > width {
            // Truncate
            if width > 3 {
                let truncated: String = text.chars().take(width - 3).collect();
                format!("{}...", truncated)
            } else {
                text.chars().take(width).collect()
            }
        } else if text_len == width {
            // Exact fit - no padding needed
            text.to_string()
        } else {
            // Pad
            match align {
                Alignment::Left => format!("{:<width$}", text, width = width),
                Alignment::Right => format!("{:>width$}", text, width = width),
                Alignment::Center => {
                    let left_pad = (width - text_len) / 2;
                    let right_pad = width - text_len - left_pad;
                    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
                }
            }
        }
    }

    /// Find the next link column after the given column index
    ///
    /// Returns None if there are no link columns after this one.
    /// A link column is one where at least one cell in the column is a link.
    pub fn find_next_link_column(&self, current_col: usize) -> Option<usize> {
        for col_idx in (current_col + 1)..self.column_headers.len() {
            // Check if any cell in this column is a link
            let has_link = self
                .cell_data
                .iter()
                .any(|row| row.get(col_idx).map(|cell| cell.is_link()).unwrap_or(false));

            if has_link {
                return Some(col_idx);
            }
        }
        None
    }

    /// Find the previous link column before the given column index
    ///
    /// Returns None if there are no link columns before this one.
    pub fn find_prev_link_column(&self, current_col: usize) -> Option<usize> {
        if current_col == 0 {
            return None;
        }

        for col_idx in (0..current_col).rev() {
            // Check if any cell in this column is a link
            let has_link = self
                .cell_data
                .iter()
                .any(|row| row.get(col_idx).map(|cell| cell.is_link()).unwrap_or(false));

            if has_link {
                return Some(col_idx);
            }
        }
        None
    }

    /// Find the first link column in the table
    ///
    /// Returns None if there are no link columns.
    pub fn find_first_link_column(&self) -> Option<usize> {
        for col_idx in 0..self.column_headers.len() {
            let has_link = self
                .cell_data
                .iter()
                .any(|row| row.get(col_idx).map(|cell| cell.is_link()).unwrap_or(false));

            if has_link {
                return Some(col_idx);
            }
        }
        None
    }

    /// Get the cell value at the given row and column
    ///
    /// Returns None if the row or column is out of bounds.
    pub fn get_cell_value(&self, row: usize, col: usize) -> Option<CellValue> {
        self.cell_data.get(row)?.get(col).cloned()
    }

    /// Get the number of rows in the table
    pub fn row_count(&self) -> usize {
        self.cell_data.len()
    }

    /// Get the number of columns in the table
    pub fn column_count(&self) -> usize {
        self.column_headers.len()
    }
}

impl ElementWidget for TableWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        self.render_internal(area, buf, ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }

    fn preferred_height(&self) -> Option<u16> {
        let col_header_height = if !self.column_headers.is_empty() {
            1
        } else {
            0
        };
        let separator_height = if !self.column_headers.is_empty() {
            1
        } else {
            0
        };
        let rows_height = self.cell_data.len() as u16;
        Some(col_header_height + separator_height + rows_height)
    }

    fn preferred_width(&self) -> Option<u16> {
        if self.column_widths.is_empty() {
            return Some(0);
        }

        let cols_width: usize = self.column_widths.iter().sum();
        let spacing = (self.column_widths.len() - 1) * 2;
        Some((SELECTOR_WIDTH + cols_width + spacing) as u16)
    }
}

#[cfg(test)]
mod tests;
