//! BigScore widget - displays game score using large digit font
//!
//! Renders scores like:
//! ```text
//!              Final
//!
//!          ▟▀▀▙    ▟▀▀▙
//! Devils    ▄▄▛ ──   ▗▛  Sabres
//!             █     ▗▛
//!          ▜▄▄▛    ▄█▄▄
//!
//!            TD Garden
//! ```
//!
//! Layout:
//! - Status line centered (e.g., "Final", "1st 09:27")
//! - Blank line
//! - Team names on the sides, vertically centered with the digits
//! - Venue centered below (with blank line)

use crate::big_digits::{get_digit, BIG_DIGIT_HEIGHT, BIG_DIGIT_WIDTH};
use crate::config::RenderContext;
use ratatui::{buffer::Buffer, layout::Rect};

use super::score_box::ScoreBoxStatus;
use super::StandaloneWidget;

/// Separator between score digits (dash)
const SEPARATOR: [&str; 4] = ["      ", "  ▄▄  ", "      ", "      "];
const SEPARATOR_WIDTH: u16 = 6;

/// Fixed base width for team name boxes
const NAME_BOX_WIDTH: u16 = 20;

/// Gap between name box and digits
const NAME_DIGIT_GAP: u16 = 2;

/// Gap between digits of the same score
const DIGIT_GAP: u16 = 1;

/// Header height: status line + blank line
const HEADER_HEIGHT: u16 = 2;

/// Footer height: blank line + venue line
const FOOTER_HEIGHT: u16 = 2;

/// Widget that displays a game score using big digits
///
/// Layout (8 rows):
/// - Status line centered (row 0)
/// - Blank line (row 1)
/// - Big digit scores with team names on sides (rows 2-5)
/// - Blank line (row 6)
/// - Venue centered (row 7)
#[derive(Debug, Clone)]
pub struct BigScore {
    /// Away team common name (e.g., "Devils")
    pub away_name: String,
    /// Home team common name (e.g., "Sabres")
    pub home_name: String,
    /// Away team score
    pub away_score: i32,
    /// Home team score
    pub home_score: i32,
    /// Game status (e.g., "Final", "1st 09:27")
    pub status: ScoreBoxStatus,
    /// Venue name (e.g., "TD Garden")
    pub venue: String,
}

impl BigScore {
    /// Create a new BigScore widget
    pub fn new(
        away_name: impl Into<String>,
        home_name: impl Into<String>,
        away_score: i32,
        home_score: i32,
        status: ScoreBoxStatus,
        venue: impl Into<String>,
    ) -> Self {
        Self {
            away_name: away_name.into(),
            home_name: home_name.into(),
            away_score,
            home_score,
            status,
            venue: venue.into(),
        }
    }

    /// Get the digits for a score (handles 0-99, returns vec of digit indices)
    fn score_digits(score: i32) -> Vec<u8> {
        if score < 0 {
            vec![0]
        } else if score < 10 {
            vec![score as u8]
        } else if score < 100 {
            vec![(score / 10) as u8, (score % 10) as u8]
        } else {
            vec![9, 9]
        }
    }

    /// Calculate width needed for a score's digits (including gaps between digits)
    fn score_width(score: i32) -> u16 {
        let digits = Self::score_digits(score);
        let num_digits = digits.len() as u16;
        // Width = digits * 4 + gaps between digits
        num_digits * BIG_DIGIT_WIDTH + num_digits.saturating_sub(1) * DIGIT_GAP
    }

    /// Calculate the balanced name box widths
    /// Returns (away_name_box_width, home_name_box_width)
    fn balanced_name_boxes(&self) -> (u16, u16) {
        let away_digits_width = Self::score_width(self.away_score);
        let home_digits_width = Self::score_width(self.home_score);

        if away_digits_width > home_digits_width {
            let imbalance = away_digits_width - home_digits_width;
            (NAME_BOX_WIDTH, NAME_BOX_WIDTH + imbalance)
        } else if home_digits_width > away_digits_width {
            let imbalance = home_digits_width - away_digits_width;
            (NAME_BOX_WIDTH + imbalance, NAME_BOX_WIDTH)
        } else {
            (NAME_BOX_WIDTH, NAME_BOX_WIDTH)
        }
    }

    /// Calculate total width including team names and gaps
    fn total_width(&self) -> u16 {
        let (away_box, home_box) = self.balanced_name_boxes();
        let away_digits = Self::score_width(self.away_score);
        let home_digits = Self::score_width(self.home_score);
        // Layout: away_name_box + gap + away_digits + separator + home_digits + gap + home_name_box
        away_box
            + NAME_DIGIT_GAP
            + away_digits
            + SEPARATOR_WIDTH
            + home_digits
            + NAME_DIGIT_GAP
            + home_box
    }
}

