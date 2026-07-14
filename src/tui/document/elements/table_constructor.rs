//! The `DocumentElement::table()` constructor: builds a `Table` variant from a
//! `TableWidget`, extracting focusable link cells along the way.

use ratatui::layout::Rect;

use crate::tui::components::TableWidget;
use crate::tui::document::focus::{FocusableElement, FocusableId};
use crate::tui::document::link::LinkTarget;
use crate::tui::types::StackedDocument;

use super::types::TABLE_COLUMN_HEADER_HEIGHT;
use super::DocumentElement;

impl DocumentElement {
    /// Create a table element from a TableWidget
    ///
    /// Extracts focusable elements from link cells in the table.
    /// The table renders at its natural height within the document.
    ///
    /// # Arguments
    /// - `name`: Unique name for this table (used to identify focusable cells)
    /// - `widget`: The table widget to embed
    pub fn table(name: impl Into<String>, widget: TableWidget) -> Self {
        use crate::tui::CellValue;

        let table_name = name.into();
        let mut focusable = Vec::new();

        // Calculate the y-offset where data rows start:
        // Column headers + separator (TABLE_COLUMN_HEADER_HEIGHT lines)
        let data_start_y = TABLE_COLUMN_HEADER_HEIGHT;

        // Extract focusable elements from link cells
        // Use TableCell IDs for row tracking, LinkTarget for activation data
        for row_idx in 0..widget.row_count() {
            for col_idx in 0..widget.column_count() {
                if let Some(cell) = widget.get_cell_value(row_idx, col_idx) {
                    let y = data_start_y + row_idx as u16;

                    // Create LinkTarget based on cell type (used for activation)
                    let link_target = match &cell {
                        CellValue::PlayerLink {
                            player_id,
                            sweater_number,
                            last_name,
                            ..
                        } => Some(LinkTarget::Push(StackedDocument::PlayerDetail {
                            player_id: *player_id,
                            sweater_number: *sweater_number,
                            last_name: last_name.clone(),
                        })),
                        CellValue::TeamLink { team_abbrev, .. } => {
                            Some(LinkTarget::Push(StackedDocument::TeamDetail {
                                abbrev: team_abbrev.clone(),
                            }))
                        }
                        _ => continue, // Skip non-link cells
                    };

                    // Use TableCell ID for row tracking (enables focused_table_row())
                    let id = FocusableId::table_cell(&table_name, row_idx, col_idx);

                    focusable.push(FocusableElement {
                        id,
                        y,
                        height: 1,
                        rect: Rect::new(0, y, cell.display_text().len() as u16, 1),
                        link_target,
                        row_position: None,
                    });
                }
            }
        }

        Self::Table { widget, focusable }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use ratatui::buffer::Buffer;

    #[test]
    fn test_table_element_height() {
        use crate::tui::components::TableWidget;
        use crate::tui::{Alignment, CellValue, ColumnDef};

        // Create a simple table with 3 rows
        let columns: Vec<ColumnDef<&str>> =
            vec![ColumnDef::new("Name", 10, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            })];
        let data = vec!["Alice", "Bob", "Charlie"];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("test_table", table);

        // TableWidget height = col headers (1) + separator (1) + 3 data rows = 5
        assert_eq!(elem.height(), 5);
    }

    #[test]
    fn test_table_element_focusable_extraction() {
        use crate::tui::components::TableWidget;
        use crate::tui::{Alignment, CellValue, ColumnDef};

        // Create a table with link cells
        let columns: Vec<ColumnDef<(&str, &str)>> = vec![ColumnDef::new(
            "Team",
            15,
            Alignment::Left,
            |row: &(&str, &str)| CellValue::TeamLink {
                display: row.0.to_string(),
                team_abbrev: row.1.to_string(),
            },
        )];
        let data = vec![("Bruins", "BOS"), ("Maple Leafs", "TOR")];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("teams", table);

        // Collect focusable elements
        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 0);

