use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect};

use nhl_api::{ClubGoalieStats, ClubSkaterStats, ClubStats, Standing};

use super::table::TableWidget;
use crate::config::DisplayConfig;
use crate::tui::helpers::{ClubGoalieStatsSorting, ClubSkaterStatsSorting};
use crate::tui::widgets::{LoadingAnimation, StandaloneWidget};
use crate::tui::{
    component::{Component, Element, ElementWidget},
    document::{Document, DocumentBuilder, DocumentElement, DocumentView, FocusContext},
    Alignment, CellValue, ColumnDef,
};

/// Props for TeamDetailDocument component
#[derive(Clone)]
pub struct TeamDetailDocumentProps {
    pub team_abbrev: String,
    pub standing: Option<Standing>,
    pub club_stats: Option<ClubStats>,
    pub loading: bool,
    pub selected_index: Option<usize>,
    pub scroll_offset: u16,
    pub animation_frame: u8,
}

/// TeamDetailDocument component - renders team info and season player stats
pub struct TeamDetailDocument;

impl Component for TeamDetailDocument {
    type Props = TeamDetailDocumentProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        Element::Widget(Box::new(TeamDetailDocumentWidget {
            team_abbrev: props.team_abbrev.clone(),
            standing: props.standing.clone(),
            club_stats: props.club_stats.clone(),
            loading: props.loading,
            selected_index: props.selected_index,
            scroll_offset: props.scroll_offset,
            animation_frame: props.animation_frame,
        }))
    }
}

/// Document content for team detail view
pub struct TeamDetailDocumentContent {
    pub team_abbrev: String,
    pub standing: Option<Standing>,
    pub club_stats: Option<ClubStats>,
}

