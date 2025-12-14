//! Document implementations for standings views
//!
//! This module contains Document implementations for the different standings
//! views (League, Conference, Division, Wildcard).

mod conference;
mod division;
mod league;
mod wildcard;

use std::sync::Arc;

use nhl_api::Standing;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::{Config, RenderContext};
use crate::tui::component::ElementWidget;
use crate::tui::document::{Document, DocumentView};

pub use conference::ConferenceStandingsDocument;
pub use division::DivisionStandingsDocument;
pub use league::LeagueStandingsDocument;
pub use wildcard::WildcardStandingsDocument;

use super::{standings_columns, TableWidget};

/// Widget that renders a standings document with DocumentView
///
/// This widget wraps DocumentView and applies focus/scroll state from AppState.
/// It can render League, Conference, or Division standings based on the document type.
pub struct StandingsDocumentWidget {
    doc: Arc<dyn Document>,
    focus_index: Option<usize>,
    scroll_offset: u16,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl StandingsDocumentWidget {
    /// Create widget for League standings
    pub fn league(
        standings: Arc<Vec<Standing>>,
        config: Config,
        focus_index: Option<usize>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(LeagueStandingsDocument::new(standings, config)),
            focus_index,
            scroll_offset,
            focused,
        }
    }

    /// Create widget for Conference standings
    pub fn conference(
        standings: Arc<Vec<Standing>>,
        config: Config,
        focus_index: Option<usize>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(ConferenceStandingsDocument::new(standings, config)),
            focus_index,
            scroll_offset,
            focused,
        }
    }

    /// Create widget for Division standings
    pub fn division(
        standings: Arc<Vec<Standing>>,
        config: Config,
        focus_index: Option<usize>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(DivisionStandingsDocument::new(standings, config)),
            focus_index,
            scroll_offset,
            focused,
        }
    }

    /// Create widget for Wildcard standings
    pub fn wildcard(
        standings: Arc<Vec<Standing>>,
        config: Config,
        focus_index: Option<usize>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(WildcardStandingsDocument::new(standings, config)),
            focus_index,
            scroll_offset,
            focused,
        }
    }
}