        // Should have 2 focusable elements (one per row) with TableCell IDs
        // TableCell IDs enable row highlighting via focused_table_row()
        assert_eq!(focusable.len(), 2);
        assert_eq!(focusable[0].id, FocusableId::table_cell("teams", 0, 0));
        assert_eq!(focusable[1].id, FocusableId::table_cell("teams", 1, 0));

        // Check link targets (contain team info for activation)
        match &focusable[0].link_target {
            Some(LinkTarget::Push(StackedDocument::TeamDetail { abbrev })) => {
                assert_eq!(abbrev, "BOS")
            }
            other => panic!("Expected Push(TeamDetail), got {other:?}"),
        }
        match &focusable[1].link_target {
            Some(LinkTarget::Push(StackedDocument::TeamDetail { abbrev })) => {
                assert_eq!(abbrev, "TOR")
            }
            other => panic!("Expected Push(TeamDetail), got {other:?}"),
        }
    }

    #[test]
    fn test_table_element_focusable_y_positions() {
        use crate::tui::components::TableWidget;
        use crate::tui::{Alignment, CellValue, ColumnDef};

        // Create a table with links
        let columns: Vec<ColumnDef<&str>> = vec![ColumnDef::new(
            "Player",
            15,
            Alignment::Left,
            |row: &&str| CellValue::PlayerLink {
                display: row.to_string(),
                player_id: 12345,
                sweater_number: None,
                last_name: row.to_string(),
            },
        )];
        let data = vec!["Player1", "Player2"];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("players", table);

        // Collect focusable at y_offset of 10
        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 10);

        // Y positions should be adjusted by offset
        // Data starts at y=2 (col headers + separator), then +10 offset = 12
        assert_eq!(focusable[0].y, 12);
        assert_eq!(focusable[1].y, 13);
    }

    #[test]
    fn test_table_element_no_focusable_text_only() {
        use crate::tui::components::TableWidget;
        use crate::tui::{Alignment, CellValue, ColumnDef};

        // Create a table with only text cells (no links)
        let columns: Vec<ColumnDef<i32>> =
            vec![ColumnDef::new("Value", 5, Alignment::Right, |row: &i32| {
                CellValue::Text(row.to_string())
            })];
        let data = vec![1, 2, 3];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("values", table);

        // Collect focusable elements
        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 0);

        // Should have no focusable elements (text cells aren't focusable)
        assert_eq!(focusable.len(), 0);
    }

    #[test]
    fn test_table_element_debug_format() {
        use crate::tui::components::TableWidget;
        use crate::tui::{Alignment, CellValue, ColumnDef};

        let columns: Vec<ColumnDef<&str>> = vec![
            ColumnDef::new("Col1", 10, Alignment::Left, |_: &&str| {
                CellValue::Text("x".to_string())
            }),
            ColumnDef::new("Col2", 10, Alignment::Left, |_: &&str| {
                CellValue::Text("y".to_string())
            }),
        ];
        let data = vec!["a", "b", "c"];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("test_table", table);

        let debug_str = format!("{:?}", elem);
        assert!(debug_str.contains("Table"));
        assert!(debug_str.contains("rows"));
        assert!(debug_str.contains("columns"));
        assert!(debug_str.contains("focusable_count"));
    }

    #[test]
    fn test_table_element_render() {
        use crate::tui::components::TableWidget;
        use crate::tui::testing::assert_buffer;
        use crate::tui::{Alignment, CellValue, ColumnDef};

        let columns: Vec<ColumnDef<&str>> =
            vec![ColumnDef::new("Name", 10, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            })];
        let data = vec!["Alice", "Bob"];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("test_table", table);

        let mut buf = Buffer::empty(Rect::new(0, 0, 15, 5));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 15, 5), &mut buf, &ctx);

        // Verify the table renders with margin, column header and data
        // TableWidget adds 2 space margin on left
        assert_buffer(&buf, &["  Name", "  ──────────", "  Alice", "  Bob", ""]);
    }
}
