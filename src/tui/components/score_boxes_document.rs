//! Document implementation for compact score boxes grid view
//!
//! This module provides a Document implementation that displays games in a
//! Row-based grid layout using compact ScoreBox widgets.

use std::collections::HashMap;
use std::sync::Arc;

use nhl_api::{DailySchedule, GameDate, GameMatchup};

use crate::commands::scores_format::format_period_text;
use crate::layout_constants::SCORE_BOX_WIDTH;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext, FocusableId};
use crate::tui::widgets::{ScoreBox, ScoreBoxStatus, loading_animation::loading_animation_text};

/// Gap between score boxes in characters
const SCORE_BOX_GAP: u16 = 8;

/// Total width of a score box with gap
const SCORE_BOX_WITH_GAP: u16 = SCORE_BOX_WIDTH + SCORE_BOX_GAP;

/// Document that displays games in a grid layout using ScoreBox widgets
pub struct ScoreBoxesDocument {
    pub schedule: Arc<Option<DailySchedule>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,
    pub boxes_per_row: u16,
    pub game_date: GameDate,
    pub animation_frame: u8,
}

impl ScoreBoxesDocument {
    pub fn new(
        schedule: Arc<Option<DailySchedule>>,
        game_info: Arc<HashMap<i64, GameMatchup>>,
        boxes_per_row: u16,
        game_date: GameDate,
        animation_frame: u8,
    ) -> Self {
        Self {
            schedule,
            game_info,
            boxes_per_row,
            game_date,
            animation_frame,
        }
    }

    /// Calculate how many score boxes fit in the given width
    pub fn boxes_per_row_for_width(width: u16) -> u16 {
        if width < SCORE_BOX_WIDTH {
            1
        } else {
            // First box takes SCORE_BOX_WIDTH, subsequent boxes take SCORE_BOX_WITH_GAP
            let after_first = width.saturating_sub(SCORE_BOX_WIDTH);
            1 + after_first / SCORE_BOX_WITH_GAP
        }
    }

    /// Get team display name from game_info or fallback to abbreviation
    fn get_team_name(&self, game_id: i64, is_away: bool, abbrev: &str) -> String {
        if let Some(info) = self.game_info.get(&game_id) {
            let team = if is_away {
                &info.away_team
            } else {
                &info.home_team
            };
            team.common_name.default.clone()
        } else {
            abbrev.to_string()
        }
    }

    /// Create a ScoreBox widget for a given game
    fn create_score_box(&self, game: &nhl_api::ScheduleGame) -> ScoreBox {
        // Get team names (prefer common_name from game_info, fall back to abbrev)
        let away_team = self.get_team_name(game.id, true, &game.away_team.abbrev);
        let home_team = self.get_team_name(game.id, false, &game.home_team.abbrev);

        // Get scores from schedule or game_info
        let (away_score, home_score) = if let Some(info) = self.game_info.get(&game.id) {
            (Some(info.away_team.score), Some(info.home_team.score))
        } else {
            (game.away_team.score, game.home_team.score)
        };

        // Determine game status
        let status = if game.game_state.is_final() {
            // Check for OT/SO from game_info
            let (overtime, shootout) = if let Some(info) = self.game_info.get(&game.id) {
                let is_ot = info.period_descriptor.number > 3
                    || info.period_descriptor.period_type == nhl_api::PeriodType::Overtime;
                let is_so = info.period_descriptor.period_type == nhl_api::PeriodType::Shootout;
                (is_ot && !is_so, is_so)
            } else {
                (false, false)
            };
            ScoreBoxStatus::Final { overtime, shootout }
        } else if game.game_state.has_started() {
            // Get period text and time from game_info
            if let Some(info) = self.game_info.get(&game.id) {
                let period = format_period_text(
                    info.period_descriptor.period_type,
                    info.period_descriptor.number,
                );
                let (time, intermission) = if let Some(clock) = &info.clock {
                    (Some(clock.time_remaining.clone()), clock.in_intermission)
                } else {
                    (None, false)
                };
                ScoreBoxStatus::Live {
                    period,
                    time,
                    intermission,
                }
            } else {
                ScoreBoxStatus::Live {
                    period: "Live".to_string(),
                    time: None,
                    intermission: false,
                }
            }
        } else {
            // Scheduled game - format start time
            let start_time =
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&game.start_time_utc) {
                    let local_time: chrono::DateTime<chrono::Local> = parsed.into();
                    // Use compact format like "7PM" or "10PM"
                    let hour = local_time.format("%l").to_string().trim().to_string();
                    let ampm = local_time.format("%p").to_string();
                    format!("{}{}", hour, ampm)
                } else {
                    game.start_time_utc.clone()
                };
            ScoreBoxStatus::Scheduled { start_time }
        };

        ScoreBox::new(away_team, home_team, away_score, home_score, status)
    }
}

