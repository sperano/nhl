use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::tui::document::{DocumentElement, FocusContext};
use crate::tui::testing::{assert_buffer, create_test_standings};
use std::sync::Arc;

#[test]
fn test_league_standings_document_renders() {
    let standings = Arc::new(create_test_standings());
    let config = Arc::new(Config::default());
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
    let config = Arc::new(Config::default());
    let doc = LeagueStandingsDocument::new(standings, config);

    let display_config = DisplayConfig::default();
    let ctx = RenderContext::focused(&display_config);
    let (buf, height) = doc.render_full(60, &ctx, &FocusContext::default());

    // Height should be: column headers (1) + separator (1) + 32 teams = 34 lines
    assert_eq!(height, 34);

    // Teams are now sorted by points descending (see LeagueStandingsDocument's
    // doc comment); ties are broken by the stable sort's input order, which
    // groups division-by-division (Atlantic, Metropolitan, Central, Pacific).
    assert_buffer(
        &buf,
        &[
            "  Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────",
            "  Avalanche                     19    16    2    1     33",
            "  Devils                        18    15    2    1     31",
            "  Golden Knights                19    15    3    1     31",
            "  Panthers                      19    14    3    2     30",
            "  Hurricanes                    19    14    3    2     30",
            "  Stars                         20    14    4    2     30",
            "  Oilers                        20    14    4    2     30",
            "  Bruins                        18    13    4    1     27",
            "  Jets                          19    13    5    1     27",
            "  Maple Leafs                   19    12    5    2     26",
            "  Rangers                       18    12    5    1     25",
            "  Kings                         19    12    6    1     25",
            "  Penguins                      19    11    6    2     24",
            "  Wild                          19    11    6    2     24",
            "  Kraken                        19    11    6    2     24",
            "  Lightning                     18    11    6    1     23",
            "  Canadiens                     18    10    5    3     23",
            "  Predators                     19    10    7    2     22",
            "  Canucks                       19    10    7    2     22",
            "  Capitals                      18    10    7    1     21",
            "  Senators                      18     9    7    2     20",
            "  Islanders                     18     9    7    2     20",
            "  Flames                        19     9    8    2     20",
            "  Blues                         19     8    8    3     19",
            "  Red Wings                     18     8    8    2     18",
            "  Flyers                        18     8    9    1     17",
            "  Ducks                         19     7   10    2     16",
            "  Blackhawks                    18     7   10    1     15",
            "  Sabres                        18     6   10    2     14",
            "  Blue Jackets                  18     5   11    2     12",
            "  Sharks                        18     5   12    1     11",
            "  Coyotes                       18     4   13    1      9",
        ],
    );
}

#[test]
fn test_league_standings_document_with_focus() {
    let standings = Arc::new(create_test_standings());
    let config = Arc::new(Config::default());
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
    let config = Arc::new(Config::default());
    let doc = LeagueStandingsDocument::new(standings, config);

    assert_eq!(doc.title(), "League Standings");
    assert_eq!(doc.id(), "league_standings");
}

#[test]
fn test_league_standings_focusable_positions() {
    let standings = Arc::new(create_test_standings());
    let config = Arc::new(Config::default());
    let doc = LeagueStandingsDocument::new(standings, config);

    let positions: Vec<u16> = doc
        .focusables(&FocusContext::default())
        .iter()
        .map(|f| f.y)
        .collect();

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
    let config = Arc::new(Config::default());
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
    let config = Arc::new(Config::default());
    let doc = ConferenceStandingsDocument::new(standings, config);

    assert_eq!(doc.title(), "Conference Standings");
    assert_eq!(doc.id(), "conference_standings");
}

