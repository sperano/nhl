//! ScoreBox widget - compact game score display
//!
//! Displays a game score in a compact box format with:
//! - Status line above (e.g., "Final", "1st 09:27", "9PM")
//! - Double-line bordered box with team names and scores
//!
//! Width: 25 characters, Height: 6 rows

use crate::config::{RenderContext, SELECTION_STYLE_MODIFIER, THEMELESS_SELECTION_STYLE_MODIFIER};
use crate::layout_constants::{SCORE_BOX_HEIGHT, SCORE_BOX_WIDTH};
use ratatui::{buffer::Buffer, layout::Rect};

use super::StandaloneWidget;

/// Width of the team-name field, matched to `format_team_name`'s output length.
const TEAM_NAME_WIDTH: usize = 17;
/// Width of the score field, matched to `format_score`'s output (`"{:>3} "`, e.g. "  2 ").
const SCORE_FIELD_WIDTH: u16 = 4;
/// Column offset (from the box's left edge `x`) of the "│"/"╤"/"╧" divider between
/// the team-name and score fields: border (1) + leading space (1) + TEAM_NAME_WIDTH.
const SEPARATOR_COL: u16 = 2 + TEAM_NAME_WIDTH as u16;
/// Column offset of the score field, immediately after the divider.
const SCORE_COL: u16 = SEPARATOR_COL + 1;
/// Column offset of the closing "║", immediately after the score field.
const RIGHT_BORDER_COL: u16 = SCORE_COL + SCORE_FIELD_WIDTH;
/// Length of the top/bottom border's first horizontal run: from just after the
/// left corner character up to (not including) the junction at `SEPARATOR_COL`.
const TOP_BORDER_NAME_SEGMENT: usize = SEPARATOR_COL as usize - 1;

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
        if name.chars().count() > TEAM_NAME_WIDTH {
            name.chars().take(TEAM_NAME_WIDTH).collect()
        } else {
            format!("{:<width$}", name, width = TEAM_NAME_WIDTH)
        }
    }
}

impl StandaloneWidget for ScoreBox {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Ensure we have enough space
        if area.width < SCORE_BOX_WIDTH || area.height < SCORE_BOX_HEIGHT {
            return;
        }

        // Fill area with background color first
        let score_box_area = Rect::new(area.x, area.y, SCORE_BOX_WIDTH, SCORE_BOX_HEIGHT);
        buf.set_style(score_box_area, ctx.base_style());

        let bc = ctx.box_chars();
        let x = area.x;
        let y = area.y;

        // Styles: fg3 for box chars, fg2 for team names and scores
        // When selected, both box and text use fg2 with reverse video
        let status_style = ctx.text_style(); // Status line never changes
        let (box_style, text_style) = if self.selected {
            let selected = if let Some(theme) = ctx.theme() {
                ctx.text_style()
                    .fg(theme.selection_text_fg)
                    .bg(theme.selection_text_bg)
                    .add_modifier(SELECTION_STYLE_MODIFIER)
            } else {
                ctx.text_style()
                    .add_modifier(THEMELESS_SELECTION_STYLE_MODIFIER)
            };
            (selected, selected)
        } else {
            (ctx.boxchar_style(), ctx.text_style()) // fg3 for box, fg2 for text
        };

        // Row 0: Status line with leading space (never reversed)
        let status_text = format!(" {}", self.status.display());
        buf.set_string(x, y, &status_text, status_style);

        // Row 1: Top border ╔══════════════════╤════╗
        // Width breakdown: ╔ (1) + ═×TOP_BORDER_NAME_SEGMENT + ╤ (1) + ═×SCORE_FIELD_WIDTH + ╗ (1) = 25
        let top_border = format!(
            "{}{}{}{}{}",
            bc.double_top_left,
            bc.double_horizontal.repeat(TOP_BORDER_NAME_SEGMENT),
            bc.double_top_junction,
            bc.double_horizontal.repeat(SCORE_FIELD_WIDTH as usize),
            bc.double_top_right
        );
        buf.set_string(x, y + 1, &top_border, box_style);

        // Row 2: Away team ║ Team Name        │ SS ║
        // Render box chars and content separately for different styles
        buf.set_string(x, y + 2, bc.double_vertical, box_style);
        buf.set_string(x + 1, y + 2, " ", box_style);
        buf.set_string(
            x + 2,
            y + 2,
            Self::format_team_name(&self.away_team),
            text_style,
        );
        buf.set_string(x + SEPARATOR_COL, y + 2, bc.vertical, box_style);
        buf.set_string(
            x + SCORE_COL,
            y + 2,
            Self::format_score(self.away_score),
            text_style,
        );
        buf.set_string(x + RIGHT_BORDER_COL, y + 2, bc.double_vertical, box_style);

        // Row 3: Separator ╟──────────────────┼────╢
        let separator = format!(
            "{}{}{}{}{}",
            bc.mixed_left_junction,
            bc.horizontal.repeat(TOP_BORDER_NAME_SEGMENT),
            bc.cross,
            bc.horizontal.repeat(SCORE_FIELD_WIDTH as usize),
            bc.mixed_right_junction
        );
        buf.set_string(x, y + 3, &separator, box_style);

        // Row 4: Home team ║ Team Name        │ SS ║
        buf.set_string(x, y + 4, bc.double_vertical, box_style);
        buf.set_string(x + 1, y + 4, " ", box_style);
        buf.set_string(
            x + 2,
            y + 4,
            Self::format_team_name(&self.home_team),
            text_style,
        );
        buf.set_string(x + SEPARATOR_COL, y + 4, bc.vertical, box_style);
        buf.set_string(
            x + SCORE_COL,
            y + 4,
            Self::format_score(self.home_score),
            text_style,
        );
        buf.set_string(x + RIGHT_BORDER_COL, y + 4, bc.double_vertical, box_style);

        // Row 5: Bottom border ╚══════════════════╧════╝
        let bottom_border = format!(
            "{}{}{}{}{}",
            bc.double_bottom_left,
            bc.double_horizontal.repeat(TOP_BORDER_NAME_SEGMENT),
            bc.double_bottom_junction,
            bc.double_horizontal.repeat(SCORE_FIELD_WIDTH as usize),
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
#[path = "score_box_tests.rs"]
mod tests;
