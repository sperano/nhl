use std::borrow::Cow;
use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect};

use nhl_api::{Boxscore, GoalieStats, SkaterStats};

use super::table::TableWidget;
use crate::config::RenderContext;
use crate::tui::component::{Component, Element, ElementWidget};
use crate::tui::document::{
    render_document_widget, Document, DocumentBuilder, DocumentElement, DocumentWidgetParams,
    FocusContext, FocusableId, TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH,
};
use crate::tui::widgets::{BigScoreParams, ScoreBoxStatus};
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
    /// Pre-built content document, or `None` while boxscore data hasn't
    /// arrived yet (rendered as a loading spinner). Built by
    /// `document::build_stacked_document`, the single production
    /// construction site shared with the input-handling path.
    pub document: Option<Arc<dyn Document>>,
    pub loading: bool,
    pub focused_id: Option<FocusableId>,
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
            document: props.document.clone(),
            loading: props.loading,
            focused_id: props.focused_id.clone(),
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
            vec![DocumentElement::big_score(BigScoreParams {
                away_name: boxscore.away_team.common_name.default.clone(),
                home_name: boxscore.home_team.common_name.default.clone(),
                away_score: boxscore.away_team.score,
                home_score: boxscore.home_team.score,
                away_sog: boxscore.away_team.sog,
                home_sog: boxscore.home_team.sog,
                status,
                venue: boxscore.venue.default.clone(),
            })]
        } else {
            let score_text = format!(
                "{}: {}  |  {}: {}  (SOG: {} - {})",
                boxscore.away_team.abbrev,
                boxscore.away_team.score,
                boxscore.home_team.abbrev,
                boxscore.home_team.score,
                boxscore.away_team.sog,
                boxscore.home_team.sog,
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

        // Player stats - side by side if wide enough, otherwise stacked.
        // A team with no player stats at all (e.g. a game that hasn't
        // started) is omitted entirely rather than rendering an empty
        // bordered shell.
        let stats = &self.boxscore.player_by_game_stats;
        let has_players = |team: &nhl_api::TeamPlayerStats| {
            !team.forwards.is_empty() || !team.defense.is_empty() || !team.goalies.is_empty()
        };
        let mut boxscores = Vec::new();
        if has_players(&stats.away_team) {
            boxscores.push(self.build_team_boxscore(focus, true));
        }
        if has_players(&stats.home_team) {
            boxscores.push(self.build_team_boxscore(focus, false));
        }

        let wide_enough = focus
            .available_width
            .map(|w| w >= TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH)
            .unwrap_or(false);

        if wide_enough {
            builder = builder.element(DocumentElement::row_center_with_gap(boxscores, 4));
        } else {
            for (i, boxscore) in boxscores.into_iter().enumerate() {
                if i > 0 {
                    builder = builder.spacer(1);
                }
                builder = builder.element(boxscore);
            }
        }

        builder.build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Owned(format!(
            "{} @ {} - Game {}",
            self.boxscore.away_team.abbrev, self.boxscore.home_team.abbrev, self.game_id
        ))
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Owned(format!("boxscore_{}", self.game_id))
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
                player_id: s.player_id.into(),
                sweater_number: Some(s.sweater_number),
                last_name: s.name.default.clone(),
            }
        }),
        ColumnDef::new("Pos", 3, Alignment::Center, |s: &SkaterStats| {
            CellValue::Text(
                s.position
                    .map_or_else(String::new, |p| p.code().to_string()),
            )
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
                player_id: g.player_id.into(),
                sweater_number: Some(g.sweater_number),
                last_name: g.name.default.clone(),
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

/// Missing period type (historical data) is treated as regulation.
fn format_period_text(number: &i32, period_type: Option<nhl_api::PeriodType>) -> String {
    match period_type.unwrap_or(nhl_api::PeriodType::Regulation) {
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
            overtime: boxscore.period_descriptor.period_type == Some(nhl_api::PeriodType::Overtime),
            shootout: boxscore.period_descriptor.period_type == Some(nhl_api::PeriodType::Shootout),
        },
        nhl_api::GameState::Postponed | nhl_api::GameState::Suspended => {
            ScoreBoxStatus::Scheduled {
                start_time: "TBD".to_string(),
            }
        }
    }
}

/// Widget for rendering boxscore document
#[derive(Clone)]
struct BoxscoreDocumentWidget {
    document: Option<Arc<dyn Document>>,
    loading: bool,
    focused_id: Option<FocusableId>,
    scroll_offset: u16,
    focused: bool,
    animation_frame: u8,
}

impl ElementWidget for BoxscoreDocumentWidget {
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
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use crate::tui::document::FocusContext;
    use nhl_api::{
        Boxscore, BoxscoreTeam, GameClock, GameScheduleState, GameState, GoalieDecision,
        GoalieStats, LocalizedString, PeriodDescriptor, PeriodType, PlayerByGameStats, Position,
        Season, SkaterStats, TeamPlayerStats,
    };

    /// Create a test skater with minimal data
    fn create_test_skater(name: &str, sweater_number: i32, position: Position) -> SkaterStats {
        SkaterStats {
            player_id: (sweater_number as i64).into(),
            name: LocalizedString {
                default: name.to_string(),
            },
            sweater_number,
            position: Some(position),
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
            player_id: (sweater_number as i64).into(),
            name: LocalizedString {
                default: name.to_string(),
            },
            sweater_number,
            position: Some(Position::Goalie),
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
            id: 2024020001.into(),
            season: Season::new(2024),
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
            game_schedule_state: GameScheduleState::Ok,
            period_descriptor: PeriodDescriptor {
                number: 3,
                period_type: Some(PeriodType::Regulation),
                max_regulation_periods: 3,
            },
            special_event: None,
            away_team: BoxscoreTeam {
                id: 1.into(),
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
                id: 7.into(),
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

    /// Count TeamBoxscore elements anywhere in the built tree (they sit
    /// inside a Row at side-by-side widths, at the top level otherwise).
    fn count_team_boxscores(elements: &[DocumentElement]) -> usize {
        elements
            .iter()
            .map(|e| match e {
                DocumentElement::TeamBoxscore { .. } => 1,
                DocumentElement::Row { children, .. } => count_team_boxscores(children),
                _ => 0,
            })
            .sum()
    }

    #[test]
    fn test_build_omits_team_with_no_player_stats() {
        // A team without any player stats (e.g. a game that hasn't started)
        // must not render an empty bordered shell.
        let mut boxscore = create_test_boxscore();
        boxscore.player_by_game_stats.away_team = TeamPlayerStats {
            forwards: vec![],
            defense: vec![],
            goalies: vec![],
        };
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        let elements = doc.build(&FocusContext::default());
        assert_eq!(count_team_boxscores(&elements), 1);
    }

    #[test]
    fn test_build_omits_all_boxscores_when_no_player_stats() {
        let mut boxscore = create_test_boxscore();
        let empty = || TeamPlayerStats {
            forwards: vec![],
            defense: vec![],
            goalies: vec![],
        };
        boxscore.player_by_game_stats.away_team = empty();
        boxscore.player_by_game_stats.home_team = empty();
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        let elements = doc.build(&FocusContext::default());
        assert_eq!(count_team_boxscores(&elements), 0);
        assert_eq!(doc.focusables(&FocusContext::default()).len(), 0);
    }

    #[test]
    fn test_mock_boxscore_document_renders_player_tables() {
        // End-to-end: the mock-mode boxscore document must show actual
        // player rows, not collapsed borders.
        let boxscore = crate::fixtures::create_mock_boxscore(2024020001);
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        let display_config = crate::config::DisplayConfig::default();
        let ctx = RenderContext::focused(&display_config);
        let (buf, _height) = doc.render_full(120, &ctx, &FocusContext::default());

        let text = crate::tui::testing::buffer_lines(&buf).join("\n");
        for name in [
            "Matthews", "Rielly", "Woll", "Stutzle", "Chabot", "Forsberg",
        ] {
            assert!(text.contains(name), "expected player {name} in:\n{text}");
        }
    }

    #[test]
    fn test_mock_boxscore_has_player_stats_for_both_teams() {
        // Mock mode exists to demo the UI; an empty player_by_game_stats
        // renders the boxscore document as bare border lines (regression:
        // 2026-07-08 manual smoke pass).
        let boxscore = crate::fixtures::create_mock_boxscore(2024020001);
        for team in [
            &boxscore.player_by_game_stats.away_team,
            &boxscore.player_by_game_stats.home_team,
        ] {
            assert!(!team.forwards.is_empty());
            assert!(!team.defense.is_empty());
            assert!(!team.goalies.is_empty());
        }
    }

    #[test]
    fn test_focusable_positions() {
        let boxscore = create_test_boxscore();
        let doc = BoxscoreDocumentContent::new(2024020001, boxscore, TeamView::Away);

        let positions = doc.focusables(&FocusContext::default());

        // Should have focusable positions for all players
        // Away: 2 forwards + 1 defense + 1 goalie = 4
        // Home: 2 forwards + 1 defense + 1 goalie = 4
        // Total = 8
        assert_eq!(positions.len(), 8);
    }

    #[test]
    fn test_loading_state_renders() {
        let widget = BoxscoreDocumentWidget {
            document: None,
            loading: true,
            focused_id: None,
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
            document: None,
            loading: false,
            focused_id: None,
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
        let document: Arc<dyn Document> = Arc::new(BoxscoreDocumentContent::new(
            2024020001,
            boxscore,
            TeamView::Away,
        ));
        let widget = BoxscoreDocumentWidget {
            document: Some(document),
            loading: false,
            focused_id: None,
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
