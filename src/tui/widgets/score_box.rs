//! ScoreBox widget - compact game score display
//!
//! Displays a game score in a compact box format with:
//! - Status line above (e.g., "Final", "1st 09:27", "9PM")
//! - Double-line bordered box with team names and scores
//!
//! Width: 25 characters, Height: 6 rows

use crate::config::{DisplayConfig, SELECTION_STYLE_MODIFIER};
use crate::layout_constants::{SCORE_BOX_HEIGHT, SCORE_BOX_WIDTH};
use ratatui::{buffer::Buffer, layout::Rect};

use super::StandaloneWidget;

/// Game status for the ScoreBox header
#[derive(Debug, Clone, PartialEq)]
pub enum ScoreBoxStatus {
    /// Game hasn't started yet - shows start time (e.g., "9PM")
    Scheduled { start_time: String },
    /// Game in progress - shows period and time (e.g., "1st 09:27" or "1st intermission")
    Live {
        period: String,
        time: Option<String>,
        intermission: bool,
    },
    /// Game finished - shows "Final", "Final (OT)", or "Final (SO)"
    Final { overtime: bool, shootout: bool },
}

impl ScoreBoxStatus {
    /// Format the status as a display string (no leading space - render adds it)
    pub fn display(&self) -> String {
        match self {
            ScoreBoxStatus::Scheduled { start_time } => start_time.clone(),
            ScoreBoxStatus::Live {
                period,
                time,
                intermission,
            } => {
                if *intermission {
                    format!("{} int.", period)
                } else if let Some(t) = time {
                    format!("{} {}", period, t)
                } else {
                    period.clone()
                }
            }
            ScoreBoxStatus::Final { overtime, shootout } => {
                if *shootout {
                    "Final (SO)".to_string()
                } else if *overtime {
                    "Final (OT)".to_string()
                } else {
                    "Final".to_string()
                }
            }
        }
    }
}

/// Compact score box widget
///
/// Renders a game score in a 25x6 character box:
/// ```text
///  Final
/// ╔══════════════════╤════╗
/// ║ Golden Knights   │ 10 ║
/// ╟──────────────────┼────╢
/// ║ Avalanche        │  3 ║
/// ╚══════════════════╧════╝
/// ```
#[derive(Debug, Clone)]
pub struct ScoreBox {
    /// Away team name (displayed first/top)
    pub away_team: String,
    /// Home team name (displayed second/bottom)
    pub home_team: String,
    /// Away team score (None shows "-")
    pub away_score: Option<i32>,
    /// Home team score (None shows "-")
    pub home_score: Option<i32>,
    /// Game status (scheduled, live, final)
    pub status: ScoreBoxStatus,
    /// Whether this box is selected/focused
    pub selected: bool,
}

impl ScoreBox {
    /// Create a new ScoreBox
    pub fn new(
        away_team: impl Into<String>,
        home_team: impl Into<String>,
        away_score: Option<i32>,
        home_score: Option<i32>,
        status: ScoreBoxStatus,
    ) -> Self {
        Self {
            away_team: away_team.into(),
            home_team: home_team.into(),
            away_score,
            home_score,
            status,
            selected: false,
        }
    }

    /// Set selected state
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Format a score value for display (right-aligned in 3 chars with trailing space)
    fn format_score(score: Option<i32>) -> String {
        match score {
            Some(s) => format!("{:>3} ", s),
            None => "  - ".to_string(),
        }
    }

    /// Truncate or pad team name to fit in the available width (17 chars)
    fn format_team_name(name: &str) -> String {
        const TEAM_NAME_WIDTH: usize = 17;
        if name.chars().count() > TEAM_NAME_WIDTH {
            name.chars().take(TEAM_NAME_WIDTH).collect()
        } else {
            format!("{:<width$}", name, width = TEAM_NAME_WIDTH)
        }
    }
}

impl StandaloneWidget for ScoreBox {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Ensure we have enough space
        if area.width < SCORE_BOX_WIDTH || area.height < SCORE_BOX_HEIGHT {
            return;
        }