impl ElementWidget for StandingsDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create DocumentView with viewport height
        let mut view = DocumentView::new(self.doc.clone(), area.height);

        // Apply focus state from AppState
        if let Some(idx) = self.focus_index {
            view.focus_by_index(idx);
        }

        // Apply scroll offset from AppState
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        let child_ctx = RenderContext::new(ctx.config, self.focused);

        // Render the document
        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(StandingsDocumentWidget {
            doc: self.doc.clone(),
            focus_index: self.focus_index,
            scroll_offset: self.scroll_offset,
            focused: self.focused,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use crate::tui::document::{DocumentElement, FocusContext};
    use crate::tui::testing::{assert_buffer, create_test_standings};
    use std::sync::Arc;

    #[test]
    fn test_league_standings_document_renders() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = LeagueStandingsDocument::new(standings, config);

        // Build with no focus
        let elements = doc.build(&FocusContext::default());

        // Should have one element: the table
        assert_eq!(elements.len(), 1);

        // Check that it's a table element
        match &elements[0] {
            DocumentElement::Table { widget, focusable } => {
                assert_eq!(widget.row_count(), 32); // 32 teams
                                                    // Should have 32 focusable elements (one per team row, col 0 is the team link)
                assert_eq!(focusable.len(), 32);
            }
            _ => panic!("Expected Table element"),
        }
    }

    #[test]
    fn test_league_standings_document_full_render() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = LeagueStandingsDocument::new(standings, config);

        let display_config = DisplayConfig::default();
        let ctx = RenderContext::focused(&display_config);
        let (buf, height) = doc.render_full(60, &ctx, &FocusContext::default());

        // Height should be: column headers (1) + separator (1) + 32 teams = 34 lines
        assert_eq!(height, 34);

        // Check all lines
        assert_buffer(
            &buf,
            &[
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Panthers                      19    14    3    2     30",
                "  Bruins                        18    13    4    1     27",
                "  Maple Leafs                   19    12    5    2     26",
                "  Lightning                     18    11    6    1     23",
                "  Canadiens                     18    10    5    3     23",
                "  Senators                      18     9    7    2     20",
                "  Red Wings                     18     8    8    2     18",
                "  Sabres                        18     6   10    2     14",
                "  Devils                        18    15    2    1     31",
                "  Hurricanes                    19    14    3    2     30",
                "  Rangers                       18    12    5    1     25",
                "  Penguins                      19    11    6    2     24",
                "  Capitals                      18    10    7    1     21",
                "  Islanders                     18     9    7    2     20",
                "  Flyers                        18     8    9    1     17",
                "  Blue Jackets                  18     5   11    2     12",
                "  Avalanche                     19    16    2    1     33",
                "  Stars                         20    14    4    2     30",
                "  Jets                          19    13    5    1     27",
                "  Wild                          19    11    6    2     24",
                "  Predators                     19    10    7    2     22",
                "  Blues                         19     8    8    3     19",
                "  Blackhawks                    18     7   10    1     15",
                "  Coyotes                       18     4   13    1      9",
                "  Golden Knights                19    15    3    1     31",
                "  Oilers                        20    14    4    2     30",
                "  Kings                         19    12    6    1     25",
                "  Kraken                        19    11    6    2     24",
                "  Canucks                       19    10    7    2     22",
                "  Flames                        19     9    8    2     20",
                "  Ducks                         19     7   10    2     16",
                "  Sharks                        18     5   12    1     11",
            ],
        );
    }

    #[test]
    fn test_league_standings_document_with_focus() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = LeagueStandingsDocument::new(standings, config);

        // Create focus context for row 2
        let focus = FocusContext::with_table_cell("league_standings", 2, 0);
        let elements = doc.build(&focus);

        // Build and check that the table was created
        // Note: We can't directly check focused_row as it's a private field,
        // but we've passed it through with_focused_row() so the table will render correctly
        match &elements[0] {
            DocumentElement::Table { widget, .. } => {
                assert_eq!(widget.row_count(), 32); // Verify it's the standings table
            }
            _ => panic!("Expected Table element"),
        }
    }

    #[test]
    fn test_league_standings_document_metadata() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = LeagueStandingsDocument::new(standings, config);

        assert_eq!(doc.title(), "League Standings");
        assert_eq!(doc.id(), "league_standings");
    }

    #[test]
    fn test_league_standings_focusable_positions() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = LeagueStandingsDocument::new(standings, config);

        let positions = doc.focusable_positions();

        // Should have 32 focusable positions (one per team row)
        assert_eq!(positions.len(), 32);

        // First focusable is at y=2 (after column headers + separator)
        assert_eq!(positions[0], 2);

        // Each subsequent focusable is 1 line below the previous
        for i in 1..positions.len() {
            assert_eq!(positions[i], positions[i - 1] + 1);
        }
    }

    // === Conference Standings Tests ===

    #[test]
    fn test_conference_standings_document_renders() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = ConferenceStandingsDocument::new(standings, config);

        // Build with no focus
        let elements = doc.build(&FocusContext::default());

        // Should have one element: a Row containing two groups
        assert_eq!(elements.len(), 1);

        // Check that it's a Row element
        match &elements[0] {
            DocumentElement::Row { children, .. } => {
                // Should have 2 children (left and right conference groups)
                assert_eq!(children.len(), 2);

                // Both children should be groups containing [Indented(SectionTitle), Table]
                for child in children {
                    match child {
                        DocumentElement::Group { children, .. } => {
                            assert_eq!(children.len(), 2);
                            // First child should be Indented(SectionTitle)
                            assert!(matches!(&children[0], DocumentElement::Indented { .. }));
                            // Second child should be Table with 16 teams
                            match &children[1] {
                                DocumentElement::Table { widget, .. } => {
                                    assert_eq!(widget.row_count(), 16);
                                }
                                _ => panic!("Expected Table element in Group"),
                            }
                        }
                        _ => panic!("Expected Group element in Row"),
                    }
                }
            }
            _ => panic!("Expected Row element"),
        }
    }

    #[test]
    fn test_conference_standings_document_metadata() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = ConferenceStandingsDocument::new(standings, config);

        assert_eq!(doc.title(), "Conference Standings");
        assert_eq!(doc.id(), "conference_standings");
    }

    #[test]
    fn test_conference_standings_focusable_positions() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = ConferenceStandingsDocument::new(standings, config);

        let positions = doc.focusable_positions();

        // Should have 32 focusable positions (16 per conference)
        assert_eq!(positions.len(), 32);

        // With Row layout, left column elements are collected first, then right column.
        // Both columns have the SAME y-positions because they're rendered side-by-side.
        // Section title (no underline) = 2 lines, then table column headers = 2 lines
        // So data starts at y=4
        // Left column (16 teams): positions 4, 5, 6, ... 19
        // Right column (16 teams): positions 4, 5, 6, ... 19

        // First 16 positions are left column
        for i in 0..16 {
            assert_eq!(
                positions[i],
                4 + i as u16,
                "Left column position {} should be {}",
                i,
                4 + i
            );
        }
        // Second 16 positions are right column - SAME y values as left
        for i in 0..16 {
            assert_eq!(
                positions[16 + i],
                4 + i as u16,
                "Right column position {} should be {}",
                i,
                4 + i
            );
        }
    }

    #[test]
    fn test_conference_standings_row_positions() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = ConferenceStandingsDocument::new(standings, config);

        let row_positions = doc.focusable_row_positions();

        // Should have 32 row positions
        assert_eq!(row_positions.len(), 32);

        // All should be Some (within a Row)
        assert!(row_positions.iter().all(|rp| rp.is_some()));

        // Check that we have elements from both columns (0 and 1)
        let column_0_count = row_positions
            .iter()
            .filter(|rp| rp.as_ref().map_or(false, |p| p.child_idx == 0))
            .count();
        let column_1_count = row_positions
            .iter()
            .filter(|rp| rp.as_ref().map_or(false, |p| p.child_idx == 1))
            .count();

        // Should have 16 teams in each column
        assert_eq!(column_0_count, 16);
        assert_eq!(column_1_count, 16);
    }

    #[test]
    fn test_conference_standings_respects_western_first_config() {
        let standings = Arc::new(create_test_standings());

        // Test with western_first = false (Eastern left, Western right)
        let mut config = Config::default();
        config.display_standings_western_first = false;
        let doc = ConferenceStandingsDocument::new(standings.clone(), config);
        let elements = doc.build(&FocusContext::default());

        // Verify Row structure
        match &elements[0] {
            DocumentElement::Row { children, .. } => {
                // Should have 2 Group children
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], DocumentElement::Group { .. }));
                assert!(matches!(children[1], DocumentElement::Group { .. }));
            }
            _ => panic!("Expected Row element"),
        }

        // Test with western_first = true (Western left, Eastern right)
        let mut config = Config::default();
        config.display_standings_western_first = true;
        let doc = ConferenceStandingsDocument::new(standings, config);
        let elements = doc.build(&FocusContext::default());

        // Verify Row structure (same structure, different internal ordering)
        match &elements[0] {
            DocumentElement::Row { children, .. } => {
                // Should have 2 Group children
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], DocumentElement::Group { .. }));
                assert!(matches!(children[1], DocumentElement::Group { .. }));
            }
            _ => panic!("Expected Row element"),
        }
    }

    // === Division Standings Tests ===

    #[test]
    fn test_division_standings_document_renders() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = DivisionStandingsDocument::new(standings, config);

        // Build with no focus
        let elements = doc.build(&FocusContext::default());

        // Should have one element: a Row containing two Groups
        assert_eq!(elements.len(), 1);

        // Check that it's a Row element with Group children
        match &elements[0] {
            DocumentElement::Row { children, .. } => {
                // Should have 2 children (left and right columns)
                assert_eq!(children.len(), 2);

                // Each child should be a Group containing:
                // [Indented(SectionTitle), Table, Spacer, Indented(SectionTitle), Table]
                for child in children {
                    match child {
                        DocumentElement::Group {
                            children: group_children,
                            ..
                        } => {
                            // Each group should have 5 children:
                            // Indented(SectionTitle), Table, Spacer, Indented(SectionTitle), Table
                            assert_eq!(group_children.len(), 5);
                            assert!(matches!(
                                group_children[0],
                                DocumentElement::Indented { .. }
                            ));
                            assert!(matches!(group_children[1], DocumentElement::Table { .. }));
                            assert!(matches!(group_children[2], DocumentElement::Spacer { .. }));
                            assert!(matches!(
                                group_children[3],
                                DocumentElement::Indented { .. }
                            ));
                            assert!(matches!(group_children[4], DocumentElement::Table { .. }));
                        }
                        _ => panic!("Expected Group element in Row"),
                    }
                }
            }
            _ => panic!("Expected Row element"),
        }
    }

    #[test]
    fn test_division_standings_document_metadata() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = DivisionStandingsDocument::new(standings, config);

        assert_eq!(doc.title(), "Division Standings");
        assert_eq!(doc.id(), "division_standings");
    }

    #[test]
    fn test_division_standings_focusable_positions() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = DivisionStandingsDocument::new(standings, config);

        let positions = doc.focusable_positions();

        // Should have 32 focusable positions (32 teams across 4 divisions)
        assert_eq!(positions.len(), 32);
    }

    #[test]
    fn test_division_standings_row_positions() {
        let standings = Arc::new(create_test_standings());
        let config = Config::default();
        let doc = DivisionStandingsDocument::new(standings, config);

        let row_positions = doc.focusable_row_positions();

        // Should have 32 row positions
        assert_eq!(row_positions.len(), 32);

        // All should be Some (within a Row)
        assert!(row_positions.iter().all(|rp| rp.is_some()));

        // Check that we have elements from both columns (0 and 1)
        let column_0_count = row_positions
            .iter()
            .filter(|rp| rp.as_ref().map_or(false, |p| p.child_idx == 0))
            .count();
        let column_1_count = row_positions
            .iter()
            .filter(|rp| rp.as_ref().map_or(false, |p| p.child_idx == 1))
            .count();

        // Should have 16 teams in each column (2 divisions x 8 teams)
        assert_eq!(column_0_count, 16);
        assert_eq!(column_1_count, 16);
    }

    #[test]
    fn test_division_standings_respects_western_first_config() {
        let standings = Arc::new(create_test_standings());

        // Test with western_first = false (Eastern divisions left, Western divisions right)
        let mut config = Config::default();
        config.display_standings_western_first = false;
        let doc = DivisionStandingsDocument::new(standings.clone(), config);
        let elements = doc.build(&FocusContext::default());

        // Verify Row structure with Groups
        match &elements[0] {
            DocumentElement::Row { children, .. } => {
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], DocumentElement::Group { .. }));
                assert!(matches!(children[1], DocumentElement::Group { .. }));
            }
            _ => panic!("Expected Row element"),
        }

        // Test with western_first = true (Western divisions left, Eastern divisions right)
        let mut config = Config::default();
        config.display_standings_western_first = true;
        let doc = DivisionStandingsDocument::new(standings, config);
        let elements = doc.build(&FocusContext::default());

        // Verify Row structure (same structure, different internal ordering)
        match &elements[0] {
            DocumentElement::Row { children, .. } => {
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], DocumentElement::Group { .. }));
                assert!(matches!(children[1], DocumentElement::Group { .. }));
            }
            _ => panic!("Expected Row element"),
        }
    }
}
