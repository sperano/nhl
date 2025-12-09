//! Document elements that can be rendered in a document
//!
//! Provides various element types (text, headings, tables, links, etc.)
//! that can be composed to build documents.

mod render;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::config::DisplayConfig;
use crate::tui::component::ElementWidget;
use crate::tui::components::TableWidget;
use crate::tui::widgets::{ScoreBox, StandaloneWidget};

use super::focus::{FocusableElement, FocusableId, RowPosition};
use super::link::LinkTarget;

/// Alignment options for Row elements with fixed-width children
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowAlignment {
    /// Align children to the left with minimum gap
    Left,
    /// Spread children across available width, maximizing gap
    #[default]
    Spread,
}

use render::{render_group, render_heading, render_link, render_row, render_section_title, render_separator, render_team_boxscore, render_text};

pub use render::TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH;

/// Height of column headers section (column names + separator)
pub(crate) const TABLE_COLUMN_HEADER_HEIGHT: u16 = 2;

/// Elements that can be part of a document
#[derive(Clone)]
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
    SectionTitle {
        content: String,
        underline: bool,
    },

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
        render_fn: fn(Rect, &mut Buffer, &DisplayConfig),
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
            Self::Custom { height, focusable, .. } => f
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
            Self::Row { children, gap, align } => f
                .debug_struct("Row")
                .field("children", &children.len())
                .field("gap", gap)
                .field("align", align)
                .finish(),
            Self::ScoreBoxElement { id, game_id, focused, .. } => f
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
            Self::TeamBoxscore { team_name, focusable, .. } => f
                .debug_struct("TeamBoxscore")
                .field("team_name", team_name)
                .field("focusable_count", &focusable.len())
                .finish(),
        }
    }
}

impl DocumentElement {
    /// Calculate the height this element needs
    pub fn height(&self) -> u16 {
        match self {
            Self::Text { content, .. } => {
                // Count lines in text (minimum 1)
                content.lines().count().max(1) as u16
            }
            Self::Heading { level, .. } => {
                // Level 1 headings have underline
                if *level == 1 {
                    2
                } else {
                    1
                }
            }
            Self::SectionTitle { underline, .. } => {
                // title + optional underline + blank line
                if *underline { 3 } else { 2 }
            }
            Self::Link { .. } => 1,
            Self::Separator => 1,
            Self::Spacer { height } => *height,
            Self::Group { children, .. } => children.iter().map(|c| c.height()).sum(),
            Self::Custom { height, .. } => *height,
            Self::Table { widget, .. } => widget.preferred_height().unwrap_or(0),
            Self::Row { children, .. } => {
                // Height is the maximum height of all children (side by side)
                children.iter().map(|c| c.height()).max().unwrap_or(0)
            }
            Self::ScoreBoxElement { score_box, .. } => {
                // ScoreBox has fixed height of 6
                score_box.preferred_height().unwrap_or(6)
            }
            Self::Indented { element, .. } => {
                // Same height as inner element
                element.height()
            }
            Self::TeamBoxscore {
                forwards_table,
                defense_table,
                goalies_table,
                ..
            } => {
                // Height = section headers + tables + spacing + bottom border
                // Each section: 1 (header) + 1 (blank) + table_height + 1 (blank before next)
                // Final section has bottom border instead of blank
                let section_chrome = 3; // header + blank after header + blank before next section
                let forwards_height = if forwards_table.row_count() > 0 {
                    section_chrome + forwards_table.preferred_height().unwrap_or(0)
                } else {
                    0
                };
                let defense_height = if defense_table.row_count() > 0 {
                    section_chrome + defense_table.preferred_height().unwrap_or(0)
                } else {
                    0
                };
                let goalies_height = if goalies_table.row_count() > 0 {
                    section_chrome + goalies_table.preferred_height().unwrap_or(0)
                } else {
                    0
                };
                let bottom_border_height = 1;
                forwards_height + defense_height + goalies_height + bottom_border_height
            }
        }
    }