impl StandaloneWidget for BigScore {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        let required_height = HEADER_HEIGHT + BIG_DIGIT_HEIGHT + FOOTER_HEIGHT;
        if area.height < required_height || area.width < self.total_width() {
            return;
        }

        let text_style = ctx.text_style();
        let x = area.x;
        let y = area.y;

        // Row 0: Status line centered
        let status_text = self.status.display();
        let status_width = status_text.chars().count() as u16;
        let status_x = x + (area.width.saturating_sub(status_width)) / 2;
        buf.set_string(status_x, y, &status_text, text_style);

        // Row 1: blank line (implicit)
        // Rows 2-5: Big digits with team names

        let digits_y = y + HEADER_HEIGHT; // Offset for header

        let total_width = self.total_width();
        let (away_box_width, _) = self.balanced_name_boxes();
        let away_name_chars = self.away_name.chars().count() as u16;
        let away_digits_width = Self::score_width(self.away_score);
        let home_digits_width = Self::score_width(self.home_score);

        // Calculate starting x position to center the entire display
        let start_x = x + (area.width.saturating_sub(total_width)) / 2;

        // Vertically centered row for team names (row 1 of 4 digit rows, 0-indexed)
        let name_row = digits_y + 1;

        // Away name: right-aligned within its box
        let away_name_x = start_x + away_box_width - away_name_chars;
        buf.set_string(away_name_x, name_row, &self.away_name, text_style);

        // Away digits start after away box + gap
        let away_digits_start_x = start_x + away_box_width + NAME_DIGIT_GAP;

        // Home digits start after away digits + separator
        let home_digits_start_x = away_digits_start_x + away_digits_width + SEPARATOR_WIDTH;

        // Home name: left-aligned within its box (after home digits + gap)
        let home_name_x = home_digits_start_x + home_digits_width + NAME_DIGIT_GAP;
        buf.set_string(home_name_x, name_row, &self.home_name, text_style);

        // Render big digits
        let away_digits = Self::score_digits(self.away_score);
        let home_digits = Self::score_digits(self.home_score);

        for row in 0..BIG_DIGIT_HEIGHT {
            let mut current_x = away_digits_start_x;

            // Away score digits (with gaps between them)
            for (i, &digit) in away_digits.iter().enumerate() {
                if i > 0 {
                    current_x += DIGIT_GAP;
                }
                let line = get_digit(digit)[row as usize];
                buf.set_string(current_x, digits_y + row, line, text_style);
                current_x += BIG_DIGIT_WIDTH;
            }

            // Separator
            buf.set_string(
                current_x,
                digits_y + row,
                SEPARATOR[row as usize],
                text_style,
            );
            current_x += SEPARATOR_WIDTH;

            // Home score digits (with gaps between them)
            for (i, &digit) in home_digits.iter().enumerate() {
                if i > 0 {
                    current_x += DIGIT_GAP;
                }
                let line = get_digit(digit)[row as usize];
                buf.set_string(current_x, digits_y + row, line, text_style);
                current_x += BIG_DIGIT_WIDTH;
            }
        }

