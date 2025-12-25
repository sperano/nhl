use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect};

use nhl_api::{Boxscore, GoalieStats, SkaterStats};

use super::table::TableWidget;
use crate::config::RenderContext;
use crate::tui::component::{Component, Element, ElementWidget};
use crate::tui::document::{
    Document, DocumentBuilder, DocumentElement, DocumentView, FocusContext,
    TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH,
};
use crate::tui::widgets::{LoadingAnimation, ScoreBoxStatus, StandaloneWidget};
use crate::tui::{Alignment, CellValue, ColumnDef};

/// View mode for boxscore panel
#[derive(Clone, Debug, PartialEq)]
pub enum TeamView {
    Away,
    Home,
}

/// BoxscoreDocument component props
#[derive(Clone)]
pub struct BoxscoreDocumentProps {
    pub game_id: i64,
    pub boxscore: Option<Boxscore>,
    pub loading: bool,
    pub team_view: TeamView,
    pub selected_index: Option<usize>,
    pub scroll_offset: u16,
    pub focused: bool,
    pub animation_frame: u8,
}

/// BoxscoreDocument component - displays detailed game statistics
pub struct BoxscoreDocument;

impl Component for BoxscoreDocument {
    type Props = BoxscoreDocumentProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        Element::Widget(Box::new(BoxscoreDocumentWidget {
            game_id: props.game_id,
            boxscore: props.boxscore.clone(),
            loading: props.loading,
            team_view: props.team_view.clone(),
            selected_index: props.selected_index,
            scroll_offset: props.scroll_offset,
            focused: props.focused,
            animation_frame: props.animation_frame,
        }))
    }
}

/// Document content for boxscore view
pub struct BoxscoreDocumentContent {
    pub game_id: i64,
    pub boxscore: Boxscore,
    pub team_view: TeamView,
}

impl BoxscoreDocumentContent {
    pub fn new(game_id: i64, boxscore: Boxscore, team_view: TeamView) -> Self {
        Self {
            game_id,
            boxscore,
            team_view,
        }
    }

    /// Build score section - uses big digits if unicode enabled, otherwise text
    fn build_score(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let boxscore = &self.boxscore;

        if focus.use_unicode {
            let status = boxscore_to_status(boxscore);
            vec![DocumentElement::big_score(
                &boxscore.away_team.common_name.default,
                &boxscore.home_team.common_name.default,
                boxscore.away_team.score,
                boxscore.home_team.score,
                status,
                &boxscore.venue.default,
            )]
        } else {
            let score_text = format!(
                "{}: {}  |  {}: {}",
                boxscore.away_team.abbrev,
                boxscore.away_team.score,
                boxscore.home_team.abbrev,
                boxscore.home_team.score
            );
            vec![
                DocumentElement::heading(2, "SCORE"),
                DocumentElement::text(&score_text),
            ]
        }
    }

    /// Build a skater table (forwards or defense)
    fn build_skater_table(
        &self,
        skaters: &[SkaterStats],
        table_id: &str,
        focus: &FocusContext,
    ) -> TableWidget {
        let columns = game_skater_columns();
        TableWidget::from_data(&columns, skaters.to_vec())
            .with_focused_row(focus.focused_table_row(table_id))
    }

    /// Build a goalies table
    fn build_goalies_table(
        &self,
        goalies: &[GoalieStats],
        table_id: &str,
        focus: &FocusContext,
    ) -> TableWidget {
        let columns = game_goalie_columns(&focus.box_chars);
        TableWidget::from_data(&columns, goalies.to_vec())
            .with_focused_row(focus.focused_table_row(table_id))
    }

