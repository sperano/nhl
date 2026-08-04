use crate::formatting::BoxChars;
use crate::layout_constants::{GAME_BOX_WIDTH, PERIOD_COL_WIDTH, TEAM_ABBREV_COL_WIDTH};
use nhl_api::{GameSummary, PeriodType};

// Score Table Constants
/// Number of base columns in score table (empty, 1, 2, 3, T)
const BASE_SCORE_COLUMNS: usize = 5;

// Period Index Constants
/// Array index for period 1 scores
const PERIOD_1_INDEX: usize = 0;

/// Array index for period 2 scores
const PERIOD_2_INDEX: usize = 1;

/// Array index for period 3 scores
const PERIOD_3_INDEX: usize = 2;

/// Array index for overtime scores
const OVERTIME_INDEX: usize = 3;

/// Array index for shootout scores
const SHOOTOUT_INDEX: usize = 4;

/// Period number for overtime (used in game state)
const OVERTIME_PERIOD_NUM: i32 = 4;

/// Period number for shootout (used in game state)
const SHOOTOUT_PERIOD_NUM: i32 = 5;

/// Period-by-period score data
#[derive(Debug, Clone)]
pub struct PeriodScores {
    pub away_periods: Vec<i32>,
    pub home_periods: Vec<i32>,
    pub has_ot: bool,
    pub has_so: bool,
}

impl PeriodScores {
    /// Calculate total score for away team
    pub fn away_total(&self) -> i32 {
        self.away_periods.iter().sum()
    }

    /// Calculate total score for home team
    pub fn home_total(&self) -> i32 {
        self.home_periods.iter().sum()
    }
}

/// Format period text (e.g., "1st Period", "Overtime", "Shootout")
/// Missing period type (historical data) is treated as regulation.
pub fn format_period_text(period_type: Option<PeriodType>, period_number: i32) -> String {
    match period_type.unwrap_or(PeriodType::Regulation) {
        PeriodType::Regulation => {
            let ordinal = match period_number {
                1 => "1st",
                2 => "2nd",
                3 => "3rd",
                n => return format!("{}th Period", n),
            };
            format!("{} Period", ordinal)
        }
        PeriodType::Overtime => "Overtime".to_string(),
        PeriodType::Shootout => "Shootout".to_string(),
    }
}

/// Total column count for the score table (base columns plus OT/SO if present)
fn total_score_columns(has_ot: bool, has_so: bool) -> usize {
    BASE_SCORE_COLUMNS + has_ot as usize + has_so as usize
}

/// Build both teams' score rows back-to-back.
#[allow(clippy::too_many_arguments)]
fn build_team_rows(
    away_team: &str,
    home_team: &str,
    away_score: Option<i32>,
    home_score: Option<i32>,
    away_periods: Option<&Vec<i32>>,
    home_periods: Option<&Vec<i32>>,
    has_ot: bool,
    has_so: bool,
    total_cols: usize,
    max_width: usize,
    should_show_period: &impl Fn(i32) -> bool,
    box_chars: &BoxChars,
) -> String {
    let mut output = String::new();
    output.push_str(&build_team_row(
        away_team,
        away_score,
        away_periods,
        has_ot,
        has_so,
        total_cols,
        max_width,
        should_show_period,
        box_chars,
    ));
    output.push_str(&build_team_row(
        home_team,
        home_score,
        home_periods,
        has_ot,
        has_so,
        total_cols,
        max_width,
        should_show_period,
        box_chars,
    ));
    output
}

#[allow(clippy::too_many_arguments)]
pub fn build_score_table(
    away_team: &str,
    home_team: &str,
    away_score: Option<i32>,
    home_score: Option<i32>,
    has_ot: bool,
    has_so: bool,
    away_periods: Option<&Vec<i32>>,
    home_periods: Option<&Vec<i32>>,
    current_period_num: Option<i32>,
    box_chars: &BoxChars,
) -> String {
    let mut output = String::new();

    // Calculate column count based on actual periods, but we'll pad to max width later
    let total_cols = total_score_columns(has_ot, has_so);
    let max_width = GAME_BOX_WIDTH as usize; // Width with all 5 periods

    // Helper to check if a period should show score or dash
    let should_show_period =
        |period: i32| -> bool { current_period_num.is_none_or(|current| period <= current) };

    // Build table components
    output.push_str(&build_top_border(total_cols, max_width, box_chars));
    output.push_str(&build_header_row(
        has_ot, has_so, total_cols, max_width, box_chars,
    ));
    output.push_str(&build_middle_border(total_cols, max_width, box_chars));
    output.push_str(&build_team_rows(
        away_team,
        home_team,
        away_score,
        home_score,
        away_periods,
        home_periods,
        has_ot,
        has_so,
        total_cols,
        max_width,
        &should_show_period,
        box_chars,
    ));
    output.push_str(&build_bottom_border(total_cols, max_width, box_chars));

    output
}

