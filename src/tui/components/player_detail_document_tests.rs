use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::tui::document::FocusableId;
use nhl_api::{Handedness, LocalizedString, Season, SeasonTotal};
use ratatui::buffer::Buffer;

fn create_test_player(player_id: i64, position: Position) -> PlayerLanding {
    PlayerLanding {
        player_id: player_id.into(),
        is_active: true,
        current_team_id: Some(10.into()),
        current_team_abbrev: Some("TOR".to_string()),
        first_name: LocalizedString {
            default: "Test".to_string(),
        },
        last_name: LocalizedString {
            default: "Player".to_string(),
        },
        sweater_number: Some(34),
        position: Some(position),
        headshot: String::new(),
        hero_image: None,
        height_in_inches: 73,
        weight_in_pounds: 200,
        birth_date: "1997-09-15".to_string(),
        birth_city: Some(LocalizedString {
            default: "Toronto".to_string(),
        }),
        birth_state_province: Some(LocalizedString {
            default: "ON".to_string(),
        }),
        birth_country: Some("CAN".to_string()),
        shoots_catches: Some(Handedness::Left),
        draft_details: None,
        player_slug: None,
        featured_stats: None,
        career_totals: None,
        season_totals: Some(vec![
            SeasonTotal {
                season: Season::new(2023),
                game_type: nhl_api::GameType::RegularSeason,
                league_abbrev: "NHL".to_string(),
                team_name: LocalizedString {
                    default: "Toronto Maple Leafs".to_string(),
                },
                team_common_name: Some(LocalizedString {
                    default: "Maple Leafs".to_string(),
                }),
                sequence: Some(1),
                games_played: 82,
                goals: Some(40),
                assists: Some(50),
                points: Some(90),
                plus_minus: Some(10),
                pim: Some(20),
            },
            SeasonTotal {
                season: Season::new(2022),
                game_type: nhl_api::GameType::RegularSeason,
                league_abbrev: "NHL".to_string(),
                team_name: LocalizedString {
                    default: "Toronto Maple Leafs".to_string(),
                },
                team_common_name: Some(LocalizedString {
                    default: "Maple Leafs".to_string(),
                }),
                sequence: Some(1),
                games_played: 78,
                goals: Some(35),
                assists: Some(45),
                points: Some(80),
                plus_minus: Some(8),
                pim: Some(18),
            },
        ]),
        awards: None,
        last_five_games: None,
    }
}

// === Document trait tests ===

#[test]
fn test_document_build_with_player_data() {
    let player = create_test_player(8479318, Position::Center);
    let doc = PlayerDetailDocumentContent::new(Some(player), 8479318);

    let elements = doc.build(&FocusContext::default());

    // Should have multiple elements: heading, text lines, table
    assert!(!elements.is_empty());

    // First element should be heading with player name
    match &elements[0] {
        DocumentElement::Heading { content, .. } => {
            assert_eq!(content, "Test Player");
        }
        _ => panic!("Expected Heading element"),
    }
}

#[test]
fn test_document_build_without_player_data() {
    let doc = PlayerDetailDocumentContent::new(None, 8479318);

    let elements = doc.build(&FocusContext::default());

    // Should have one text element with "no data" message
    assert_eq!(elements.len(), 1);
    match &elements[0] {
        DocumentElement::Text { content, .. } => {
            assert!(content.contains("No data available"));
        }
        _ => panic!("Expected Text element"),
    }
}

#[test]
fn test_document_title_with_player() {
    let player = create_test_player(8479318, Position::Center);
    let doc = PlayerDetailDocumentContent::new(Some(player), 8479318);

    assert_eq!(doc.title(), "Test Player");
}

#[test]
fn test_document_title_without_player() {
    let doc = PlayerDetailDocumentContent::new(None, 8479318);

    assert_eq!(doc.title(), "Player 8479318");
}

#[test]
fn test_document_id() {
    let doc = PlayerDetailDocumentContent::new(None, 8479318);

    assert_eq!(doc.id(), "player_detail_8479318");
}

#[test]
fn test_document_focusable_positions() {
    let player = create_test_player(8479318, Position::Center);
    let doc = PlayerDetailDocumentContent::new(Some(player), 8479318);

    let positions = doc.focusables(&FocusContext::default());

    // Should have 2 focusable positions (one per season with TableCell)
    assert_eq!(positions.len(), 2);
}