impl TeamDetailDocumentContent {
    pub fn new(
        team_abbrev: String,
        standing: Option<Standing>,
        club_stats: Option<ClubStats>,
    ) -> Self {
        Self {
            team_abbrev,
            standing,
            club_stats,
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
            builder = builder.heading(1, &format!("{} {}", team_name, common_name));

            // Team record
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
        } else {
            builder = builder.heading(1, &self.team_abbrev);
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

    fn title(&self) -> String {
        if let Some(ref standing) = self.standing {
            format!("{} {}", standing.team_name.default, standing.team_common_name.default)
        } else {
            self.team_abbrev.clone()
        }
    }

    fn id(&self) -> String {
        format!("team_detail_{}", self.team_abbrev)
    }
}

/// Define columns for skater stats table
fn skater_columns() -> Vec<ColumnDef<ClubSkaterStats>> {
    vec![
        ColumnDef::new(
            "Player",
            20,
            Alignment::Left,
            |s: &ClubSkaterStats| CellValue::PlayerLink {
                display: format!("{} {}", s.first_name.default, s.last_name.default),
                player_id: s.player_id,
            },
        ),
        ColumnDef::new("Pos", 3, Alignment::Left, |s: &ClubSkaterStats| {
            CellValue::Text(s.position.to_string())
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
        ColumnDef::new(
            "Player",
            20,
            Alignment::Left,
            |g: &ClubGoalieStats| CellValue::PlayerLink {
                display: format!("{} {}", g.first_name.default, g.last_name.default),
                player_id: g.player_id,
            },
        ),
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
struct TeamDetailDocumentWidget {
    team_abbrev: String,
    standing: Option<Standing>,
    club_stats: Option<ClubStats>,
    loading: bool,
    selected_index: Option<usize>,
    scroll_offset: u16,
    animation_frame: u8,
}

impl ElementWidget for TeamDetailDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Show animation if loading or data hasn't arrived yet
        if self.loading || self.club_stats.is_none() {
            LoadingAnimation::new(self.animation_frame).render(area, buf, config);
            return;
        }

        if area.width == 0 || area.height == 0 {
            return;
        }

        // Create document and render with DocumentView
        let doc = TeamDetailDocumentContent::new(
            self.team_abbrev.clone(),
            self.standing.clone(),
            self.club_stats.clone(),
        );

        let mut view = DocumentView::new(Arc::new(doc), area.height);

        // Apply focus state
        if let Some(idx) = self.selected_index {
            view.focus_by_index(idx);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Render the document
        view.render(area, buf, config);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(TeamDetailDocumentWidget {
            team_abbrev: self.team_abbrev.clone(),
            standing: self.standing.clone(),
            club_stats: self.club_stats.clone(),
            loading: self.loading,
            selected_index: self.selected_index,
            scroll_offset: self.scroll_offset,
            animation_frame: self.animation_frame,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document::FocusContext;
    use nhl_api::{ClubGoalieStats, ClubSkaterStats, LocalizedString, Position};
    use ratatui::{buffer::Buffer, layout::Rect};

    fn create_test_skater(
        player_id: i64,
        first_name: &str,
        last_name: &str,
        position: Position,
        gp: i32,
        goals: i32,
        assists: i32,
        points: i32,
    ) -> ClubSkaterStats {
        ClubSkaterStats {
            player_id,
            headshot: String::new(),
            first_name: LocalizedString {
                default: first_name.to_string(),
            },
            last_name: LocalizedString {
                default: last_name.to_string(),
            },
            position,
            games_played: gp,
            goals,
            assists,
            points,
            plus_minus: 5,
            penalty_minutes: 10,
            power_play_goals: 2,
            shorthanded_goals: 0,
            game_winning_goals: 1,
            overtime_goals: 0,
            shots: 50,
            shooting_pctg: 0.15,
            avg_time_on_ice_per_game: 18.5,
            avg_shifts_per_game: 22.0,
            faceoff_win_pctg: 0.52,
        }
    }

    fn create_test_goalie(
        player_id: i64,
        first_name: &str,
        last_name: &str,
        gp: i32,
        wins: i32,
    ) -> ClubGoalieStats {
        ClubGoalieStats {
            player_id,
            headshot: String::new(),
            first_name: LocalizedString {
                default: first_name.to_string(),
            },
            last_name: LocalizedString {
                default: last_name.to_string(),
            },
            games_played: gp,
            games_started: gp,
            wins,
            losses: 5,
            overtime_losses: 2,
            goals_against_average: 2.50,
            save_percentage: 0.915,
            shots_against: 500,
            saves: 457,
            goals_against: 43,
            shutouts: 2,
            goals: 0,
            assists: 1,
            points: 1,
            penalty_minutes: 0,
            time_on_ice: 1500,
        }
    }

    fn create_test_standing() -> Standing {
        Standing {
            conference_abbrev: Some("Eastern".to_string()),
            conference_name: Some("Eastern".to_string()),
            division_abbrev: "Atlantic".to_string(),
            division_name: "Atlantic".to_string(),
            team_name: LocalizedString {
                default: "Test Team".to_string(),
            },
            team_common_name: LocalizedString {
                default: "Test".to_string(),
            },
            team_abbrev: LocalizedString {
                default: "TST".to_string(),
            },
            team_logo: String::new(),
            wins: 10,
            losses: 5,
            ot_losses: 2,
            points: 22,
        }
    }

    fn create_test_club_stats() -> ClubStats {
        let skaters = vec![
            create_test_skater(1, "John", "Doe", Position::Center, 20, 10, 15, 25),
            create_test_skater(2, "Jane", "Smith", Position::LeftWing, 18, 8, 12, 20),
        ];
        let goalies = vec![create_test_goalie(3, "Bob", "Johnson", 15, 8)];

        ClubStats {
            season: "20242025".to_string(),
            game_type: nhl_api::GameType::RegularSeason,
            skaters,
            goalies,
        }
    }

    #[test]
    fn test_document_builds_with_data() {
        let standing = create_test_standing();
        let club_stats = create_test_club_stats();

        let doc = TeamDetailDocumentContent::new(
            "TST".to_string(),
            Some(standing),
            Some(club_stats),
        );

        let elements = doc.build(&FocusContext::default());

        // Should have: heading, text (record), blank, skaters table, blank, goalies table
        assert!(elements.len() >= 4);
    }

    #[test]
    fn test_document_metadata() {
        let standing = create_test_standing();

        let doc = TeamDetailDocumentContent::new(
            "TST".to_string(),
            Some(standing),
            None,
        );

        assert_eq!(doc.title(), "Test Team Test");
        assert_eq!(doc.id(), "team_detail_TST");
    }

    #[test]
    fn test_document_without_standing() {
        let doc = TeamDetailDocumentContent::new(
            "TST".to_string(),
            None,
            None,
        );

        assert_eq!(doc.title(), "TST");
        assert_eq!(doc.id(), "team_detail_TST");
    }

    #[test]
    fn test_focusable_positions() {
        let standing = create_test_standing();
        let club_stats = create_test_club_stats();

        let doc = TeamDetailDocumentContent::new(
            "TST".to_string(),
            Some(standing),
            Some(club_stats),
        );

        let positions = doc.focusable_positions();

        // Should have 3 focusable positions: 2 skaters + 1 goalie
        assert_eq!(positions.len(), 3);
    }

    #[test]
    fn test_focusable_ids() {
        let standing = create_test_standing();
        let club_stats = create_test_club_stats();

        let doc = TeamDetailDocumentContent::new(
            "TST".to_string(),
            Some(standing),
            Some(club_stats),
        );

        let ids = doc.focusable_ids();

        // Should have 3 focusable IDs: 2 skaters + 1 goalie
        assert_eq!(ids.len(), 3);
    }

    /// Regression test for buffer overflow when rendering tables with limited height.
    #[test]
    fn test_rendering_with_limited_height_does_not_panic() {
        let mut skaters = vec![];
        for i in 0..30 {
            skaters.push(create_test_skater(
                i,
                "Test",
                &format!("Player{}", i),
                Position::Center,
                20,
                10,
                15,
                25,
            ));
        }

        let mut goalies = vec![];
        for i in 0..5 {
            goalies.push(create_test_goalie(
                i + 100,
                "Test",
                &format!("Goalie{}", i),
                15,
                8,
            ));
        }

        let club_stats = ClubStats {
            season: "20242025".to_string(),
            game_type: nhl_api::GameType::RegularSeason,
            skaters,
            goalies,
        };

        let standing = create_test_standing();

        let widget = TeamDetailDocumentWidget {
            team_abbrev: "TST".to_string(),
            standing: Some(standing),
            club_stats: Some(club_stats),
            loading: false,
            selected_index: None,
            scroll_offset: 0,
            animation_frame: 0,
        };

        // Create a small area that is definitely smaller than the preferred height
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();

        // This should NOT panic
        widget.render(area, &mut buf, &config);

        assert_eq!(*buf.area(), area);
    }

    #[test]
    fn test_rendering_at_exact_boundary_height() {
        let skaters = vec![create_test_skater(
            1,
            "John",
            "Doe",
            Position::Center,
            20,
            10,
            15,
            25,
        )];
        let goalies = vec![create_test_goalie(2, "Jane", "Smith", 15, 8)];

        let club_stats = ClubStats {
            season: "20242025".to_string(),
            game_type: nhl_api::GameType::RegularSeason,
            skaters,
            goalies,
        };

        let standing = create_test_standing();

        let widget = TeamDetailDocumentWidget {
            team_abbrev: "TST".to_string(),
            standing: Some(standing),
            club_stats: Some(club_stats),
            loading: false,
            selected_index: None,
            scroll_offset: 0,
            animation_frame: 0,
        };

        let area = Rect::new(0, 0, 80, 15);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();

        widget.render(area, &mut buf, &config);

        assert_eq!(*buf.area(), area);
    }

    #[test]
    fn test_loading_state_renders() {
        let widget = TeamDetailDocumentWidget {
            team_abbrev: "TST".to_string(),
            standing: None,
            club_stats: None,
            loading: true,
            selected_index: None,
            scroll_offset: 0,
            animation_frame: 0,
        };

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();

        widget.render(area, &mut buf, &config);

        // Should render loading message without panic
        assert_eq!(*buf.area(), area);
    }

    #[test]
    fn test_no_stats_renders() {
        let widget = TeamDetailDocumentWidget {
            team_abbrev: "TST".to_string(),
            standing: None,
            club_stats: None,
            loading: false,
            selected_index: None,
            scroll_offset: 0,
            animation_frame: 0,
        };

        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();

        widget.render(area, &mut buf, &config);

        // Should render "no stats" message without panic
        assert_eq!(*buf.area(), area);
    }
}