        let bc = &config.box_chars;
        let x = area.x;
        let y = area.y;

        // Styles: fg3 for box chars, fg2 for team names and scores
        // When selected, both box and text use fg2 with reverse video
        let status_style = config.text_style(); // Status line never changes
        let (box_style, text_style) = if self.selected {
            let selected = config.text_style().add_modifier(SELECTION_STYLE_MODIFIER);
            (selected, selected)
        } else {
            (config.muted_style(), config.text_style()) // fg3 for box, fg2 for text
        };

        // Row 0: Status line with leading space (never reversed)
        let status_text = format!(" {}", self.status.display());
        buf.set_string(x, y, &status_text, status_style);

        // Row 1: Top border ╔══════════════════╤════╗
        // Width breakdown: ╔ (1) + ═×18 + ╤ (1) + ═×4 + ╗ (1) = 25
        let top_border = format!(
            "{}{}{}{}{}",
            bc.double_top_left,
            bc.double_horizontal.repeat(18),
            bc.double_top_junction,
            bc.double_horizontal.repeat(4),
            bc.double_top_right
        );
        buf.set_string(x, y + 1, &top_border, box_style);

        // Row 2: Away team ║ Team Name        │ SS ║
        // Render box chars and content separately for different styles
        buf.set_string(x, y + 2, &bc.double_vertical, box_style);
        buf.set_string(x + 1, y + 2, " ", box_style);
        buf.set_string(x + 2, y + 2, Self::format_team_name(&self.away_team), text_style);
        buf.set_string(x + 19, y + 2, &bc.vertical, box_style);
        buf.set_string(x + 20, y + 2, Self::format_score(self.away_score), text_style);
        buf.set_string(x + 24, y + 2, &bc.double_vertical, box_style);

        // Row 3: Separator ╟──────────────────┼────╢
        let separator = format!(
            "{}{}{}{}{}",
            bc.mixed_left_junction,
            bc.horizontal.repeat(18),
            bc.cross,
            bc.horizontal.repeat(4),
            bc.mixed_right_junction
        );
        buf.set_string(x, y + 3, &separator, box_style);

        // Row 4: Home team ║ Team Name        │ SS ║
        buf.set_string(x, y + 4, &bc.double_vertical, box_style);
        buf.set_string(x + 1, y + 4, " ", box_style);
        buf.set_string(x + 2, y + 4, Self::format_team_name(&self.home_team), text_style);
        buf.set_string(x + 19, y + 4, &bc.vertical, box_style);
        buf.set_string(x + 20, y + 4, Self::format_score(self.home_score), text_style);
        buf.set_string(x + 24, y + 4, &bc.double_vertical, box_style);

