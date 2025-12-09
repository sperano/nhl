//! Document builder utilities
//!
//! Provides a fluent builder API for constructing documents from elements.

use ratatui::style::Style;

use crate::tui::components::TableWidget;

use super::elements::DocumentElement;
use super::link::LinkTarget;

/// Builder for constructing documents
#[derive(Debug, Default)]
pub struct DocumentBuilder {
    elements: Vec<DocumentElement>,
}

impl DocumentBuilder {
    /// Create a new empty document builder
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Add a heading element
    ///
    /// # Arguments
    /// - `level`: Heading level (1-6, where 1 is largest)
    /// - `content`: Heading text
    pub fn heading(mut self, level: u8, content: impl Into<String>) -> Self {
        self.elements.push(DocumentElement::heading(level, content));
        self
    }

    /// Add a text paragraph
    pub fn text(mut self, content: impl Into<String>) -> Self {
        self.elements.push(DocumentElement::text(content));
        self
    }

    /// Add a styled text paragraph
    pub fn styled_text(mut self, content: impl Into<String>, style: Style) -> Self {
        self.elements
            .push(DocumentElement::styled_text(content, style));
        self
    }

    /// Add a link element
    ///
    /// # Arguments
    /// - `display`: Text to display for the link
    /// - `target`: Link target (document, anchor, or action)
    pub fn link(mut self, display: impl Into<String>, target: LinkTarget) -> Self {
        let display = display.into();
        let id = format!("link_{}", self.elements.len());
        self.elements
            .push(DocumentElement::link(id, display, target));
        self
    }

    /// Add a link element with a custom ID
    pub fn link_with_id(
        mut self,
        id: impl Into<String>,
        display: impl Into<String>,
        target: LinkTarget,
    ) -> Self {
        self.elements
            .push(DocumentElement::link(id, display, target));
        self
    }

    /// Add a link element with a custom ID and focus context
    ///
    /// The link will be rendered as focused if the focus context's focused_id
    /// matches the provided ID.
    pub fn link_with_focus(
        mut self,
        id: impl Into<String>,
        display: impl Into<String>,
        target: LinkTarget,
        focus: &super::FocusContext,
    ) -> Self {
        let id = id.into();
        let is_focused = focus.is_link_focused(&id);
        if is_focused {
            self.elements
                .push(DocumentElement::focused_link(id, display, target));
        } else {
            self.elements
                .push(DocumentElement::link(id, display, target));
        }
        self
    }

    /// Add a horizontal separator
    pub fn separator(mut self) -> Self {
        self.elements.push(DocumentElement::separator());
        self
    }

    /// Add vertical spacing
    pub fn spacer(mut self, height: u16) -> Self {
        self.elements.push(DocumentElement::spacer(height));
        self
    }

    /// Add a pre-built element
    pub fn element(mut self, element: DocumentElement) -> Self {
        self.elements.push(element);
        self
    }

    /// Add a table element
    ///
    /// Tables render at their natural height and extract focusable elements
    /// from link cells (PlayerLink, TeamLink).
    ///
    /// # Arguments
    /// - `name`: Unique name for this table (used to identify focusable cells)
    /// - `widget`: The table widget to embed
    pub fn table(mut self, name: impl Into<String>, widget: TableWidget) -> Self {
        self.elements.push(DocumentElement::table(name, widget));
        self
    }

    /// Add multiple elements at once
    pub fn elements(mut self, elements: impl IntoIterator<Item = DocumentElement>) -> Self {
        self.elements.extend(elements);
        self
    }

    /// Add a horizontal row of elements (side by side)
    ///
    /// Elements are laid out horizontally with equal width distribution.
    pub fn row(mut self, children: Vec<DocumentElement>) -> Self {
        self.elements.push(DocumentElement::row(children));
        self
    }

    /// Add a horizontal row of elements with custom gap
    ///
    /// Elements are laid out horizontally with the specified gap between them.
    pub fn row_with_gap(mut self, children: Vec<DocumentElement>, gap: u16) -> Self {
        self.elements
            .push(DocumentElement::row_with_gap(children, gap));
        self
    }

    /// Create a nested group of elements using a nested builder
    ///
    /// # Example
    /// ```ignore
    /// let doc = DocumentBuilder::new()
    ///     .heading(1, "Title")
    ///     .group(|b| b
    ///         .text("Grouped content")
    ///         .link("Link", target)
    ///     )
    ///     .build();
    /// ```
    pub fn group<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DocumentBuilder) -> DocumentBuilder,
    {
        let group_builder = DocumentBuilder::new();
        let group_builder = f(group_builder);
        self.elements
            .push(DocumentElement::group(group_builder.elements));
        self
    }

    /// Create a styled nested group of elements
    pub fn styled_group<F>(mut self, style: Style, f: F) -> Self
    where
        F: FnOnce(DocumentBuilder) -> DocumentBuilder,
    {
        let group_builder = DocumentBuilder::new();
        let group_builder = f(group_builder);
        self.elements
            .push(DocumentElement::styled_group(group_builder.elements, style));
        self
    }

    /// Conditionally add an element
    ///
    /// # Example
    /// ```ignore
    /// let doc = DocumentBuilder::new()
    ///     .heading(1, "Title")
    ///     .when(show_details, |b| b.text("Details here"))
    ///     .build();
    /// ```
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Conditionally add an element with an else branch
    pub fn when_else<F1, F2>(self, condition: bool, if_true: F1, if_false: F2) -> Self
    where
        F1: FnOnce(Self) -> Self,
        F2: FnOnce(Self) -> Self,
    {
        if condition {
            if_true(self)
        } else {
            if_false(self)
        }
    }

    /// Add elements from an iterator
    ///
    /// # Example
    /// ```ignore
    /// let doc = DocumentBuilder::new()
    ///     .heading(1, "Teams")
    ///     .for_each(teams.iter(), |b, team| {
    ///         b.link(&team.name, LinkTarget::Document(DocumentLink::team(&team.abbrev)))
    ///     })
    ///     .build();
    /// ```
    pub fn for_each<I, T, F>(mut self, iter: I, f: F) -> Self
    where
        I: IntoIterator<Item = T>,
        F: Fn(Self, T) -> Self,
    {
        for item in iter {
            self = f(self, item);
        }
        self
    }

    /// Get the current number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the builder has no elements
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Consume the builder and return the elements
    pub fn build(self) -> Vec<DocumentElement> {
        self.elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document::link::DocumentLink;

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
        let target = LinkTarget::Document(DocumentLink::team("BOS"));
        let elements = DocumentBuilder::new().link("Boston Bruins", target).build();

        assert_eq!(elements.len(), 1);
    }

    #[test]
    fn test_builder_link_with_id() {
        let target = LinkTarget::Action("test".to_string());
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
        let items = vec!["One", "Two", "Three"];
        let elements = DocumentBuilder::new()
            .heading(1, "List")
            .for_each(items.iter(), |b, item| b.text(*item))
            .build();

        assert_eq!(elements.len(), 4); // heading + 3 items
    }

    #[test]
    fn test_builder_complex_document() {
        let teams = vec![("BOS", "Boston Bruins"), ("TOR", "Toronto Maple Leafs")];

        let elements = DocumentBuilder::new()
            .heading(1, "NHL Teams")
            .spacer(1)
            .for_each(teams.iter(), |b, (abbrev, name)| {
                b.link(*name, LinkTarget::Document(DocumentLink::team(*abbrev)))
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
}
