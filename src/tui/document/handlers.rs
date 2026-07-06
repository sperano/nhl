//! Key handling for stacked documents (Boxscore, TeamDetail, PlayerDetail)
//!
//! Stacked documents used to each own a `StackedDocumentHandler` impl with
//! per-type `populate_focusable_metadata()` (and, before F3, per-type
//! `activate()`). With activation generic over `LinkTarget::Push` (F3),
//! focusable metadata collapsed into one `Vec<FocusableElement>` synced by
//! one function (F2), and document construction collapsed into
//! [`super::build_stacked_document`] (F4), there is no per-type logic left:
//! every stacked document is handled by the same sequence of steps. This
//! module holds that single free function.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document_nav::{handle_message, DocumentNavState};
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::DataState;
use crate::tui::types::StackedDocument;

use super::{build_stacked_document, FocusContext, LinkTarget};

/// Handle a key event routed to the document on top of the document stack.
///
/// Syncs focusable metadata on demand (via the same factory the render path
/// uses, so the two can't disagree about what's on screen), then handles
/// navigation via [`key_to_nav_msg`] and delegates `Enter` to activating the
/// focused element's [`LinkTarget`].
///
/// If the document's backing data hasn't loaded yet, `nav`'s existing
/// metadata is left untouched (there is nothing new to sync); navigation
/// simply has nothing to move through until the data arrives.
pub fn handle_stacked_document_key(
    doc: &StackedDocument,
    key: KeyEvent,
    nav: &mut DocumentNavState,
    data: &DataState,
    width: u16,
) -> Effect {
    if let Some(document) = build_stacked_document(doc, data) {
        let ctx = FocusContext::default().with_width(width);
        nav.sync_focusables(document.as_ref(), &ctx);
    }

    if let Some(nav_msg) = key_to_nav_msg(key) {
        return handle_message(nav, &nav_msg);
    }

    if key.code == KeyCode::Enter {
        return match nav.focused_link_target() {
            Some(LinkTarget::Push(target)) => Effect::Action(Action::PushDocument(target.clone())),
            _ => Effect::None,
        };
    }

    Effect::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document::{FocusableElement, FocusableId};
    use nhl_api::{
        Boxscore, BoxscoreTeam, ClubGoalieStats, ClubSkaterStats, ClubStats, GameClock, GameState,
        GameType, GoalieDecision, GoalieStats, Handedness, LocalizedString, PeriodDescriptor,
        PeriodType, PlayerByGameStats, PlayerLanding, Position, SeasonTotal, SkaterStats, Standing,
        TeamPlayerStats,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Width passed to `handle_stacked_document_key` in tests; large enough
    /// to force the boxscore's side-by-side layout, which is irrelevant to
    /// focusable-item count.
    const TEST_WIDTH: u16 = 100;
    /// Viewport height used for page-up/page-down tests, chosen to be larger
    /// than `MIN_PAGE_SIZE` in `nav_handler.rs` so the page size is
    /// deterministic.
    const TEST_VIEWPORT_HEIGHT: u16 = 20;

    // ========================================================================
    // Fixtures
    // ========================================================================

    fn test_skater_stats(player_id: i64, name: &str, sweater_number: i32) -> SkaterStats {
        SkaterStats {
            player_id,
            sweater_number,
            name: LocalizedString {
                default: name.to_string(),
            },
            position: Position::Center,
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

    fn test_goalie_stats(player_id: i64, name: &str, sweater_number: i32) -> GoalieStats {
        GoalieStats {
            player_id,
            sweater_number,
            name: LocalizedString {
                default: name.to_string(),
            },
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

    /// Boxscore with 2 away forwards, 1 away defenseman, 1 away goalie, 2 home
    /// forwards, 1 home defenseman, and 1 home goalie (8 players total), laid out
    /// in the same flattened order the boxscore's focusable table rows appear in.
    fn test_boxscore(game_id: i64) -> Boxscore {
        let away_forwards = vec![
            test_skater_stats(1001, "Away Forward One", 10),
            test_skater_stats(1002, "Away Forward Two", 11),
        ];
        let away_defense = vec![test_skater_stats(1003, "Away Defense One", 20)];
        let away_goalies = vec![test_goalie_stats(1004, "Away Goalie", 30)];

        let home_forwards = vec![
            test_skater_stats(2001, "Home Forward One", 12),
            test_skater_stats(2002, "Home Forward Two", 13),
        ];
        let home_defense = vec![test_skater_stats(2003, "Home Defense One", 21)];
        let home_goalies = vec![test_goalie_stats(2004, "Home Goalie", 31)];

        Boxscore {
            id: game_id,
            season: 20242025,
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

    fn data_with_boxscore(game_id: i64, boxscore: Boxscore) -> DataState {
        let mut map = HashMap::new();
        map.insert(game_id, boxscore);
        DataState {
            boxscores: Arc::new(map),
            ..Default::default()
        }
    }

    fn test_boxscore_doc(game_id: i64) -> StackedDocument {
        StackedDocument::Boxscore {
            game_id,
            away_abbrev: "NJD".to_string(),
            home_abbrev: "BUF".to_string(),
            away_score: 3,
            home_score: 2,
            game_date: "2024-10-04".to_string(),
        }
    }

    fn test_club_skater(player_id: i64, last_name: &str, points: i32) -> ClubSkaterStats {
        ClubSkaterStats {
            player_id,
            headshot: String::new(),
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: last_name.to_string(),
            },
            position: Position::Center,
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
            player_id,
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

    /// Roster with 3 skaters (points 10/20/30, so sort_by_points_desc yields
    /// High(30)/Mid(20)/Low(10)) and 2 goalies (games played 10/30, so
    /// sort_by_games_played_desc yields High(30)/Low(10)).
    fn test_club_stats() -> ClubStats {
        ClubStats {
            season: "20242025".to_string(),
            game_type: GameType::RegularSeason,
            skaters: vec![
                test_club_skater(100, "Low", 10),
                test_club_skater(200, "High", 30),
                test_club_skater(300, "Mid", 20),
            ],
            goalies: vec![
                test_club_goalie(400, "GoalieLow", 10),
                test_club_goalie(500, "GoalieHigh", 30),
            ],
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

    fn data_with_roster(abbrev: &str, club_stats: ClubStats) -> DataState {
        let mut map = HashMap::new();
        map.insert(abbrev.to_string(), club_stats);
        DataState {
            team_roster_stats: Arc::new(map),
            ..Default::default()
        }
    }

    fn data_with_roster_and_standings(
        abbrev: &str,
        club_stats: ClubStats,
        standings: Vec<Standing>,
    ) -> DataState {
        let mut map = HashMap::new();
        map.insert(abbrev.to_string(), club_stats);
        DataState {
            team_roster_stats: Arc::new(map),
            standings: Arc::new(Some(standings)),
            ..Default::default()
        }
    }

    fn test_team_detail_doc(abbrev: &str) -> StackedDocument {
        StackedDocument::TeamDetail {
            abbrev: abbrev.to_string(),
        }
    }

    fn test_season_total(
        season: i32,
        game_type: GameType,
        league_abbrev: &str,
        team_common_name: Option<&str>,
    ) -> SeasonTotal {
        SeasonTotal {
            season,
            game_type,
            league_abbrev: league_abbrev.to_string(),
            team_name: LocalizedString {
                default: team_common_name.unwrap_or("Unknown").to_string(),
            },
            team_common_name: team_common_name.map(|name| LocalizedString {
                default: name.to_string(),
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

    /// Player landing with 6 season-total rows exercising every filter/guard
    /// activation used to re-derive by hand:
    /// - idx0 -> 2023-24 Oilers (RegularSeason/NHL, valid abbrev) -> EDM
    /// - idx1 -> 2022-23 Maple Leafs (RegularSeason/NHL, valid abbrev) -> TOR
    /// - (2021-22 Oilers Playoffs, and 2020-21 AHL team, are filtered out entirely)
    /// - idx2 -> 2019-20 with no team_common_name -> None guard
    /// - idx3 -> 2018-19 with an unmapped team name -> abbrev lookup guard
    fn test_player_with_seasons(player_id: i64) -> PlayerLanding {
        let seasons = vec![
            test_season_total(20232024, GameType::RegularSeason, "NHL", Some("Oilers")),
            test_season_total(
                20222023,
                GameType::RegularSeason,
                "NHL",
                Some("Maple Leafs"),
            ),
            test_season_total(20212022, GameType::Playoffs, "NHL", Some("Oilers")),
            test_season_total(
                20202021,
                GameType::RegularSeason,
                "AHL",
                Some("Some AHL Team"),
            ),
            test_season_total(20192020, GameType::RegularSeason, "NHL", None),
            test_season_total(
                20182019,
                GameType::RegularSeason,
                "NHL",
                Some("Nonexistent Team"),
            ),
        ];
        test_player_landing(player_id, Some(seasons))
    }

    fn test_player_landing(
        player_id: i64,
        season_totals: Option<Vec<SeasonTotal>>,
    ) -> PlayerLanding {
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
            position: Position::Center,
            headshot: String::new(),
            hero_image: None,
            height_in_inches: 73,
            weight_in_pounds: 200,
            birth_date: "1997-09-15".to_string(),
            birth_city: None,
            birth_state_province: None,
            birth_country: None,
            shoots_catches: Handedness::Left,
            draft_details: None,
            player_slug: None,
            featured_stats: None,
            career_totals: None,
            season_totals,
            awards: None,
            last_five_games: None,
        }
    }

    fn data_with_player(player_id: i64, player: PlayerLanding) -> DataState {
        let mut map = HashMap::new();
        map.insert(player_id, player);
        DataState {
            player_data: Arc::new(map),
            ..Default::default()
        }
    }

    fn test_player_detail_doc(player_id: i64) -> StackedDocument {
        StackedDocument::PlayerDetail {
            player_id,
            sweater_number: Some(34),
            last_name: "Player".to_string(),
        }
    }

    /// Sync focusable metadata via `handle_stacked_document_key`'s own sync
    /// step, without dispatching a key -- lets activation tests set an
    /// arbitrary `focus_index` the way `handle_key` would leave it after
    /// real navigation.
    fn sync_metadata(doc: &StackedDocument, data: &DataState) -> DocumentNavState {
        let mut nav = DocumentNavState::default();
        if let Some(document) = build_stacked_document(doc, data) {
            nav.sync_focusables(
                document.as_ref(),
                &FocusContext::default().with_width(TEST_WIDTH),
            );
        }
        nav
    }

    fn activate_at(doc: &StackedDocument, data: &DataState, focus_idx: usize) -> Effect {
        let mut nav = sync_metadata(doc, data);
        nav.focus_index = Some(focus_idx);
        handle_stacked_document_key(
            doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            data,
            TEST_WIDTH,
        )
    }

    // ========================================================================
    // Boxscore activation (via handle_stacked_document_key)
    //
    // The player pushed by activation comes straight from the focused cell's
    // `LinkTarget::Push(PlayerDetail { .. })`, attached by
    // `DocumentElement::team_boxscore()` when the tables were built.
    // ========================================================================

    #[test]
    fn boxscore_activate_no_focus_returns_none() {
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));
        let mut nav = sync_metadata(&doc, &data);

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn boxscore_activate_missing_boxscore_returns_none() {
        let doc = test_boxscore_doc(1);
        let data = DataState::default();
        let mut nav = DocumentNavState {
            focus_index: Some(0),
            ..Default::default()
        };

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn boxscore_activate_pushes_first_away_forward() {
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));

        match activate_at(&doc, &data, 0) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                sweater_number,
                last_name,
            })) => {
                assert_eq!(player_id, 1001);
                assert_eq!(sweater_number, Some(10));
                assert_eq!(last_name, "Away Forward One");
            }
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn boxscore_activate_pushes_away_goalie_at_away_home_boundary() {
        // Index 3 is the last away-team row (away goalie), the boundary right
        // before the home team's rows begin.
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));

        match activate_at(&doc, &data, 3) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                sweater_number,
                last_name,
            })) => {
                assert_eq!(player_id, 1004);
                assert_eq!(sweater_number, Some(30));
                assert_eq!(last_name, "Away Goalie");
            }
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn boxscore_activate_pushes_first_home_forward_after_boundary() {
        // Index 4 is the first home-team row, right after the away/home boundary.
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));

        match activate_at(&doc, &data, 4) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                ..
            })) => {
                assert_eq!(player_id, 2001);
            }
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn boxscore_activate_pushes_last_home_goalie() {
        // Index 7 is the last focusable element overall (boundary: last index).
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));

        match activate_at(&doc, &data, 7) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                sweater_number,
                last_name,
            })) => {
                assert_eq!(player_id, 2004);
                assert_eq!(sweater_number, Some(31));
                assert_eq!(last_name, "Home Goalie");
            }
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn boxscore_activate_out_of_range_returns_none() {
        // Only 8 focusable rows exist (index 0..=7); index 8 is one past the end.
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));

        assert!(matches!(activate_at(&doc, &data, 8), Effect::None));
    }

    // ========================================================================
    // Boxscore metadata sync (via handle_stacked_document_key's own sync step)
    // ========================================================================

    #[test]
    fn boxscore_sync_fills_focusables_from_all_players() {
        let doc = test_boxscore_doc(1);
        let data = data_with_boxscore(1, test_boxscore(1));

        let nav = sync_metadata(&doc, &data);

        assert_eq!(nav.focusables.len(), 8);
        assert!(
            nav.focusables.iter().all(|f| f.link_target.is_some()),
            "every boxscore row is a player link"
        );
    }

    #[test]
    fn boxscore_sync_leaves_nav_untouched_when_missing() {
        // When the boxscore hasn't loaded yet, `build_stacked_document`
        // returns `None` and the sync step is skipped entirely -- stale nav
        // metadata is left exactly as it was rather than cleared to empty.
        let doc = test_boxscore_doc(1);
        let data = DataState::default();
        let stale = vec![FocusableElement::at(1, 1, FocusableId::link("stale"))];
        let mut nav = DocumentNavState {
            focusables: stale.clone(),
            ..Default::default()
        };

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );

        assert_eq!(nav.focusables, stale);
    }

    // ========================================================================
    // TeamDetail activation (via handle_stacked_document_key)
    // ========================================================================

    #[test]
    fn team_detail_activate_no_focus_returns_none() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = sync_metadata(&doc, &data);

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn team_detail_activate_missing_roster_returns_none() {
        let doc = test_team_detail_doc("TST");
        let data = DataState::default();
        let mut nav = DocumentNavState {
            focus_index: Some(0),
            ..Default::default()
        };

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn team_detail_activate_pushes_highest_scoring_skater() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());

        match activate_at(&doc, &data, 0) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                sweater_number,
                last_name,
            })) => {
                assert_eq!(player_id, 200);
                assert_eq!(sweater_number, None);
                assert_eq!(last_name, "High");
            }
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn team_detail_activate_pushes_mid_scoring_skater() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());

        match activate_at(&doc, &data, 1) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                ..
            })) => assert_eq!(player_id, 300),
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn team_detail_activate_pushes_highest_games_played_goalie_at_skater_goalie_boundary() {
        // 3 skaters precede the goalies, so index 3 is the first sorted goalie
        // -- the boundary right after the skater rows end.
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());

        match activate_at(&doc, &data, 3) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                last_name,
                ..
            })) => {
                assert_eq!(player_id, 500);
                assert_eq!(last_name, "GoalieHigh");
            }
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn team_detail_activate_pushes_last_goalie() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());

        // Boundary: focus at the last focusable index.
        match activate_at(&doc, &data, 4) {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                ..
            })) => assert_eq!(player_id, 400),
            other => panic!("expected Effect::Action(PushDocument(PlayerDetail)), got {other:?}"),
        }
    }

    #[test]
    fn team_detail_activate_out_of_range_returns_none() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        // 3 skaters + 2 goalies = 5 total; index 5 is one past the end.
        assert!(matches!(activate_at(&doc, &data, 5), Effect::None));
    }

    // ========================================================================
    // TeamDetail metadata sync
    // ========================================================================

    #[test]
    fn team_detail_sync_fills_focusables_from_roster() {
        let doc = test_team_detail_doc("TST");
        let data =
            data_with_roster_and_standings("TST", test_club_stats(), vec![test_standing("TST")]);

        let nav = sync_metadata(&doc, &data);

        assert_eq!(nav.focusables.len(), 5);
        assert!(
            nav.focusables.iter().all(|f| f.link_target.is_some()),
            "every roster row is a player link"
        );
    }

    #[test]
    fn team_detail_sync_ignores_non_matching_standing() {
        let doc = test_team_detail_doc("TST");
        // Standings present, but none match this document's abbrev.
        let data =
            data_with_roster_and_standings("TST", test_club_stats(), vec![test_standing("OTH")]);

        let nav = sync_metadata(&doc, &data);

        // Roster-derived focusable count is unaffected by the missing standing.
        assert_eq!(nav.focusables.len(), 5);
    }

    #[test]
    fn team_detail_sync_leaves_nav_untouched_when_roster_missing() {
        // Unified with Boxscore/PlayerDetail: when data hasn't loaded,
        // `build_stacked_document` returns `None` and the sync step is
        // skipped, so stale nav metadata is left alone rather than cleared.
        let doc = test_team_detail_doc("ZZZ");
        let data = DataState::default();
        let stale = vec![FocusableElement::at(1, 1, FocusableId::link("stale"))];
        let mut nav = DocumentNavState {
            focusables: stale.clone(),
            ..Default::default()
        };

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );

        assert_eq!(nav.focusables, stale);
    }

    // ========================================================================
    // PlayerDetail activation
    //
    // The pushed team comes from the focused season row's
    // `LinkTarget::Push(TeamDetail { .. })`, attached (via the generic
    // `DocumentElement::table()` cell match) only when the row's team
    // resolves to a real abbreviation. Seasons that don't resolve simply
    // never appear in `focusables`, so they can't be reached by navigation at
    // all -- see `player_detail_activate_focus_index_mismatch_is_fixed` below.
    // ========================================================================

    #[test]
    fn player_detail_activate_no_focus_returns_none() {
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_with_seasons(1));
        let mut nav = sync_metadata(&doc, &data);

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn player_detail_activate_missing_player_returns_none() {
        let doc = test_player_detail_doc(1);
        let data = DataState::default();
        let mut nav = DocumentNavState {
            focus_index: Some(0),
            ..Default::default()
        };

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn player_detail_activate_missing_season_totals_returns_none() {
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_landing(1, None));
        let mut nav = sync_metadata(&doc, &data);
        nav.focus_index = Some(0);

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn player_detail_activate_pushes_team_detail_for_first_season() {
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_with_seasons(1));

        match activate_at(&doc, &data, 0) {
            Effect::Action(Action::PushDocument(StackedDocument::TeamDetail { abbrev })) => {
                assert_eq!(abbrev, "EDM");
            }
            other => panic!("expected Effect::Action(PushDocument(TeamDetail)), got {other:?}"),
        }
    }

    #[test]
    fn player_detail_activate_pushes_team_detail_for_second_season() {
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_with_seasons(1));

        match activate_at(&doc, &data, 1) {
            Effect::Action(Action::PushDocument(StackedDocument::TeamDetail { abbrev })) => {
                assert_eq!(abbrev, "TOR");
            }
            other => panic!("expected Effect::Action(PushDocument(TeamDetail)), got {other:?}"),
        }
    }

    #[test]
    fn player_detail_activate_beyond_resolvable_seasons_returns_none() {
        // Only 2 of the fixture's seasons resolve to a real team abbreviation
        // (see `player_detail_sync_fills_focusables_from_linkable_seasons`),
        // so index 2 is one past the end of the focusable list regardless of
        // how many seasons survive the RegularSeason/NHL filter upstream (4,
        // in this fixture). Unresolvable seasons (missing team_common_name,
        // or an unmapped team name) never produce a focusable row in the
        // first place, so there's no distinct "wrong reason" for this to
        // fail -- it's a plain out-of-range read.
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_with_seasons(1));

        assert!(matches!(activate_at(&doc, &data, 2), Effect::None));
    }

    // ========================================================================
    // PlayerDetail metadata sync
    // ========================================================================

    #[test]
    fn player_detail_sync_fills_focusables_from_linkable_seasons() {
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_with_seasons(1));

        let nav = sync_metadata(&doc, &data);

        // 4 of the 6 seasons survive the RegularSeason/NHL filter, but a
        // season's table row is only focusable when its team column renders
        // as a link (see `DocumentElement::table()`, which skips non-link
        // cells). Of the 4 filtered seasons, only 2 (Oilers, Maple Leafs)
        // resolve to a real team abbreviation and thus a focusable
        // TableCell; see `player_detail_activate_focus_index_mismatch_is_fixed`
        // for why this divergence between the filtered array and the
        // focusable list used to matter (and no longer can).
        assert_eq!(nav.focusables.len(), 2);
        assert!(
            nav.focusables.iter().all(|f| f.link_target.is_some()),
            "every linkable season row carries a team link"
        );
    }

    #[test]
    fn player_detail_activate_focus_index_mismatch_is_fixed() {
        // FIXED LATENT BUG: activation used to index directly into the
        // RegularSeason/NHL filtered, season-sorted array using
        // `nav.focus_index`. But `nav.focus_index` is set by the navigation
        // system to an index into the FOCUSABLE table cells, and a season
        // row only becomes focusable when its team column resolves to a real
        // abbreviation -- a season with an unresolvable team name was still
        // included in the filtered array but contributed zero focusable
        // rows, so the two index spaces could diverge.
        //
        // Here the newest season (sorts first, array index 0) has no
        // resolvable team, so it produces no focusable row. The only
        // focusable table cell belongs to the older, resolvable season, and
        // the focus system assigns it focus_index 0 (the first and only
        // focusable element).
        //
        // Now that activation reads `nav.focusables[nav.focus_index].link_target`
        // directly -- populated by walking the same focusable cells the user
        // navigates -- the two index spaces can't diverge: pressing Enter on
        // the visibly-focused row pushes its real destination.
        let seasons = vec![
            test_season_total(20242025, GameType::RegularSeason, "NHL", None),
            test_season_total(20232024, GameType::RegularSeason, "NHL", Some("Oilers")),
        ];
        let doc = test_player_detail_doc(1);
        let data = data_with_player(1, test_player_landing(1, Some(seasons)));

        let nav = sync_metadata(&doc, &data);
        // Only one focusable row exists (the Oilers season), so real
        // keyboard navigation can only ever produce focus_index 0 here.
        assert_eq!(nav.focusables.len(), 1);

        match activate_at(&doc, &data, 0) {
            Effect::Action(Action::PushDocument(StackedDocument::TeamDetail { abbrev })) => {
                assert_eq!(abbrev, "EDM");
            }
            other => {
                panic!("expected Effect::Action(PushDocument(TeamDetail{{EDM}})), got {other:?}")
            }
        }
    }

    #[test]
    fn player_detail_sync_leaves_nav_untouched_when_player_missing() {
        let doc = test_player_detail_doc(999);
        let data = DataState::default();
        let stale = vec![FocusableElement::at(1, 1, FocusableId::link("stale"))];
        let mut nav = DocumentNavState {
            focusables: stale.clone(),
            ..Default::default()
        };

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );

        assert_eq!(nav.focusables, stale);
    }

    // ========================================================================
    // handle_stacked_document_key integration tests (navigation, scrolling,
    // activation)
    // ========================================================================

    #[test]
    fn handle_key_down_focuses_first_then_advances() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = DocumentNavState::default();

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.focus_index, Some(0));

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.focus_index, Some(1));
    }

    #[test]
    fn handle_key_up_wraps_from_first_to_last() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = DocumentNavState::default();

        // Up from no focus wraps directly to the last element (index 4 of 5).
        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Up),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.focus_index, Some(4));
    }

    #[test]
    fn handle_key_enter_activates_focused_skater() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = DocumentNavState::default();

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Enter),
            &mut nav,
            &data,
            TEST_WIDTH,
        );

        match effect {
            Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id,
                ..
            })) => assert_eq!(player_id, 200),
            _ => panic!("expected Effect::Action(PushDocument(PlayerDetail))"),
        }
    }

    #[test]
    fn handle_key_page_down_and_page_up_scroll_by_viewport_size() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = DocumentNavState {
            viewport_height: TEST_VIEWPORT_HEIGHT,
            ..Default::default()
        };

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::PageDown),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.scroll_offset, TEST_VIEWPORT_HEIGHT);

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::PageUp),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.scroll_offset, 0);
    }

    #[test]
    fn handle_key_home_and_end_scroll_to_top_and_bottom() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = DocumentNavState::default();

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::End),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.scroll_offset, u16::MAX);

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Home),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.scroll_offset, 0);
    }

    #[test]
    fn handle_key_on_empty_document_does_not_set_focus() {
        // Boundary: no roster loaded for this abbrev, so there are zero
        // focusable elements. Navigation keys must be a no-op.
        let doc = test_team_detail_doc("ZZZ");
        let data = DataState::default();
        let mut nav = DocumentNavState::default();

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );

        assert!(matches!(effect, Effect::None));
        assert_eq!(nav.focus_index, None);
    }

    #[test]
    fn handle_key_on_single_element_document_wraps_to_itself() {
        // Boundary: a roster with exactly one focusable element (one skater, no
        // goalies) should keep focus pinned to index 0 across repeated FocusNext.
        let doc = test_team_detail_doc("TST");
        let club_stats = ClubStats {
            season: "20242025".to_string(),
            game_type: GameType::RegularSeason,
            skaters: vec![test_club_skater(100, "Solo", 10)],
            goalies: vec![],
        };
        let data = data_with_roster("TST", club_stats);
        let mut nav = DocumentNavState::default();

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.focus_index, Some(0));

        handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Down),
            &mut nav,
            &data,
            TEST_WIDTH,
        );
        assert_eq!(nav.focus_index, Some(0));
    }

    #[test]
    fn handle_key_non_navigation_key_returns_none() {
        let doc = test_team_detail_doc("TST");
        let data = data_with_roster("TST", test_club_stats());
        let mut nav = DocumentNavState::default();

        let effect = handle_stacked_document_key(
            &doc,
            KeyEvent::from(KeyCode::Char('x')),
            &mut nav,
            &data,
            TEST_WIDTH,
        );

        assert!(matches!(effect, Effect::None));
        assert_eq!(nav.focus_index, None);
    }
}
