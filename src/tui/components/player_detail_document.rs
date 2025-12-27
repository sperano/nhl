use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect};

use nhl_api::{PlayerLanding, Position, SeasonTotal};

use super::table::TableWidget;
use crate::config::RenderContext;
use crate::team_abbrev::common_name_to_abbrev;
use crate::tui::component::{Component, Element, ElementWidget};
use crate::tui::document::{
    Document, DocumentBuilder, DocumentElement, DocumentView, FocusContext,
};
use crate::tui::helpers::SeasonSorting;
use crate::tui::widgets::{LoadingAnimation, StandaloneWidget};
use crate::tui::{Alignment, CellValue, ColumnDef};

/// Props for PlayerDetailDocument component
#[derive(Clone)]
pub struct PlayerDetailDocumentProps {
    pub player_id: i64,
    pub player_data: Option<PlayerLanding>,
    pub loading: bool,
    pub selected_index: Option<usize>,
    pub scroll_offset: u16,
    pub animation_frame: u8,
    /// Whether this document has focus (affects dim/bright rendering)
    /// Stacked documents are always focused
    pub focused: bool,
}

/// PlayerDetailDocument component - renders player info and career stats
pub struct PlayerDetailDocument;

impl Component for PlayerDetailDocument {
    type Props = PlayerDetailDocumentProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        Element::Widget(Box::new(PlayerDetailDocumentWidget {
            player_id: props.player_id,
            player_data: props.player_data.clone(),
            loading: props.loading,
            focus_index: props.selected_index,
            scroll_offset: props.scroll_offset,
            animation_frame: props.animation_frame,
            focused: props.focused,
        }))
    }
}

/// Document implementation for player detail content
///
/// This struct implements the Document trait, providing:
/// - Declarative element tree construction via build()
/// - Focus navigation through table rows
/// - Team links that can be activated to navigate to team details
pub struct PlayerDetailDocumentContent {
    pub player_data: Option<PlayerLanding>,
    pub player_id: i64,
}

impl PlayerDetailDocumentContent {
    pub fn new(player_data: Option<PlayerLanding>, player_id: i64) -> Self {
        Self {
            player_data,
            player_id,
        }
    }

