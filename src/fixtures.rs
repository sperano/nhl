/// Mock fixture data for testing and development
///
/// This module provides consistent, deterministic fixture data that can be used for:
/// 1. Unit and integration tests - ensuring tests have predictable data
/// 2. Development mock mode - running the app with fake data for screenshots and debugging
/// 3. Benchmarks - providing consistent data for performance testing
///
/// The fixtures represent realistic NHL data with all 32 teams and various game states.
use nhl_api::{
    Boxscore, BoxscoreTeam, DailySchedule, DefendingSide, Franchise, GameClock, GameDate, GameLog,
    GameMatchup, GameOutcome, GameScheduleState, GameState, GameType, Handedness, HomeRoad,
    LocalizedString, PeriodDescriptor, PeriodType, PlayByPlay, PlayEvent, PlayEventDetails,
    PlayEventType, PlayerByGameStats, PlayerGameLog, PlayerLanding, PlayerSearchResult, Position,
    RosterSpot, ScheduleGame, ScheduleTeam, Standing, TeamPlayerStats, ZoneCode,
};

/// Create mock standings data - reusing the test data structure
pub fn create_mock_standings() -> Vec<Standing> {
    crate::tui::testing::create_test_standings()
}

/// Create mock daily schedule with games in various states
pub fn create_mock_schedule(date: Option<GameDate>) -> DailySchedule {
    let date = date.unwrap_or_else(GameDate::today);
    let date_string = date.to_api_string();

    let games = vec![
        create_mock_game(2024020001, "BOS", "MTL", GameState::Future),
        create_mock_game(2024020002, "TOR", "OTT", GameState::Live),
        create_mock_game(2024020003, "NYR", "NJD", GameState::Final),
        create_mock_game(2024020004, "VGK", "LA", GameState::Final),
    ];

    DailySchedule {
        date: date_string,
        next_start_date: Some(date.add_days(1).to_api_string()),
        previous_start_date: Some(date.add_days(-1).to_api_string()),
        number_of_games: games.len(),
        games,
    }
}

/// Helper to create a mock game
fn create_mock_game(
    id: i64,
    away_abbrev: &str,
    home_abbrev: &str,
    status: GameState,
) -> ScheduleGame {
    ScheduleGame {
        id,
        game_type: nhl_api::GameType::RegularSeason,
        game_date: Some("2024-11-20".to_string()),
        start_time_utc: "2024-11-21T00:00:00Z".to_string(),
        game_state: status,
        away_team: ScheduleTeam {
            id: away_abbrev.chars().map(|c| c as i32).sum::<i32>() as i64,
            abbrev: away_abbrev.to_string(),
            score: if status == GameState::Live || status == GameState::Final {
                Some(2)
            } else {
                None
            },
            logo: format!(
                "https://assets.nhle.com/logos/nhl/svg/{}_light.svg",
                away_abbrev
            ),
            place_name: None,
        },
        home_team: ScheduleTeam {
            id: home_abbrev.chars().map(|c| c as i32).sum::<i32>() as i64,
            abbrev: home_abbrev.to_string(),
            score: if status == GameState::Live || status == GameState::Final {
                Some(3)
            } else {
                None
            },
            logo: format!(
                "https://assets.nhle.com/logos/nhl/svg/{}_light.svg",
                home_abbrev
            ),
            place_name: None,
        },
    }
}

/// Create mock game matchup (landing page data)
pub fn create_mock_game_matchup(game_id: i64) -> GameMatchup {
    match game_id {
        2024020001 => create_game_matchup_not_started(),
        2024020002 => create_game_matchup_in_progress(1),
        2024020003 => create_game_matchup_in_progress(2),
        2024020004 => create_game_matchup_in_progress(3),
        2024020005 => create_game_matchup_final(false),
        2024020006 => create_game_matchup_final(true),
        _ => create_game_matchup_final(false),
    }
}

fn create_game_matchup_not_started() -> GameMatchup {
    GameMatchup {
        id: 2024020001,
        season: 20242025,
        game_type: nhl_api::GameType::RegularSeason,
        limited_scoring: false,
        game_date: "2024-11-20".to_string(),
        venue: nhl_api::LocalizedString {
            default: "TD Garden".to_string(),
        },
        venue_location: nhl_api::LocalizedString {
            default: "Boston, MA".to_string(),
        },
        start_time_utc: "2024-11-21T00:00:00Z".to_string(),
        eastern_utc_offset: "-05:00".to_string(),
        venue_utc_offset: "-05:00".to_string(),
        venue_timezone: "America/New_York".to_string(),
        period_descriptor: nhl_api::PeriodDescriptor {
            number: 0,
            period_type: PeriodType::Regulation,
            max_regulation_periods: 3,
        },
        tv_broadcasts: vec![],
        game_state: GameState::Future,
        game_schedule_state: nhl_api::GameScheduleState::Ok,
        special_event: None,
        away_team: create_matchup_team("MTL", "Canadiens", "Montreal", 10, 5, 3, 0, 0),
        home_team: create_matchup_team("BOS", "Bruins", "Boston", 13, 4, 1, 0, 0),
        shootout_in_use: true,
        max_periods: 5,
        reg_periods: 3,
        ot_in_use: true,
        ties_in_use: false,
        summary: None,
        clock: None,
    }
}

