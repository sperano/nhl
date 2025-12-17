/// Generic table framework types
///
/// This module provides core types for building tables with mixed cell types:
/// - CellValue: Text or clickable links (Player, Team)
/// - ColumnDef: Column definition with cell extraction function
/// - Alignment: Text alignment for cells
/// - TableProps: Props for Table component
use std::fmt;

/// Value types that can appear in table cells
///
/// Cells can contain plain text or clickable links to players/teams.
/// Links are focusable and can be activated with Enter key.
#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    /// Plain text cell (not focusable, no selection styling)
    Text(String),

    /// Styled text cell (not focusable, but receives selection styling when row is focused)
    StyledText(String),

    /// Link to player profile (focusable)
    PlayerLink { display: String, player_id: i64 },

    /// Link to team page (focusable)
    TeamLink {
        display: String,
        team_abbrev: String,
    },
}

impl CellValue {
    /// Returns true if this cell is a link (focusable)
    pub fn is_link(&self) -> bool {
        matches!(self, Self::PlayerLink { .. } | Self::TeamLink { .. })
    }

    /// Returns true if this cell receives selection styling when row is focused
    pub fn receives_selection_style(&self) -> bool {
        matches!(
            self,
            Self::StyledText(_) | Self::PlayerLink { .. } | Self::TeamLink { .. }
        )
    }

    /// Get the display text for this cell
    pub fn display_text(&self) -> &str {
        match self {
            Self::Text(s) | Self::StyledText(s) => s,
            Self::PlayerLink { display, .. } => display,
            Self::TeamLink { display, .. } => display,
        }
    }

    /// Get debug info for link activation logging
    pub fn link_info(&self) -> String {
        match self {
            Self::Text(_) | Self::StyledText(_) => "Not a link".to_string(),
            Self::PlayerLink { display, player_id } => {
                format!("PlayerLink(display='{}', id={})", display, player_id)
            }
            Self::TeamLink {
                display,
                team_abbrev,
            } => {
                format!("TeamLink(display='{}', abbrev='{}')", display, team_abbrev)
            }
        }
    }
}

/// Text alignment for table cells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Right,
    Center,
}

/// Column definition for a table
///
/// Defines how to extract and display a column's data from row items.
///
/// # Type Parameters
/// - `T`: The row data type
///
/// # Example
/// ```ignore
/// use nhl::tui::table::{ColumnDef, CellValue, Alignment};
///
/// struct Player {
///     name: String,
///     id: i64,
///     games: i32,
/// }
///
/// let name_col = ColumnDef::new(
///     "Player",
///     25,
///     Alignment::Left,
///     |p: &Player| CellValue::PlayerLink {
///         display: p.name.clone(),
///         player_id: p.id,
///     }
/// );
///
/// let games_col = ColumnDef::new(
///     "GP",
///     4,
///     Alignment::Right,
///     |p: &Player| CellValue::Text(p.games.to_string())
/// );
/// ```
pub struct ColumnDef<T> {
    /// Column header text
    pub header: String,

    /// Column width in characters
    pub width: usize,

    /// Text alignment
    pub align: Alignment,

    /// Function to extract cell value from row data
    pub cell_fn: Box<dyn Fn(&T) -> CellValue + Send + Sync>,
}

impl<T> ColumnDef<T> {
    /// Create a new column definition
    ///
    /// # Arguments
    /// - `header`: Column header text
    /// - `width`: Column width in characters
    /// - `align`: Text alignment (Left, Right, Center)
    /// - `cell_fn`: Function to extract CellValue from row data
    pub fn new<F>(header: impl Into<String>, width: usize, align: Alignment, cell_fn: F) -> Self
    where
        F: Fn(&T) -> CellValue + Send + Sync + 'static,
    {
        Self {
            header: header.into(),
            width,
            align,
            cell_fn: Box::new(cell_fn),
        }
    }
}

// Manual Clone implementation for ColumnDef
impl<T> Clone for ColumnDef<T> {
    fn clone(&self) -> Self {
        // We can't clone the Box<dyn Fn> directly, so we create a note about this limitation
        panic!("ColumnDef cannot be cloned due to boxed closure. Create columns fresh each time.")
    }
}