    /// Get NHL regular season stats, sorted by season descending
    fn get_nhl_regular_seasons(player: &PlayerLanding) -> Vec<SeasonTotal> {
        let mut season_stats: Vec<SeasonTotal> = player
            .season_totals
            .as_ref()
            .map(|seasons| {
                seasons
                    .iter()
                    .filter(|s| {
                        s.game_type == nhl_api::GameType::RegularSeason && s.league_abbrev == "NHL"
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        season_stats.sort_by_season_desc();
        season_stats
    }

    /// Build skater season columns
    fn skater_season_columns() -> Vec<ColumnDef<SeasonTotal>> {
        vec![
            ColumnDef::new("Season", 9, Alignment::Left, |s: &SeasonTotal| {
                let season_str = s.season.to_string();
                let formatted = format!("{}-{}", &season_str[0..4], &season_str[4..8]);
                CellValue::Text(formatted)
            }),
            ColumnDef::new("Team", 25, Alignment::Left, |s: &SeasonTotal| {
                if let Some(ref common_name) = s.team_common_name {
                    if let Some(abbrev) = common_name_to_abbrev(&common_name.default) {
                        return CellValue::TeamLink {
                            display: s.team_name.default.clone(),
                            team_abbrev: abbrev.to_string(),
                        };
                    }
                }
                CellValue::Text(s.team_name.default.clone())
            }),
            ColumnDef::new("GP", 4, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(s.games_played.to_string())
            }),
            ColumnDef::new("G", 3, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(s.goals.unwrap_or(0).to_string())
            }),
            ColumnDef::new("A", 3, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(s.assists.unwrap_or(0).to_string())
            }),
            ColumnDef::new("PTS", 4, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(s.points.unwrap_or(0).to_string())
            }),
            ColumnDef::new("+/-", 4, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(
                    s.plus_minus
                        .map(|v| format!("{:+}", v))
                        .unwrap_or_else(|| "0".to_string()),
                )
            }),
            ColumnDef::new("PIM", 4, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(s.pim.unwrap_or(0).to_string())
            }),
        ]
    }

    /// Build goalie season columns
    fn goalie_season_columns() -> Vec<ColumnDef<SeasonTotal>> {
        vec![
            ColumnDef::new("Season", 9, Alignment::Left, |s: &SeasonTotal| {
                let season_str = s.season.to_string();
                let formatted = format!("{}-{}", &season_str[0..4], &season_str[4..8]);
                CellValue::Text(formatted)
            }),
            ColumnDef::new("Team", 25, Alignment::Left, |s: &SeasonTotal| {
                if let Some(ref common_name) = s.team_common_name {
                    if let Some(abbrev) = common_name_to_abbrev(&common_name.default) {
                        return CellValue::TeamLink {
                            display: s.team_name.default.clone(),
                            team_abbrev: abbrev.to_string(),
                        };
                    }
                }
                CellValue::Text(s.team_name.default.clone())
            }),
            ColumnDef::new("GP", 4, Alignment::Right, |s: &SeasonTotal| {
                CellValue::Text(s.games_played.to_string())
            }),
            // Note: SeasonTotal doesn't include goalie-specific stats (W, L, GAA, SV%)
            // Those would need to come from a different API endpoint
        ]
    }

    /// Format career stats as a string
    fn format_career_stats(player: &PlayerLanding) -> Option<String> {
        let career = player.career_totals.as_ref()?;
        let rs = &career.regular_season;

        Some(if player.position == Position::Goalie {
            format!(
                "GP: {} | W: {} | L: {} | OTL: {} | GAA: {:.2} | SV%: {:.3} | SO: {}",
                rs.games_played.unwrap_or(0),
                rs.wins.unwrap_or(0),
                rs.losses.unwrap_or(0),
                rs.ot_losses.unwrap_or(0),
                rs.goals_against_avg.unwrap_or(0.0),
                rs.save_pctg.unwrap_or(0.0),
                rs.shutouts.unwrap_or(0)
            )
        } else {
            format!(
                "GP: {} | G: {} | A: {} | PTS: {} | +/-: {} | PIM: {}",
                rs.games_played.unwrap_or(0),
                rs.goals.unwrap_or(0),
                rs.assists.unwrap_or(0),
                rs.points.unwrap_or(0),
                rs.plus_minus.unwrap_or(0),
                rs.pim.unwrap_or(0)
            )
        })
    }
}

impl Document for PlayerDetailDocumentContent {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let Some(ref player) = self.player_data else {
            return DocumentBuilder::new()
                .text(format!("No data available for player {}", self.player_id))
                .build();
        };

        let mut builder = DocumentBuilder::new();

        // Player name header
        let full_name = format!("{} {}", player.first_name.default, player.last_name.default);
        builder = builder.heading(1, full_name);

        // Player details line 1: Team, number, position, handedness
        let team_info = player
            .current_team_abbrev
            .as_ref()
            .map(|t| format!("Team: {} | ", t))
            .unwrap_or_default();
        let sweater = player
            .sweater_number
            .map(|n| format!("#{} | ", n))
            .unwrap_or_default();
        let hand_label = if player.position == Position::Goalie {
            "Catches"
        } else {
            "Shoots"
        };
        let details1 = format!(
            "{}{}{} | {}/{}",
            team_info, sweater, player.position, player.shoots_catches, hand_label
        );
        builder = builder.text(details1);

        // Player details line 2: Height, weight, birth date
        let height_feet = player.height_in_inches / 12;
        let height_inches = player.height_in_inches % 12;
        let details2 = format!(
            "Height: {}'{}\" | Weight: {} lbs | Born: {}",
            height_feet, height_inches, player.weight_in_pounds, player.birth_date
        );
        builder = builder.text(details2);
        builder = builder.spacer(1);

        // Draft info (if available)
        if let Some(ref draft) = player.draft_details {
            let draft_info = format!(
                "Draft: {} - Round {}, Pick {} (#{} overall) by {}",
                draft.year, draft.round, draft.pick_in_round, draft.overall_pick, draft.team_abbrev
            );
            builder = builder.text(draft_info);
            builder = builder.spacer(1);
        }

