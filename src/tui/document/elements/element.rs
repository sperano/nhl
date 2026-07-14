//! The `DocumentElement` enum: the core data model for renderable document content.
//!
//! Behavior (height, focusable collection, rendering) lives in `behavior.rs`;
//! constructors live in `constructors.rs`, `table_constructor.rs`, and `row_constructor.rs`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::config::RenderContext;
use crate::tui::components::TableWidget;
use crate::tui::document::focus::FocusableElement;
use crate::tui::document::link::LinkTarget;
use crate::tui::widgets::{BigScore, ScoreBox};

use super::{DocTabDef, RowAlignment};

/// Elements that can be part of a document
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum DocumentElement {
    /// Plain text paragraph
    Text {
        content: String,
        style: Option<Style>,
    },

    /// Heading (different levels)
    Heading {
        level: u8, // 1-6
        content: String,
    },

    /// Section title (for division/conference names)
    ///
    /// Renders as bold text, optionally with underline, always followed by blank line:
    /// ```text
    /// Atlantic
    /// ════════
    /// (blank)
    /// ```
    /// Height is 2 (no underline) or 3 (with underline).
    SectionTitle { content: String, underline: bool },

    /// A link that can be focused and activated
    Link {
        display: String,
        target: LinkTarget,
        id: String,
        /// Whether this link is currently focused
        focused: bool,
    },

    /// Horizontal separator
    Separator,

    /// Vertical spacing
    Spacer { height: u16 },

    /// Container for grouping elements
    Group {
        children: Vec<DocumentElement>,
        style: Option<Style>,
    },

    /// Raw content with pre-calculated focusable elements
    /// Used for complex widgets like tables that manage their own focus
    Custom {
        /// Render function that draws to a buffer
        render_fn: fn(Rect, &mut Buffer, &RenderContext),
        /// Height of the element
        height: u16,
        /// Focusable elements within this custom element
        focusable: Vec<FocusableElement>,
    },

    /// A table widget rendered at natural height
    ///
    /// Tables are rendered using the existing TableWidget, which supports:
    /// - Column headers and alignment
    /// - Player and team links as focusable cells
    /// - Selection highlighting
    Table {
        /// The table widget (already contains all cell data)
        widget: TableWidget,
        /// Focusable elements extracted from link cells
        focusable: Vec<FocusableElement>,
    },

    /// Horizontal row of elements (side by side)
    ///
    /// Elements are laid out horizontally with equal width distribution.
    /// Height is determined by the tallest child element.
    Row {
        /// Child elements to render side by side
        children: Vec<DocumentElement>,
        /// Gap between elements in characters (minimum gap for Spread alignment)
        gap: u16,
        /// How to align fixed-width children horizontally
        align: RowAlignment,
    },

    /// A compact score box widget for displaying NHL game scores
    ///
    /// Fixed height of 6 rows (1 status + 5 box with double borders).
    /// Used in the score boxes grid for a compact view.
    ScoreBoxElement {
        /// Unique identifier for focus/activation (e.g., "scorebox_12345")
        id: String,
        /// Game ID for activation
        game_id: i64,
        /// The score box widget containing score data
        score_box: ScoreBox,
        /// Whether this score box is currently focused
        focused: bool,
        /// Destination pushed when this box is activated, attached by the
        /// caller at build time (it has the full game data in hand)
        link_target: LinkTarget,
    },

    /// Wrapper that adds left margin to any element
    ///
    /// Renders the inner element with the specified left margin (in characters).
    /// Height is the same as the inner element.
    Indented {
        /// The element to render with margin
        element: Box<DocumentElement>,
        /// Left margin in characters
        margin: u16,
    },

    /// Team boxscore with decorative borders
    ///
    /// Wraps three tables (forwards, defense, goalies) with section headers
    /// and decorative box borders. Fixed width of 85 characters.
    TeamBoxscore {
        /// Team name for section headers
        team_name: String,
        /// Forwards table
        forwards_table: TableWidget,
        /// Defense table
        defense_table: TableWidget,
        /// Goalies table
        goalies_table: TableWidget,
        /// Focusable elements collected from all three tables
        focusable: Vec<FocusableElement>,
    },

    /// Big score display using large digit font
    ///
    /// Renders score with team abbreviations and big digits:
    /// ```text
    /// NJD    BUF
    /// ▟▀▀▙    ▟▀▀▙
    ///  ▄▄▛ ──   ▗▛
    ///    █     ▗▛
    /// ▜▄▄▛    ▄█▄▄
    /// ```
    BigScoreElement {
        /// The BigScore widget
        big_score: BigScore,
    },

    /// Tabbed panel within a document
    ///
    /// Renders a tab bar with multiple tabs, showing only the active tab's content.
    /// Tab selection is managed via FocusContext and stored in DocumentNavState.
    ///
    /// Height = TAB_BAR_HEIGHT (2) + active tab content height
    Tabs {
        /// Unique identifier for this tabs element
        id: String,
        /// Tab definitions with their content
        tabs: Vec<DocTabDef>,
        /// Index of the currently active tab (0-based)
        active_index: usize,
    },
}