    /// Build player stats section for one team using TeamBoxscore element
    fn build_team_boxscore(&self, focus: &FocusContext, is_away: bool) -> DocumentElement {
        let boxscore = &self.boxscore;
        let (team_stats, team_name, prefix) = if is_away {
            (
                &boxscore.player_by_game_stats.away_team,
                &boxscore.away_team.common_name.default,
                "away",
            )
        } else {
            (
                &boxscore.player_by_game_stats.home_team,
                &boxscore.home_team.common_name.default,
                "home",
            )
        };

        let forwards_table =
            self.build_skater_table(&team_stats.forwards, &format!("{}_forwards", prefix), focus);
        let defense_table =
            self.build_skater_table(&team_stats.defense, &format!("{}_defense", prefix), focus);
        let goalies_table =
            self.build_goalies_table(&team_stats.goalies, &format!("{}_goalies", prefix), focus);

        DocumentElement::team_boxscore(
            prefix,
            team_name,
            forwards_table,
            defense_table,
            goalies_table,
        )
    }
}

impl Document for BoxscoreDocumentContent {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let mut builder = DocumentBuilder::new();

        // Score section
        for elem in self.build_score(focus) {
            builder = builder.element(elem);
        }
        builder = builder.spacer(1);

        // Player stats - side by side if wide enough, otherwise stacked
        let away_boxscore = self.build_team_boxscore(focus, true);
        let home_boxscore = self.build_team_boxscore(focus, false);

        let wide_enough = focus
            .available_width
            .map(|w| w >= TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH)
            .unwrap_or(false);

        if wide_enough {
            builder = builder.element(DocumentElement::row_center_with_gap(
                vec![away_boxscore, home_boxscore],
                4,
            ));
        } else {
            builder = builder.element(away_boxscore);
            builder = builder.spacer(1);
            builder = builder.element(home_boxscore);
        }

        builder.build()
    }

    fn title(&self) -> String {
        format!(
            "{} @ {} - Game {}",
            self.boxscore.away_team.abbrev, self.boxscore.home_team.abbrev, self.game_id
        )
    }

    fn id(&self) -> String {
        format!("boxscore_{}", self.game_id)
    }
}

/// Column definitions for game-level skater stats
fn game_skater_columns() -> Vec<ColumnDef<SkaterStats>> {
    vec![
        ColumnDef::new("#", 2, Alignment::Right, |s: &SkaterStats| {
            CellValue::StyledText(s.sweater_number.to_string())
        }),
        ColumnDef::new("Player", 20, Alignment::Left, |s: &SkaterStats| {
            CellValue::PlayerLink {
                display: s.name.default.clone(),
                player_id: s.player_id,
            }
        }),
        ColumnDef::new("Pos", 3, Alignment::Center, |s: &SkaterStats| {
            CellValue::Text(s.position.to_string())
        }),
        ColumnDef::new("G", 2, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.goals.to_string())
        }),
        ColumnDef::new("A", 2, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.assists.to_string())
        }),
        ColumnDef::new("PTS", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.points.to_string())
        }),
        ColumnDef::new("PPG", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.power_play_goals.to_string())
        }),
        ColumnDef::new("+/-", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(format!("{:+}", s.plus_minus))
        }),
        ColumnDef::new("SOG", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.sog.to_string())
        }),
        ColumnDef::new("Hits", 4, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.hits.to_string())
        }),
        ColumnDef::new("Blk", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.blocked_shots.to_string())
        }),
        ColumnDef::new("GA", 2, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.giveaways.to_string())
        }),
        ColumnDef::new("TA", 2, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.takeaways.to_string())
        }),
        ColumnDef::new("PIM", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.pim.to_string())
        }),
        ColumnDef::new("FO%", 5, Alignment::Right, |s: &SkaterStats| {
            if s.faceoff_winning_pctg > 0.0 {
                CellValue::Text(format!("{:.1}", s.faceoff_winning_pctg * 100.0))
            } else {
                CellValue::Text("-".to_string())
            }
        }),
        ColumnDef::new("SH", 3, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.shifts.to_string())
        }),
        ColumnDef::new("TOI", 6, Alignment::Right, |s: &SkaterStats| {
            CellValue::Text(s.toi.clone())
        }),
    ]
}