fn create_game_matchup_in_progress(period: i32) -> GameMatchup {
    let (away_score, home_score, shots_away, shots_home) = match period {
        1 => (1, 0, 8, 6),
        2 => (2, 3, 18, 15),
        3 => (4, 3, 28, 25),
        _ => (0, 0, 0, 0),
    };

    GameMatchup {
        id: 2024020002 + (period - 1) as i64,
        season: 20242025,
        game_type: nhl_api::GameType::RegularSeason,
        limited_scoring: false,
        game_date: "2024-11-20".to_string(),
        venue: nhl_api::LocalizedString {
            default: "Scotiabank Arena".to_string(),
        },
        venue_location: nhl_api::LocalizedString {
            default: "Toronto, ON".to_string(),
        },
        start_time_utc: "2024-11-21T00:00:00Z".to_string(),
        eastern_utc_offset: "-05:00".to_string(),
        venue_utc_offset: "-05:00".to_string(),
        venue_timezone: "America/Toronto".to_string(),
        period_descriptor: nhl_api::PeriodDescriptor {
            number: period,
            period_type: PeriodType::Regulation,
            max_regulation_periods: 3,
        },
        tv_broadcasts: vec![],
        game_state: GameState::Live,
        game_schedule_state: nhl_api::GameScheduleState::Ok,
        special_event: None,
        away_team: create_matchup_team(
            "TOR",
            "Maple Leafs",
            "Toronto",
            12,
            5,
            2,
            away_score,
            shots_away,
        ),
        home_team: create_matchup_team(
            "OTT", "Senators", "Ottawa", 9, 7, 2, home_score, shots_home,
        ),
        shootout_in_use: true,
        max_periods: 5,
        reg_periods: 3,
        ot_in_use: true,
        ties_in_use: false,
        summary: Some(create_game_summary(period, away_score, home_score)),
        clock: Some(nhl_api::GameClock {
            time_remaining: "12:34".to_string(),
            seconds_remaining: 754,
            running: true,
            in_intermission: false,
        }),
    }
}