impl std::fmt::Debug for DocumentElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text { content, style } => f
                .debug_struct("Text")
                .field("content", content)
                .field("style", style)
                .finish(),
            Self::Heading { level, content } => f
                .debug_struct("Heading")
                .field("level", level)
                .field("content", content)
                .finish(),
            Self::SectionTitle { content, underline } => f
                .debug_struct("SectionTitle")
                .field("content", content)
                .field("underline", underline)
                .finish(),
            Self::Link {
                display,
                target,
                id,
                focused,
            } => f
                .debug_struct("Link")
                .field("display", display)
                .field("target", target)
                .field("id", id)
                .field("focused", focused)
                .finish(),
            Self::Separator => write!(f, "Separator"),
            Self::Spacer { height } => f.debug_struct("Spacer").field("height", height).finish(),
            Self::Group { children, style } => f
                .debug_struct("Group")
                .field("children", children)
                .field("style", style)
                .finish(),
            Self::Custom {
                height, focusable, ..
            } => f
                .debug_struct("Custom")
                .field("height", height)
                .field("focusable_count", &focusable.len())
                .finish(),
            Self::Table { widget, focusable } => f
                .debug_struct("Table")
                .field("rows", &widget.row_count())
                .field("columns", &widget.column_count())
                .field("focusable_count", &focusable.len())
                .finish(),
            Self::Row {
                children,
                gap,
                align,
            } => f
                .debug_struct("Row")
                .field("children", &children.len())
                .field("gap", gap)
                .field("align", align)
                .finish(),
            Self::ScoreBoxElement {
                id,
                game_id,
                focused,
                ..
            } => f
                .debug_struct("ScoreBoxElement")
                .field("id", id)
                .field("game_id", game_id)
                .field("focused", focused)
                .finish(),
            Self::Indented { element, margin } => f
                .debug_struct("Indented")
                .field("element", element)
                .field("margin", margin)
                .finish(),
            Self::TeamBoxscore {
                team_name,
                focusable,
                ..
            } => f
                .debug_struct("TeamBoxscore")
                .field("team_name", team_name)
                .field("focusable_count", &focusable.len())
                .finish(),
            Self::BigScoreElement { big_score } => f
                .debug_struct("BigScoreElement")
                .field("away", &big_score.away_name)
                .field("home", &big_score.home_name)
                .finish(),
            Self::Tabs {
                id,
                tabs,
                active_index,
            } => f
                .debug_struct("Tabs")
                .field("id", id)
                .field("tab_count", &tabs.len())
                .field("active_index", active_index)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_element_debug() {
        let elem = DocumentElement::text("Hello");
        let debug_str = format!("{:?}", elem);
        assert!(debug_str.contains("Text"));
        assert!(debug_str.contains("Hello"));

        let elem = DocumentElement::separator();
        let debug_str = format!("{:?}", elem);
        assert!(debug_str.contains("Separator"));
    }
}