/// Column definitions for game-level goalie stats
fn game_goalie_columns(box_chars: &crate::formatting::BoxChars) -> Vec<ColumnDef<GoalieStats>> {
    let checkmark = box_chars.checkmark.to_string();
    vec![
        ColumnDef::new("#", 2, Alignment::Right, |g: &GoalieStats| {
            CellValue::StyledText(g.sweater_number.to_string())
        }),
        ColumnDef::new("Player", 20, Alignment::Left, |g: &GoalieStats| {
            CellValue::PlayerLink {
                display: g.name.default.clone(),
                player_id: g.player_id,
            }
        }),
        ColumnDef::new("DEC", 3, Alignment::Center, |g: &GoalieStats| {
            let text = match &g.decision {
                Some(d) => d.to_string(),
                None => "-".to_string(),
            };
            CellValue::Text(text)
        }),
        ColumnDef::new("S", 1, Alignment::Center, move |g: &GoalieStats| {
            let text = match g.starter {
                Some(true) => checkmark.clone(),
                _ => " ".to_string(),
            };
            CellValue::Text(text)
        }),
        ColumnDef::new("SA", 3, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.shots_against.to_string())
        }),
        ColumnDef::new("GA", 2, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.goals_against.to_string())
        }),
        ColumnDef::new("SV", 3, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.saves.to_string())
        }),
        ColumnDef::new("SV%", 5, Alignment::Right, |g: &GoalieStats| {
            if let Some(pct) = g.save_pctg {
                CellValue::Text(format!("{:.3}", pct))
            } else {
                CellValue::Text("-".to_string())
            }
        }),
        ColumnDef::new("ES", 6, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.even_strength_shots_against.clone())
        }),
        ColumnDef::new("PP", 4, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.power_play_shots_against.clone())
        }),
        ColumnDef::new("SH", 4, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.shorthanded_shots_against.clone())
        }),
        ColumnDef::new("TOI", 7, Alignment::Right, |g: &GoalieStats| {
            CellValue::Text(g.toi.clone())
        }),
        ColumnDef::new("PIM", 3, Alignment::Right, |g: &GoalieStats| {
            if let Some(pim) = g.pim {
                CellValue::Text(pim.to_string())
            } else {
                CellValue::Text("-".to_string())
            }
        }),
    ]
}

fn format_period_text(number: &i32, period_type: nhl_api::PeriodType) -> String {
    match period_type {
        nhl_api::PeriodType::Regulation => format!("{}", number),
        nhl_api::PeriodType::Overtime => "OT".to_string(),
        nhl_api::PeriodType::Shootout => "SO".to_string(),
    }
}

fn boxscore_to_status(boxscore: &Boxscore) -> ScoreBoxStatus {
    match boxscore.game_state {
        nhl_api::GameState::Future | nhl_api::GameState::PreGame => ScoreBoxStatus::Scheduled {
            start_time: boxscore.start_time_utc.clone(),
        },
        nhl_api::GameState::Live | nhl_api::GameState::Critical => {
            let period = format_period_text(
                &boxscore.period_descriptor.number,
                boxscore.period_descriptor.period_type,
            );
            let time = if boxscore.clock.time_remaining.is_empty() {
                None
            } else {
                Some(boxscore.clock.time_remaining.clone())
            };
            ScoreBoxStatus::Live {
                period,
                time,
                intermission: boxscore.clock.in_intermission,
            }
        }
        nhl_api::GameState::Final | nhl_api::GameState::Off => ScoreBoxStatus::Final {
            overtime: boxscore.period_descriptor.period_type == nhl_api::PeriodType::Overtime,
            shootout: boxscore.period_descriptor.period_type == nhl_api::PeriodType::Shootout,
        },
        nhl_api::GameState::Postponed | nhl_api::GameState::Suspended => {
            ScoreBoxStatus::Scheduled {
                start_time: "TBD".to_string(),
            }
        }
    }
}

/// Widget for rendering boxscore document
struct BoxscoreDocumentWidget {
    game_id: i64,
    boxscore: Option<Boxscore>,
    loading: bool,
    team_view: TeamView,
    selected_index: Option<usize>,
    scroll_offset: u16,
    focused: bool,
    animation_frame: u8,
}