fn create_game_matchup_final(overtime: bool) -> GameMatchup {
    let (away_score, home_score, shots_away, shots_home) = if overtime {
        (3, 4, 35, 32)
    } else {
        (2, 5, 28, 34)
    };

    GameMatchup {
        id: if overtime { 2024020006 } else { 2024020005 },
        season: 20242025,
        game_type: nhl_api::GameType::RegularSeason,
        limited_scoring: false,
        game_date: "2024-11-20".to_string(),
        venue: nhl_api::LocalizedString {
            default: if overtime {
                "T-Mobile Arena"
            } else {
                "Rogers Place"
            }
            .to_string(),
        },
        venue_location: nhl_api::LocalizedString {
            default: if overtime {
                "Las Vegas, NV"
            } else {
                "Edmonton, AB"
            }
            .to_string(),
        },
        start_time_utc: "2024-11-21T03:00:00Z".to_string(),
        eastern_utc_offset: "-05:00".to_string(),
        venue_utc_offset: if overtime { "-08:00" } else { "-07:00" }.to_string(),
        venue_timezone: if overtime {
            "America/Los_Angeles"
        } else {
            "America/Edmonton"
        }
        .to_string(),
        period_descriptor: nhl_api::PeriodDescriptor {
            number: if overtime { 4 } else { 3 },
            period_type: if overtime {
                PeriodType::Overtime
            } else {
                PeriodType::Regulation
            },
            max_regulation_periods: 3,
        },
        tv_broadcasts: vec![],
        game_state: GameState::Final,
        game_schedule_state: nhl_api::GameScheduleState::Ok,
        special_event: None,
        away_team: create_matchup_team(
            if overtime { "VGK" } else { "CGY" },
            if overtime { "Golden Knights" } else { "Flames" },
            if overtime { "Vegas" } else { "Calgary" },
            if overtime { 15 } else { 9 },
            if overtime { 3 } else { 8 },
            if overtime { 1 } else { 2 },
            away_score,
            shots_away,
        ),
        home_team: create_matchup_team(
            if overtime { "LA" } else { "EDM" },
            if overtime { "Kings" } else { "Oilers" },
            if overtime { "Los Angeles" } else { "Edmonton" },
            if overtime { 12 } else { 14 },
            if overtime { 6 } else { 4 },
            if overtime { 1 } else { 2 },
            home_score,
            shots_home,
        ),
        shootout_in_use: true,
        max_periods: 5,
        reg_periods: 3,
        ot_in_use: true,
        ties_in_use: false,
        summary: Some(create_game_summary(
            if overtime { 4 } else { 3 },
            away_score,
            home_score,
        )),
        clock: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn create_matchup_team(
    abbrev: &str,
    name: &str,
    place: &str,
    _wins: i32,
    _losses: i32,
    _ot: i32,
    score: i32,
    sog: i32,
) -> nhl_api::MatchupTeam {
    nhl_api::MatchupTeam {
        id: abbrev.chars().map(|c| c as i32).sum::<i32>() as i64,
        common_name: nhl_api::LocalizedString {
            default: name.to_string(),
        },
        abbrev: abbrev.to_string(),
        place_name: nhl_api::LocalizedString {
            default: place.to_string(),
        },
        place_name_with_preposition: nhl_api::LocalizedString {
            default: format!("in {}", place),
        },
        score,
        sog,
        logo: format!("https://assets.nhle.com/logos/nhl/svg/{}_light.svg", abbrev),
        dark_logo: format!("https://assets.nhle.com/logos/nhl/svg/{}_dark.svg", abbrev),
    }
}

fn create_game_summary(_period: i32, _away_score: i32, _home_score: i32) -> nhl_api::GameSummary {
    nhl_api::GameSummary {
        scoring: vec![],
        shootout: None,
        three_stars: None,
        penalties: vec![],
    }
}

/// Create mock boxscore
pub fn create_mock_boxscore(game_id: i64) -> Boxscore {
    let is_live = game_id == 2024020002 || game_id == 2024020003 || game_id == 2024020004;
    let period = if game_id == 2024020002 {
        1
    } else if game_id == 2024020003 {
        2
    } else {
        3
    };

    Boxscore {
        id: game_id,
        season: 20242025,
        game_type: nhl_api::GameType::RegularSeason,
        limited_scoring: false,
        game_date: "2024-11-20".to_string(),
        venue: LocalizedString {
            default: "Scotiabank Arena".to_string(),
        },
        venue_location: LocalizedString {
            default: "Toronto, ON".to_string(),
        },
        start_time_utc: "2024-11-21T00:00:00Z".to_string(),
        eastern_utc_offset: "-05:00".to_string(),
        venue_utc_offset: "-05:00".to_string(),
        tv_broadcasts: vec![
            nhl_api::TvBroadcast {
                id: 1,
                market: "N".to_string(),
                country_code: "US".to_string(),
                network: "ESPN+".to_string(),
                sequence_number: 1,
            },
            nhl_api::TvBroadcast {
                id: 2,
                market: "H".to_string(),
                country_code: "CA".to_string(),
                network: "SN".to_string(),
                sequence_number: 2,
            },
        ],
        game_state: if is_live {
            GameState::Live
        } else {
            GameState::Final
        },
        game_schedule_state: "OK".to_string(),
        period_descriptor: PeriodDescriptor {
            number: period,
            period_type: PeriodType::Regulation,
            max_regulation_periods: 3,
        },
        special_event: None,
        away_team: BoxscoreTeam {
            id: 10,
            common_name: LocalizedString {
                default: "Maple Leafs".to_string(),
            },
            abbrev: "TOR".to_string(),
            score: if is_live { 2 } else { 3 },
            sog: if is_live { 20 } else { 32 },
            logo: "https://assets.nhle.com/logos/nhl/svg/TOR_light.svg".to_string(),
            dark_logo: "https://assets.nhle.com/logos/nhl/svg/TOR_dark.svg".to_string(),
            place_name: LocalizedString {
                default: "Toronto".to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: "in Toronto".to_string(),
            },
        },
        home_team: BoxscoreTeam {
            id: 9,
            common_name: LocalizedString {
                default: "Senators".to_string(),
            },
            abbrev: "OTT".to_string(),
            score: if is_live { 3 } else { 4 },
            sog: if is_live { 18 } else { 28 },
            logo: "https://assets.nhle.com/logos/nhl/svg/OTT_light.svg".to_string(),
            dark_logo: "https://assets.nhle.com/logos/nhl/svg/OTT_dark.svg".to_string(),
            place_name: LocalizedString {
                default: "Ottawa".to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: "in Ottawa".to_string(),
            },
        },
        clock: if is_live {
            GameClock {
                time_remaining: "12:34".to_string(),
                seconds_remaining: 754,
                running: true,
                in_intermission: false,
            }
        } else {
            GameClock {
                time_remaining: "00:00".to_string(),
                seconds_remaining: 0,
                running: false,
                in_intermission: false,
            }
        },
        player_by_game_stats: PlayerByGameStats {
            away_team: TeamPlayerStats {
                forwards: vec![],
                defense: vec![],
                goalies: vec![],
            },
            home_team: TeamPlayerStats {
                forwards: vec![],
                defense: vec![],
                goalies: vec![],
            },
        },
    }
}

/// Create mock franchises
pub fn create_mock_franchises() -> Vec<Franchise> {
    vec![
        Franchise {
            id: 1,
            full_name: "Montreal Canadiens".to_string(),
            team_common_name: "Canadiens".to_string(),
            team_place_name: "Montreal".to_string(),
        },
        Franchise {
            id: 6,
            full_name: "Boston Bruins".to_string(),
            team_common_name: "Bruins".to_string(),
            team_place_name: "Boston".to_string(),
        },
        Franchise {
            id: 10,
            full_name: "Toronto Maple Leafs".to_string(),
            team_common_name: "Maple Leafs".to_string(),
            team_place_name: "Toronto".to_string(),
        },
        Franchise {
            id: 9,
            full_name: "Ottawa Senators".to_string(),
            team_common_name: "Senators".to_string(),
            team_place_name: "Ottawa".to_string(),
        },
        Franchise {
            id: 15,
            full_name: "Florida Panthers".to_string(),
            team_common_name: "Panthers".to_string(),
            team_place_name: "Florida".to_string(),
        },
    ]
}

/// Create mock club stats
pub fn create_mock_club_stats(
    _team: &str,
    season: i32,
    game_type: nhl_api::GameType,
) -> nhl_api::ClubStats {
    nhl_api::ClubStats {
        season: season.to_string(),
        game_type,
        skaters: vec![],
        goalies: vec![],
    }
}

/// Create mock player landing
pub fn create_mock_player_landing(player_id: i64) -> PlayerLanding {
    PlayerLanding {
        player_id,
        is_active: true,
        current_team_id: Some(22),
        current_team_abbrev: Some("EDM".to_string()),
        first_name: LocalizedString {
            default: "Connor".to_string(),
        },
        last_name: LocalizedString {
            default: "McDavid".to_string(),
        },
        sweater_number: Some(97),
        position: Position::Center,
        headshot: "https://assets.nhle.com/mugs/nhl/20242025/EDM/8478402.png".to_string(),
        hero_image: None,
        height_in_inches: 73,
        weight_in_pounds: 193,
        birth_date: "1997-01-13".to_string(),
        birth_city: Some(LocalizedString {
            default: "Richmond Hill".to_string(),
        }),
        birth_state_province: Some(LocalizedString {
            default: "ON".to_string(),
        }),
        birth_country: Some("CAN".to_string()),
        shoots_catches: Handedness::Left,
        draft_details: None,
        player_slug: Some("connor-mcdavid-8478402".to_string()),
        featured_stats: None,
        career_totals: None,
        season_totals: None,
        awards: None,
        last_five_games: Some(vec![
            GameLog {
                game_id: 2024020500,
                game_date: "2024-12-28".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Home,
                opponent_abbrev: "CGY".to_string(),
                goals: 2,
                assists: 1,
                points: 3,
                plus_minus: 2,
                power_play_goals: 1,
                power_play_points: 2,
                shots: 5,
                shifts: 24,
                toi: "21:45".to_string(),
                game_winning_goals: Some(1),
                ot_goals: None,
                pim: Some(0),
            },
            GameLog {
                game_id: 2024020480,
                game_date: "2024-12-26".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Road,
                opponent_abbrev: "VAN".to_string(),
                goals: 1,
                assists: 2,
                points: 3,
                plus_minus: 1,
                power_play_goals: 0,
                power_play_points: 0,
                shots: 4,
                shifts: 22,
                toi: "20:30".to_string(),
                game_winning_goals: None,
                ot_goals: None,
                pim: Some(2),
            },
            GameLog {
                game_id: 2024020460,
                game_date: "2024-12-23".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Home,
                opponent_abbrev: "SEA".to_string(),
                goals: 0,
                assists: 3,
                points: 3,
                plus_minus: 2,
                power_play_goals: 0,
                power_play_points: 0,
                shots: 6,
                shifts: 25,
                toi: "22:15".to_string(),
                game_winning_goals: None,
                ot_goals: None,
                pim: Some(0),
            },
            GameLog {
                game_id: 2024020440,
                game_date: "2024-12-21".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Road,
                opponent_abbrev: "LA".to_string(),
                goals: 1,
                assists: 1,
                points: 2,
                plus_minus: 0,
                power_play_goals: 1,
                power_play_points: 1,
                shots: 3,
                shifts: 23,
                toi: "21:00".to_string(),
                game_winning_goals: None,
                ot_goals: None,
                pim: Some(0),
            },
            GameLog {
                game_id: 2024020420,
                game_date: "2024-12-19".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Home,
                opponent_abbrev: "ANA".to_string(),
                goals: 3,
                assists: 0,
                points: 3,
                plus_minus: 3,
                power_play_goals: 1,
                power_play_points: 2,
                shots: 7,
                shifts: 26,
                toi: "23:10".to_string(),
                game_winning_goals: Some(1),
                ot_goals: None,
                pim: Some(2),
            },
        ]),
    }
}

/// Create mock player search results
pub fn create_mock_player_search(query: &str, limit: Option<i32>) -> Vec<PlayerSearchResult> {
    let all_players = vec![
        PlayerSearchResult {
            player_id: "8478402".to_string(),
            name: "Connor McDavid".to_string(),
            position: Position::Center,
            team_id: Some("22".to_string()),
            team_abbrev: Some("EDM".to_string()),
            sweater_number: Some(97),
            active: true,
            height: Some("6'1\"".to_string()),
            birth_city: Some("Richmond Hill".to_string()),
            birth_state_province: Some("ON".to_string()),
            birth_country: Some("CAN".to_string()),
        },
        PlayerSearchResult {
            player_id: "8478483".to_string(),
            name: "Auston Matthews".to_string(),
            position: Position::Center,
            team_id: Some("10".to_string()),
            team_abbrev: Some("TOR".to_string()),
            sweater_number: Some(34),
            active: true,
            height: Some("6'3\"".to_string()),
            birth_city: Some("San Ramon".to_string()),
            birth_state_province: Some("CA".to_string()),
            birth_country: Some("USA".to_string()),
        },
        PlayerSearchResult {
            player_id: "8477492".to_string(),
            name: "Nathan MacKinnon".to_string(),
            position: Position::Center,
            team_id: Some("21".to_string()),
            team_abbrev: Some("COL".to_string()),
            sweater_number: Some(29),
            active: true,
            height: Some("6'0\"".to_string()),
            birth_city: Some("Halifax".to_string()),
            birth_state_province: Some("NS".to_string()),
            birth_country: Some("CAN".to_string()),
        },
    ];

    let query_lower = query.to_lowercase();
    let mut results: Vec<PlayerSearchResult> = all_players
        .into_iter()
        .filter(|p| p.name.to_lowercase().contains(&query_lower))
        .collect();

    let limit = limit.unwrap_or(20) as usize;
    results.truncate(limit);
    results
}

/// Create mock player game log
pub fn create_mock_player_game_log(
    player_id: i64,
    season: i32,
    game_type: GameType,
) -> PlayerGameLog {
    PlayerGameLog {
        player_id,
        season,
        game_type,
        game_log: vec![
            GameLog {
                game_id: 2024020500,
                game_date: "2024-12-28".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Home,
                opponent_abbrev: "CGY".to_string(),
                goals: 2,
                assists: 1,
                points: 3,
                plus_minus: 2,
                power_play_goals: 1,
                power_play_points: 2,
                shots: 5,
                shifts: 24,
                toi: "21:45".to_string(),
                game_winning_goals: Some(1),
                ot_goals: None,
                pim: Some(0),
            },
            GameLog {
                game_id: 2024020480,
                game_date: "2024-12-26".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Road,
                opponent_abbrev: "VAN".to_string(),
                goals: 1,
                assists: 2,
                points: 3,
                plus_minus: 1,
                power_play_goals: 0,
                power_play_points: 0,
                shots: 4,
                shifts: 22,
                toi: "20:30".to_string(),
                game_winning_goals: None,
                ot_goals: None,
                pim: Some(2),
            },
            GameLog {
                game_id: 2024020460,
                game_date: "2024-12-23".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Home,
                opponent_abbrev: "SEA".to_string(),
                goals: 0,
                assists: 3,
                points: 3,
                plus_minus: 2,
                power_play_goals: 0,
                power_play_points: 0,
                shots: 6,
                shifts: 25,
                toi: "22:15".to_string(),
                game_winning_goals: None,
                ot_goals: None,
                pim: Some(0),
            },
            GameLog {
                game_id: 2024020440,
                game_date: "2024-12-21".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Road,
                opponent_abbrev: "LA".to_string(),
                goals: 1,
                assists: 1,
                points: 2,
                plus_minus: 0,
                power_play_goals: 1,
                power_play_points: 1,
                shots: 3,
                shifts: 23,
                toi: "21:00".to_string(),
                game_winning_goals: None,
                ot_goals: None,
                pim: Some(0),
            },
            GameLog {
                game_id: 2024020420,
                game_date: "2024-12-19".to_string(),
                team_abbrev: "EDM".to_string(),
                home_road_flag: HomeRoad::Home,
                opponent_abbrev: "ANA".to_string(),
                goals: 3,
                assists: 0,
                points: 3,
                plus_minus: 3,
                power_play_goals: 1,
                power_play_points: 2,
                shots: 7,
                shifts: 26,
                toi: "23:10".to_string(),
                game_winning_goals: Some(1),
                ot_goals: None,
                pim: Some(2),
            },
        ],
    }
}

/// Create mock play-by-play data
pub fn create_mock_play_by_play(game_id: i64) -> PlayByPlay {
    let is_live = game_id == 2024020002 || game_id == 2024020003 || game_id == 2024020004;
    let period = if game_id == 2024020002 {
        1
    } else if game_id == 2024020003 {
        2
    } else {
        3
    };

    let (away_score, home_score) = if is_live { (2, 3) } else { (3, 4) };

    PlayByPlay {
        id: game_id,
        season: 20242025,
        game_type: nhl_api::GameType::RegularSeason,
        limited_scoring: false,
        game_date: "2024-11-20".to_string(),
        venue: LocalizedString {
            default: "Scotiabank Arena".to_string(),
        },
        venue_location: LocalizedString {
            default: "Toronto, ON".to_string(),
        },
        start_time_utc: "2024-11-21T00:00:00Z".to_string(),
        eastern_utc_offset: "-05:00".to_string(),
        venue_utc_offset: "-05:00".to_string(),
        tv_broadcasts: vec![],
        game_state: if is_live {
            GameState::Live
        } else {
            GameState::Final
        },
        game_schedule_state: GameScheduleState::Ok,
        period_descriptor: PeriodDescriptor {
            number: period,
            period_type: PeriodType::Regulation,
            max_regulation_periods: 3,
        },
        special_event: None,
        away_team: BoxscoreTeam {
            id: 10,
            common_name: LocalizedString {
                default: "Maple Leafs".to_string(),
            },
            abbrev: "TOR".to_string(),
            score: away_score,
            sog: 28,
            logo: "https://assets.nhle.com/logos/nhl/svg/TOR_light.svg".to_string(),
            dark_logo: "https://assets.nhle.com/logos/nhl/svg/TOR_dark.svg".to_string(),
            place_name: LocalizedString {
                default: "Toronto".to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: "in Toronto".to_string(),
            },
        },
        home_team: BoxscoreTeam {
            id: 9,
            common_name: LocalizedString {
                default: "Senators".to_string(),
            },
            abbrev: "OTT".to_string(),
            score: home_score,
            sog: 25,
            logo: "https://assets.nhle.com/logos/nhl/svg/OTT_light.svg".to_string(),
            dark_logo: "https://assets.nhle.com/logos/nhl/svg/OTT_dark.svg".to_string(),
            place_name: LocalizedString {
                default: "Ottawa".to_string(),
            },
            place_name_with_preposition: LocalizedString {
                default: "in Ottawa".to_string(),
            },
        },
        shootout_in_use: true,
        ot_in_use: true,
        clock: if is_live {
            GameClock {
                time_remaining: "12:34".to_string(),
                seconds_remaining: 754,
                running: true,
                in_intermission: false,
            }
        } else {
            GameClock {
                time_remaining: "00:00".to_string(),
                seconds_remaining: 0,
                running: false,
                in_intermission: false,
            }
        },
        display_period: period,
        max_periods: 5,
        game_outcome: Some(GameOutcome {
            last_period_type: PeriodType::Regulation,
        }),
        plays: create_mock_plays(period, away_score, home_score),
        roster_spots: create_mock_roster_spots(),
        reg_periods: Some(3),
        summary: None,
    }
}

/// Create empty PlayEventDetails with all fields set to None
fn empty_details() -> PlayEventDetails {
    PlayEventDetails {
        x_coord: None,
        y_coord: None,
        zone_code: None,
        event_owner_team_id: None,
        shot_type: None,
        shooting_player_id: None,
        goalie_in_net_id: None,
        blocking_player_id: None,
        scoring_player_id: None,
        scoring_player_total: None,
        assist1_player_id: None,
        assist1_player_total: None,
        assist2_player_id: None,
        assist2_player_total: None,
        away_score: None,
        home_score: None,
        highlight_clip: None,
        highlight_clip_sharing_url: None,
        discrete_clip: None,
        type_code: None,
        desc_key: None,
        duration: None,
        committed_by_player_id: None,
        drawn_by_player_id: None,
        hitting_player_id: None,
        hittee_player_id: None,
        winning_player_id: None,
        losing_player_id: None,
        player_id: None,
        reason: None,
        away_sog: None,
        home_sog: None,
    }
}

fn create_mock_plays(period: i32, away_score: i32, home_score: i32) -> Vec<PlayEvent> {
    let mut plays = Vec::new();
    let mut event_id = 100;

    // Period start
    plays.push(create_play_event(
        event_id,
        period,
        "00:00",
        "20:00",
        PlayEventType::PeriodStart,
        None,
    ));
    event_id += 1;

    // Opening faceoff
    plays.push(create_play_event(
        event_id,
        period,
        "00:00",
        "20:00",
        PlayEventType::Faceoff,
        Some(PlayEventDetails {
            winning_player_id: Some(8478483), // Matthews
            losing_player_id: Some(8478469),  // Stutzle
            zone_code: Some(ZoneCode::Neutral),
            x_coord: Some(0),
            y_coord: Some(0),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Shot on goal
    plays.push(create_play_event(
        event_id,
        period,
        "01:23",
        "18:37",
        PlayEventType::ShotOnGoal,
        Some(PlayEventDetails {
            shooting_player_id: Some(8478483), // Matthews
            goalie_in_net_id: Some(8476341),   // Forsberg
            shot_type: Some("wrist".to_string()),
            zone_code: Some(ZoneCode::Offensive),
            x_coord: Some(75),
            y_coord: Some(-10),
            away_sog: Some(1),
            home_sog: Some(0),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Hit
    plays.push(create_play_event(
        event_id,
        period,
        "02:45",
        "17:15",
        PlayEventType::Hit,
        Some(PlayEventDetails {
            hitting_player_id: Some(8479325), // Chabot
            hittee_player_id: Some(8478483),  // Matthews
            zone_code: Some(ZoneCode::Neutral),
            x_coord: Some(-25),
            y_coord: Some(35),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Penalty
    plays.push(create_play_event(
        event_id,
        period,
        "05:30",
        "14:30",
        PlayEventType::Penalty,
        Some(PlayEventDetails {
            committed_by_player_id: Some(8479325), // Chabot
            drawn_by_player_id: Some(8478483),     // Matthews
            desc_key: Some("tripping".to_string()),
            type_code: Some("MIN".to_string()),
            duration: Some(2),
            zone_code: Some(ZoneCode::Neutral),
            x_coord: Some(-30),
            y_coord: Some(0),
            event_owner_team_id: Some(9),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Power play goal
    plays.push(create_play_event(
        event_id,
        period,
        "06:15",
        "13:45",
        PlayEventType::Goal,
        Some(PlayEventDetails {
            scoring_player_id: Some(8478483), // Matthews
            scoring_player_total: Some(15),
            assist1_player_id: Some(8478444), // Marner
            assist1_player_total: Some(25),
            assist2_player_id: Some(8478858), // Nylander
            assist2_player_total: Some(18),
            shot_type: Some("slap".to_string()),
            zone_code: Some(ZoneCode::Offensive),
            x_coord: Some(80),
            y_coord: Some(5),
            away_score: Some(1),
            home_score: Some(0),
            goalie_in_net_id: Some(8476341),
            event_owner_team_id: Some(10),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Another shot
    plays.push(create_play_event(
        event_id,
        period,
        "08:00",
        "12:00",
        PlayEventType::ShotOnGoal,
        Some(PlayEventDetails {
            shooting_player_id: Some(8479325), // Chabot
            goalie_in_net_id: Some(8477970),   // Woll
            shot_type: Some("slap".to_string()),
            zone_code: Some(ZoneCode::Offensive),
            x_coord: Some(-72),
            y_coord: Some(15),
            away_sog: Some(1),
            home_sog: Some(1),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Blocked shot
    plays.push(create_play_event(
        event_id,
        period,
        "09:30",
        "10:30",
        PlayEventType::BlockedShot,
        Some(PlayEventDetails {
            shooting_player_id: Some(8479469), // Stutzle
            blocking_player_id: Some(8479318), // Rielly
            zone_code: Some(ZoneCode::Defensive),
            x_coord: Some(65),
            y_coord: Some(-20),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Home team goal
    plays.push(create_play_event(
        event_id,
        period,
        "12:34",
        "07:26",
        PlayEventType::Goal,
        Some(PlayEventDetails {
            scoring_player_id: Some(8479469), // Stutzle
            scoring_player_total: Some(12),
            assist1_player_id: Some(8480801), // Batherson
            assist1_player_total: Some(20),
            shot_type: Some("wrist".to_string()),
            zone_code: Some(ZoneCode::Offensive),
            x_coord: Some(-78),
            y_coord: Some(0),
            away_score: Some(away_score - 1),
            home_score: Some(home_score - 2),
            goalie_in_net_id: Some(8477970),
            event_owner_team_id: Some(9),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Takeaway
    plays.push(create_play_event(
        event_id,
        period,
        "14:00",
        "06:00",
        PlayEventType::Takeaway,
        Some(PlayEventDetails {
            player_id: Some(8478483), // Matthews
            zone_code: Some(ZoneCode::Neutral),
            x_coord: Some(10),
            y_coord: Some(-15),
            event_owner_team_id: Some(10),
            ..empty_details()
        }),
    ));
    event_id += 1;

    // Giveaway
    plays.push(create_play_event(
        event_id,
        period,
        "15:30",
        "04:30",
        PlayEventType::Giveaway,
        Some(PlayEventDetails {
            player_id: Some(8479469), // Stutzle
            zone_code: Some(ZoneCode::Defensive),
            x_coord: Some(-60),
            y_coord: Some(25),
            event_owner_team_id: Some(9),
            ..empty_details()
        }),
    ));

    plays
}

fn create_play_event(
    event_id: i64,
    period: i32,
    time_in_period: &str,
    time_remaining: &str,
    event_type: PlayEventType,
    details: Option<PlayEventDetails>,
) -> PlayEvent {
    PlayEvent {
        event_id,
        period_descriptor: PeriodDescriptor {
            number: period,
            period_type: PeriodType::Regulation,
            max_regulation_periods: 3,
        },
        time_in_period: time_in_period.to_string(),
        time_remaining: time_remaining.to_string(),
        situation_code: "1551".to_string(),
        home_team_defending_side: DefendingSide::Right,
        type_code: match event_type {
            PlayEventType::Goal => 505,
            PlayEventType::ShotOnGoal => 506,
            PlayEventType::BlockedShot => 508,
            PlayEventType::Penalty => 509,
            PlayEventType::Faceoff => 502,
            PlayEventType::Hit => 503,
            PlayEventType::Giveaway => 504,
            PlayEventType::Takeaway => 504,
            _ => 500,
        },
        type_desc_key: event_type,
        sort_order: event_id as i32,
        details,
        ppt_replay_url: None,
    }
}

fn create_mock_roster_spots() -> Vec<RosterSpot> {
    vec![
        // Toronto Maple Leafs
        RosterSpot {
            team_id: 10,
            player_id: 8478483,
            first_name: LocalizedString {
                default: "Auston".to_string(),
            },
            last_name: LocalizedString {
                default: "Matthews".to_string(),
            },
            sweater_number: 34,
            position: Position::Center,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/TOR/8478483.png".to_string(),
        },
        RosterSpot {
            team_id: 10,
            player_id: 8478444,
            first_name: LocalizedString {
                default: "Mitch".to_string(),
            },
            last_name: LocalizedString {
                default: "Marner".to_string(),
            },
            sweater_number: 16,
            position: Position::RightWing,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/TOR/8478444.png".to_string(),
        },
        RosterSpot {
            team_id: 10,
            player_id: 8478858,
            first_name: LocalizedString {
                default: "William".to_string(),
            },
            last_name: LocalizedString {
                default: "Nylander".to_string(),
            },
            sweater_number: 88,
            position: Position::RightWing,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/TOR/8478858.png".to_string(),
        },
        RosterSpot {
            team_id: 10,
            player_id: 8479318,
            first_name: LocalizedString {
                default: "Morgan".to_string(),
            },
            last_name: LocalizedString {
                default: "Rielly".to_string(),
            },
            sweater_number: 44,
            position: Position::Defense,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/TOR/8479318.png".to_string(),
        },
        RosterSpot {
            team_id: 10,
            player_id: 8477970,
            first_name: LocalizedString {
                default: "Joseph".to_string(),
            },
            last_name: LocalizedString {
                default: "Woll".to_string(),
            },
            sweater_number: 60,
            position: Position::Goalie,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/TOR/8477970.png".to_string(),
        },
        // Ottawa Senators
        RosterSpot {
            team_id: 9,
            player_id: 8479469,
            first_name: LocalizedString {
                default: "Tim".to_string(),
            },
            last_name: LocalizedString {
                default: "Stutzle".to_string(),
            },
            sweater_number: 18,
            position: Position::Center,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/OTT/8479469.png".to_string(),
        },
        RosterSpot {
            team_id: 9,
            player_id: 8480801,
            first_name: LocalizedString {
                default: "Drake".to_string(),
            },
            last_name: LocalizedString {
                default: "Batherson".to_string(),
            },
            sweater_number: 19,
            position: Position::RightWing,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/OTT/8480801.png".to_string(),
        },
        RosterSpot {
            team_id: 9,
            player_id: 8479325,
            first_name: LocalizedString {
                default: "Thomas".to_string(),
            },
            last_name: LocalizedString {
                default: "Chabot".to_string(),
            },
            sweater_number: 72,
            position: Position::Defense,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/OTT/8479325.png".to_string(),
        },
        RosterSpot {
            team_id: 9,
            player_id: 8476341,
            first_name: LocalizedString {
                default: "Anton".to_string(),
            },
            last_name: LocalizedString {
                default: "Forsberg".to_string(),
            },
            sweater_number: 31,
            position: Position::Goalie,
            headshot: "https://assets.nhle.com/mugs/nhl/20242025/OTT/8476341.png".to_string(),
        },
    ]
}
