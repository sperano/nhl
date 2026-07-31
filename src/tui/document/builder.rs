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
    ///         b.link(&team.name, LinkTarget::Push(StackedDocument::TeamDetail {
    ///             abbrev: team.abbrev.clone(),
    ///         }))
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

    /// Add a tabbed panel element
    ///
    /// # Arguments
    /// - `id`: Unique identifier for this tabs element (used for state tracking)
    /// - `tabs`: Vec of (key, title, content) tuples
    /// - `active_index`: Index of the initially active tab
    ///
    /// # Example
    /// ```ignore
    /// let doc = DocumentBuilder::new()
    ///     .heading(1, "Demo")
    ///     .tabs(
    ///         "demo_tabs",
    ///         vec![
    ///             ("tab1", "First Tab", vec![DocumentElement::text("Content 1")]),
    ///             ("tab2", "Second Tab", vec![DocumentElement::text("Content 2")]),
    ///         ],
    ///         0, // Start with first tab active
    ///     )
    ///     .build();
    /// ```
    pub fn tabs(
        mut self,
        id: impl Into<String>,
        tabs: Vec<(impl Into<String>, impl Into<String>, Vec<DocumentElement>)>,
        active_index: usize,
    ) -> Self {
        let tab_defs: Vec<super::elements::DocTabDef> = tabs
            .into_iter()
            .map(|(key, title, content)| {
                super::elements::DocTabDef::new(key.into(), title.into(), content)
            })
            .collect();
        self.elements
            .push(DocumentElement::tabs(id, tab_defs, active_index));
        self
    }

    /// Add a tabbed panel element using focus context for active tab selection
    ///
    /// This is the preferred way to add tabs when building documents,
    /// as it reads the active tab from the FocusContext (which gets it
    /// from DocumentNavState).
    ///
    /// # Arguments
    /// - `id`: Unique identifier for this tabs element
    /// - `tabs`: Vec of (key, title, content) tuples
    /// - `focus`: Focus context containing tab selections
    ///
    /// # Example
    /// ```ignore
    /// fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
    ///     DocumentBuilder::new()
    ///         .heading(1, "Demo")
    ///         .tabs_with_focus(
    ///             "demo_tabs",
    ///             vec![
    ///                 ("tab1", "First Tab", vec![DocumentElement::text("Content 1")]),
    ///                 ("tab2", "Second Tab", vec![DocumentElement::text("Content 2")]),
    ///             ],
    ///             focus,
    ///         )
    ///         .build()
    /// }
    /// ```
    pub fn tabs_with_focus(
        mut self,
        id: impl Into<String>,
        tabs: Vec<(impl Into<String>, impl Into<String>, Vec<DocumentElement>)>,
        focus: &super::FocusContext,
    ) -> Self {
        let tab_defs: Vec<super::elements::DocTabDef> = tabs
            .into_iter()
            .map(|(key, title, content)| {
                super::elements::DocTabDef::new(key.into(), title.into(), content)
            })
            .collect();
        self.elements
            .push(DocumentElement::tabs_from_context(id, tab_defs, focus));
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
#[path = "builder_tests.rs"]
mod tests;