impl ElementWidget for BoxscoreDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create child RenderContext with our focus state
        let child_ctx = RenderContext::new(ctx.config, self.focused);

        // Show animation if loading or data hasn't arrived yet
        if self.loading || self.boxscore.is_none() {
            LoadingAnimation::new(self.animation_frame).render(area, buf, &child_ctx);
            return;
        }

        // Safe to unwrap since we checked is_none() above
        let boxscore = self.boxscore.as_ref().unwrap();

        if area.width == 0 || area.height == 0 {
            return;
        }

        // Create document and render with DocumentView
        let doc =
            BoxscoreDocumentContent::new(self.game_id, boxscore.clone(), self.team_view.clone());

        let mut view = DocumentView::new(Arc::new(doc), area.height);

        // Apply focus state
        if let Some(idx) = self.selected_index {
            view.focus_by_index(idx);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Render the document
        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(BoxscoreDocumentWidget {
            game_id: self.game_id,
            boxscore: self.boxscore.clone(),
            loading: self.loading,
            team_view: self.team_view.clone(),
            selected_index: self.selected_index,
            scroll_offset: self.scroll_offset,
            focused: self.focused,
            animation_frame: self.animation_frame,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use crate::tui::document::FocusContext;
    use nhl_api::{
        Boxscore, BoxscoreTeam, GameClock, GameState, GoalieDecision, GoalieStats, LocalizedString,
        PeriodDescriptor, PeriodType, PlayerByGameStats, Position, SkaterStats, TeamPlayerStats,
    };

    /// Create a test skater with minimal data
    fn create_test_skater(name: &str, sweater_number: i32, position: Position) -> SkaterStats {
        SkaterStats {
            player_id: sweater_number as i64,
            name: LocalizedString {
                default: name.to_string(),
            },
            sweater_number,
            position,
            goals: 1,
            assists: 2,
            points: 3,
            plus_minus: 1,
            pim: 2,
            hits: 3,
            power_play_goals: 0,
            sog: 4,
            faceoff_winning_pctg: 0.5,
            toi: "15:30".to_string(),
            blocked_shots: 1,
            shifts: 20,
            giveaways: 1,
            takeaways: 2,
        }
    }

    /// Create a test goalie with minimal data
    fn create_test_goalie(name: &str, sweater_number: i32) -> GoalieStats {
        GoalieStats {
            player_id: sweater_number as i64,
            name: LocalizedString {
                default: name.to_string(),
            },
            sweater_number,
            position: Position::Goalie,
            even_strength_shots_against: "20".to_string(),
            power_play_shots_against: "5".to_string(),
            shorthanded_shots_against: "0".to_string(),
            save_shots_against: "25".to_string(),
            save_pctg: Some(0.920),
            even_strength_goals_against: 1,
            power_play_goals_against: 1,
            shorthanded_goals_against: 0,
            pim: Some(0),
            goals_against: 2,
            toi: "60:00".to_string(),
            starter: Some(true),
            decision: Some(GoalieDecision::Win),
            shots_against: 25,
            saves: 23,
        }
    }

    fn create_test_boxscore() -> Boxscore {
        let away_forwards = vec![
            create_test_skater("A. Forward1", 10, Position::Center),
            create_test_skater("A. Forward2", 11, Position::LeftWing),
        ];
        let away_defense = vec![create_test_skater("A. Defense1", 20, Position::Defense)];
        let away_goalies = vec![create_test_goalie("A. Goalie", 30)];

        let home_forwards = vec![
            create_test_skater("H. Forward1", 12, Position::Center),
            create_test_skater("H. Forward2", 13, Position::RightWing),
        ];
        let home_defense = vec![create_test_skater("H. Defense1", 21, Position::Defense)];
        let home_goalies = vec![create_test_goalie("H. Goalie", 31)];

        Boxscore {
            id: 2024020001,
            season: 20242025,
            game_type: nhl_api::GameType::RegularSeason,
            limited_scoring: false,
            game_date: "2024-10-04".to_string(),
            venue: LocalizedString {
                default: "Test Arena".to_string(),
            },
            venue_location: LocalizedString {
                default: "Test City".to_string(),
            },
            start_time_utc: "2024-10-04T19:00:00Z".to_string(),
            eastern_utc_offset: "-04:00".to_string(),
            venue_utc_offset: "-04:00".to_string(),
            tv_broadcasts: vec![],
            game_state: GameState::Final,
            game_schedule_state: "OK".to_string(),
            period_descriptor: PeriodDescriptor {
                number: 3,
                period_type: PeriodType::Regulation,
                max_regulation_periods: 3,
            },
            special_event: None,
            away_team: BoxscoreTeam {
                id: 1,
                common_name: LocalizedString {
                    default: "Devils".to_string(),
                },
                abbrev: "NJD".to_string(),
                score: 3,
                sog: 30,
                logo: String::new(),
                dark_logo: String::new(),
                place_name: LocalizedString {
                    default: "New Jersey".to_string(),
                },
                place_name_with_preposition: LocalizedString {
                    default: "New Jersey".to_string(),
                },
            },
            home_team: BoxscoreTeam {
                id: 7,
                common_name: LocalizedString {
                    default: "Sabres".to_string(),
                },
                abbrev: "BUF".to_string(),
                score: 2,
                sog: 25,
                logo: String::new(),
                dark_logo: String::new(),
                place_name: LocalizedString {
                    default: "Buffalo".to_string(),
                },
                place_name_with_preposition: LocalizedString {
                    default: "Buffalo".to_string(),
                },
            },
            clock: GameClock {
                time_remaining: "00:00".to_string(),
                seconds_remaining: 0,
                running: false,
                in_intermission: false,
            },
            player_by_game_stats: PlayerByGameStats {
                away_team: TeamPlayerStats {
                    forwards: away_forwards,
                    defense: away_defense,
                    goalies: away_goalies,
                },
                home_team: TeamPlayerStats {
                    forwards: home_forwards,
                    defense: home_defense,
                    goalies: home_goalies,
                },
            },
        }
    }

    #[test]
    fn test_document_builds_with_data() {
        let boxscore = create_test_boxscore();
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        let elements = doc.build(&FocusContext::default());

        // Should have header, score, and player stats sections
        assert!(!elements.is_empty());
    }

    #[test]
    fn test_document_metadata() {
        let boxscore = create_test_boxscore();
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        assert_eq!(doc.title(), "NJD @ BUF - Game 2024020001");
        assert_eq!(doc.id(), "boxscore_2024020001");
    }

    #[test]
    fn test_focusable_positions() {
        let boxscore = create_test_boxscore();
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        let positions = doc.focusable_positions();

        // Should have focusable positions for all players
        // Away: 2 forwards + 1 defense + 1 goalie = 4
        // Home: 2 forwards + 1 defense + 1 goalie = 4
        // Total = 8
        assert_eq!(positions.len(), 8);
    }

    #[test]
    fn test_loading_state_renders() {
        let widget = BoxscoreDocumentWidget {
            game_id: 2024020001,
            boxscore: None,
            loading: true,
            team_view: TeamView::Away,
            selected_index: None,
            scroll_offset: 0,
            focused: true,
            animation_frame: 0,
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
    fn test_no_boxscore_renders() {
        let widget = BoxscoreDocumentWidget {
            game_id: 2024020001,
            boxscore: None,
            loading: false,
            team_view: TeamView::Away,
            selected_index: None,
            scroll_offset: 0,
            focused: true,
            animation_frame: 0,
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
    fn test_boxscore_renders() {
        let boxscore = create_test_boxscore();
        let widget = BoxscoreDocumentWidget {
            game_id: 2024020001,
            boxscore: Some(boxscore),
            loading: false,
            team_view: TeamView::Away,
            selected_index: None,
            scroll_offset: 0,
            focused: true,
            animation_frame: 0,
        };

        let area = Rect::new(0, 0, 100, 50);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        widget.render(area, &mut buf, &ctx);

        // Should render without panic
        assert_eq!(*buf.area(), area);
    }
}
