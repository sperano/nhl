use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::tui::testing::{assert_buffer, RENDER_WIDTH};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

// Helper to render ElementWidget for testing
fn render_framework_widget(
    widget: &impl crate::tui::component::ElementWidget,
    width: u16,
    height: u16,
    config: &DisplayConfig,
) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    let ctx = RenderContext::focused(config);
    widget.render(buf.area, &mut buf, &ctx);
    buf
}

fn test_config() -> DisplayConfig {
    DisplayConfig::default()
}

#[derive(Clone)]
struct TestRow {
    name: String,
    id: i64,
    value: i32,
}

fn create_test_rows() -> Vec<TestRow> {
    vec![
        TestRow {
            name: "Auston Matthews".to_string(),
            id: 8479318,
            value: 42,
        },
        TestRow {
            name: "Mitchell Marner".to_string(),
            id: 8478483,
            value: 18,
        },
        TestRow {
            name: "William Nylander".to_string(),
            id: 8477939,
            value: 28,
        },
    ]
}

fn create_test_columns() -> Vec<ColumnDef<TestRow>> {
    vec![
        ColumnDef::new("Player", 20, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("G", 4, Alignment::Right, |r: &TestRow| {
            CellValue::Text(r.value.to_string())
        }),
    ]
}

#[test]
fn test_empty_table() {
    let columns: Vec<ColumnDef<TestRow>> = vec![];
    let rows: Vec<TestRow> = vec![];

    let widget = TableWidget::from_data(&columns, rows);
    let config = test_config();
    let buf = render_framework_widget(&widget, RENDER_WIDTH, 1, &config);

    // Empty table renders nothing (no selector space without content)
    assert_buffer(&buf, &[""]);
}

#[test]
fn test_table_with_text_cells() {
    let rows = vec![
        TestRow {
            name: "Row1".to_string(),
            id: 1,
            value: 10,
        },
        TestRow {
            name: "Row2".to_string(),
            id: 2,
            value: 20,
        },
    ];

    let columns = vec![
        ColumnDef::new("Name", 10, Alignment::Left, |r: &TestRow| {
            CellValue::Text(r.name.clone())
        }),
        ColumnDef::new("Val", 5, Alignment::Right, |r: &TestRow| {
            CellValue::Text(r.value.to_string())
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);
    let config = test_config();
    let height = widget.preferred_height().unwrap();
    let buf = render_framework_widget(&widget, RENDER_WIDTH, height, &config);

    assert_buffer(
        &buf,
        &[
            "  Name          Val",
            "  ─────────────────",
            "  Row1           10",
            "  Row2           20",
        ],
    );
}

#[test]
fn test_table_alignment() {
    let rows = vec![TestRow {
        name: "X".to_string(),
        id: 1,
        value: 5,
    }];

    let columns = vec![
        ColumnDef::new("Left", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("L".to_string())
        }),
        ColumnDef::new("Right", 10, Alignment::Right, |_: &TestRow| {
            CellValue::Text("R".to_string())
        }),
        ColumnDef::new("Center", 10, Alignment::Center, |_: &TestRow| {
            CellValue::Text("C".to_string())
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);
    let config = test_config();
    let height = widget.preferred_height().unwrap();
    let buf = render_framework_widget(&widget, RENDER_WIDTH, height, &config);

    assert_buffer(
        &buf,
        &[
            "  Left             Right    Center",
            "  ──────────────────────────────────",
            "  L                    R      C",
        ],
    );
}

#[test]
fn test_table_truncation() {
    let rows = vec![TestRow {
        name: "Very Long Name That Exceeds Width".to_string(),
        id: 1,
        value: 5,
    }];

    let columns = vec![ColumnDef::new(
        "Name",
        10,
        Alignment::Left,
        |r: &TestRow| CellValue::Text(r.name.clone()),
    )];

    let widget = TableWidget::from_data(&columns, rows);
    let config = test_config();
    let height = widget.preferred_height().unwrap();
    let buf = render_framework_widget(&widget, RENDER_WIDTH, height, &config);

    assert_buffer(&buf, &["  Name", "  ──────────", "  Very Lo..."]);
}

#[test]
fn test_table_preferred_dimensions() {
    let rows = create_test_rows();
    let columns = create_test_columns();

    let widget = TableWidget::from_data(&columns, rows);

    // Height: col_header(1) + separator(1) + 3 rows = 5
    assert_eq!(widget.preferred_height(), Some(5));

    // Width: selector(2) + col1(20) + spacing(2) + col2(4) = 28
    assert_eq!(widget.preferred_width(), Some(28));
}

#[test]
fn test_table_with_selection_focused() {
    let rows = vec![
        TestRow {
            name: "Row1".to_string(),
            id: 1,
            value: 10,
        },
        TestRow {
            name: "Row2".to_string(),
            id: 2,
            value: 20,
        },
    ];

    let columns = vec![ColumnDef::new(
        "Name",
        10,
        Alignment::Left,
        |r: &TestRow| CellValue::Text(r.name.clone()),
    )];

    let widget = TableWidget::from_data(&columns, rows).with_focused_row(Some(1)); // Select row 1

    let config = test_config();
    let height = widget.preferred_height().unwrap();
    let buf = render_framework_widget(&widget, RENDER_WIDTH, height, &config);

    // Row 1 should be highlighted with selection_fg and show selector
    // Note: We can't easily test the color in assert_buffer, but we can verify the text
    assert_buffer(
        &buf,
        &[
            "  Name",
            "  ──────────",
            "  Row1",
            "▶ Row2", // This row should have selection_fg and selector
        ],
    );
}

// === Navigation Tests ===

#[test]
fn test_find_next_link_column() {
    let rows = vec![create_test_rows()[0].clone()];

    let columns = vec![
        ColumnDef::new("Text1", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("A".to_string())
        }),
        ColumnDef::new("Link1", 10, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("Text2", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("B".to_string())
        }),
        ColumnDef::new("Link2", 10, Alignment::Left, |_: &TestRow| {
            CellValue::TeamLink {
                display: "Team".to_string(),
                team_abbrev: "TOR".to_string(),
                season: None,
            }
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);

    // From column 0 (Text1), next link is column 1 (Link1)
    assert_eq!(widget.find_next_link_column(0), Some(1));

    // From column 1 (Link1), next link is column 3 (Link2)
    assert_eq!(widget.find_next_link_column(1), Some(3));

    // From column 2 (Text2), next link is column 3 (Link2)
    assert_eq!(widget.find_next_link_column(2), Some(3));

    // From column 3 (Link2), no next link
    assert_eq!(widget.find_next_link_column(3), None);
}

#[test]
fn test_find_prev_link_column() {
    let rows = vec![create_test_rows()[0].clone()];

    let columns = vec![
        ColumnDef::new("Link1", 10, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("Text1", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("A".to_string())
        }),
        ColumnDef::new("Link2", 10, Alignment::Left, |_: &TestRow| {
            CellValue::TeamLink {
                display: "Team".to_string(),
                team_abbrev: "TOR".to_string(),
                season: None,
            }
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);

    // From column 0 (Link1), no previous link
    assert_eq!(widget.find_prev_link_column(0), None);

    // From column 1 (Text1), previous link is column 0 (Link1)
    assert_eq!(widget.find_prev_link_column(1), Some(0));

    // From column 2 (Link2), previous link is column 0 (Link1)
    assert_eq!(widget.find_prev_link_column(2), Some(0));
}

#[test]
fn test_find_first_link_column() {
    let rows = vec![create_test_rows()[0].clone()];

    // Table with link in middle
    let columns = vec![
        ColumnDef::new("Text1", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("A".to_string())
        }),
        ColumnDef::new("Link", 10, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("Text2", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("B".to_string())
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);
    assert_eq!(widget.find_first_link_column(), Some(1));
}

#[test]
fn test_find_first_link_column_no_links() {
    let rows = vec![create_test_rows()[0].clone()];

    // Table with all text columns
    let columns = vec![
        ColumnDef::new("Text1", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("A".to_string())
        }),
        ColumnDef::new("Text2", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("B".to_string())
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);
    assert_eq!(widget.find_first_link_column(), None);
}

#[test]
fn test_get_cell_value() {
    let rows = create_test_rows();

    let columns = vec![
        ColumnDef::new("Player", 20, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("Value", 10, Alignment::Right, |r: &TestRow| {
            CellValue::Text(r.value.to_string())
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);

    // Get player link cell
    let cell = widget.get_cell_value(0, 0);
    assert!(cell.is_some());
    assert!(cell.unwrap().is_link());

    // Get text cell
    let cell = widget.get_cell_value(0, 1);
    assert!(cell.is_some());
    assert!(!cell.unwrap().is_link());

    // Out of bounds
    assert!(widget.get_cell_value(100, 0).is_none());
    assert!(widget.get_cell_value(0, 100).is_none());
}

#[test]
fn test_row_and_column_count() {
    let rows = create_test_rows();
    let columns = create_test_columns();

    let widget = TableWidget::from_data(&columns, rows);

    assert_eq!(widget.row_count(), 3);
    assert_eq!(widget.column_count(), 2);
}

#[test]
fn test_table_with_mixed_cell_types() {
    let rows = create_test_rows();

    let columns = vec![
        ColumnDef::new("Player", 20, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("Team", 15, Alignment::Left, |_: &TestRow| {
            CellValue::TeamLink {
                display: "Toronto".to_string(),
                team_abbrev: "TOR".to_string(),
                season: None,
            }
        }),
        ColumnDef::new("G", 4, Alignment::Right, |r: &TestRow| {
            CellValue::Text(r.value.to_string())
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);
    let config = test_config();
    let height = widget.preferred_height().unwrap();
    render_framework_widget(&widget, 50, height, &config);

    // Just verify it renders without panicking
    assert_eq!(height, 5); // col_header(1) + separator(1) + 3 rows
}

#[test]
fn test_link_activation_player() {
    let rows = vec![create_test_rows()[0].clone()];

    let columns = vec![ColumnDef::new(
        "Player",
        20,
        Alignment::Left,
        |r: &TestRow| CellValue::PlayerLink {
            display: r.name.clone(),
            player_id: r.id,
            sweater_number: None,
            last_name: String::new(),
        },
    )];

    let widget = TableWidget::from_data(&columns, rows);

    // Get the player link cell
    let cell = widget.get_cell_value(0, 0).unwrap();
    assert!(cell.is_link());

    // Check link info for logging
    let link_info = cell.link_info();
    assert!(link_info.contains("PlayerLink"));
    assert!(link_info.contains("8479318")); // player_id
    assert!(link_info.contains("Auston Matthews"));
}

#[test]
fn test_link_activation_team() {
    let rows = vec![create_test_rows()[0].clone()];

    let columns = vec![ColumnDef::new(
        "Team",
        15,
        Alignment::Left,
        |_: &TestRow| CellValue::TeamLink {
            display: "Toronto Maple Leafs".to_string(),
            team_abbrev: "TOR".to_string(),
            season: None,
        },
    )];

    let widget = TableWidget::from_data(&columns, rows);

    // Get the team link cell
    let cell = widget.get_cell_value(0, 0).unwrap();
    assert!(cell.is_link());

    // Check link info for logging
    let link_info = cell.link_info();
    assert!(link_info.contains("TeamLink"));
    assert!(link_info.contains("TOR"));
    assert!(link_info.contains("Toronto Maple Leafs"));
}

#[test]
fn test_navigation_skips_text_columns() {
    let rows = vec![create_test_rows()[0].clone()];

    // Pattern: Link, Text, Text, Link, Text, Link
    let columns = vec![
        ColumnDef::new("Col0", 10, Alignment::Left, |r: &TestRow| {
            CellValue::PlayerLink {
                display: r.name.clone(),
                player_id: r.id,
                sweater_number: None,
                last_name: String::new(),
            }
        }),
        ColumnDef::new("Col1", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("T1".to_string())
        }),
        ColumnDef::new("Col2", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("T2".to_string())
        }),
        ColumnDef::new("Col3", 10, Alignment::Left, |_: &TestRow| {
            CellValue::TeamLink {
                display: "Team1".to_string(),
                team_abbrev: "T1".to_string(),
                season: None,
            }
        }),
        ColumnDef::new("Col4", 10, Alignment::Left, |_: &TestRow| {
            CellValue::Text("T3".to_string())
        }),
        ColumnDef::new("Col5", 10, Alignment::Left, |_: &TestRow| {
            CellValue::TeamLink {
                display: "Team2".to_string(),
                team_abbrev: "T2".to_string(),
                season: None,
            }
        }),
    ];

    let widget = TableWidget::from_data(&columns, rows);

    // Navigate right from col 0 (Link) should jump to col 3 (Link), skipping cols 1-2 (Text)
    assert_eq!(widget.find_next_link_column(0), Some(3));

    // Navigate right from col 3 (Link) should jump to col 5 (Link), skipping col 4 (Text)
    assert_eq!(widget.find_next_link_column(3), Some(5));

    // Navigate left from col 5 (Link) should jump to col 3 (Link), skipping col 4 (Text)
    assert_eq!(widget.find_prev_link_column(5), Some(3));

    // Navigate left from col 3 (Link) should jump to col 0 (Link), skipping cols 1-2 (Text)
    assert_eq!(widget.find_prev_link_column(3), Some(0));
}
