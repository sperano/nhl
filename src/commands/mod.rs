pub mod boxscore;
pub mod franchises;
pub mod player_stats;
pub mod schedule;
pub mod scores;
pub mod scores_format;
pub mod standings;
pub mod tail;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate};
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

/// Format a UTC ISO-8601 (RFC 3339) timestamp as local time using the given strftime format.
///
/// Falls back to returning the original string unchanged if it cannot be parsed, so callers
/// always get a displayable value even for malformed input.
pub fn format_local_time(utc_iso: &str, time_format: &str) -> String {
    match DateTime::parse_from_rfc3339(utc_iso) {
        Ok(parsed) => {
            let local_time: DateTime<Local> = parsed.into();
            local_time.format(time_format).to_string()
        }
        Err(_) => utc_iso.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_local_time_valid_rfc3339() {
        let result = format_local_time("2024-11-04T00:00:00Z", "%H:%M:%S");
        // Local time varies by timezone, but the format should always match HH:MM:SS
        assert_eq!(result.len(), 8);
        assert_eq!(result.chars().nth(2), Some(':'));
        assert_eq!(result.chars().nth(5), Some(':'));
    }

    #[test]
    fn test_format_local_time_respects_custom_format() {
        let result = format_local_time("2024-11-04T00:00:00Z", "%Y-%m-%d");
        assert_eq!(result.len(), 10);
        assert_eq!(result.chars().nth(4), Some('-'));
    }

    #[test]
    fn test_format_local_time_invalid_input_falls_back_to_original() {
        let result = format_local_time("not-a-timestamp", "%H:%M:%S");
        assert_eq!(result, "not-a-timestamp");
    }
}
