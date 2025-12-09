/// Mock fixture data for testing and development
///
/// This module provides consistent, deterministic fixture data that can be used for:
/// 1. Unit and integration tests - ensuring tests have predictable data
/// 2. Development mock mode - running the app with fake data for screenshots and debugging
/// 3. Benchmarks - providing consistent data for performance testing
///
/// The fixtures represent realistic NHL data with all 32 teams and various game states.
use nhl_api::{
    Boxscore, BoxscoreTeam, DailySchedule, Franchise, GameClock, GameDate, GameMatchup, GameState,
    Handedness, LocalizedString, PeriodDescriptor, PeriodType, PlayerByGameStats, PlayerLanding,
    Position, ScheduleGame, ScheduleTeam, Standing, TeamPlayerStats,
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
    } else if game_id == 2024020004 {
        3
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
        last_five_games: None,
    }
}
