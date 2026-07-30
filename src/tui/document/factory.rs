//! Single construction path for stacked-document content
//!
//! Before this module existed, each stacked document type's content struct
//! (`BoxscoreDocumentContent`, `TeamDetailDocumentContent`,
//! `PlayerDetailDocumentContent`) was built in two unrelated places: once by
//! the render path (inside each document widget's `render()`), and once by
//! the input path (`document::handlers`, to extract focus metadata). Two
//! construction sites per type meant two places that had to agree on which
//! view/width/lookup to use -- a sync-by-discipline seam rather than a
//! structural guarantee.
//!
//! `build_stacked_document` is now the only place these structs are
//! constructed in production. Both the render path
//! (`components::app::App::render_stacked_document`) and the input path
//! (`document::handle_stacked_document_key`) call it and get the same
//! document.

use std::sync::Arc;

use crate::tui::components::boxscore_document::{BoxscoreDocumentContent, TeamView};
use crate::tui::components::player_detail_document::PlayerDetailDocumentContent;
use crate::tui::components::team_detail_document::TeamDetailDocumentContent;
use crate::tui::state::DataState;
use crate::tui::types::StackedDocument;

use super::Document;

/// Build the content document for a stacked document from loaded app data.
///
/// Returns `None` while the variant's backing data hasn't arrived yet. The
/// render path shows a loading spinner in that case; the input path leaves
/// navigation metadata untouched rather than clearing it (matching what the
/// key-event handling has always done while data is loading).
pub fn build_stacked_document(
    doc: &StackedDocument,
    data: &DataState,
) -> Option<Arc<dyn Document>> {
    match doc {
        StackedDocument::Boxscore { game_id, .. } => {
            let boxscore = data.boxscores.get(game_id)?.clone();
            Some(Arc::new(BoxscoreDocumentContent::new(
                *game_id,
                boxscore,
                TeamView::Away,
            )))
        }
        StackedDocument::TeamDetail { abbrev, season } => {
            // None = "latest" still resolving; the render path shows the
            // loading spinner until the first fetch rewrites it.
            let season_id = (*season)?;
            let club_stats = data
                .team_roster_stats
                .get(&(abbrev.clone(), season_id))?
                .clone();
            let is_current_season = data
                .team_seasons
                .get(abbrev)
                .and_then(|ids| ids.last())
                .is_none_or(|latest| *latest == season_id);
            let standing = data.standings.as_ref().as_ref().and_then(|standings| {
                standings
                    .iter()
                    .find(|s| s.team_abbrev.default == *abbrev)
                    .cloned()
            });
            Some(Arc::new(TeamDetailDocumentContent::new(
                abbrev.clone(),
                standing,
                Some(club_stats),
                is_current_season,
            )))
        }
        StackedDocument::PlayerDetail { player_id, .. } => {
            let player_data = data.player_data.get(player_id)?.clone();
            Some(Arc::new(PlayerDetailDocumentContent::new(
                Some(player_data),
                *player_id,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document::{FocusContext, LinkTarget};
    use crate::tui::document_nav::DocumentNavState;
    use nhl_api::{
        Boxscore, BoxscoreTeam, ClubGoalieStats, ClubSkaterStats, ClubStats, GameClock,
        GameScheduleState, GameState, GameType, GoalieStats, LocalizedString, PeriodDescriptor,
        PeriodType, PlayerByGameStats, PlayerLanding, Position, Season, SeasonTotal, SkaterStats,
        Standing, TeamPlayerStats,
    };
    use std::collections::HashMap;

    fn test_skater(player_id: i64, name: &str, sweater_number: i32) -> SkaterStats {
        SkaterStats {
            player_id: player_id.into(),
            sweater_number,
            name: LocalizedString {
                default: name.to_string(),
            },
            position: Some(Position::Center),
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

    fn test_goalie(player_id: i64, name: &str, sweater_number: i32) -> GoalieStats {
        GoalieStats {
            player_id: player_id.into(),
            sweater_number,
            name: LocalizedString {
                default: name.to_string(),
            },
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
            decision: None,
            shots_against: 25,
            saves: 23,
        }
    }

    fn test_boxscore(game_id: i64) -> Boxscore {
        Boxscore {
            id: game_id.into(),
            season: Season::new(2024),
            game_type: GameType::RegularSeason,
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
                    forwards: vec![test_skater(1001, "Away Forward One", 10)],
                    defense: vec![],
                    goalies: vec![test_goalie(1004, "Away Goalie", 30)],
                },
                home_team: TeamPlayerStats {
                    forwards: vec![test_skater(2001, "Home Forward One", 12)],
                    defense: vec![],
                    goalies: vec![test_goalie(2004, "Home Goalie", 31)],
                },
            },
        }
    }

    fn test_club_skater(player_id: i64, last_name: &str, points: i32) -> ClubSkaterStats {
        ClubSkaterStats {
            player_id: player_id.into(),
            headshot: String::new(),
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: last_name.to_string(),
            },
            position: Some(Position::Center),
            games_played: 40,
            goals: points / 2,
            assists: points - points / 2,
            points,
            plus_minus: 0,
            penalty_minutes: 0,
            power_play_goals: 0,
            shorthanded_goals: 0,
            game_winning_goals: 0,
            overtime_goals: 0,
            shots: 100,
            shooting_pctg: 0.1,
            avg_time_on_ice_per_game: 18.0,
            avg_shifts_per_game: 20.0,
            faceoff_win_pctg: 0.5,
        }
    }

    fn test_club_goalie(player_id: i64, last_name: &str, games_played: i32) -> ClubGoalieStats {
        ClubGoalieStats {
            player_id: player_id.into(),
            headshot: String::new(),
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: last_name.to_string(),
            },
            games_played,
            games_started: games_played,
            wins: games_played / 2,
            losses: games_played - games_played / 2,
            overtime_losses: 0,
            goals_against_average: 2.5,
            save_percentage: 0.9,
            shots_against: 500,
            saves: 450,
            goals_against: 50,
            shutouts: 0,
            goals: 0,
            assists: 0,
            points: 0,
            penalty_minutes: 0,
            time_on_ice: 1000,
        }
    }

    fn test_standing(abbrev: &str) -> Standing {
        Standing {
            conference_abbrev: Some("Eastern".to_string()),
            conference_name: Some("Eastern".to_string()),
            division_abbrev: "Atlantic".to_string(),
            division_name: "Atlantic".to_string(),
            team_name: LocalizedString {
                default: format!("Test {}", abbrev),
            },
            team_common_name: LocalizedString {
                default: "Test".to_string(),
            },
            team_abbrev: LocalizedString {
                default: abbrev.to_string(),
            },
            team_logo: String::new(),
            wins: 10,
            losses: 5,
            ot_losses: 2,
            points: 22,
        }
    }

    fn test_season_total(team_common_name: &str) -> SeasonTotal {
        SeasonTotal {
            season: Season::new(2023),
            game_type: GameType::RegularSeason,
            league_abbrev: "NHL".to_string(),
            team_name: LocalizedString {
                default: team_common_name.to_string(),
            },
            team_common_name: Some(LocalizedString {
                default: team_common_name.to_string(),
            }),
            sequence: Some(1),
            games_played: 50,
            goals: Some(10),
            assists: Some(20),
            points: Some(30),
            plus_minus: Some(5),
            pim: Some(10),
        }
    }

    fn test_player(player_id: i64) -> PlayerLanding {
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
            position: Some(Position::Center),
            headshot: String::new(),
            hero_image: None,
            height_in_inches: 73,
            weight_in_pounds: 200,
            birth_date: "1997-09-15".to_string(),
            birth_city: None,
            birth_state_province: None,
            birth_country: None,
            shoots_catches: Some(nhl_api::Handedness::Left),
            draft_details: None,
            player_slug: None,
            featured_stats: None,
            career_totals: None,
            season_totals: Some(vec![test_season_total("Oilers")]),
            awards: None,
            last_five_games: None,
        }
    }

    // ========================================================================
    // build_stacked_document: returns None while data hasn't arrived
    // ========================================================================

    #[test]
    fn boxscore_returns_none_when_not_loaded() {
        let doc = StackedDocument::Boxscore {
            game_id: 1,
            away_abbrev: "TOR".to_string(),
            home_abbrev: "BOS".to_string(),
            away_score: 0,
            home_score: 0,
            game_date: "12/24".to_string(),
        };
        assert!(build_stacked_document(&doc, &DataState::default()).is_none());
    }

    #[test]
    fn team_detail_returns_none_when_not_loaded() {
        let doc = StackedDocument::TeamDetail {
            abbrev: "TOR".to_string(),
            season: None,
        };
        assert!(build_stacked_document(&doc, &DataState::default()).is_none());
    }

    #[test]
    fn player_detail_returns_none_when_not_loaded() {
        let doc = StackedDocument::PlayerDetail {
            player_id: 1,
            sweater_number: None,
            last_name: "Player".to_string(),
        };
        assert!(build_stacked_document(&doc, &DataState::default()).is_none());
    }

    // ========================================================================
    // Render/input parity: the factory's document has the same focusables
    // and activation targets regardless of which caller (render or input)
    // asked for it.
    // ========================================================================

    #[test]
    fn boxscore_parity_focused_row_activates_to_player_detail() {
        let game_id = 1;
        let mut boxscores = HashMap::new();
        boxscores.insert(game_id, test_boxscore(game_id));
        let data = DataState {
            boxscores: Arc::new(boxscores),
            ..Default::default()
        };
        let doc = StackedDocument::Boxscore {
            game_id,
            away_abbrev: "NJD".to_string(),
            home_abbrev: "BUF".to_string(),
            away_score: 3,
            home_score: 2,
            game_date: "12/24".to_string(),
        };

        let document = build_stacked_document(&doc, &data).expect("boxscore data is loaded");
        let ctx = FocusContext::default();
        let focusables = document.focusables(&ctx);
        assert!(!focusables.is_empty(), "boxscore rows must be focusable");

        let mut nav = DocumentNavState::default();
        nav.sync_focusables(document.as_ref(), &ctx);
        nav.focus_index = Some(0);
        match nav.focused_link_target() {
            Some(LinkTarget::Push(StackedDocument::PlayerDetail { player_id, .. })) => {
                assert_eq!(*player_id, 1001);
            }
            other => panic!("expected Push(PlayerDetail), got {other:?}"),
        }
    }

    #[test]
    fn team_detail_parity_focused_row_activates_to_player_detail() {
        let abbrev = "TST";
        let mut roster = HashMap::new();
        roster.insert(
            (abbrev.to_string(), 20242025),
            ClubStats {
                season: Season::new(2024),
                game_type: GameType::RegularSeason,
                skaters: vec![test_club_skater(200, "High", 30)],
                goalies: vec![test_club_goalie(500, "GoalieHigh", 30)],
            },
        );
        let data = DataState {
            team_roster_stats: Arc::new(roster),
            standings: Arc::new(Some(vec![test_standing(abbrev)])),
            ..Default::default()
        };
        let doc = StackedDocument::TeamDetail {
            abbrev: abbrev.to_string(),
            season: Some(20242025),
        };

        let document = build_stacked_document(&doc, &data).expect("roster data is loaded");
        let ctx = FocusContext::default();
        let focusables = document.focusables(&ctx);
        assert!(!focusables.is_empty(), "roster rows must be focusable");

        let mut nav = DocumentNavState::default();
        nav.sync_focusables(document.as_ref(), &ctx);
        nav.focus_index = Some(0);
        match nav.focused_link_target() {
            Some(LinkTarget::Push(StackedDocument::PlayerDetail { player_id, .. })) => {
                assert_eq!(*player_id, 200);
            }
            other => panic!("expected Push(PlayerDetail), got {other:?}"),
        }
    }

    #[test]
    fn player_detail_parity_focused_row_activates_to_team_detail() {
        let player_id = 1;
        let mut players = HashMap::new();
        players.insert(player_id, test_player(player_id));
        let data = DataState {
            player_data: Arc::new(players),
            ..Default::default()
        };
        let doc = StackedDocument::PlayerDetail {
            player_id,
            sweater_number: Some(34),
            last_name: "Player".to_string(),
        };

        let document = build_stacked_document(&doc, &data).expect("player data is loaded");
        let ctx = FocusContext::default();
        let focusables = document.focusables(&ctx);
        assert!(!focusables.is_empty(), "season rows must be focusable");

        let mut nav = DocumentNavState::default();
        nav.sync_focusables(document.as_ref(), &ctx);
        nav.focus_index = Some(0);
        match nav.focused_link_target() {
            Some(LinkTarget::Push(StackedDocument::TeamDetail { abbrev, .. })) => {
                assert_eq!(abbrev, "EDM");
            }
            other => panic!("expected Push(TeamDetail), got {other:?}"),
        }
    }

    // ========================================================================
    // Season-aware TeamDetail construction
    // ========================================================================

    fn data_with_bos_roster(season: i32, all_seasons: Vec<i32>) -> DataState {
        let mut roster = HashMap::new();
        roster.insert(
            ("BOS".to_string(), season),
            ClubStats {
                season: season.try_into().expect("valid test season id"),
                game_type: GameType::RegularSeason,
                skaters: vec![test_club_skater(200, "High", 30)],
                goalies: vec![],
            },
        );
        let mut seasons = HashMap::new();
        seasons.insert("BOS".to_string(), all_seasons);
        DataState {
            team_roster_stats: Arc::new(roster),
            team_seasons: Arc::new(seasons),
            ..Default::default()
        }
    }

    #[test]
    fn team_detail_unresolved_season_returns_none_even_with_data() {
        // season: None means "latest still resolving" -- the spinner shows
        // even if some season's data is already in the map.
        let data = data_with_bos_roster(20242025, vec![20242025]);
        let doc = StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season: None,
        };
        assert!(build_stacked_document(&doc, &data).is_none());
    }

    /// Collect every Text element's content, recursing into groups.
    fn text_contents(elements: &[crate::tui::document::DocumentElement]) -> Vec<String> {
        use crate::tui::document::DocumentElement;
        let mut out = Vec::new();
        for elem in elements {
            match elem {
                DocumentElement::Text { content, .. } => out.push(content.clone()),
                DocumentElement::Group { children, .. } => {
                    out.extend(text_contents(children));
                }
                _ => {}
            }
        }
        out
    }

    #[test]
    fn team_detail_historical_season_hides_record_and_ids_by_season() {
        let mut data = data_with_bos_roster(20232024, vec![20232024, 20242025]);
        data.standings = Arc::new(Some(vec![test_standing("BOS")]));
        let doc = StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season: Some(20232024),
        };

        let document = build_stacked_document(&doc, &data).expect("roster data is loaded");
        assert_eq!(document.id(), "team_detail_BOS_20232024");

        let texts = text_contents(&document.build(&FocusContext::default()));
        assert!(
            texts.iter().any(|t| t.starts_with("Season: 2023-24")),
            "season line must show the viewed season, got {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.starts_with("Record:")),
            "current record must be hidden on a historical roster"
        );
    }

    #[test]
    fn team_detail_latest_season_shows_record() {
        let mut data = data_with_bos_roster(20242025, vec![20232024, 20242025]);
        data.standings = Arc::new(Some(vec![test_standing("BOS")]));
        let doc = StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
            season: Some(20242025),
        };

        let document = build_stacked_document(&doc, &data).expect("roster data is loaded");
        let texts = text_contents(&document.build(&FocusContext::default()));
        assert!(texts.iter().any(|t| t.starts_with("Record:")));
        assert!(texts.iter().any(|t| t.starts_with("Season: 2024-25")));
    }
}
