//! Constructors for the simpler `DocumentElement` variants, plus the more
//! involved `team_boxscore()` builder. Table and row constructors, which carry
//! their own larger test suites, live in `table_constructor.rs` and
//! `row_constructor.rs`.

use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

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
        let team_name = team_name.into();
        let mut focusable = Vec::new();

        // Walk the three sections in render order, mirroring the section
        // layout in render::render_team_boxscore via the shared
        // TEAM_BOXSCORE_SECTION_* constants.
        let mut current_y: u16 = 0;
        for (table, suffix) in [
            (&forwards_table, "forwards"),
            (&defense_table, "defense"),
            (&goalies_table, "goalies"),
        ] {
            current_y = collect_section_focusables(
                table,
                &format!("{table_prefix}_{suffix}"),
                current_y,
                &mut focusable,
            );
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

/// Walk one boxscore section's table for [`DocumentElement::team_boxscore`],
/// appending a `FocusableElement` for every `PlayerLink` cell, and return
/// `current_y` advanced past the section (header chrome, rows, trailing
/// blank). An empty section contributes no chrome and no advance, matching
/// `render::render_team_boxscore`.
fn collect_section_focusables(
    table: &TableWidget,
    table_name: &str,
    mut current_y: u16,
    focusable: &mut Vec<FocusableElement>,
) -> u16 {
    use crate::tui::CellValue;

    if table.row_count() == 0 {
        return current_y;
    }

    current_y += TEAM_BOXSCORE_SECTION_HEADER_ROWS;
    let data_start_y = current_y + TABLE_COLUMN_HEADER_HEIGHT;

    for row_idx in 0..table.row_count() {
        for col_idx in 0..table.column_count() {
            if let Some(cell) = table.get_cell_value(row_idx, col_idx) {
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
                    id: FocusableId::table_cell(table_name, row_idx, col_idx),
                    y,
                    height: 1,
                    rect: Rect::new(0, y, cell.display_text().width() as u16, 1),
                    link_target,
                    row_position: None,
                });
            }
        }
    }

    current_y + table.preferred_height().unwrap_or(0) + TEAM_BOXSCORE_SECTION_TRAILING_BLANK
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
    fn test_team_boxscore_focusable_rect_uses_display_width() {
        use crate::tui::{Alignment, CellValue, ColumnDef};

        fn player_columns() -> Vec<ColumnDef<&'static str>> {
            vec![ColumnDef::new(
                "Player",
                20,
                Alignment::Left,
                |name: &&str| CellValue::PlayerLink {
                    display: name.to_string(),
                    player_id: 1,
                    sweater_number: None,
                    last_name: name.to_string(),
                },
            )]
        }

        // "Génie" is 6 bytes but 5 display columns; the old byte-length rect
        // was one column too wide.
        let elem = DocumentElement::team_boxscore(
            "away",
            "T",
            TableWidget::from_data(&player_columns(), vec!["Génie"]),
            TableWidget::from_data(&player_columns(), vec![]),
            TableWidget::from_data(&player_columns(), vec![]),
        );

        match elem {
            DocumentElement::TeamBoxscore { focusable, .. } => {
                assert_eq!(focusable.len(), 1);
                assert_eq!(focusable[0].rect.width, 5);
            }
            _ => panic!("Expected TeamBoxscore"),
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
