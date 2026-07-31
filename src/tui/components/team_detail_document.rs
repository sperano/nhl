use std::borrow::Cow;
use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect};

use nhl_api::{ClubGoalieStats, ClubSkaterStats, ClubStats, Standing};

use super::table::TableWidget;
use crate::config::RenderContext;
use crate::tui::helpers::{ClubGoalieStatsSorting, ClubSkaterStatsSorting};
use crate::tui::{
    component::{Component, Element, ElementWidget},
    document::{
        render_document_widget, Document, DocumentBuilder, DocumentElement, DocumentWidgetParams,
        FocusContext, FocusableId,
    },
    Alignment, CellValue, ColumnDef,
};

/// Props for TeamDetailDocument component
#[derive(Clone)]
pub struct TeamDetailDocumentProps {
    /// Pre-built content document, or `None` while roster data hasn't
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

/// TeamDetailDocument component - renders team info and season player stats
pub struct TeamDetailDocument;

impl Component for TeamDetailDocument {
    type Props = TeamDetailDocumentProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        Element::Widget(Box::new(TeamDetailDocumentWidget {
            document: props.document.clone(),
            loading: props.loading,
            focused_id: props.focused_id.clone(),
            scroll_offset: props.scroll_offset,
            animation_frame: props.animation_frame,
            focused: props.focused,
        }))
    }
}

/// Document content for team detail view
pub struct TeamDetailDocumentContent {
    pub team_abbrev: String,
    pub standing: Option<Standing>,
    pub club_stats: Option<ClubStats>,
    /// Whether the displayed season is the team's latest. The standings
    /// record describes the current season, so it is only shown when true.
    pub is_current_season: bool,
}

impl TeamDetailDocumentContent {
    pub fn new(
        team_abbrev: String,
        standing: Option<Standing>,
        club_stats: Option<ClubStats>,
        is_current_season: bool,
    ) -> Self {
        Self {
            team_abbrev,
            standing,
            club_stats,
            is_current_season,
        }
    }

    /// Build skater stats table
    fn build_skaters_table(&self, focus: &FocusContext) -> Option<DocumentElement> {
        let stats = self.club_stats.as_ref()?;
        if stats.skaters.is_empty() {
            return None;
        }

        let mut sorted_skaters = stats.skaters.clone();
        sorted_skaters.sort_by_points_desc();

        let title = format!("SKATERS ({}) - Regular Season", stats.skaters.len());
        let columns = skater_columns();
        let table = TableWidget::from_data(&columns, sorted_skaters)
            .with_focused_row(focus.focused_table_row("skaters"));

        Some(DocumentElement::group(vec![
            DocumentElement::section_title(title, true),
            DocumentElement::table("skaters", table),
        ]))
    }

    /// Build goalie stats table
    fn build_goalies_table(&self, focus: &FocusContext) -> Option<DocumentElement> {
        let stats = self.club_stats.as_ref()?;
        if stats.goalies.is_empty() {
            return None;
        }

        let mut sorted_goalies = stats.goalies.clone();
        sorted_goalies.sort_by_games_played_desc();

        let title = format!("GOALIES ({}) - Regular Season", stats.goalies.len());
        let columns = goalie_columns();
        let table = TableWidget::from_data(&columns, sorted_goalies)
            .with_focused_row(focus.focused_table_row("goalies"));

        Some(DocumentElement::group(vec![
            DocumentElement::section_title(title, true),
            DocumentElement::table("goalies", table),
        ]))
    }
}

impl Document for TeamDetailDocumentContent {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let mut builder = DocumentBuilder::new();

        // Team header
        if let Some(ref standing) = self.standing {
            let team_name = &standing.team_name.default;
            let common_name = &standing.team_common_name.default;
            builder = builder.heading(1, format!("{} {}", team_name, common_name));

            // Team record describes the current season only; showing it next
            // to a historical roster would be wrong.
            if self.is_current_season {
                let record = format!(
                    "Record: {}-{}-{} ({} pts) | Division: {} | Conference: {}",
                    standing.wins,
                    standing.losses,
                    standing.ot_losses,
                    standing.points,
                    standing.division_name,
                    standing.conference_name.as_deref().unwrap_or("Unknown")
                );
                builder = builder.text(&record);
            }
        } else {
            builder = builder.heading(1, &self.team_abbrev);
        }

