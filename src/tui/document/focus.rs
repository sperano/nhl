//! Focus management for document navigation
//!
//! Provides Tab/Shift-Tab navigation through focusable elements within documents.
//! Tracks focus state and provides methods for navigating and activating elements.

use ratatui::layout::Rect;

use super::link::LinkTarget;

/// Position of an element within a Row for left/right navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowPosition {
    /// Y position that uniquely identifies the Row
    pub row_y: u16,
    /// Index of the child container within the Row (0 = leftmost)
    pub child_idx: usize,
    /// Index within the child container
    pub idx_within_child: usize,
}

/// Type-safe identifier for focusable elements
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FocusableId {
    /// A table cell identified by table name, row, and column
    TableCell {
        table_name: String,
        row: usize,
        col: usize,
    },
    /// A standalone link with a string identifier
    /// TODO: Remove once all usages are migrated to typed variants (TeamLink, PlayerLink, GameLink)
    Link(String),
    /// A team link with the team abbreviation (e.g., "TOR", "BOS")
    TeamLink(String),
    /// A player link with the player ID
    PlayerLink(i64),
    /// A game link with the game ID
    GameLink(i64),
}

impl FocusableId {
    /// Create a table cell ID
    pub fn table_cell(table_name: impl Into<String>, row: usize, col: usize) -> Self {
        Self::TableCell {
            table_name: table_name.into(),
            row,
            col,
        }
    }

    /// Create a link ID
    pub fn link(id: impl Into<String>) -> Self {
        Self::Link(id.into())
    }

    /// Create a team link ID
    pub fn team_link(abbrev: impl Into<String>) -> Self {
        Self::TeamLink(abbrev.into())
    }

    /// Create a player link ID
    pub fn player_link(player_id: i64) -> Self {
        Self::PlayerLink(player_id)
    }

    /// Create a game link ID
    pub fn game_link(game_id: i64) -> Self {
        Self::GameLink(game_id)
    }

    /// Format for user-friendly display
    pub fn display_name(&self) -> String {
        match self {
            Self::TableCell { row, .. } => format!("Table row {}", row + 1),
            Self::Link(id) => format_link_id(id),
            Self::TeamLink(abbrev) => format!("Team {}", abbrev),
            Self::PlayerLink(id) => format!("Player {}", id),
            Self::GameLink(id) => format!("Game {}", id),
        }
    }
}

/// Format a link ID for user-friendly display
fn format_link_id(id: &str) -> String {
    match id {
        "bos" => "Boston Bruins".to_string(),
        "tor" => "Toronto Maple Leafs".to_string(),
        "nyr" => "New York Rangers".to_string(),
        "mtl" => "Montreal Canadiens".to_string(),
        _ => id.to_string(),
    }
}

/// A focusable element within a document
#[derive(Debug, Clone, PartialEq)]
pub struct FocusableElement {
    /// Unique ID for this focusable element
    pub id: FocusableId,
    /// Y position in the document (for scrolling)
    pub y: u16,
    /// Height of the element
    pub height: u16,
    /// Rectangle of the focusable area (for highlighting)
    pub rect: Rect,
    /// Optional link target if this is a link
    pub link_target: Option<LinkTarget>,
    /// Row membership for left/right navigation within Row elements
    pub row_position: Option<RowPosition>,
}

impl FocusableElement {
    /// Create a new focusable element
    pub fn new(
        id: FocusableId,
        y: u16,
        height: u16,
        rect: Rect,
        link_target: Option<LinkTarget>,
    ) -> Self {
        Self {
            id,
            y,
            height,
            rect,
            link_target,
            row_position: None,
        }
    }

    /// Test-only builder: a minimal focusable element with just position,
    /// height, and ID set (a synthetic 1-wide `rect`, no row position or link
    /// target). Chain `.with_link_target(..)` / `.with_row_position(..)` to
    /// add those, so `DocumentNavState.focusables` fixtures stay one-liners
    /// instead of hand-populating five parallel Vecs.
    #[cfg(test)]
    pub fn at(y: u16, height: u16, id: FocusableId) -> Self {
        Self {
            id,
            y,
            height,
            rect: Rect::new(0, y, 1, height),
            link_target: None,
            row_position: None,
        }
    }

