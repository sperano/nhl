use super::*;
use crate::tui::types::StackedDocument;

#[test]
fn test_builder_new() {
    let builder = DocumentBuilder::new();
    assert!(builder.is_empty());
    assert_eq!(builder.len(), 0);
}

#[test]
fn test_builder_heading() {
    let elements = DocumentBuilder::new()
        .heading(1, "Title")
        .heading(2, "Subtitle")
        .build();

    assert_eq!(elements.len(), 2);
}

#[test]
fn test_builder_text() {
    let elements = DocumentBuilder::new()
        .text("Hello world")
        .text("Another line")
        .build();

    assert_eq!(elements.len(), 2);
}

#[test]
fn test_builder_styled_text() {
    let style = ratatui::style::Style::default().fg(ratatui::style::Color::Red);
    let elements = DocumentBuilder::new()
        .styled_text("Red text", style)
        .build();

    assert_eq!(elements.len(), 1);
}

#[test]
fn test_builder_link() {
    let target = LinkTarget::Push(StackedDocument::TeamDetail {
        abbrev: "BOS".to_string(),
        season: None,
    });
    let elements = DocumentBuilder::new().link("Boston Bruins", target).build();

    assert_eq!(elements.len(), 1);
}

#[test]
fn test_builder_link_with_id() {
    let target = LinkTarget::Anchor("test".to_string());
    let elements = DocumentBuilder::new()
        .link_with_id("custom_id", "Click me", target)
        .build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Link { id, .. } => assert_eq!(id, "custom_id"),
        _ => panic!("Expected Link"),
    }
}

#[test]
fn test_builder_separator() {
    let elements = DocumentBuilder::new().separator().build();

    assert_eq!(elements.len(), 1);
    assert!(matches!(elements[0], DocumentElement::Separator));
}

#[test]
fn test_builder_spacer() {
    let elements = DocumentBuilder::new().spacer(5).build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Spacer { height } => assert_eq!(*height, 5),
        _ => panic!("Expected Spacer"),
    }
}

#[test]
fn test_builder_element() {
    let elem = DocumentElement::text("Direct element");
    let elements = DocumentBuilder::new().element(elem).build();

    assert_eq!(elements.len(), 1);
}

#[test]
fn test_builder_elements() {
    let elems = vec![
        DocumentElement::text("First"),
        DocumentElement::text("Second"),
    ];
    let elements = DocumentBuilder::new().elements(elems).build();

    assert_eq!(elements.len(), 2);
}

#[test]
fn test_builder_group() {
    let elements = DocumentBuilder::new()
        .heading(1, "Title")
        .group(|b| b.text("Inside group").text("Also inside"))
        .text("Outside group")
        .build();

    assert_eq!(elements.len(), 3);
    match &elements[1] {
        DocumentElement::Group { children, .. } => {
            assert_eq!(children.len(), 2);
        }
        _ => panic!("Expected Group"),
    }
}

#[test]
fn test_builder_styled_group() {
    let style = ratatui::style::Style::default().bg(ratatui::style::Color::Blue);
    let elements = DocumentBuilder::new()
        .styled_group(style, |b| b.text("Styled content"))
        .build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Group { style: s, .. } => assert_eq!(*s, Some(style)),
        _ => panic!("Expected Group"),
    }
}

#[test]
fn test_builder_when_true() {
    let show = true;
    let elements = DocumentBuilder::new()
        .text("Always shown")
        .when(show, |b| b.text("Conditionally shown"))
        .build();

    assert_eq!(elements.len(), 2);
}

#[test]
fn test_builder_when_false() {
    let show = false;
    let elements = DocumentBuilder::new()
        .text("Always shown")
        .when(show, |b| b.text("Not shown"))
        .build();

    assert_eq!(elements.len(), 1);
}

