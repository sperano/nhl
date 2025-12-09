//! Standings table component for displaying NHL standings
//!
//! This module provides a reusable standings table that can be embedded
//! in both the Standings tab and the Demo tab's document system.

use std::sync::LazyLock;

use nhl_api::Standing;

use crate::tui::{Alignment, CellValue, ColumnDef};

use super::TableWidget;

/// Cached column definitions for standings table
/// Uses LazyLock to initialize once and reuse across all calls
static STANDINGS_COLUMNS: LazyLock<Vec<ColumnDef<Standing>>> = LazyLock::new(|| {
    vec![
        ColumnDef::new("Team", 26, Alignment::Left, |s: &Standing| {
            CellValue::TeamLink {
                display: s.team_common_name.default.clone(),
                team_abbrev: s.team_abbrev.default.clone(),
            }
        }),
        ColumnDef::new("GP", 4, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.games_played().to_string())
        }),
        ColumnDef::new("W", 4, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.wins.to_string())
        }),
        ColumnDef::new("L", 3, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.losses.to_string())
        }),
        ColumnDef::new("OT", 3, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.ot_losses.to_string())
        }),
        ColumnDef::new("PTS", 5, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.points.to_string())
        }),
    ]
});

/// Get the shared column definitions for standings tables
pub fn standings_columns() -> &'static Vec<ColumnDef<Standing>> {
    &STANDINGS_COLUMNS
}

/// Create a standings table widget with the standard columns
///
/// # Arguments
/// * `standings` - The standings data to display
pub fn create_standings_table(standings: Vec<Standing>) -> TableWidget {
    TableWidget::from_data(standings_columns(), standings)
}

/// Create a standings table widget with focus on a specific row
///
/// # Arguments
/// * `standings` - The standings data to display
/// * `focused_row` - Which row is focused (None for no focus)
pub fn create_standings_table_with_selection(
    standings: Vec<Standing>,
    focused_row: Option<usize>,
) -> TableWidget {
    TableWidget::from_data(standings_columns(), standings).with_focused_row(focused_row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::component::ElementWidget;
    use crate::tui::testing::{assert_buffer, create_test_standings};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn render_widget(widget: &impl ElementWidget, width: u16, height: u16) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        let config = DisplayConfig::default();
        widget.render(buf.area, &mut buf, &config);
        buf
    }

    #[test]
    fn test_standings_columns_initialized() {
        let columns = standings_columns();
        assert_eq!(columns.len(), 6);
        assert_eq!(columns[0].header, "Team");
        assert_eq!(columns[1].header, "GP");
        assert_eq!(columns[2].header, "W");
        assert_eq!(columns[3].header, "L");
        assert_eq!(columns[4].header, "OT");
        assert_eq!(columns[5].header, "PTS");
    }

    #[test]
    fn test_create_standings_table() {
        let standings = create_test_standings();
        let table = create_standings_table(standings.clone());

        // Should have correct row count
        assert_eq!(table.row_count(), standings.len());
    }

    #[test]
    fn test_create_standings_table_renders_correctly() {
        let standings = create_test_standings();
        // Take just first 4 teams for a compact test
        let standings: Vec<_> = standings.into_iter().take(4).collect();
        let table = create_standings_table(standings);

        let height = table.preferred_height().unwrap();
        let buf = render_widget(&table, 60, height);

        assert_buffer(
            &buf,
            &[
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Panthers                      19    14    3    2     30",
                "  Bruins                        18    13    4    1     27",
                "  Maple Leafs                   19    12    5    2     26",
                "  Lightning                     18    11    6    1     23",
            ],
        );
    }

    #[test]
    fn test_create_standings_table_with_selection() {
        let standings = create_test_standings();
        let standings: Vec<_> = standings.into_iter().take(3).collect();
        let table = create_standings_table_with_selection(standings, Some(1));

        let height = table.preferred_height().unwrap();
        let buf = render_widget(&table, 60, height);

        // Row 1 should show the selector
        assert_buffer(
            &buf,
            &[
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Panthers                      19    14    3    2     30",
                "▶ Bruins                        18    13    4    1     27",
                "  Maple Leafs                   19    12    5    2     26",
            ],
        );
    }
}
