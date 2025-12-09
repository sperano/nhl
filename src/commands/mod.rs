pub mod boxscore;
pub mod franchises;
pub mod schedule;
pub mod scores;
pub mod scores_format;
pub mod standings;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use nhl_api::GameDate;

/// Parse optional date string to GameDate, defaulting to today
///
/// Accepts dates in YYYY-MM-DD format. If no date is provided, returns today's date.
/// Returns an error if the date string is malformed.
pub fn parse_game_date(date: Option<String>) -> Result<GameDate> {
    if let Some(date_str) = date {
        let parsed_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .with_context(|| format!("Invalid date format '{}'. Use YYYY-MM-DD", date_str))?;
        Ok(GameDate::Date(parsed_date))
    } else {
        Ok(GameDate::today())
    }
}