        // Row 5: Bottom border ╚══════════════════╧════╝
        let bottom_border = format!(
            "{}{}{}{}{}",
            bc.double_bottom_left,
            bc.double_horizontal.repeat(18),
            bc.double_bottom_junction,
            bc.double_horizontal.repeat(4),
            bc.double_bottom_right
        );
        buf.set_string(x, y + 5, &bottom_border, box_style);
    }

    fn preferred_width(&self) -> Option<u16> {
        Some(SCORE_BOX_WIDTH)
    }

    fn preferred_height(&self) -> Option<u16> {
        Some(SCORE_BOX_HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::assert_buffer;
    use crate::tui::widgets::testing::{render_widget_with_config, test_config};

    #[test]
    fn test_score_box_final() {
        let score_box = ScoreBox::new(
            "Golden Knights",
            "Avalanche",
            Some(10),
            Some(3),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " Final                   ",
                "╔══════════════════╤════╗",
                "║ Golden Knights   │ 10 ║",
                "╟──────────────────┼────╢",
                "║ Avalanche        │  3 ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_score_box_live() {
        let score_box = ScoreBox::new(
            "Maple Leafs",
            "Bruins",
            Some(2),
            Some(0),
            ScoreBoxStatus::Live {
                period: "1st".to_string(),
                time: Some("09:27".to_string()),
                intermission: false,
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " 1st 09:27               ",
                "╔══════════════════╤════╗",
                "║ Maple Leafs      │  2 ║",
                "╟──────────────────┼────╢",
                "║ Bruins           │  0 ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_score_box_scheduled() {
        let score_box = ScoreBox::new(
            "Canucks",
            "Blue Jackets",
            None,
            None,
            ScoreBoxStatus::Scheduled {
                start_time: "9PM".to_string(),
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " 9PM                     ",
                "╔══════════════════╤════╗",
                "║ Canucks          │  - ║",
                "╟──────────────────┼────╢",
                "║ Blue Jackets     │  - ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_score_box_final_ot() {
        let score_box = ScoreBox::new(
            "Canadiens",
            "Sabres",
            Some(3),
            Some(2),
            ScoreBoxStatus::Final {
                overtime: true,
                shootout: false,
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " Final (OT)              ",
                "╔══════════════════╤════╗",
                "║ Canadiens        │  3 ║",
                "╟──────────────────┼────╢",
                "║ Sabres           │  2 ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_score_box_intermission() {
        let score_box = ScoreBox::new(
            "Senators",
            "Capitals",
            Some(0),
            Some(0),
            ScoreBoxStatus::Live {
                period: "1st".to_string(),
                time: None,
                intermission: true,
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " 1st int.                ",
                "╔══════════════════╤════╗",
                "║ Senators         │  0 ║",
                "╟──────────────────┼────╢",
                "║ Capitals         │  0 ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_score_box_final_so() {
        let score_box = ScoreBox::new(
            "Rangers",
            "Devils",
            Some(4),
            Some(3),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: true,
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " Final (SO)              ",
                "╔══════════════════╤════╗",
                "║ Rangers          │  4 ║",
                "╟──────────────────┼────╢",
                "║ Devils           │  3 ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_score_box_long_team_name_truncated() {
        let score_box = ScoreBox::new(
            "Very Long Team Name That Should Be Truncated",
            "Short",
            Some(1),
            Some(2),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );

        let config = test_config();
        let buf = render_widget_with_config(&score_box, 25, 6, &config);

        assert_buffer(
            &buf,
            &[
                " Final                   ",
                "╔══════════════════╤════╗",
                "║ Very Long Team Na│  1 ║",
                "╟──────────────────┼────╢",
                "║ Short            │  2 ║",
                "╚══════════════════╧════╝",
            ],
        );
    }

    #[test]
    fn test_format_score() {
        assert_eq!(ScoreBox::format_score(Some(0)), "  0 ");
        assert_eq!(ScoreBox::format_score(Some(3)), "  3 ");
        assert_eq!(ScoreBox::format_score(Some(10)), " 10 ");
        assert_eq!(ScoreBox::format_score(None), "  - ");
    }

    #[test]
    fn test_format_team_name() {
        assert_eq!(ScoreBox::format_team_name("Bruins"), "Bruins           "); // 17 chars
        assert_eq!(
            ScoreBox::format_team_name("Golden Knights"),
            "Golden Knights   " // 17 chars
        );
        assert_eq!(
            ScoreBox::format_team_name("Very Long Team Name"),
            "Very Long Team Na" // 17 chars truncated
        );
    }

    #[test]
    fn test_status_display() {
        assert_eq!(
            ScoreBoxStatus::Scheduled {
                start_time: "7PM".to_string()
            }
            .display(),
            "7PM"
        );

        assert_eq!(
            ScoreBoxStatus::Live {
                period: "2nd".to_string(),
                time: Some("05:30".to_string()),
                intermission: false
            }
            .display(),
            "2nd 05:30"
        );

        assert_eq!(
            ScoreBoxStatus::Live {
                period: "2nd".to_string(),
                time: None,
                intermission: true
            }
            .display(),
            "2nd int."
        );

        assert_eq!(
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false
            }
            .display(),
            "Final"
        );

        assert_eq!(
            ScoreBoxStatus::Final {
                overtime: true,
                shootout: false
            }
            .display(),
            "Final (OT)"
        );

        assert_eq!(
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: true
            }
            .display(),
            "Final (SO)"
        );
    }
}
