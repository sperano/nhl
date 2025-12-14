//! BigScore widget - displays game score using large digit font
//!
//! Renders scores like:
//! ```text
//! NJD                     BUF
//! ▟▀▀▙       ▟▀▀▙       ▟▀▀▙
//! █  █  ───  █  █  ───    ▗▛
//! █  █        ▄▄▛        ▗▛
//! ▜▄▄▛       ▜▄▄▛       ▄█▄▄
//! ```

use crate::big_digits::{BIG_DIGITS, BIG_DIGIT_HEIGHT, BIG_DIGIT_WIDTH};
use crate::config::RenderContext;
use ratatui::{buffer::Buffer, layout::Rect};

use super::StandaloneWidget;

/// Separator between score digits (dash)
const SEPARATOR: [&str; 4] = ["    ", " ── ", "    ", "    "];
const SEPARATOR_WIDTH: u16 = 4;

/// Widget that displays a game score using big digits
///
/// Layout:
/// - Row 0: Team abbreviations (left-aligned away, right-aligned home)
/// - Rows 1-4: Big digit score with separator
#[derive(Debug, Clone)]
pub struct BigScore {
    /// Away team abbreviation (e.g., "NJD")
    pub away_abbrev: String,
    /// Home team abbreviation (e.g., "BUF")
    pub home_abbrev: String,
    /// Away team score
    pub away_score: i32,
    /// Home team score
    pub home_score: i32,
}

impl BigScore {
    /// Create a new BigScore widget
    pub fn new(
        away_abbrev: impl Into<String>,
        home_abbrev: impl Into<String>,
        away_score: i32,
        home_score: i32,
    ) -> Self {
        Self {
            away_abbrev: away_abbrev.into(),
            home_abbrev: home_abbrev.into(),
            away_score,
            home_score,
        }
    }

    /// Get the digits for a score (handles 0-99, returns vec of digit indices)
    fn score_digits(score: i32) -> Vec<usize> {
        if score < 0 {
            vec![0]
        } else if score < 10 {
            vec![score as usize]
        } else if score < 100 {
            vec![(score / 10) as usize, (score % 10) as usize]
        } else {
            // Cap at 99
            vec![9, 9]
        }
    }

    /// Calculate width needed for a score's digits
    fn score_width(score: i32) -> u16 {
        let digits = Self::score_digits(score);
        digits.len() as u16 * BIG_DIGIT_WIDTH
    }

    /// Calculate total width of the score display
    fn total_score_width(&self) -> u16 {
        Self::score_width(self.away_score) + SEPARATOR_WIDTH + Self::score_width(self.home_score)
    }
}

impl StandaloneWidget for BigScore {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        if area.height < BIG_DIGIT_HEIGHT + 1 || area.width < self.total_score_width() {
            return;
        }

        let text_style = ctx.text_style();
        let x = area.x;
        let y = area.y;

        let score_width = self.total_score_width();

        // Calculate starting x position to center the score
        let score_start_x = x + (area.width.saturating_sub(score_width)) / 2;

        // Row 0: Team abbreviations positioned above their respective scores
        let away_digits_width = Self::score_width(self.away_score);
        let home_digits_start = score_start_x + away_digits_width + SEPARATOR_WIDTH;

        // Away abbrev: right-aligned above away score digits
        let away_abbrev_x =
            score_start_x + away_digits_width - self.away_abbrev.chars().count() as u16;
        buf.set_string(away_abbrev_x, y, &self.away_abbrev, text_style);

        // Home abbrev: left-aligned above home score digits
        buf.set_string(home_digits_start, y, &self.home_abbrev, text_style);

        // Rows 1-4: Big digits
        let away_digits = Self::score_digits(self.away_score);
        let home_digits = Self::score_digits(self.home_score);

        for row in 0..BIG_DIGIT_HEIGHT {
            let mut current_x = score_start_x;

            // Away score digits
            for &digit in &away_digits {
                let line = BIG_DIGITS[digit][row as usize];
                buf.set_string(current_x, y + 1 + row, line, text_style);
                current_x += BIG_DIGIT_WIDTH;
            }

            // Separator
            buf.set_string(current_x, y + 1 + row, SEPARATOR[row as usize], text_style);
            current_x += SEPARATOR_WIDTH;

            // Home score digits
            for &digit in &home_digits {
                let line = BIG_DIGITS[digit][row as usize];
                buf.set_string(current_x, y + 1 + row, line, text_style);
                current_x += BIG_DIGIT_WIDTH;
            }
        }
    }

    fn preferred_height(&self) -> Option<u16> {
        // 1 row for abbrevs + 4 rows for digits
        Some(BIG_DIGIT_HEIGHT + 1)
    }

    fn preferred_width(&self) -> Option<u16> {
        Some(self.total_score_width())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::assert_buffer;
    use crate::tui::widgets::testing::{render_widget_with_config, test_config};

    #[test]
    fn test_single_digit_scores() {
        let widget = BigScore::new("NJD", "BUF", 3, 2);
        let config = test_config();
        let buf = render_widget_with_config(&widget, 20, 5, &config);

        assert_buffer(
            &buf,
            &[
                "     NJD    BUF     ",
                "    ▟▀▀▙    ▟▀▀▙    ",
                "     ▄▄▛ ──   ▗▛    ",
                "       █     ▗▛     ",
                "    ▜▄▄▛    ▄█▄▄    ",
            ],
        );
    }

    #[test]
    fn test_double_digit_scores() {
        let widget = BigScore::new("VGK", "COL", 10, 3);
        let config = test_config();
        // Width 16: 4+4 (away digits) + 4 (sep) + 4 (home digit) = 16
        // In 24 wide area, centered means 4 leading spaces
        let buf = render_widget_with_config(&widget, 24, 5, &config);

        assert_buffer(
            &buf,
            &[
                "         VGK    COL     ",
                "    ▗█  ▟▀▀▙    ▟▀▀▙    ",
                "     █  █  █ ──  ▄▄▛    ",
                "     █  █  █       █    ",
                "    ▗█▖ ▜▄▄▛    ▜▄▄▛    ",
            ],
        );
    }

    #[test]
    fn test_score_digits() {
        assert_eq!(BigScore::score_digits(0), vec![0]);
        assert_eq!(BigScore::score_digits(5), vec![5]);
        assert_eq!(BigScore::score_digits(10), vec![1, 0]);
        assert_eq!(BigScore::score_digits(99), vec![9, 9]);
    }

    #[test]
    fn test_preferred_dimensions() {
        let widget = BigScore::new("NJD", "BUF", 3, 2);
        assert_eq!(widget.preferred_height(), Some(5));
        assert_eq!(widget.preferred_width(), Some(12)); // 4 + 4 + 4
    }
}