#[test]
fn test_conference_standings_focusable_positions() {
    let standings = Arc::new(create_test_standings());
    let config = Arc::new(Config::default());
    let doc = ConferenceStandingsDocument::new(standings, config);

    let positions: Vec<u16> = doc
        .focusables(&FocusContext::default())
        .iter()
        .map(|f| f.y)
        .collect();

    // Should have 32 focusable positions (16 per conference)
    assert_eq!(positions.len(), 32);

    // With Row layout, left column elements are collected first, then right column.
    // Both columns have the SAME y-positions because they're rendered side-by-side.
    // Section title (no underline) = 2 lines, then table column headers = 2 lines
    // So data starts at y=4
    // Left column (16 teams): positions 4, 5, 6, ... 19
    // Right column (16 teams): positions 4, 5, 6, ... 19

    // First 16 positions are left column
    for (i, position) in positions.iter().take(16).enumerate() {
        assert_eq!(
            *position,
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
    let config = Arc::new(Config::default());
    let doc = ConferenceStandingsDocument::new(standings, config);

    let row_positions: Vec<_> = doc
        .focusables(&FocusContext::default())
        .iter()
        .map(|f| f.row_position)
        .collect();

    // Should have 32 row positions
    assert_eq!(row_positions.len(), 32);

    // All should be Some (within a Row)
    assert!(row_positions.iter().all(|rp| rp.is_some()));

    // Check that we have elements from both columns (0 and 1)
    let column_0_count = row_positions
        .iter()
        .filter(|rp| rp.as_ref().is_some_and(|p| p.child_idx == 0))
        .count();
    let column_1_count = row_positions
        .iter()
        .filter(|rp| rp.as_ref().is_some_and(|p| p.child_idx == 1))
        .count();

    // Should have 16 teams in each column
    assert_eq!(column_0_count, 16);
    assert_eq!(column_1_count, 16);
}

#[test]
fn test_conference_standings_respects_western_first_config() {
    let standings = Arc::new(create_test_standings());

    // Test with western_first = false (Eastern left, Western right)
    let config = Config {
        display_standings_western_first: false,
        ..Default::default()
    };
    let doc = ConferenceStandingsDocument::new(standings.clone(), Arc::new(config));
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
    let config = Config {
        display_standings_western_first: true,
        ..Default::default()
    };
    let doc = ConferenceStandingsDocument::new(standings, Arc::new(config));
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
    let config = Arc::new(Config::default());
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
    let config = Arc::new(Config::default());
    let doc = DivisionStandingsDocument::new(standings, config);

    assert_eq!(doc.title(), "Division Standings");
    assert_eq!(doc.id(), "division_standings");
}

#[test]
fn test_division_standings_focusable_positions() {
    let standings = Arc::new(create_test_standings());
    let config = Arc::new(Config::default());
    let doc = DivisionStandingsDocument::new(standings, config);

    let positions = doc.focusables(&FocusContext::default());

    // Should have 32 focusable positions (32 teams across 4 divisions)
    assert_eq!(positions.len(), 32);
}

#[test]
fn test_division_standings_row_positions() {
    let standings = Arc::new(create_test_standings());
    let config = Arc::new(Config::default());
    let doc = DivisionStandingsDocument::new(standings, config);

    let row_positions: Vec<_> = doc
        .focusables(&FocusContext::default())
        .iter()
        .map(|f| f.row_position)
        .collect();

    // Should have 32 row positions
    assert_eq!(row_positions.len(), 32);

    // All should be Some (within a Row)
    assert!(row_positions.iter().all(|rp| rp.is_some()));

    // Check that we have elements from both columns (0 and 1)
    let column_0_count = row_positions
        .iter()
        .filter(|rp| rp.as_ref().is_some_and(|p| p.child_idx == 0))
        .count();
    let column_1_count = row_positions
        .iter()
        .filter(|rp| rp.as_ref().is_some_and(|p| p.child_idx == 1))
        .count();

    // Should have 16 teams in each column (2 divisions x 8 teams)
    assert_eq!(column_0_count, 16);
    assert_eq!(column_1_count, 16);
}

#[test]
fn test_division_standings_respects_western_first_config() {
    let standings = Arc::new(create_test_standings());

    // Test with western_first = false (Eastern divisions left, Western divisions right)
    let config = Config {
        display_standings_western_first: false,
        ..Default::default()
    };
    let doc = DivisionStandingsDocument::new(standings.clone(), Arc::new(config));
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
    let config = Config {
        display_standings_western_first: true,
        ..Default::default()
    };
    let doc = DivisionStandingsDocument::new(standings, Arc::new(config));
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