        // Career totals
        if let Some(career_stats) = Self::format_career_stats(player) {
            builder = builder.heading(2, "CAREER TOTALS - Regular Season");
            builder = builder.text(career_stats);
            builder = builder.spacer(1);
        }

        // Season-by-season table
        let seasons = Self::get_nhl_regular_seasons(player);
        if !seasons.is_empty() {
            let columns = if player.position == Position::Goalie {
                Self::goalie_season_columns()
            } else {
                Self::skater_season_columns()
            };

            let focused_row = focus.focused_table_row("season_stats");
            let total_seasons = seasons.len();

            let title = format!("SEASON BY SEASON ({} NHL seasons)", total_seasons);
            let table = TableWidget::from_data(&columns, seasons).with_focused_row(focused_row);

            builder = builder
                .element(DocumentElement::section_title(title, true))
                .table("season_stats", table);
        }

        builder.build()
    }

    fn title(&self) -> String {
        self.player_data
            .as_ref()
            .map(|p| format!("{} {}", p.first_name.default, p.last_name.default))
            .unwrap_or_else(|| format!("Player {}", self.player_id))
    }

    fn id(&self) -> String {
        format!("player_detail_{}", self.player_id)
    }
}

/// Widget for rendering the player detail document
///
/// This widget uses DocumentView to render the PlayerDetailDocumentContent
/// with proper scrolling and focus support.
#[derive(Clone)]
pub struct PlayerDetailDocumentWidget {
    player_id: i64,
    player_data: Option<PlayerLanding>,
    loading: bool,
    focus_index: Option<usize>,
    scroll_offset: u16,
    animation_frame: u8,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl ElementWidget for PlayerDetailDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create child RenderContext with our focus state
        let child_ctx = RenderContext::new(ctx.config, self.focused);

        // Handle loading state - show animation if loading or data hasn't arrived yet
        if self.loading || self.player_data.is_none() {
            LoadingAnimation::new(self.animation_frame).render(area, buf, &child_ctx);
            return;
        }

        // Create document
        let doc = Arc::new(PlayerDetailDocumentContent::new(
            self.player_data.clone(),
            self.player_id,
        ));

        // Create DocumentView and render
        let mut view = DocumentView::new(doc, area.height);
        if let Some(idx) = self.focus_index {
            view.focus_by_index(idx);
        }
        view.set_scroll_offset(self.scroll_offset);
        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use crate::tui::document::FocusableId;
    use nhl_api::{Handedness, LocalizedString, SeasonTotal};
    use ratatui::buffer::Buffer;

    fn create_test_player(player_id: i64, position: Position) -> PlayerLanding {
        PlayerLanding {
            player_id,
            is_active: true,
            current_team_id: Some(10),
            current_team_abbrev: Some("TOR".to_string()),
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: "Player".to_string(),
            },
            sweater_number: Some(34),
            position,
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
            shoots_catches: Handedness::Left,
            draft_details: None,
            player_slug: None,
            featured_stats: None,
            career_totals: None,
            season_totals: Some(vec![
                SeasonTotal {
                    season: 20232024,
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
                    season: 20222023,
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

        let positions = doc.focusable_positions();

        // Should have 2 focusable positions (one per season with TableCell)
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_document_focusable_ids() {
        let player = create_test_player(8479318, Position::Center);
        let doc = PlayerDetailDocumentContent::new(Some(player), 8479318);

        let ids = doc.focusable_ids();

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

    #[test]
    fn test_widget_renders_with_data() {
        let player = create_test_player(8479318, Position::Center);

        let widget = PlayerDetailDocumentWidget {
            player_id: 8479318,
            player_data: Some(player),
            loading: false,
            focus_index: None,
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
            player_id: 8479318,
            player_data: None,
            loading: true,
            focus_index: None,
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
            player_id: 8479318,
            player_data: None,
            loading: false,
            focus_index: None,
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

        let widget = PlayerDetailDocumentWidget {
            player_id: 8479318,
            player_data: Some(player),
            loading: false,
            focus_index: Some(0), // Focus on first focusable element
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
            player_id: 8479318,
            player_data: Some(player),
            loading: false,
            focus_index: None,
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
}
