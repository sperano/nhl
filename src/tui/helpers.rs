#[cfg(test)]
use nhl_api::Position;
/// Helper methods and extension traits for common TUI operations
use nhl_api::{ClubGoalieStats, ClubSkaterStats, SeasonTotal, Standing};

/// Extension trait for sorting standings by points (descending)
pub trait StandingsSorting {
    fn sort_by_points_desc(&mut self);
}

impl StandingsSorting for Vec<Standing> {
    fn sort_by_points_desc(&mut self) {
        self.sort_by(|a, b| b.points.cmp(&a.points));
    }
}

impl StandingsSorting for Vec<&Standing> {
    fn sort_by_points_desc(&mut self) {
        self.sort_by(|a, b| b.points.cmp(&a.points));
    }
}

/// Extension trait for sorting club skater stats by points (descending)
pub trait ClubSkaterStatsSorting {
    fn sort_by_points_desc(&mut self);
}

impl ClubSkaterStatsSorting for Vec<ClubSkaterStats> {
    fn sort_by_points_desc(&mut self) {
        self.sort_by(|a, b| b.points.cmp(&a.points));
    }
}

/// Extension trait for sorting club goalie stats by games played (descending)
pub trait ClubGoalieStatsSorting {
    fn sort_by_games_played_desc(&mut self);
}

impl ClubGoalieStatsSorting for Vec<ClubGoalieStats> {
    fn sort_by_games_played_desc(&mut self) {
        self.sort_by(|a, b| b.games_played.cmp(&a.games_played));
    }
}

/// Extension trait for sorting season totals by season (descending)
pub trait SeasonSorting {
    fn sort_by_season_desc(&mut self);
}

impl SeasonSorting for Vec<SeasonTotal> {
    fn sort_by_season_desc(&mut self) {
        self.sort_by(|a, b| b.season.cmp(&a.season));
    }
}

impl SeasonSorting for Vec<&SeasonTotal> {
    fn sort_by_season_desc(&mut self) {
        self.sort_by(|a, b| b.season.cmp(&a.season));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nhl_api::LocalizedString;

    #[test]
    fn test_standings_sort_by_points_desc() {
        // Test actual sorting with real data
        let mut standings = vec![
            create_minimal_standing("BOS", 50),
            create_minimal_standing("TBL", 75),
            create_minimal_standing("TOR", 60),
        ];

        standings.sort_by_points_desc();

        assert_eq!(standings[0].points, 75); // TBL first
        assert_eq!(standings[1].points, 60); // TOR second
        assert_eq!(standings[2].points, 50); // BOS third
    }

    #[test]
    fn test_club_skater_stats_sort_by_points_desc() {
        let mut skaters = vec![
            create_minimal_skater(1, 30),
            create_minimal_skater(2, 50),
            create_minimal_skater(3, 40),
        ];

        skaters.sort_by_points_desc();

        assert_eq!(skaters[0].points, 50);
        assert_eq!(skaters[1].points, 40);
        assert_eq!(skaters[2].points, 30);
    }

    #[test]
    fn test_club_goalie_stats_sort_by_games_played_desc() {
        let mut goalies = vec![
            create_minimal_goalie(1, 20),
            create_minimal_goalie(2, 45),
            create_minimal_goalie(3, 30),
        ];

        goalies.sort_by_games_played_desc();

        assert_eq!(goalies[0].games_played, 45);
        assert_eq!(goalies[1].games_played, 30);
        assert_eq!(goalies[2].games_played, 20);
    }

    #[test]
    fn test_season_sort_by_season_desc() {
        let season1 = create_minimal_season(20222023);
        let season2 = create_minimal_season(20232024);
        let season3 = create_minimal_season(20212022);

        let mut seasons = vec![&season1, &season2, &season3];

        seasons.sort_by_season_desc();

        assert_eq!(seasons[0].season, 20232024);
        assert_eq!(seasons[1].season, 20222023);
        assert_eq!(seasons[2].season, 20212022);
    }

    #[test]
    fn test_season_sort_by_season_desc_owned() {
        let mut seasons = vec![
            create_minimal_season(20222023),
            create_minimal_season(20232024),
            create_minimal_season(20212022),
        ];

        seasons.sort_by_season_desc();

        assert_eq!(seasons[0].season, 20232024);
        assert_eq!(seasons[1].season, 20222023);
        assert_eq!(seasons[2].season, 20212022);
    }

    // Minimal test data construction helpers
    fn create_minimal_standing(abbrev: &str, points: i32) -> Standing {
        Standing {
            conference_abbrev: Some("E".to_string()),
            conference_name: Some("Eastern".to_string()),
            division_abbrev: "A".to_string(),
            division_name: "Atlantic".to_string(),
            team_name: LocalizedString {
                default: abbrev.to_string(),
            },
            team_common_name: LocalizedString {
                default: abbrev.to_string(),
            },
            team_abbrev: LocalizedString {
                default: abbrev.to_string(),
            },
            team_logo: String::new(),
            wins: points / 2,
            losses: 0,
            ot_losses: 0,
            points,
        }
    }

    fn create_minimal_skater(player_id: i64, points: i32) -> ClubSkaterStats {
        ClubSkaterStats {
            player_id,
            points,
            goals: points / 2,
            assists: points - (points / 2),
            headshot: String::new(),
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: "Player".to_string(),
            },
            position: Position::Center,
            games_played: 10,
            plus_minus: 0,
            penalty_minutes: 0,
            power_play_goals: 0,
            shorthanded_goals: 0,
            game_winning_goals: 0,
            overtime_goals: 0,
            shots: 0,
            shooting_pctg: 0.0,
            avg_time_on_ice_per_game: 0.0,
            avg_shifts_per_game: 0.0,
            faceoff_win_pctg: 0.0,
        }
    }

    fn create_minimal_goalie(player_id: i64, games_played: i32) -> ClubGoalieStats {
        ClubGoalieStats {
            player_id,
            games_played,
            headshot: String::new(),
            first_name: LocalizedString {
                default: "Test".to_string(),
            },
            last_name: LocalizedString {
                default: "Goalie".to_string(),
            },
            games_started: 0,
            wins: 0,
            losses: 0,
            overtime_losses: 0,
            goals_against_average: 0.0,
            save_percentage: 0.0,
            shots_against: 0,
            saves: 0,
            goals_against: 0,
            shutouts: 0,
            goals: 0,
            assists: 0,
            points: 0,
            penalty_minutes: 0,
            time_on_ice: 0,
        }
    }

    fn create_minimal_season(season: i32) -> SeasonTotal {
        SeasonTotal {
            season,
            game_type: nhl_api::GameType::RegularSeason,
            league_abbrev: "NHL".to_string(),
            team_name: LocalizedString {
                default: "Test Team".to_string(),
            },
            team_common_name: None,
            sequence: None,
            games_played: 0,
            goals: None,
            assists: None,
            points: None,
            plus_minus: None,
            pim: None,
        }
    }
}