#[test]
fn test_document_focusable_ids() {
    let player = create_test_player(8479318, Position::Center);
    let doc = PlayerDetailDocumentContent::new(Some(player), 8479318);

    let ids: Vec<_> = doc
        .focusables(&FocusContext::default())
        .into_iter()
        .map(|f| f.id)
        .collect();

    // Should have 2 focusable IDs (one per season with TableCell)
    // TableCell IDs enable row highlighting via focused_table_row()
    assert_eq!(ids.len(), 2);

    // Both should be TableCell IDs (team info is in link_targets, not IDs)
    for (i, id) in ids.iter().enumerate() {
        match id {
            FocusableId::TableCell {
                table_name,
                row,
                col,
            } => {
                assert_eq!(table_name, "season_stats");
                assert_eq!(*row, i);
                assert_eq!(*col, 1); // team column
            }
            _ => panic!("Expected TableCell focusable ID, got {:?}", id),
        }
    }
}

#[test]
fn test_document_builds_table_with_focus() {
    let player = create_test_player(8479318, Position::Center);
    let doc = PlayerDetailDocumentContent::new(Some(player), 8479318);

    // Build with focus on first row
    let focus = FocusContext::with_table_cell("season_stats", 0, 1);
    let elements = doc.build(&focus);

    // Find the table element
    let table_elem = elements
        .iter()
        .find(|e| matches!(e, DocumentElement::Table { .. }));
    assert!(table_elem.is_some(), "Should contain a Table element");
}

// === Widget tests ===

fn test_document(player: Option<PlayerLanding>) -> Arc<dyn Document> {
    Arc::new(PlayerDetailDocumentContent::new(player, 8479318))
}

#[test]
fn test_widget_renders_with_data() {
    let player = create_test_player(8479318, Position::Center);

    let widget = PlayerDetailDocumentWidget {
        document: Some(test_document(Some(player))),
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Verify rendering completed without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_widget_shows_loading() {
    let widget = PlayerDetailDocumentWidget {
        document: None,
        loading: true,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    assert_eq!(*buf.area(), area);
}

#[test]
fn test_widget_handles_no_data() {
    let widget = PlayerDetailDocumentWidget {
        document: None,
        loading: false,
        focused_id: None,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 10);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    assert_eq!(*buf.area(), area);
}

#[test]
fn test_widget_with_focus() {
    let player = create_test_player(8479318, Position::Center);
    let document = test_document(Some(player));

    // Focus on the first focusable element
    let first_focusable = document
        .focusables(&FocusContext::default())
        .into_iter()
        .next()
        .map(|f| f.id);
    let widget = PlayerDetailDocumentWidget {
        document: Some(document),
        loading: false,
        focused_id: first_focusable,
        scroll_offset: 0,
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_widget_with_scroll_offset() {
    let player = create_test_player(8479318, Position::Center);

    let widget = PlayerDetailDocumentWidget {
        document: Some(test_document(Some(player))),
        loading: false,
        focused_id: None,
        scroll_offset: 5, // Scroll down 5 lines
        animation_frame: 0,
        focused: true,
    };

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(area, &mut buf, &ctx);

    // Should render without panic
    assert_eq!(*buf.area(), area);
}

#[test]
fn test_goalie_columns_differ_from_skater() {
    // Create a goalie player
    let goalie = create_test_player(8479318, Position::Goalie);
    let doc_goalie = PlayerDetailDocumentContent::new(Some(goalie), 8479318);

    // Create a skater player
    let skater = create_test_player(8479318, Position::Center);
    let doc_skater = PlayerDetailDocumentContent::new(Some(skater), 8479318);

    let goalie_cols = PlayerDetailDocumentContent::goalie_season_columns();
    let skater_cols = PlayerDetailDocumentContent::skater_season_columns();

    // Goalie columns should have fewer columns (no G, A, PTS, +/-, PIM)
    assert!(goalie_cols.len() < skater_cols.len());

    // Both should still produce valid elements
    let goalie_elements = doc_goalie.build(&FocusContext::default());
    let skater_elements = doc_skater.build(&FocusContext::default());

    assert!(!goalie_elements.is_empty());
    assert!(!skater_elements.is_empty());
}