/// Calculate padding needed to reach max width of 37 characters
fn calculate_padding(total_cols: usize, max_width: usize) -> usize {
    // Calculate actual width: 1 (left border) + 5 (team column) + (total_cols-1) * (1 separator + 4 chars) + 1 (right border)
    let current_width = 1 + 5 + (total_cols - 1) * 5 + 1;
    max_width.saturating_sub(current_width)
}

/// Build top border for the score table
fn build_top_border(total_cols: usize, max_width: usize, box_chars: &BoxChars) -> String {
    let mut border = String::new();
    border.push_str(box_chars.top_left);
    border.push_str(&box_chars.horizontal.repeat(TEAM_ABBREV_COL_WIDTH)); // team name column
    for _ in 1..total_cols {
        border.push_str(box_chars.top_junction);
        border.push_str(&box_chars.horizontal.repeat(PERIOD_COL_WIDTH));
    }
    border.push_str(box_chars.top_right);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        border.push_str(&" ".repeat(padding));
    }
    border.push('\n');
    border
}

/// Build middle border for the score table
fn build_middle_border(total_cols: usize, max_width: usize, box_chars: &BoxChars) -> String {
    let mut border = String::new();
    border.push_str(box_chars.left_junction);
    border.push_str(&box_chars.horizontal.repeat(TEAM_ABBREV_COL_WIDTH));
    for _ in 1..total_cols {
        border.push_str(box_chars.cross);
        border.push_str(&box_chars.horizontal.repeat(PERIOD_COL_WIDTH));
    }
    border.push_str(box_chars.right_junction);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        border.push_str(&" ".repeat(padding));
    }
    border.push('\n');
    border
}

/// Build bottom border for the score table
fn build_bottom_border(total_cols: usize, max_width: usize, box_chars: &BoxChars) -> String {
    let mut border = String::new();
    border.push_str(box_chars.bottom_left);
    border.push_str(&box_chars.horizontal.repeat(TEAM_ABBREV_COL_WIDTH));
    for _ in 1..total_cols {
        border.push_str(box_chars.bottom_junction);
        border.push_str(&box_chars.horizontal.repeat(PERIOD_COL_WIDTH));
    }
    border.push_str(box_chars.bottom_right);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        border.push_str(&" ".repeat(padding));
    }
    border.push('\n');
    border
}

/// Build header row showing period numbers (1, 2, 3, OT, SO, T)
fn build_header_row(
    has_ot: bool,
    has_so: bool,
    total_cols: usize,
    max_width: usize,
    box_chars: &BoxChars,
) -> String {
    let mut row = String::new();
    row.push_str(box_chars.vertical);
    row.push_str(&format!("{:^5}", ""));
    row.push_str(box_chars.vertical);
    row.push_str(&format!("{:^4}", "1"));
    row.push_str(box_chars.vertical);
    row.push_str(&format!("{:^4}", "2"));
    row.push_str(box_chars.vertical);
    row.push_str(&format!("{:^4}", "3"));

    if has_ot {
        row.push_str(box_chars.vertical);
        row.push_str(&format!("{:^4}", "OT"));
    }

    if has_so {
        row.push_str(box_chars.vertical);
        row.push_str(&format!("{:^4}", "SO"));
    }

    row.push_str(box_chars.vertical);
    row.push_str(&format!("{:^4}", "T"));
    row.push_str(box_chars.vertical);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        row.push_str(&" ".repeat(padding));
    }
    row.push('\n');
    row
}

/// Compute the display value for one period cell. Shows "-" when the period hasn't
/// been reached yet (per `should_show_period`) or when no per-period data is available.
fn period_cell_value(
    periods: Option<&Vec<i32>>,
    period_num: i32,
    index: usize,
    should_show_period: &impl Fn(i32) -> bool,
) -> String {
    if !should_show_period(period_num) {
        return "-".to_string();
    }
    periods
        .and_then(|p| p.get(index))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string())
}