    /// Attach a link target (test builder, see [`Self::at`]).
    #[cfg(test)]
    pub fn with_link_target(mut self, target: LinkTarget) -> Self {
        self.link_target = Some(target);
        self
    }

    /// Attach a row position (test builder, see [`Self::at`]).
    #[cfg(test)]
    pub fn with_row_position(mut self, row_position: RowPosition) -> Self {
        self.row_position = Some(row_position);
        self
    }
}

/// Builds the focusable list from a document's element tree and tracks which
/// one is focused.
///
/// This is the render path's only consumer of focus state: `DocumentView`
/// (the render shim in `document/mod.rs`) uses it to look up the currently
/// focused element's ID when constructing the `FocusContext` for the next
/// frame, and to apply the focus index carried over from `DocumentNavState`.
///
/// All navigation semantics -- next/prev, wrapping, scrolling, paging,
/// autoscroll -- live in `document_nav.rs`. This type does not implement any
/// of them; it is pure "given an index, what's focused" bookkeeping.
#[derive(Debug, Clone)]
pub struct FocusManager {
    /// All focusable elements in tab order
    elements: Vec<FocusableElement>,
    /// Currently focused element index (None = no focus)
    current_focus: Option<usize>,
}

impl FocusManager {
    /// Build a focus manager from a list of document elements.
    ///
    /// Elements are collected in document order (top to bottom, left to right for rows).
    pub fn from_elements(elements: &[super::elements::DocumentElement]) -> Self {
        let mut focusable = Vec::new();
        let mut y_offset = 0u16;

        for element in elements {
            element.collect_focusable(&mut focusable, y_offset);
            y_offset += element.height();
        }

        Self {
            elements: focusable,
            current_focus: None,
        }
    }

    /// Focus a specific element by index.
    ///
    /// Returns true if the index was valid. An invalid index leaves the
    /// current focus unchanged.
    pub fn focus_by_index(&mut self, index: usize) -> bool {
        if index < self.elements.len() {
            self.current_focus = Some(index);
            true
        } else {
            false
        }
    }

    /// Get the currently focused element's ID.
    pub fn get_current_id(&self) -> Option<&FocusableId> {
        self.current_focus.map(|idx| &self.elements[idx].id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document::elements::DocumentElement;
    use crate::tui::document::link::LinkTarget;

    #[test]
    fn test_from_elements_no_focus_by_default() {
        let elements = vec![DocumentElement::link(
            "a",
            "A",
            LinkTarget::Anchor("a".to_string()),
        )];
        let fm = FocusManager::from_elements(&elements);

        assert_eq!(fm.get_current_id(), None);
    }

    #[test]
    fn test_focus_by_index() {
        let elements = vec![
            DocumentElement::link("a", "A", LinkTarget::Anchor("a".to_string())),
            DocumentElement::link("b", "B", LinkTarget::Anchor("b".to_string())),
            DocumentElement::link("c", "C", LinkTarget::Anchor("c".to_string())),
        ];
        let mut fm = FocusManager::from_elements(&elements);

        assert!(fm.focus_by_index(2));
        assert_eq!(fm.get_current_id(), Some(&FocusableId::link("c")));

        // Invalid index returns false but leaves focus unchanged
        assert!(!fm.focus_by_index(10));
        assert_eq!(fm.get_current_id(), Some(&FocusableId::link("c")));
    }

    #[test]
    fn test_from_elements_builds_focusable_list_in_document_order() {
        let elements = vec![
            DocumentElement::link("a", "A", LinkTarget::Anchor("a".to_string())),
            DocumentElement::link("b", "B", LinkTarget::Anchor("b".to_string())),
        ];
        let mut fm = FocusManager::from_elements(&elements);

        assert!(fm.focus_by_index(0));
        assert_eq!(fm.get_current_id(), Some(&FocusableId::link("a")));

        assert!(fm.focus_by_index(1));
        assert_eq!(fm.get_current_id(), Some(&FocusableId::link("b")));
    }
}