#[test]
fn test_builder_when_else_true() {
    let condition = true;
    let elements = DocumentBuilder::new()
        .when_else(
            condition,
            |b| b.text("True branch"),
            |b| b.text("False branch"),
        )
        .build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Text { content, .. } => assert_eq!(content, "True branch"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_builder_when_else_false() {
    let condition = false;
    let elements = DocumentBuilder::new()
        .when_else(
            condition,
            |b| b.text("True branch"),
            |b| b.text("False branch"),
        )
        .build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Text { content, .. } => assert_eq!(content, "False branch"),
        _ => panic!("Expected Text"),
    }
}

#[test]
fn test_builder_for_each() {
    let items = ["One", "Two", "Three"];
    let elements = DocumentBuilder::new()
        .heading(1, "List")
        .for_each(items.iter(), |b, item| b.text(*item))
        .build();

    assert_eq!(elements.len(), 4); // heading + 3 items
}

#[test]
fn test_builder_complex_document() {
    let teams = [("BOS", "Boston Bruins"), ("TOR", "Toronto Maple Leafs")];

    let elements = DocumentBuilder::new()
        .heading(1, "NHL Teams")
        .spacer(1)
        .for_each(teams.iter(), |b, (abbrev, name)| {
            b.link(
                *name,
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: abbrev.to_string(),
                    season: None,
                }),
            )
        })
        .separator()
        .text("Click a team for details")
        .build();

    assert_eq!(elements.len(), 6); // heading + spacer + 2 links + separator + text
}

#[test]
fn test_builder_len_and_is_empty() {
    let mut builder = DocumentBuilder::new();
    assert!(builder.is_empty());
    assert_eq!(builder.len(), 0);

    builder = builder.text("Hello");
    assert!(!builder.is_empty());
    assert_eq!(builder.len(), 1);

    builder = builder.text("World");
    assert_eq!(builder.len(), 2);
}

#[test]
fn test_builder_default() {
    let builder = DocumentBuilder::default();
    assert!(builder.is_empty());
}

#[test]
fn test_builder_nested_groups() {
    let elements = DocumentBuilder::new()
        .group(|outer| {
            outer
                .text("Outer start")
                .group(|inner| inner.text("Inner content"))
                .text("Outer end")
        })
        .build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Group { children, .. } => {
            assert_eq!(children.len(), 3);
        }
        _ => panic!("Expected Group"),
    }
}

#[test]
fn test_builder_table() {
    use crate::tui::components::TableWidget;
    use crate::tui::{Alignment, CellValue, ColumnDef};

    let columns: Vec<ColumnDef<&str>> =
        vec![ColumnDef::new("Name", 10, Alignment::Left, |row: &&str| {
            CellValue::Text(row.to_string())
        })];
    let data = vec!["Alice", "Bob"];
    let table = TableWidget::from_data(&columns, data);

    let elements = DocumentBuilder::new()
        .heading(1, "Table Demo")
        .table("test_table", table)
        .build();

    assert_eq!(elements.len(), 2);
    match &elements[1] {
        DocumentElement::Table { widget, .. } => {
            assert_eq!(widget.row_count(), 2);
            assert_eq!(widget.column_count(), 1);
        }
        _ => panic!("Expected Table"),
    }
}

#[test]
fn test_builder_table_with_links() {
    use crate::tui::components::TableWidget;
    use crate::tui::{Alignment, CellValue, ColumnDef};

    let columns: Vec<ColumnDef<(&str, &str)>> = vec![ColumnDef::new(
        "Team",
        15,
        Alignment::Left,
        |row: &(&str, &str)| CellValue::TeamLink {
            display: row.0.to_string(),
            team_abbrev: row.1.to_string(),
        },
    )];
    let data = vec![("Bruins", "BOS"), ("Leafs", "TOR")];
    let table = TableWidget::from_data(&columns, data);

    let elements = DocumentBuilder::new().table("teams", table).build();

    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Table { focusable, .. } => {
            // Should extract focusable elements from link cells
            assert_eq!(focusable.len(), 2);
        }
        _ => panic!("Expected Table"),
    }
}