/// Render period scores for a team
fn render_team_periods(
    output: &mut String,
    periods: Option<&Vec<i32>>,
    has_ot: bool,
    has_so: bool,
    should_show_period: &impl Fn(i32) -> bool,
    box_chars: &BoxChars,
) {
    let cell = |period_num: i32, index: usize| {
        format!(
            "{:^width$}",
            period_cell_value(periods, period_num, index, should_show_period),
            width = PERIOD_COL_WIDTH
        )
    };

    output.push_str(&cell(1, PERIOD_1_INDEX));
    output.push_str(box_chars.vertical);
    output.push_str(&cell(2, PERIOD_2_INDEX));
    output.push_str(box_chars.vertical);
    output.push_str(&cell(3, PERIOD_3_INDEX));

    if has_ot {
        output.push_str(box_chars.vertical);
        output.push_str(&cell(OVERTIME_PERIOD_NUM, OVERTIME_INDEX));
    }

    if has_so {
        output.push_str(box_chars.vertical);
        output.push_str(&cell(SHOOTOUT_PERIOD_NUM, SHOOTOUT_INDEX));
    }
}

/// Build a team row with period scores
#[allow(clippy::too_many_arguments)]
fn build_team_row(
    team_abbrev: &str,
    team_score: Option<i32>,
    team_periods: Option<&Vec<i32>>,
    has_ot: bool,
    has_so: bool,
    total_cols: usize,
    max_width: usize,
    should_show_period: &impl Fn(i32) -> bool,
    box_chars: &BoxChars,
) -> String {
    let mut row = String::new();
    row.push_str(box_chars.vertical);
    row.push_str(&format!("{:^5}", team_abbrev));
    row.push_str(box_chars.vertical);

    render_team_periods(
        &mut row,
        team_periods,
        has_ot,
        has_so,
        should_show_period,
        box_chars,
    );

    row.push_str(box_chars.vertical);
    row.push_str(&format!(
        "{:^4}",
        team_score
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    row.push_str(box_chars.vertical);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        row.push_str(&" ".repeat(padding));
    }
    row.push('\n');
    row
}

/// Ensure `away_periods`/`home_periods` have enough slots for this period's score, and
/// update the has_ot/has_so flags based on its type.
fn track_period_slot(
    period_type: Option<PeriodType>,
    away_periods: &mut Vec<i32>,
    home_periods: &mut Vec<i32>,
    has_ot: &mut bool,
    has_so: &mut bool,
) {
    if period_type == Some(PeriodType::Overtime) {
        *has_ot = true;
        // Ensure we have enough slots (up to OVERTIME_INDEX + 1)
        if away_periods.len() < OVERTIME_INDEX + 1 {
            away_periods.push(0);
            home_periods.push(0);
        }
    } else if period_type == Some(PeriodType::Shootout) {
        *has_so = true;
        // Ensure we have enough slots (up to SHOOTOUT_INDEX + 1)
        while away_periods.len() < SHOOTOUT_INDEX + 1 {
            away_periods.push(0);
            home_periods.push(0);
        }
    }
}

/// Index within `away_periods`/`home_periods` where this period's score belongs.
fn period_score_index(period_type: Option<PeriodType>, period_num: usize) -> usize {
    match period_type {
        // Missing period type (historical data) is treated as regulation
        Some(PeriodType::Regulation) | None => (period_num - 1).min(PERIOD_3_INDEX), // P1=0, P2=1, P3=2
        Some(PeriodType::Overtime) => OVERTIME_INDEX,
        Some(PeriodType::Shootout) => SHOOTOUT_INDEX,
    }
}

/// Extract period scores from GameSummary
pub fn extract_period_scores(summary: &GameSummary) -> PeriodScores {
    let mut away_periods = vec![0, 0, 0]; // P1, P2, P3
    let mut home_periods = vec![0, 0, 0];
    let mut has_ot = false;
    let mut has_so = false;

    let mut prev_away_score = 0;
    let mut prev_home_score = 0;

    for period in &summary.scoring {
        let period_type = period.period_descriptor.period_type;
        track_period_slot(
            period_type,
            &mut away_periods,
            &mut home_periods,
            &mut has_ot,
            &mut has_so,
        );

        // Get the final score after this period (from last goal)
        if let Some(last_goal) = period.goals.last() {
            let period_away_score = last_goal.away_score;
            let period_home_score = last_goal.home_score;

            // Calculate goals scored in this period
            let away_goals_in_period = period_away_score - prev_away_score;
            let home_goals_in_period = period_home_score - prev_home_score;

            let idx = period_score_index(period_type, period.period_descriptor.number as usize);

            if idx < away_periods.len() {
                away_periods[idx] = away_goals_in_period;
                home_periods[idx] = home_goals_in_period;
            }

            prev_away_score = period_away_score;
            prev_home_score = period_home_score;
        }
    }

    PeriodScores {
        away_periods,
        home_periods,
        has_ot,
        has_so,
    }
}

#[cfg(test)]
#[path = "scores_format_tests.rs"]
mod tests;