        // Row 6: blank line (implicit)
        // Row 7: Venue centered
        let venue_width = self.venue.chars().count() as u16;
        let venue_x = x + (area.width.saturating_sub(venue_width)) / 2;
        let venue_row = digits_y + BIG_DIGIT_HEIGHT + 1;
        buf.set_string(venue_x, venue_row, &self.venue, text_style);
    }

    fn preferred_height(&self) -> Option<u16> {
        // 1 status + 1 blank + 4 digits + 1 blank + 1 venue = 8
        Some(HEADER_HEIGHT + BIG_DIGIT_HEIGHT + FOOTER_HEIGHT)
    }

    fn preferred_width(&self) -> Option<u16> {
        Some(self.total_width())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::assert_buffer;
    use crate::tui::widgets::testing::{render_widget_with_config, test_config};

    fn final_status() -> ScoreBoxStatus {
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: false,
        }
    }

    #[test]
    fn test_single_digit_scores() {
        // Layout: 20 (away box) + 2 (gap) + 4 (digit) + 6 (sep) + 4 (digit) + 2 (gap) + 20 (home box) = 58
        let widget = BigScore::new("Devils", "Sabres", 3, 2, final_status(), "TD Garden");
        let config = test_config();
        let buf = render_widget_with_config(&widget, 58, 8, &config);

        assert_buffer(
            &buf,
            &[
                "                          Final                          ",
                "                                                          ",
                "                      ▟▀▀▙      ▟▀▀▙                      ",
                "              Devils   ▄▄▛  ▄▄    ▗▛  Sabres              ",
                "                         █       ▗▛                       ",
                "                      ▜▄▄▛      ▄█▄▄                      ",
                "                                                          ",
                "                        TD Garden                         ",
            ],
        );
    }

    #[test]
    fn test_score_10_4() {
        // 10-4: away=9 (4+1+4), home=4, imbalance=5, home_box=25
        // Width: 20 + 2 + 9 + 6 + 4 + 2 + 25 = 68
        let widget = BigScore::new("Devils", "Sabres", 10, 4, final_status(), "TD Garden");
        let config = test_config();
        let buf = render_widget_with_config(&widget, 68, 8, &config);

        assert_buffer(
            &buf,
            &[
                "                               Final                                ",
                "                                                                    ",
                "                      ▗█   ▟▀▀▙       ▗█                            ",
                "              Devils   █   █  █  ▄▄  ▗▘█   Sabres                   ",
                "                       █   █  █      ▙▄█▄                           ",
                "                      ▗█▖  ▜▄▄▛        █                            ",
                "                                                                    ",
                "                             TD Garden                              ",
            ],
        );
    }

    #[test]
    fn test_score_4_10() {
        // 4-10: away=4, home=9, imbalance=5, away_box=25
        // Width: 25 + 2 + 4 + 6 + 9 + 2 + 20 = 68
        let widget = BigScore::new("Devils", "Sabres", 4, 10, final_status(), "TD Garden");
        let config = test_config();
        let buf = render_widget_with_config(&widget, 68, 8, &config);

        assert_buffer(
            &buf,
            &[
                "                               Final                                ",
                "                                                                    ",
                "                            ▗█       ▗█   ▟▀▀▙                      ",
                "                   Devils  ▗▘█   ▄▄   █   █  █  Sabres              ",
                "                           ▙▄█▄       █   █  █                      ",
                "                             █       ▗█▖  ▜▄▄▛                      ",
                "                                                                    ",
                "                             TD Garden                              ",
            ],
        );
    }

    #[test]
    fn test_score_10_10() {
        // 10-10: both=9, balanced
        // Width: 20 + 2 + 9 + 6 + 9 + 2 + 20 = 68
        let widget = BigScore::new("Devils", "Sabres", 10, 10, final_status(), "TD Garden");
        let config = test_config();
        let buf = render_widget_with_config(&widget, 68, 8, &config);

        assert_buffer(
            &buf,
            &[
                "                               Final                                ",
                "                                                                    ",
                "                      ▗█   ▟▀▀▙      ▗█   ▟▀▀▙                      ",
                "              Devils   █   █  █  ▄▄   █   █  █  Sabres              ",
                "                       █   █  █       █   █  █                      ",
                "                      ▗█▖  ▜▄▄▛      ▗█▖  ▜▄▄▛                      ",
                "                                                                    ",
                "                             TD Garden                              ",
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
        // Single digit scores: 20 + 2 + 4 + 6 + 4 + 2 + 20 = 58
        let widget = BigScore::new("Devils", "Sabres", 3, 2, final_status(), "KeyBank Center");
        assert_eq!(widget.preferred_height(), Some(8)); // 1 status + 1 blank + 4 digits + 1 blank + 1 venue
        assert_eq!(widget.preferred_width(), Some(58));

        // 10-4: away=9 (4+1+4), home=4, imbalance=5, home_box=25
        // Width: 20 + 2 + 9 + 6 + 4 + 2 + 25 = 68
        let widget_10_4 = BigScore::new("Devils", "Sabres", 10, 4, final_status(), "TD Garden");
        assert_eq!(widget_10_4.preferred_width(), Some(68));

        // 4-10: away=4, home=9, imbalance=5, away_box=25
        // Width: 25 + 2 + 4 + 6 + 9 + 2 + 20 = 68
        let widget_4_10 = BigScore::new("Devils", "Sabres", 4, 10, final_status(), "TD Garden");
        assert_eq!(widget_4_10.preferred_width(), Some(68));

        // 10-10: both=9, balanced
        // Width: 20 + 2 + 9 + 6 + 9 + 2 + 20 = 68
        let widget_10_10 = BigScore::new("Devils", "Sabres", 10, 10, final_status(), "TD Garden");
        assert_eq!(widget_10_10.preferred_width(), Some(68));
    }
}