    /// Collect focusable elements from this element
    ///
    /// # Arguments
    /// - `out`: Vector to append focusable elements to
    /// - `y_offset`: Current y offset in the document
    pub fn collect_focusable(&self, out: &mut Vec<FocusableElement>, y_offset: u16) {
        match self {
            Self::Link {
                display,
                target,
                id,
                ..
            } => {
                out.push(FocusableElement {
                    id: FocusableId::link(id),
                    y: y_offset,
                    height: 1,
                    rect: Rect::new(0, y_offset, display.chars().count() as u16, 1),
                    link_target: Some(target.clone()),
                    row_position: None,
                });
            }
            Self::Group { children, .. } => {
                let mut child_offset = y_offset;
                for child in children {
                    child.collect_focusable(out, child_offset);
                    child_offset += child.height();
                }
            }
            Self::Custom { focusable, .. } => {
                // Add focusable elements with adjusted y positions
                for elem in focusable {
                    let mut adjusted = elem.clone();
                    adjusted.y += y_offset;
                    adjusted.rect.y += y_offset;
                    out.push(adjusted);
                }
            }
            Self::Table { focusable, .. } => {
                for elem in focusable {
                    let mut adjusted = elem.clone();
                    adjusted.y += y_offset;
                    adjusted.rect.y += y_offset;
                    out.push(adjusted);
                }
            }
            Self::Row { children, .. } => {
                // Collect left to right - all elements from first child, then second, etc.
                // Set row_position so left/right navigation can jump between children
                for (child_idx, child) in children.iter().enumerate() {
                    let start_idx = out.len();
                    child.collect_focusable(out, y_offset);
                    // Tag each element with its row position
                    for (idx_within_child, elem) in out[start_idx..].iter_mut().enumerate() {
                        elem.row_position = Some(RowPosition {
                            row_y: y_offset,
                            child_idx,
                            idx_within_child,
                        });
                    }
                }
            }
            Self::ScoreBoxElement { game_id, score_box, .. } => {
                // ScoreBox is a single focusable element with typed GameLink ID
                let height = score_box.preferred_height().unwrap_or(6);
                let width = score_box.preferred_width().unwrap_or(25);
                out.push(FocusableElement {
                    id: FocusableId::game_link(*game_id),
                    y: y_offset,
                    height,
                    rect: Rect::new(0, y_offset, width, height),
                    link_target: Some(LinkTarget::Action(format!("open_boxscore_{}", game_id))),
                    row_position: None,
                });
            }
            Self::Indented { element, .. } => {
                // Delegate to inner element (margin doesn't affect focusable collection)
                element.collect_focusable(out, y_offset);
            }
            Self::TeamBoxscore { focusable, .. } => {
                // Add focusable elements with adjusted y positions
                for elem in focusable {
                    let mut adjusted = elem.clone();
                    adjusted.y += y_offset;
                    adjusted.rect.y += y_offset;
                    out.push(adjusted);
                }
            }
            _ => {}
        }
    }

    /// Collect focusable element IDs from this element (simpler version for display)
    ///
    /// # Arguments
    /// - `out`: Vector to append IDs to
    /// - `y_offset`: Current y offset for tracking position in document
    pub fn collect_focusable_ids(&self, out: &mut Vec<FocusableId>, y_offset: u16) {
        match self {
            Self::Link { id, .. } => {
                out.push(FocusableId::link(id));
            }
            Self::Group { children, .. } => {
                let mut child_offset = y_offset;
                for child in children {
                    child.collect_focusable_ids(out, child_offset);
                    child_offset += child.height();
                }
            }
            Self::Custom { focusable, .. } | Self::Table { focusable, .. } => {
                for elem in focusable {
                    out.push(elem.id.clone());
                }
            }
            Self::Row { children, .. } => {
                for child in children {
                    child.collect_focusable_ids(out, y_offset);
                }
            }
            Self::ScoreBoxElement { game_id, .. } => {
                out.push(FocusableId::game_link(*game_id));
            }
            Self::Indented { element, .. } => {
                element.collect_focusable_ids(out, y_offset);
            }
            Self::TeamBoxscore { focusable, .. } => {
                for elem in focusable {
                    out.push(elem.id.clone());
                }
            }
            _ => {}
        }
    }