// Debug implementation for ColumnDef
impl<T> fmt::Debug for ColumnDef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColumnDef")
            .field("header", &self.header)
            .field("width", &self.width)
            .field("align", &self.align)
            .field("cell_fn", &"<function>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_value_is_link() {
        let text = CellValue::Text("Hello".to_string());
        assert!(!text.is_link());

        let styled_text = CellValue::StyledText("42".to_string());
        assert!(!styled_text.is_link());

        let player_link = CellValue::PlayerLink {
            display: "Connor McDavid".to_string(),
            player_id: 8478402,
        };
        assert!(player_link.is_link());

        let team_link = CellValue::TeamLink {
            display: "Edmonton Oilers".to_string(),
            team_abbrev: "EDM".to_string(),
        };
        assert!(team_link.is_link());
    }

    #[test]
    fn test_cell_value_receives_selection_style() {
        let text = CellValue::Text("Hello".to_string());
        assert!(!text.receives_selection_style());

        let styled_text = CellValue::StyledText("42".to_string());
        assert!(styled_text.receives_selection_style());

        let player_link = CellValue::PlayerLink {
            display: "Connor McDavid".to_string(),
            player_id: 8478402,
        };
        assert!(player_link.receives_selection_style());

        let team_link = CellValue::TeamLink {
            display: "Edmonton Oilers".to_string(),
            team_abbrev: "EDM".to_string(),
        };
        assert!(team_link.receives_selection_style());
    }

    #[test]
    fn test_cell_value_display_text() {
        let text = CellValue::Text("Hello".to_string());
        assert_eq!(text.display_text(), "Hello");

        let styled_text = CellValue::StyledText("42".to_string());
        assert_eq!(styled_text.display_text(), "42");

        let player_link = CellValue::PlayerLink {
            display: "Connor McDavid".to_string(),
            player_id: 8478402,
        };
        assert_eq!(player_link.display_text(), "Connor McDavid");

        let team_link = CellValue::TeamLink {
            display: "Edmonton Oilers".to_string(),
            team_abbrev: "EDM".to_string(),
        };
        assert_eq!(team_link.display_text(), "Edmonton Oilers");
    }

    #[test]
    fn test_cell_value_link_info() {
        let text = CellValue::Text("Hello".to_string());
        assert_eq!(text.link_info(), "Not a link");

        let styled_text = CellValue::StyledText("42".to_string());
        assert_eq!(styled_text.link_info(), "Not a link");

        let player_link = CellValue::PlayerLink {
            display: "Connor McDavid".to_string(),
            player_id: 8478402,
        };
        assert_eq!(
            player_link.link_info(),
            "PlayerLink(display='Connor McDavid', id=8478402)"
        );

        let team_link = CellValue::TeamLink {
            display: "Edmonton Oilers".to_string(),
            team_abbrev: "EDM".to_string(),
        };
        assert_eq!(
            team_link.link_info(),
            "TeamLink(display='Edmonton Oilers', abbrev='EDM')"
        );
    }

    #[test]
    fn test_alignment() {
        let left = Alignment::Left;
        let right = Alignment::Right;
        let center = Alignment::Center;

        assert_eq!(left, Alignment::Left);
        assert_ne!(left, right);
        assert_ne!(right, center);
    }

    #[test]
    fn test_column_def_creation() {
        #[derive(Clone)]
        struct TestRow {
            name: String,
            #[allow(dead_code)]
            value: i32,
        }

        let col = ColumnDef::new("Test Column", 20, Alignment::Left, |row: &TestRow| {
            CellValue::Text(row.name.clone())
        });

        assert_eq!(col.header, "Test Column");
        assert_eq!(col.width, 20);
        assert_eq!(col.align, Alignment::Left);

        // Test that cell_fn works
        let test_row = TestRow {
            name: "Test".to_string(),
            value: 42,
        };
        let cell = (col.cell_fn)(&test_row);
        assert_eq!(cell.display_text(), "Test");
    }

    #[test]
    fn test_column_def_with_link() {
        #[derive(Clone)]
        struct Player {
            name: String,
            id: i64,
        }

        let col = ColumnDef::new("Player", 25, Alignment::Left, |p: &Player| {
            CellValue::PlayerLink {
                display: p.name.clone(),
                player_id: p.id,
            }
        });

        let player = Player {
            name: "Auston Matthews".to_string(),
            id: 8479318,
        };

        let cell = (col.cell_fn)(&player);
        assert!(cell.is_link());
        assert_eq!(cell.display_text(), "Auston Matthews");

        if let CellValue::PlayerLink { player_id, .. } = cell {
            assert_eq!(player_id, 8479318);
        } else {
            panic!("Expected PlayerLink");
        }
    }

    #[test]
    #[should_panic(expected = "ColumnDef cannot be cloned due to boxed closure")]
    fn test_column_def_clone_panics() {
        struct TestRow;

        let col = ColumnDef::new("Test", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("test".to_string())
        });

        let _cloned = col.clone();
    }

    #[test]
    fn test_column_def_debug() {
        struct TestRow;

        let col = ColumnDef::new("Test Header", 15, Alignment::Right, |_: &TestRow| {
            CellValue::Text("test".to_string())
        });

        let debug_str = format!("{:?}", col);
        assert!(debug_str.contains("Test Header"));
        assert!(debug_str.contains("15"));
        assert!(debug_str.contains("Right"));
        assert!(debug_str.contains("<function>"));
    }

    #[test]
    fn test_cell_value_equality() {
        let text1 = CellValue::Text("Hello".to_string());
        let text2 = CellValue::Text("Hello".to_string());
        let text3 = CellValue::Text("World".to_string());

        assert_eq!(text1, text2);
        assert_ne!(text1, text3);

        let player1 = CellValue::PlayerLink {
            display: "Player".to_string(),
            player_id: 123,
        };
        let player2 = CellValue::PlayerLink {
            display: "Player".to_string(),
            player_id: 123,
        };
        let player3 = CellValue::PlayerLink {
            display: "Player".to_string(),
            player_id: 456,
        };

        assert_eq!(player1, player2);
        assert_ne!(player1, player3);
    }
}