        // Season selector line, derived from the data itself
        if let Some(ref stats) = self.club_stats {
            builder = builder.text(format!(
                "Season: {}  ([ / ] to change season)",
                stats.season.short_label()
            ));
        }

        builder = builder.spacer(1);

        // Skaters table
        if let Some(skaters_table) = self.build_skaters_table(focus) {
            builder = builder.element(skaters_table);
            builder = builder.spacer(1);
        }

        // Goalies table
        if let Some(goalies_table) = self.build_goalies_table(focus) {
            builder = builder.element(goalies_table);
        }

        builder.build()
    }

    fn title(&self) -> Cow<'static, str> {
        if let Some(ref standing) = self.standing {
            Cow::Owned(format!(
                "{} {}",
                standing.team_name.default, standing.team_common_name.default
            ))
        } else {
            Cow::Owned(self.team_abbrev.clone())
        }
    }

    fn id(&self) -> Cow<'static, str> {
        let season_id = self.club_stats.as_ref().map_or(0, |s| s.season.id());
        Cow::Owned(format!("team_detail_{}_{}", self.team_abbrev, season_id))
    }
}

/// Define columns for skater stats table
fn skater_columns() -> Vec<ColumnDef<ClubSkaterStats>> {
    vec![
        ColumnDef::new("Player", 20, Alignment::Left, |s: &ClubSkaterStats| {
            CellValue::PlayerLink {
                display: format!("{} {}", s.first_name.default, s.last_name.default),
                player_id: s.player_id.into(),
                // ClubSkaterStats doesn't carry a sweater number.
                sweater_number: None,
                last_name: s.last_name.default.clone(),
            }
        }),
        ColumnDef::new("Pos", 3, Alignment::Left, |s: &ClubSkaterStats| {
            CellValue::Text(
                s.position
                    .map_or_else(String::new, |p| p.code().to_string()),
            )
        }),
        ColumnDef::new("GP", 4, Alignment::Right, |s: &ClubSkaterStats| {
            CellValue::Text(s.games_played.to_string())
        }),
        ColumnDef::new("G", 3, Alignment::Right, |s: &ClubSkaterStats| {
            CellValue::Text(s.goals.to_string())
        }),
        ColumnDef::new("A", 3, Alignment::Right, |s: &ClubSkaterStats| {
            CellValue::Text(s.assists.to_string())
        }),
        ColumnDef::new("PTS", 4, Alignment::Right, |s: &ClubSkaterStats| {
            CellValue::Text(s.points.to_string())
        }),
        ColumnDef::new("+/-", 4, Alignment::Right, |s: &ClubSkaterStats| {
            CellValue::Text(format!("{:+}", s.plus_minus))
        }),
        ColumnDef::new("PIM", 4, Alignment::Right, |s: &ClubSkaterStats| {
            CellValue::Text(s.penalty_minutes.to_string())
        }),
    ]
}

/// Define columns for goalie stats table
fn goalie_columns() -> Vec<ColumnDef<ClubGoalieStats>> {
    vec![
        ColumnDef::new("Player", 20, Alignment::Left, |g: &ClubGoalieStats| {
            CellValue::PlayerLink {
                display: format!("{} {}", g.first_name.default, g.last_name.default),
                player_id: g.player_id.into(),
                // ClubGoalieStats doesn't carry a sweater number.
                sweater_number: None,
                last_name: g.last_name.default.clone(),
            }
        }),
        ColumnDef::new("GP", 4, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(g.games_played.to_string())
        }),
        ColumnDef::new("W", 3, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(g.wins.to_string())
        }),
        ColumnDef::new("L", 3, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(g.losses.to_string())
        }),
        ColumnDef::new("OTL", 3, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(g.overtime_losses.to_string())
        }),
        ColumnDef::new("GAA", 5, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(format!("{:.2}", g.goals_against_average))
        }),
        ColumnDef::new("SV%", 5, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(format!("{:.3}", g.save_percentage))
        }),
        ColumnDef::new("SO", 3, Alignment::Right, |g: &ClubGoalieStats| {
            CellValue::Text(g.shutouts.to_string())
        }),
    ]
}

/// Widget for rendering the team detail document
#[derive(Clone)]
struct TeamDetailDocumentWidget {
    document: Option<Arc<dyn Document>>,
    loading: bool,
    focused_id: Option<FocusableId>,
    scroll_offset: u16,
    animation_frame: u8,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl ElementWidget for TeamDetailDocumentWidget {
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
#[path = "team_detail_document_tests.rs"]
mod tests;
