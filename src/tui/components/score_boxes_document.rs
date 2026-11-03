//! Document implementation for compact score boxes grid view
//!
//! This module provides a Document implementation that displays games in a
//! Row-based grid layout using compact ScoreBox widgets.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use nhl_api::{DailySchedule, GameDate, GameMatchup};

use crate::commands::scores_format::format_period_text;
use crate::layout_constants::SCORE_BOX_WIDTH;
use crate::team_abbrev::abbrev_to_common_name;
use crate::tui::document::{
    Document, DocumentBuilder, DocumentElement, FocusContext, FocusableId, LinkTarget,
};
use crate::tui::reducer::format_date_for_breadcrumb;
use crate::tui::types::StackedDocument;
use crate::tui::widgets::{loading_animation::loading_animation_text, ScoreBox, ScoreBoxStatus};

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

    /// Get team display name from game_info or fallback to common name lookup
    fn get_team_name(&self, game_id: i64, is_away: bool, abbrev: &str) -> String {
        if let Some(info) = self.game_info.get(&game_id) {
            let team = if is_away {
                &info.away_team
            } else {
                &info.home_team
            };
            team.common_name.default.clone()
        } else {
            // Use common name from lookup, fallback to abbreviation if not found
            abbrev_to_common_name(abbrev)
                .map(|s| s.to_string())
                .unwrap_or_else(|| abbrev.to_string())
        }
    }

    /// Create a ScoreBox widget for a given game
    fn create_score_box(&self, game: &nhl_api::ScheduleGame) -> ScoreBox {
        // Get team names (prefer common_name from game_info, fall back to common name lookup)
        let away_team = self.get_team_name(game.id.into(), true, &game.away_team.abbrev);
        let home_team = self.get_team_name(game.id.into(), false, &game.home_team.abbrev);

        let (away_score, home_score) = self.score_box_scores(game);
        let status = self.score_box_status(game);

        ScoreBox::new(away_team, home_team, away_score, home_score, status)
    }

    /// Scores from `game_info` if available (live/final data), else from the schedule fixture
    fn score_box_scores(&self, game: &nhl_api::ScheduleGame) -> (Option<i32>, Option<i32>) {
        if let Some(info) = self.game_info.get(&game.id.into()) {
            (info.away_team.score, info.home_team.score)
        } else {
            (game.away_team.score, game.home_team.score)
        }
    }

    /// Determine the score box status (final/live/scheduled) for a game
    fn score_box_status(&self, game: &nhl_api::ScheduleGame) -> ScoreBoxStatus {
        if game.game_state.is_final() {
            self.final_status(game)
        } else if game.game_state.has_started() {
            self.live_status(game)
        } else {
            Self::scheduled_status(game)
        }
    }

    /// Status for a completed game, including OT/SO detection from `game_info`
    fn final_status(&self, game: &nhl_api::ScheduleGame) -> ScoreBoxStatus {
        let (overtime, shootout) = if let Some(info) = self.game_info.get(&game.id.into()) {
            let is_ot = info.period_descriptor.number > 3
                || info.period_descriptor.period_type == Some(nhl_api::PeriodType::Overtime);
            let is_so = info.period_descriptor.period_type == Some(nhl_api::PeriodType::Shootout);
            (is_ot && !is_so, is_so)
        } else {
            (false, false)
        };
        ScoreBoxStatus::Final { overtime, shootout }
    }

    /// Status for an in-progress game, using period/clock data from `game_info` if available
    fn live_status(&self, game: &nhl_api::ScheduleGame) -> ScoreBoxStatus {
        let Some(info) = self.game_info.get(&game.id.into()) else {
            return ScoreBoxStatus::Live {
                period: "Live".to_string(),
                time: None,
                intermission: false,
            };
        };

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
    }

    /// Status for a not-yet-started game: the formatted local start time
    fn scheduled_status(game: &nhl_api::ScheduleGame) -> ScoreBoxStatus {
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
    }

    /// Build the activation target for a game's score box
    ///
    /// Resolves the same away/home abbrev and score data `create_score_box`
    /// uses for display, so the pushed `StackedDocument::Boxscore` always
    /// matches what's on screen.
    fn build_link_target(&self, game: &nhl_api::ScheduleGame) -> LinkTarget {
        let (away_score, home_score) = self.score_box_scores(game);
        let (away_score, home_score) = (away_score.unwrap_or(0), home_score.unwrap_or(0));

        LinkTarget::Push(StackedDocument::Boxscore {
            game_id: game.id.into(),
            away_abbrev: game.away_team.abbrev.clone(),
            home_abbrev: game.home_team.abbrev.clone(),
            away_score,
            home_score,
            game_date: format_date_for_breadcrumb(&self.game_date),
        })
    }

    /// Build the score-box elements for one row of games
    fn build_row_elements(
        &self,
        chunk: &[&nhl_api::ScheduleGame],
        focus: &FocusContext,
    ) -> Vec<DocumentElement> {
        chunk
            .iter()
            .map(|game| {
                // ScoreBoxElement uses FocusableId::GameLink(game_id)
                let focused = focus.focused_id == Some(FocusableId::GameLink(game.id.into()));

                // Create the ScoreBox widget
                let score_box = self.create_score_box(game);
                let link_target = self.build_link_target(game);

                // Use the ScoreBoxElement variant
                DocumentElement::score_box_element(game.id.into(), score_box, focused, link_target)
            })
            .collect()
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

            // Create ScoreBox elements for this row and add it with a custom gap
            let score_elements = self.build_row_elements(chunk, focus);
            builder = builder.row_with_gap(score_elements, SCORE_BOX_GAP);
        }

        builder.build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Owned(format!("Scores for {}", self.game_date))
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Owned(format!("scoreboxes_{}", self.game_date))
    }
}

#[cfg(test)]
#[path = "score_boxes_document_tests.rs"]
mod tests;
