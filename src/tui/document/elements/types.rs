//! Small supporting types shared across the element behavior/constructor modules:
//! tab definitions, row alignment, and layout constants.

use super::DocumentElement;

/// Tab bar height (labels line + separator line)
pub const TAB_BAR_HEIGHT: u16 = 2;

/// Height of column headers section (column names + separator)
pub(crate) const TABLE_COLUMN_HEADER_HEIGHT: u16 = 2;

/// Definition of a single tab within a Tabs element
#[derive(Clone)]
pub struct DocTabDef {
    /// Unique key identifying this tab
    pub key: String,
    /// Display title for the tab header
    pub title: String,
    /// Content elements for this tab
    pub content: Vec<DocumentElement>,
}

impl DocTabDef {
    /// Create a new tab definition
    pub fn new(
        key: impl Into<String>,
        title: impl Into<String>,
        content: Vec<DocumentElement>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            content,
        }
    }
}

impl std::fmt::Debug for DocTabDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocTabDef")
            .field("key", &self.key)
            .field("title", &self.title)
            .field("content_count", &self.content.len())
            .finish()
    }
}

/// Alignment options for Row elements with fixed-width children
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowAlignment {
    /// Align children to the left with minimum gap
    Left,
    /// Spread children across available width, maximizing gap
    #[default]
    Spread,
    /// Center children with minimum gap between them
    Center,
}
