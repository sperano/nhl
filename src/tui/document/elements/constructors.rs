//! Constructors for the simpler `DocumentElement` variants, plus the more
//! involved `team_boxscore()` builder. Table and row constructors, which carry
//! their own larger test suites, live in `table_constructor.rs` and
//! `row_constructor.rs`.

use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::tui::component::ElementWidget;
use crate::tui::components::TableWidget;
use crate::tui::document::focus::{FocusableElement, FocusableId};
use crate::tui::document::link::LinkTarget;
use crate::tui::document::FocusContext;
use crate::tui::types::StackedDocument;
use crate::tui::widgets::{BigScore, BigScoreParams, ScoreBox};

use super::render::{TEAM_BOXSCORE_SECTION_HEADER_ROWS, TEAM_BOXSCORE_SECTION_TRAILING_BLANK};
use super::types::TABLE_COLUMN_HEADER_HEIGHT;
use super::{DocTabDef, DocumentElement};

impl DocumentElement {
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

    /// Create a score box element
    ///
    /// # Arguments
    /// - `game_id`: The NHL API game ID (used for activation and as part of the element ID)
    /// - `score_box`: The ScoreBox widget containing score data
    /// - `focused`: Whether this score box is currently focused
    /// - `link_target`: Destination pushed when this box is activated
    pub fn score_box_element(
        game_id: i64,
        score_box: ScoreBox,
        focused: bool,
        link_target: LinkTarget,
    ) -> Self {
        Self::ScoreBoxElement {
            id: format!("scorebox_{}", game_id),
            game_id,
            score_box,
            focused,
            link_target,
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

        // Calculate y offset for each section's focusable elements, mirroring
        // the section layout in render::render_team_boxscore via the shared
        // TEAM_BOXSCORE_SECTION_* constants.
        let mut current_y: u16 = 0;

        // Forwards section
        if forwards_table.row_count() > 0 {
            let table_name = format!("{}_forwards", table_prefix);
            current_y += TEAM_BOXSCORE_SECTION_HEADER_ROWS;
            let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

            for row_idx in 0..forwards_table.row_count() {
                for col_idx in 0..forwards_table.column_count() {
                    if let Some(cell) = forwards_table.get_cell_value(row_idx, col_idx) {
                        let y = data_start_y + row_idx as u16;
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
            current_y += forwards_table.preferred_height().unwrap_or(0)
                + TEAM_BOXSCORE_SECTION_TRAILING_BLANK;
        }

        // Defense section
        if defense_table.row_count() > 0 {
            let table_name = format!("{}_defense", table_prefix);
            current_y += TEAM_BOXSCORE_SECTION_HEADER_ROWS;
            let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

            for row_idx in 0..defense_table.row_count() {
                for col_idx in 0..defense_table.column_count() {
                    if let Some(cell) = defense_table.get_cell_value(row_idx, col_idx) {
                        let y = data_start_y + row_idx as u16;
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
            current_y += defense_table.preferred_height().unwrap_or(0)
                + TEAM_BOXSCORE_SECTION_TRAILING_BLANK;
        }

        // Goalies section
        if goalies_table.row_count() > 0 {
            let table_name = format!("{}_goalies", table_prefix);
            current_y += TEAM_BOXSCORE_SECTION_HEADER_ROWS;
            let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

            for row_idx in 0..goalies_table.row_count() {
                for col_idx in 0..goalies_table.column_count() {
                    if let Some(cell) = goalies_table.get_cell_value(row_idx, col_idx) {
                        let y = data_start_y + row_idx as u16;
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

    /// Create a big score element
    pub fn big_score(params: BigScoreParams) -> Self {
        Self::BigScoreElement {
            big_score: BigScore::new(params),
        }
    }

    /// Create a tabbed panel element
    ///
    /// # Arguments
    /// - `id`: Unique identifier for this tabs element (used for state tracking)
    /// - `tabs`: Vec of tab definitions
    /// - `active_index`: Index of the initially active tab
    pub fn tabs(id: impl Into<String>, tabs: Vec<DocTabDef>, active_index: usize) -> Self {
        Self::Tabs {
            id: id.into(),
            tabs,
            active_index,
        }
    }

    /// Create a tabbed panel from the focus context
    ///
    /// The active index is read from the focus context's tab_selections map.
    /// Falls back to 0 if not found.
    pub fn tabs_from_context(
        id: impl Into<String>,
        tabs: Vec<DocTabDef>,
        focus: &FocusContext,
    ) -> Self {
        let id = id.into();
        let active_index = focus
            .tab_selections
            .get(&id)
            .copied()
            .unwrap_or(0)
            .min(tabs.len().saturating_sub(1));
        Self::Tabs {
            id,
            tabs,
            active_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

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
        let elem = DocumentElement::styled_group(vec![DocumentElement::text("Content")], style);

        match elem {
            DocumentElement::Group { style: s, .. } => assert_eq!(s, Some(style)),
            _ => panic!("Expected Group"),
        }
    }
}