    /// Render this element to a buffer
    pub fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        match self {
            Self::Text { content, style } => {
                render_text(content, *style, area, buf, config);
            }
            Self::Heading { level, content } => {
                render_heading(*level, content, area, buf, config);
            }
            Self::SectionTitle { content, underline } => {
                render_section_title(content, *underline, area, buf, config);
            }
            Self::Link {
                display, focused, ..
            } => {
                render_link(display, *focused, area, buf, config);
            }
            Self::Separator => {
                render_separator(area, buf, config);
            }
            Self::Spacer { .. } => {
                // Just empty space, nothing to render
            }
            Self::Group { children, style } => {
                render_group(children, *style, area, buf, config);
            }
            Self::Custom { render_fn, .. } => {
                render_fn(area, buf, config);
            }
            Self::Table { widget, .. } => {
                widget.render(area, buf, config);
            }
            Self::Row { children, gap, align } => {
                render_row(children, *gap, *align, area, buf, config);
            }
            Self::ScoreBoxElement { score_box, focused, .. } => {
                // Clone and set selection based on focus state
                let mut box_to_render = score_box.clone();
                box_to_render.selected = *focused;
                box_to_render.render(area, buf, config);
            }
            Self::Indented { element, margin } => {
                // Render inner element with adjusted area (shifted right by margin)
                if area.width > *margin {
                    let indented_area = Rect::new(
                        area.x + margin,
                        area.y,
                        area.width - margin,
                        area.height,
                    );
                    element.render(indented_area, buf, config);
                }
            }
            Self::TeamBoxscore {
                team_name,
                forwards_table,
                defense_table,
                goalies_table,
                ..
            } => {
                render_team_boxscore(
                    team_name,
                    forwards_table,
                    defense_table,
                    goalies_table,
                    area,
                    buf,
                    config,
                );
            }
        }
    }

    /// Create a text element
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
            style: None,
        }
    }

    /// Create a styled text element
    pub fn styled_text(content: impl Into<String>, style: Style) -> Self {
        Self::Text {
            content: content.into(),
            style: Some(style),
        }
    }

    /// Create a heading element
    pub fn heading(level: u8, content: impl Into<String>) -> Self {
        Self::Heading {
            level: level.clamp(1, 6),
            content: content.into(),
        }
    }

    /// Create a section title element (bold text with optional underline)
    ///
    /// Used for division/conference names in standings tables.
    pub fn section_title(content: impl Into<String>, underline: bool) -> Self {
        Self::SectionTitle {
            content: content.into(),
            underline,
        }
    }

    /// Create a link element
    pub fn link(id: impl Into<String>, display: impl Into<String>, target: LinkTarget) -> Self {
        Self::Link {
            id: id.into(),
            display: display.into(),
            target,
            focused: false,
        }
    }

    /// Create a focused link element
    pub fn focused_link(
        id: impl Into<String>,
        display: impl Into<String>,
        target: LinkTarget,
    ) -> Self {
        Self::Link {
            id: id.into(),
            display: display.into(),
            target,
            focused: true,
        }
    }

    /// Create a separator element
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Create a spacer element
    pub fn spacer(height: u16) -> Self {
        Self::Spacer { height }
    }

    /// Create a group element
    pub fn group(children: Vec<DocumentElement>) -> Self {
        Self::Group {
            children,
            style: None,
        }
    }

    /// Create a styled group element
    pub fn styled_group(children: Vec<DocumentElement>, style: Style) -> Self {
        Self::Group {
            children,
            style: Some(style),
        }
    }

    /// Wrap an element with left margin
    ///
    /// The inner element is rendered with the specified left margin in characters.
    pub fn indented(element: DocumentElement, margin: u16) -> Self {
        Self::Indented {
            element: Box::new(element),
            margin,
        }
    }

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
                        CellValue::PlayerLink { player_id, .. } => {
                            Some(LinkTarget::Action(format!("player:{}", player_id)))
                        }
                        CellValue::TeamLink { team_abbrev, .. } => {
                            Some(LinkTarget::Action(format!("team:{}", team_abbrev)))
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

    /// Create a horizontal row of elements (side by side)
    ///
    /// Elements are laid out horizontally. Fixed-width children are spread
    /// across available width by default (maximizing gap).
    pub fn row(children: Vec<DocumentElement>) -> Self {
        Self::Row { children, gap: 2, align: RowAlignment::Spread }
    }

    /// Create a horizontal row with custom gap
    pub fn row_with_gap(children: Vec<DocumentElement>, gap: u16) -> Self {
        Self::Row { children, gap, align: RowAlignment::Spread }
    }

    /// Create a horizontal row with left alignment (no gap maximization)
    pub fn row_left(children: Vec<DocumentElement>) -> Self {
        Self::Row { children, gap: 2, align: RowAlignment::Left }
    }

    /// Create a horizontal row with left alignment and custom gap
    pub fn row_left_with_gap(children: Vec<DocumentElement>, gap: u16) -> Self {
        Self::Row { children, gap, align: RowAlignment::Left }
    }

    /// Create a score box element
    ///
    /// # Arguments
    /// - `game_id`: The NHL API game ID (used for activation and as part of the element ID)
    /// - `score_box`: The ScoreBox widget containing score data
    /// - `focused`: Whether this score box is currently focused
    pub fn score_box_element(game_id: i64, score_box: ScoreBox, focused: bool) -> Self {
        Self::ScoreBoxElement {
            id: format!("scorebox_{}", game_id),
            game_id,
            score_box,
            focused,
        }
    }

    /// Create a team boxscore element with decorative borders
    ///
    /// Wraps three tables (forwards, defense, goalies) with section headers
    /// and decorative box borders.
    ///
    /// # Arguments
    /// - `table_prefix`: Prefix for table names (e.g., "away" or "home")
    /// - `team_name`: Team name for section headers (e.g., "Avalanche")
    /// - `forwards_table`: TableWidget for forwards stats
    /// - `defense_table`: TableWidget for defense stats
    /// - `goalies_table`: TableWidget for goalies stats
    pub fn team_boxscore(
        table_prefix: &str,
        team_name: impl Into<String>,
        forwards_table: TableWidget,
        defense_table: TableWidget,
        goalies_table: TableWidget,
    ) -> Self {
        use crate::tui::CellValue;

        let team_name = team_name.into();
        let mut focusable = Vec::new();

        // Calculate y offset for each section's focusable elements
        // Each section: 1 (header) + 1 (blank) + table_height + 1 (blank before next)
        let mut current_y: u16 = 0;

        // Forwards section
        if forwards_table.row_count() > 0 {
            let table_name = format!("{}_forwards", table_prefix);
            current_y += 2; // header + blank after header
            let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

            for row_idx in 0..forwards_table.row_count() {
                for col_idx in 0..forwards_table.column_count() {
                    if let Some(cell) = forwards_table.get_cell_value(row_idx, col_idx) {
                        let y = data_start_y + row_idx as u16;
                        let link_target = match &cell {
                            CellValue::PlayerLink { player_id, .. } => {
                                Some(LinkTarget::Action(format!("player:{}", player_id)))
                            }
                            _ => continue,
                        };
                        focusable.push(FocusableElement {
                            id: FocusableId::table_cell(&table_name, row_idx, col_idx),
                            y,
                            height: 1,
                            rect: Rect::new(0, y, cell.display_text().len() as u16, 1),
                            link_target,
                            row_position: None,
                        });
                    }
                }
            }
            current_y += forwards_table.preferred_height().unwrap_or(0) + 1; // table + blank before next
        }

        // Defense section
        if defense_table.row_count() > 0 {
            let table_name = format!("{}_defense", table_prefix);
            current_y += 2; // header + blank after header
            let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

            for row_idx in 0..defense_table.row_count() {
                for col_idx in 0..defense_table.column_count() {
                    if let Some(cell) = defense_table.get_cell_value(row_idx, col_idx) {
                        let y = data_start_y + row_idx as u16;
                        let link_target = match &cell {
                            CellValue::PlayerLink { player_id, .. } => {
                                Some(LinkTarget::Action(format!("player:{}", player_id)))
                            }
                            _ => continue,
                        };
                        focusable.push(FocusableElement {
                            id: FocusableId::table_cell(&table_name, row_idx, col_idx),
                            y,
                            height: 1,
                            rect: Rect::new(0, y, cell.display_text().len() as u16, 1),
                            link_target,
                            row_position: None,
                        });
                    }
                }
            }
            current_y += defense_table.preferred_height().unwrap_or(0) + 1;
        }

        // Goalies section
        if goalies_table.row_count() > 0 {
            let table_name = format!("{}_goalies", table_prefix);
            current_y += 2; // header + blank after header
            let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

            for row_idx in 0..goalies_table.row_count() {
                for col_idx in 0..goalies_table.column_count() {
                    if let Some(cell) = goalies_table.get_cell_value(row_idx, col_idx) {
                        let y = data_start_y + row_idx as u16;
                        let link_target = match &cell {
                            CellValue::PlayerLink { player_id, .. } => {
                                Some(LinkTarget::Action(format!("player:{}", player_id)))
                            }
                            _ => continue,
                        };
                        focusable.push(FocusableElement {
                            id: FocusableId::table_cell(&table_name, row_idx, col_idx),
                            y,
                            height: 1,
                            rect: Rect::new(0, y, cell.display_text().len() as u16, 1),
                            link_target,
                            row_position: None,
                        });
                    }
                }
            }
        }

        Self::TeamBoxscore {
            team_name,
            forwards_table,
            defense_table,
            goalies_table,
            focusable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document::link::DocumentLink;
    use ratatui::style::Color;

    #[test]
    fn test_text_element_height() {
        let elem = DocumentElement::text("Hello");
        assert_eq!(elem.height(), 1);

        let elem = DocumentElement::text("Line 1\nLine 2\nLine 3");
        assert_eq!(elem.height(), 3);

        let elem = DocumentElement::text("");
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_heading_element_height() {
        let elem = DocumentElement::heading(1, "Title");
        assert_eq!(elem.height(), 2); // Heading + underline

        let elem = DocumentElement::heading(2, "Subtitle");
        assert_eq!(elem.height(), 1);

        let elem = DocumentElement::heading(3, "Section");
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_link_element_height() {
        let elem = DocumentElement::link(
            "link1",
            "Click me",
            LinkTarget::Action("test".to_string()),
        );
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_separator_element_height() {
        let elem = DocumentElement::separator();
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_spacer_element_height() {
        let elem = DocumentElement::spacer(5);
        assert_eq!(elem.height(), 5);
    }

    #[test]
    fn test_group_element_height() {
        let elem = DocumentElement::group(vec![
            DocumentElement::text("Line 1"),
            DocumentElement::spacer(2),
            DocumentElement::text("Line 2"),
        ]);
        assert_eq!(elem.height(), 4); // 1 + 2 + 1
    }

    #[test]
    fn test_collect_focusable_link() {
        let elem = DocumentElement::link(
            "my_link",
            "Click here",
            LinkTarget::Document(DocumentLink::team("BOS")),
        );

        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 10);

        assert_eq!(focusable.len(), 1);
        assert_eq!(focusable[0].id, FocusableId::link("my_link"));
        assert_eq!(focusable[0].y, 10);
        assert_eq!(focusable[0].rect.width, 10); // "Click here" = 10 chars
    }

    #[test]
    fn test_collect_focusable_group() {
        let elem = DocumentElement::group(vec![
            DocumentElement::text("Not focusable"),
            DocumentElement::link("link1", "First", LinkTarget::Action("a".to_string())),
            DocumentElement::spacer(2),
            DocumentElement::link("link2", "Second", LinkTarget::Action("b".to_string())),
        ]);

        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 0);

        assert_eq!(focusable.len(), 2);
        assert_eq!(focusable[0].id, FocusableId::link("link1"));
        assert_eq!(focusable[0].y, 1); // After "Not focusable"
        assert_eq!(focusable[1].id, FocusableId::link("link2"));
        assert_eq!(focusable[1].y, 4); // 1 (text) + 1 (link) + 2 (spacer)
    }

    #[test]
    fn test_collect_focusable_nested_groups() {
        let elem = DocumentElement::group(vec![
            DocumentElement::group(vec![
                DocumentElement::link("inner1", "Inner", LinkTarget::Action("x".to_string())),
            ]),
            DocumentElement::link("outer1", "Outer", LinkTarget::Action("y".to_string())),
        ]);

        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 5);

        assert_eq!(focusable.len(), 2);
        assert_eq!(focusable[0].id, FocusableId::link("inner1"));
        assert_eq!(focusable[0].y, 5);
        assert_eq!(focusable[1].id, FocusableId::link("outer1"));
        assert_eq!(focusable[1].y, 6);
    }

    #[test]
    fn test_render_text() {
        let elem = DocumentElement::text("Hello");
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &config);

        // Check that "Hello" was rendered
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "H");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "e");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "l");
        assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "l");
        assert_eq!(buf.cell((4, 0)).unwrap().symbol(), "o");
    }

    #[test]
    fn test_render_heading_level_1() {
        let elem = DocumentElement::heading(1, "Title");
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &config);

        // Check heading text
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "T");
        // Check underline
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "═");
    }

    #[test]
    fn test_render_link() {
        let elem = DocumentElement::link(
            "test_link",
            "Click",
            LinkTarget::Action("test".to_string()),
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &config);

        // Unfocused links have "  " prefix for alignment
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "C");
        assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "l");
        // Style uses fg2 from theme (or default if no theme)
        // No specific color assertion since we use theme.fg2
    }

    #[test]
    fn test_render_focused_link() {
        let elem = DocumentElement::focused_link(
            "test_link",
            "Click",
            LinkTarget::Action("test".to_string()),
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &config);

        // Focused links have "▶ " prefix
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "▶");
        // Check that "Click" starts at position 2
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "C");
        // Focused links use BOLD + REVERSED modifiers
        let style = buf.cell((2, 0)).unwrap().style();
        assert!(style.add_modifier.contains(ratatui::style::Modifier::BOLD));
        assert!(style.add_modifier.contains(ratatui::style::Modifier::REVERSED));
    }

    #[test]
    fn test_render_separator() {
        let elem = DocumentElement::separator();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 10, 1), &mut buf, &config);

        // All cells should be horizontal line
        for x in 0..10 {
            let symbol = buf.cell((x, 0)).unwrap().symbol();
            assert!(symbol == "─" || symbol == "-"); // Unicode or ASCII
        }
    }

    #[test]
    fn test_render_spacer() {
        let elem = DocumentElement::spacer(3);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        // Fill buffer with 'X' first
        for y in 0..5 {
            for x in 0..10 {
                buf.cell_mut((x, y)).unwrap().set_char('X');
            }
        }
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 10, 3), &mut buf, &config);

        // Spacer doesn't change buffer - cells should still be 'X'
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "X");
    }

    #[test]
    fn test_styled_text() {
        let style = Style::default().fg(Color::Red);
        let elem = DocumentElement::styled_text("Red text", style);

        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 20, 1), &mut buf, &config);

        assert_eq!(buf.cell((0, 0)).unwrap().style().fg, Some(Color::Red));
    }

    #[test]
    fn test_heading_level_clamping() {
        let elem = DocumentElement::heading(0, "Zero");
        match elem {
            DocumentElement::Heading { level, .. } => assert_eq!(level, 1),
            _ => panic!("Expected Heading"),
        }

        let elem = DocumentElement::heading(10, "Ten");
        match elem {
            DocumentElement::Heading { level, .. } => assert_eq!(level, 6),
            _ => panic!("Expected Heading"),
        }
    }

    #[test]
    fn test_styled_group() {
        let style = Style::default().bg(Color::Blue);
        let elem = DocumentElement::styled_group(
            vec![DocumentElement::text("Content")],
            style,
        );

        match elem {
            DocumentElement::Group { style: s, .. } => assert_eq!(s, Some(style)),
            _ => panic!("Expected Group"),
        }
    }

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

    #[test]
    fn test_table_element_height() {
        use crate::tui::{Alignment, CellValue, ColumnDef};
        use crate::tui::components::TableWidget;

        // Create a simple table with 3 rows
        let columns: Vec<ColumnDef<&str>> = vec![
            ColumnDef::new("Name", 10, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            }),
        ];
        let data = vec!["Alice", "Bob", "Charlie"];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("test_table", table);

        // TableWidget height = col headers (1) + separator (1) + 3 data rows = 5
        assert_eq!(elem.height(), 5);
    }

    #[test]
    fn test_table_element_focusable_extraction() {
        use crate::tui::{Alignment, CellValue, ColumnDef};
        use crate::tui::components::TableWidget;

        // Create a table with link cells
        let columns: Vec<ColumnDef<(&str, &str)>> = vec![
            ColumnDef::new("Team", 15, Alignment::Left, |row: &(&str, &str)| {
                CellValue::TeamLink {
                    display: row.0.to_string(),
                    team_abbrev: row.1.to_string(),
                }
            }),
        ];
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
            Some(LinkTarget::Action(action)) => assert_eq!(action, "team:BOS"),
            _ => panic!("Expected team action"),
        }
        match &focusable[1].link_target {
            Some(LinkTarget::Action(action)) => assert_eq!(action, "team:TOR"),
            _ => panic!("Expected team action"),
        }
    }

    #[test]
    fn test_table_element_focusable_y_positions() {
        use crate::tui::{Alignment, CellValue, ColumnDef};
        use crate::tui::components::TableWidget;

        // Create a table with links
        let columns: Vec<ColumnDef<&str>> = vec![
            ColumnDef::new("Player", 15, Alignment::Left, |row: &&str| {
                CellValue::PlayerLink {
                    display: row.to_string(),
                    player_id: 12345,
                }
            }),
        ];
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
        use crate::tui::{Alignment, CellValue, ColumnDef};
        use crate::tui::components::TableWidget;

        // Create a table with only text cells (no links)
        let columns: Vec<ColumnDef<i32>> = vec![
            ColumnDef::new("Value", 5, Alignment::Right, |row: &i32| {
                CellValue::Text(row.to_string())
            }),
        ];
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
        use crate::tui::{Alignment, CellValue, ColumnDef};
        use crate::tui::components::TableWidget;

        let columns: Vec<ColumnDef<&str>> = vec![
            ColumnDef::new("Col1", 10, Alignment::Left, |_: &&str| CellValue::Text("x".to_string())),
            ColumnDef::new("Col2", 10, Alignment::Left, |_: &&str| CellValue::Text("y".to_string())),
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
        use crate::tui::{Alignment, CellValue, ColumnDef};
        use crate::tui::components::TableWidget;
        use crate::tui::testing::assert_buffer;

        let columns: Vec<ColumnDef<&str>> = vec![
            ColumnDef::new("Name", 10, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            }),
        ];
        let data = vec!["Alice", "Bob"];
        let table = TableWidget::from_data(&columns, data);
        let elem = DocumentElement::table("test_table", table);

        let mut buf = Buffer::empty(Rect::new(0, 0, 15, 5));
        let config = DisplayConfig::default();

        elem.render(Rect::new(0, 0, 15, 5), &mut buf, &config);

        // Verify the table renders with margin, column header and data
        // TableWidget adds 2 space margin on left
        assert_buffer(
            &buf,
            &[
                "  Name",
                "  ──────────",
                "  Alice",
                "  Bob",
                "",
            ],
        );
    }

    #[test]
    fn test_row_spread_alignment_maximizes_gap() {
        use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};

        // Create two score boxes (each 25 chars wide)
        let score_box1 = ScoreBox::new("Team A", "Team B", Some(3), Some(2), ScoreBoxStatus::Final { overtime: false, shootout: false });
        let score_box2 = ScoreBox::new("Team C", "Team D", Some(1), Some(4), ScoreBoxStatus::Final { overtime: false, shootout: false });

        // Row with Spread alignment (default)
        let row = DocumentElement::row(vec![
            DocumentElement::score_box_element(1, score_box1.clone(), false),
            DocumentElement::score_box_element(2, score_box2.clone(), false),
        ]);

        // With area of 60 wide: 25 + 25 = 50, leaving 10 for gap
        // Spread should put gap of 10 between them
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 6));
        let config = DisplayConfig::default();
        row.render(Rect::new(0, 0, 60, 6), &mut buf, &config);

        // Second box should start at position 35 (25 + 10 gap)
        // Check the status line of the second box
        let line0 = (0..60).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        // First box status starts at 1, second should start around 35+1=36
        assert!(line0[36..].trim_start().starts_with("Final"), "Second box should start around position 36, got: '{}'", line0);
    }

    #[test]
    fn test_row_left_alignment_uses_minimum_gap() {
        use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};

        // Create two score boxes (each 25 chars wide)
        let score_box1 = ScoreBox::new("Team A", "Team B", Some(3), Some(2), ScoreBoxStatus::Final { overtime: false, shootout: false });
        let score_box2 = ScoreBox::new("Team C", "Team D", Some(1), Some(4), ScoreBoxStatus::Final { overtime: false, shootout: false });

        // Row with Left alignment
        let row = DocumentElement::row_left(vec![
            DocumentElement::score_box_element(1, score_box1.clone(), false),
            DocumentElement::score_box_element(2, score_box2.clone(), false),
        ]);

        // With area of 60 wide, Left alignment should use minimum gap (2)
        // Second box should start at 25 + 2 = 27
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 6));
        let config = DisplayConfig::default();
        row.render(Rect::new(0, 0, 60, 6), &mut buf, &config);

        // Check that second box status line starts near position 27
        let line0 = (0..60).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        // Second box status should start at 27+1=28 (with space prefix)
        assert!(line0[28..].trim_start().starts_with("Final"), "Second box should start at position 28 with Left alignment, got: '{}'", line0);
    }

    #[test]
    fn test_row_spread_respects_minimum_gap() {
        use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};

        // Create two score boxes (each 25 chars wide)
        let score_box1 = ScoreBox::new("Team A", "Team B", Some(3), Some(2), ScoreBoxStatus::Final { overtime: false, shootout: false });
        let score_box2 = ScoreBox::new("Team C", "Team D", Some(1), Some(4), ScoreBoxStatus::Final { overtime: false, shootout: false });

        // Row with minimum gap of 5
        let row = DocumentElement::row_with_gap(vec![
            DocumentElement::score_box_element(1, score_box1.clone(), false),
            DocumentElement::score_box_element(2, score_box2.clone(), false),
        ], 5);

        // Area of 52: 25 + 25 = 50, leaving only 2 for gap
        // But minimum gap is 5, so it should use 5
        let mut buf = Buffer::empty(Rect::new(0, 0, 52, 6));
        let config = DisplayConfig::default();
        row.render(Rect::new(0, 0, 52, 6), &mut buf, &config);

        // Second box should start at position 30 (25 + 5 minimum gap)
        let line0 = (0..52).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        assert!(line0[31..].trim_start().starts_with("Final"), "Second box should start at position 31 with minimum gap of 5, got: '{}'", line0);
    }

    #[test]
    fn test_row_alignment_default_is_spread() {
        let row = DocumentElement::row(vec![DocumentElement::text("test")]);
        match row {
            DocumentElement::Row { align, .. } => {
                assert_eq!(align, RowAlignment::Spread);
            }
            _ => panic!("Expected Row variant"),
        }
    }

    #[test]
    fn test_row_left_alignment_variant() {
        let row = DocumentElement::row_left(vec![DocumentElement::text("test")]);
        match row {
            DocumentElement::Row { align, .. } => {
                assert_eq!(align, RowAlignment::Left);
            }
            _ => panic!("Expected Row variant"),
        }
    }
}