impl Document for ScoreBoxesDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        // Return loading animation if no schedule data yet
        let Some(schedule) = self.schedule.as_ref() else {
            return DocumentBuilder::new()
                .text(loading_animation_text(self.animation_frame))
                .build();
        };

        if schedule.games.is_empty() {
            return DocumentBuilder::new()
                .text("No games scheduled for this date")
                .build();
        }

        let mut builder = DocumentBuilder::new();

        // Group games into rows
        let games: Vec<&nhl_api::ScheduleGame> = schedule.games.iter().collect();
        let chunks: Vec<&[&nhl_api::ScheduleGame]> =
            games.chunks(self.boxes_per_row as usize).collect();

        for chunk in chunks.iter() {
            // Add blank line before each row
            builder = builder.spacer(1);

            // Create ScoreBox elements for this row
            let score_elements: Vec<DocumentElement> = chunk
                .iter()
                .map(|game| {
                    // ScoreBoxElement uses FocusableId::GameLink(game_id)
                    let focused = focus.focused_id == Some(FocusableId::GameLink(game.id));

                    // Create the ScoreBox widget
                    let score_box = self.create_score_box(game);

                    // Use the ScoreBoxElement variant
                    DocumentElement::score_box_element(game.id, score_box, focused)
                })
                .collect();

            // Add row with custom gap to document
            builder = builder.row_with_gap(score_elements, SCORE_BOX_GAP);
        }

        builder.build()
    }

    fn title(&self) -> String {
        format!("Scores for {}", self.game_date)
    }

    fn id(&self) -> String {
        format!("scoreboxes_{}", self.game_date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nhl_api::{GameState as ApiGameState, ScheduleGame, ScheduleTeam};

    fn create_test_game(id: i64, away: &str, home: &str) -> ScheduleGame {
        ScheduleGame {
            id,
            game_type: nhl_api::GameType::RegularSeason,
            game_date: Some("2024-01-15".to_string()),
            start_time_utc: "2024-01-15T20:00:00Z".to_string(),
            game_state: ApiGameState::Final,
            away_team: ScheduleTeam {
                id: 1,
                abbrev: away.to_string(),
                score: Some(2),
                logo: String::new(),
                place_name: None,
            },
            home_team: ScheduleTeam {
                id: 2,
                abbrev: home.to_string(),
                score: Some(3),
                logo: String::new(),
                place_name: None,
            },
        }
    }

    #[test]
    fn test_empty_schedule_returns_loading_animation() {
        let doc = ScoreBoxesDocument::new(
            Arc::new(None),
            Arc::new(HashMap::new()),
            2,
            GameDate::today(),
            0,
        );

        let focus = FocusContext { focused_id: None };
        let elements = doc.build(&focus);

        // Should return a text element with loading animation
        assert_eq!(elements.len(), 1);
    }

    #[test]
    fn test_single_game_layout() {
        let game = create_test_game(1, "TOR", "MTL");
        let schedule = DailySchedule {
            date: "2024-01-15".to_string(),
            games: vec![game],
            next_start_date: None,
            previous_start_date: None,
            number_of_games: 1,
        };

        let doc = ScoreBoxesDocument::new(
            Arc::new(Some(schedule)),
            Arc::new(HashMap::new()),
            2,
            GameDate::today(),
            0,
        );

        let focus = FocusContext { focused_id: None };
        let elements = doc.build(&focus);

        // Should have 1 spacer + 1 row with 1 game = 2 elements
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_two_games_in_row() {
        let game1 = create_test_game(1, "TOR", "MTL");
        let game2 = create_test_game(2, "BOS", "NYR");
        let schedule = DailySchedule {
            date: "2024-01-15".to_string(),
            games: vec![game1, game2],
            next_start_date: None,
            previous_start_date: None,
            number_of_games: 2,
        };

        let doc = ScoreBoxesDocument::new(
            Arc::new(Some(schedule)),
            Arc::new(HashMap::new()),
            2, // 2 boxes per row
            GameDate::today(),
            0,
        );

        let focus = FocusContext { focused_id: None };
        let elements = doc.build(&focus);

        // Should have 1 spacer + 1 row with 2 games = 2 elements
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_three_games_two_rows() {
        let game1 = create_test_game(1, "TOR", "MTL");
        let game2 = create_test_game(2, "BOS", "NYR");
        let game3 = create_test_game(3, "EDM", "VAN");
        let schedule = DailySchedule {
            date: "2024-01-15".to_string(),
            games: vec![game1, game2, game3],
            next_start_date: None,
            previous_start_date: None,
            number_of_games: 3,
        };

        let doc = ScoreBoxesDocument::new(
            Arc::new(Some(schedule)),
            Arc::new(HashMap::new()),
            2, // 2 boxes per row
            GameDate::today(),
            0,
        );

        let focus = FocusContext { focused_id: None };
        let elements = doc.build(&focus);

        // Should have 2 spacers + 2 rows = 4 elements
        assert_eq!(elements.len(), 4);
    }

    #[test]
    fn test_title_and_id() {
        let doc = ScoreBoxesDocument::new(
            Arc::new(None),
            Arc::new(HashMap::new()),
            2,
            GameDate::today(),
            0,
        );

        assert!(doc.title().contains("Scores for"));
        assert!(doc.id().starts_with("scoreboxes_"));
    }

    #[test]
    fn test_boxes_per_row_for_width() {
        // Gap is 8 chars, so each additional box needs 33 chars (25 + 8)

        // Width 25 = 1 box (exactly fits)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(25), 1);

        // Width 32 = 1 box (not enough for second, need 33 more)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(32), 1);

        // Width 58 = 2 boxes (25 + 8 gap + 25)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(58), 2);

        // Width 90 = 2 boxes (25 + 33 + 32 remaining, not enough for third)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(90), 2);

        // Width 91 = 3 boxes (25 + 33 + 33)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(91), 3);

        // Width 100 = 3 boxes
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(100), 3);

        // Width 200 = 6 boxes (25 + 5*33 = 190, next would be 223)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(200), 6);

        // Width 20 = 1 box (minimum)
        assert_eq!(ScoreBoxesDocument::boxes_per_row_for_width(20), 1);
    }
}
