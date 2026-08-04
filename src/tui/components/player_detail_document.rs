use std::borrow::Cow;
use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect};

use nhl_api::{PlayerLanding, Position, SeasonTotal};

use super::table::TableWidget;
use crate::config::RenderContext;
use crate::team_abbrev::common_name_to_abbrev;
use crate::tui::component::{Component, Element, ElementWidget};
use crate::tui::document::{
    render_document_widget, Document, DocumentBuilder, DocumentElement, DocumentWidgetParams,
    FocusContext, FocusableId,
};
use crate::tui::helpers::SeasonSorting;
use crate::tui::{Alignment, CellValue, ColumnDef};

/// Props for PlayerDetailDocument component
#[derive(Clone)]
pub struct PlayerDetailDocumentProps {
    /// Pre-built content document, or `None` while player data hasn't
    /// arrived yet (rendered as a loading spinner). Built by
    /// `document::build_stacked_document`, the single production
    /// construction site shared with the input-handling path.
    pub document: Option<Arc<dyn Document>>,
    pub loading: bool,
    pub focused_id: Option<FocusableId>,
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
            document: props.document.clone(),
            loading: props.loading,
            focused_id: props.focused_id.clone(),
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
                CellValue::Text(s.season.to_string())
            }),
            ColumnDef::new("Team", 25, Alignment::Left, |s: &SeasonTotal| {
                if let Some(ref common_name) = s.team_common_name {
                    if let Some(abbrev) = common_name_to_abbrev(&common_name.default) {
                        return CellValue::TeamLink {
                            display: s.team_name.default.clone(),
                            team_abbrev: abbrev.to_string(),
                            season: Some(s.season.id()),
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
                CellValue::Text(s.season.to_string())
            }),
            ColumnDef::new("Team", 25, Alignment::Left, |s: &SeasonTotal| {
                if let Some(ref common_name) = s.team_common_name {
                    if let Some(abbrev) = common_name_to_abbrev(&common_name.default) {
                        return CellValue::TeamLink {
                            display: s.team_name.default.clone(),
                            team_abbrev: abbrev.to_string(),
                            season: Some(s.season.id()),
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

        Some(if player.position == Some(Position::Goalie) {
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

    /// Name heading plus the two detail lines (team/number/position/hand,
    /// height/weight/birth date).
    fn build_player_header(builder: DocumentBuilder, player: &PlayerLanding) -> DocumentBuilder {
        let full_name = format!("{} {}", player.first_name.default, player.last_name.default);

        let team_info = player
            .current_team_abbrev
            .as_ref()
            .map(|t| format!("Team: {} | ", t))
            .unwrap_or_default();
        let sweater = player
            .sweater_number
            .map(|n| format!("#{} | ", n))
            .unwrap_or_default();
        let hand_label = if player.position == Some(Position::Goalie) {
            "Catches"
        } else {
            "Shoots"
        };
        let details1 = format!(
            "{}{}{} | {}/{}",
            team_info,
            sweater,
            player.position.map_or("N/A", |p| p.code()),
            player.shoots_catches.map_or("N/A", |h| h.code()),
            hand_label
        );

        let height_feet = player.height_in_inches / 12;
        let height_inches = player.height_in_inches % 12;
        let details2 = format!(
            "Height: {}'{}\" | Weight: {} lbs | Born: {}",
            height_feet, height_inches, player.weight_in_pounds, player.birth_date
        );

        builder
            .heading(1, full_name)
            .text(details1)
            .text(details2)
            .spacer(1)
    }

    /// Draft round/pick info, if the player was drafted
    fn build_draft_info(builder: DocumentBuilder, player: &PlayerLanding) -> DocumentBuilder {
        let Some(ref draft) = player.draft_details else {
            return builder;
        };

        let draft_info = format!(
            "Draft: {} - Round {}, Pick {} (#{} overall) by {}",
            draft.year, draft.round, draft.pick_in_round, draft.overall_pick, draft.team_abbrev
        );
        builder.text(draft_info).spacer(1)
    }

    /// Career regular-season totals summary, if available
    fn build_career_totals(builder: DocumentBuilder, player: &PlayerLanding) -> DocumentBuilder {
        let Some(career_stats) = Self::format_career_stats(player) else {
            return builder;
        };

        builder
            .heading(2, "CAREER TOTALS - Regular Season")
            .text(career_stats)
            .spacer(1)
    }

    /// Season-by-season stats table, if the player has any NHL regular-season data
    fn build_season_table(
        builder: DocumentBuilder,
        player: &PlayerLanding,
        focus: &FocusContext,
    ) -> DocumentBuilder {
        let seasons = Self::get_nhl_regular_seasons(player);
        if seasons.is_empty() {
            return builder;
        }

        let columns = if player.position == Some(Position::Goalie) {
            Self::goalie_season_columns()
        } else {
            Self::skater_season_columns()
        };

        let focused_row = focus.focused_table_row("season_stats");
        let total_seasons = seasons.len();

        let title = format!("SEASON BY SEASON ({} NHL seasons)", total_seasons);
        let table = TableWidget::from_data(&columns, seasons).with_focused_row(focused_row);

        builder
            .element(DocumentElement::section_title(title, true))
            .table("season_stats", table)
    }
}

impl Document for PlayerDetailDocumentContent {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let Some(ref player) = self.player_data else {
            return DocumentBuilder::new()
                .text(format!("No data available for player {}", self.player_id))
                .build();
        };

        let builder = Self::build_player_header(DocumentBuilder::new(), player);
        let builder = Self::build_draft_info(builder, player);
        let builder = Self::build_career_totals(builder, player);
        let builder = Self::build_season_table(builder, player, focus);

        builder.build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Owned(
            self.player_data
                .as_ref()
                .map(|p| format!("{} {}", p.first_name.default, p.last_name.default))
                .unwrap_or_else(|| format!("Player {}", self.player_id)),
        )
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Owned(format!("player_detail_{}", self.player_id))
    }
}

/// Widget for rendering the player detail document
///
/// This widget uses DocumentView to render the PlayerDetailDocumentContent
/// with proper scrolling and focus support.
#[derive(Clone)]
pub struct PlayerDetailDocumentWidget {
    document: Option<Arc<dyn Document>>,
    loading: bool,
    focused_id: Option<FocusableId>,
    scroll_offset: u16,
    animation_frame: u8,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl ElementWidget for PlayerDetailDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        render_document_widget(
            &DocumentWidgetParams {
                document: &self.document,
                loading: self.loading,
                focused_id: self.focused_id.clone(),
                scroll_offset: self.scroll_offset,
                animation_frame: self.animation_frame,
                focused: self.focused,
            },
            area,
            buf,
            ctx,
        );
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
#[path = "player_detail_document_tests.rs"]
mod tests;
