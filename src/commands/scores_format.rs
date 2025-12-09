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
pub fn format_period_text(period_type: PeriodType, period_number: i32) -> String {
    match period_type {
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
    let base_cols = BASE_SCORE_COLUMNS; // empty, 1, 2, 3, T
    let ot_cols = if has_ot { 1 } else { 0 };
    let so_cols = if has_so { 1 } else { 0 };
    let total_cols = base_cols + ot_cols + so_cols;
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
    output.push_str(&build_team_row(
        away_team,
        away_score,
        away_periods,
        has_ot,
        has_so,
        total_cols,
        max_width,
        &should_show_period,
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
    border.push_str(&box_chars.top_left);
    border.push_str(&box_chars.horizontal.repeat(TEAM_ABBREV_COL_WIDTH)); // team name column
    for _ in 1..total_cols {
        border.push_str(&box_chars.top_junction);
        border.push_str(&box_chars.horizontal.repeat(PERIOD_COL_WIDTH));
    }
    border.push_str(&box_chars.top_right);

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
    border.push_str(&box_chars.left_junction);
    border.push_str(&box_chars.horizontal.repeat(TEAM_ABBREV_COL_WIDTH));
    for _ in 1..total_cols {
        border.push_str(&box_chars.cross);
        border.push_str(&box_chars.horizontal.repeat(PERIOD_COL_WIDTH));
    }
    border.push_str(&box_chars.right_junction);

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
    border.push_str(&box_chars.bottom_left);
    border.push_str(&box_chars.horizontal.repeat(TEAM_ABBREV_COL_WIDTH));
    for _ in 1..total_cols {
        border.push_str(&box_chars.bottom_junction);
        border.push_str(&box_chars.horizontal.repeat(PERIOD_COL_WIDTH));
    }
    border.push_str(&box_chars.bottom_right);

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
    row.push_str(&box_chars.vertical);
    row.push_str(&format!("{:^5}", ""));
    row.push_str(&box_chars.vertical);
    row.push_str(&format!("{:^4}", "1"));
    row.push_str(&box_chars.vertical);
    row.push_str(&format!("{:^4}", "2"));
    row.push_str(&box_chars.vertical);
    row.push_str(&format!("{:^4}", "3"));

    if has_ot {
        row.push_str(&box_chars.vertical);
        row.push_str(&format!("{:^4}", "OT"));
    }

    if has_so {
        row.push_str(&box_chars.vertical);
        row.push_str(&format!("{:^4}", "SO"));
    }

    row.push_str(&box_chars.vertical);
    row.push_str(&format!("{:^4}", "T"));
    row.push_str(&box_chars.vertical);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        row.push_str(&" ".repeat(padding));
    }
    row.push('\n');
    row
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
    if let Some(periods) = periods {
        // Period 1
        let p1_value = if should_show_period(1) {
            periods
                .get(PERIOD_1_INDEX)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string())
        } else {
            "-".to_string()
        };
        output.push_str(&format!("{:^width$}", p1_value, width = PERIOD_COL_WIDTH));
        output.push_str(&box_chars.vertical);

        // Period 2
        let p2_value = if should_show_period(2) {
            periods
                .get(PERIOD_2_INDEX)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string())
        } else {
            "-".to_string()
        };
        output.push_str(&format!("{:^width$}", p2_value, width = PERIOD_COL_WIDTH));
        output.push_str(&box_chars.vertical);

        // Period 3
        let p3_value = if should_show_period(3) {
            periods
                .get(PERIOD_3_INDEX)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string())
        } else {
            "-".to_string()
        };
        output.push_str(&format!("{:^width$}", p3_value, width = PERIOD_COL_WIDTH));

        if has_ot {
            output.push_str(&box_chars.vertical);
            let ot_value = if should_show_period(OVERTIME_PERIOD_NUM) {
                periods
                    .get(OVERTIME_INDEX)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".to_string())
            } else {
                "-".to_string()
            };
            output.push_str(&format!("{:^width$}", ot_value, width = PERIOD_COL_WIDTH));
        }

        if has_so {
            output.push_str(&box_chars.vertical);
            let so_value = if should_show_period(SHOOTOUT_PERIOD_NUM) {
                periods
                    .get(SHOOTOUT_INDEX)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".to_string())
            } else {
                "-".to_string()
            };
            output.push_str(&format!("{:^width$}", so_value, width = PERIOD_COL_WIDTH));
        }
    } else {
        output.push_str(&format!("{:^width$}", "-", width = PERIOD_COL_WIDTH)); // P1
        output.push_str(&box_chars.vertical);
        output.push_str(&format!("{:^width$}", "-", width = PERIOD_COL_WIDTH)); // P2
        output.push_str(&box_chars.vertical);
        output.push_str(&format!("{:^width$}", "-", width = PERIOD_COL_WIDTH)); // P3

        if has_ot {
            output.push_str(&box_chars.vertical);
            output.push_str(&format!("{:^width$}", "-", width = PERIOD_COL_WIDTH));
            // OT
        }

        if has_so {
            output.push_str(&box_chars.vertical);
            output.push_str(&format!("{:^width$}", "-", width = PERIOD_COL_WIDTH));
            // SO
        }
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
    row.push_str(&box_chars.vertical);
    row.push_str(&format!("{:^5}", team_abbrev));
    row.push_str(&box_chars.vertical);

    render_team_periods(
        &mut row,
        team_periods,
        has_ot,
        has_so,
        should_show_period,
        box_chars,
    );

    row.push_str(&box_chars.vertical);
    row.push_str(&format!(
        "{:^4}",
        team_score
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    row.push_str(&box_chars.vertical);

    let padding = calculate_padding(total_cols, max_width);
    if padding > 0 {
        row.push_str(&" ".repeat(padding));
    }
    row.push('\n');
    row
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
        let period_num = period.period_descriptor.number as usize;

        // Determine if this is OT or SO
        if period.period_descriptor.period_type == PeriodType::Overtime {
            has_ot = true;
            // Ensure we have enough slots (up to OVERTIME_INDEX + 1)
            if away_periods.len() < OVERTIME_INDEX + 1 {
                away_periods.push(0);
                home_periods.push(0);
            }
        } else if period.period_descriptor.period_type == PeriodType::Shootout {
            has_so = true;
            // Ensure we have enough slots (up to SHOOTOUT_INDEX + 1)
            while away_periods.len() < SHOOTOUT_INDEX + 1 {
                away_periods.push(0);
                home_periods.push(0);
            }
        }

        // Get the final score after this period (from last goal)
        if let Some(last_goal) = period.goals.last() {
            let period_away_score = last_goal.away_score;
            let period_home_score = last_goal.home_score;

            // Calculate goals scored in this period
            let away_goals_in_period = period_away_score - prev_away_score;
            let home_goals_in_period = period_home_score - prev_home_score;

            // Store in the appropriate slot
            let idx = match period.period_descriptor.period_type {
                PeriodType::Regulation => (period_num - 1).min(PERIOD_3_INDEX), // P1=0, P2=1, P3=2
                PeriodType::Overtime => OVERTIME_INDEX,
                PeriodType::Shootout => SHOOTOUT_INDEX,
            };

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
mod tests {
    use super::*;

    #[test]
    fn test_calculate_padding() {
        // Test basic padding calculation
        // With 5 total_cols: current_width = 1 + 5 + (5-1)*5 + 1 = 27
        // Padding = 37 - 27 = 10
        assert_eq!(calculate_padding(5, 37), 10);

        // With 7 total_cols: current_width = 1 + 5 + (7-1)*5 + 1 = 37
        // Padding = 37 - 37 = 0
        assert_eq!(calculate_padding(7, 37), 0);
    }

    #[test]
    fn test_build_top_border() {
        let box_chars = BoxChars::unicode();
        let border = build_top_border(5, 37, &box_chars);
        assert!(border.starts_with('╭'));
        assert!(border.contains('┬'));
        assert!(border.contains('╮'));
        assert!(border.ends_with('\n'));
    }

    #[test]
    fn test_build_middle_border() {
        let box_chars = BoxChars::unicode();
        let border = build_middle_border(5, 37, &box_chars);
        assert!(border.starts_with('├'));
        assert!(border.contains('┼'));
        assert!(border.contains('┤'));
        assert!(border.ends_with('\n'));
    }

    #[test]
    fn test_build_bottom_border() {
        let box_chars = BoxChars::unicode();
        let border = build_bottom_border(5, 37, &box_chars);
        assert!(border.starts_with('╰'));
        assert!(border.contains('┴'));
        assert!(border.contains('╯'));
        assert!(border.ends_with('\n'));
    }

    #[test]
    fn test_build_header_row_basic() {
        let box_chars = BoxChars::unicode();
        let header = build_header_row(false, false, 5, 37, &box_chars);
        assert!(header.contains('│'));
        assert!(header.contains('1'));
        assert!(header.contains('2'));
        assert!(header.contains('3'));
        assert!(header.contains('T'));
        assert!(!header.contains("OT"));
        assert!(!header.contains("SO"));
    }

    #[test]
    fn test_build_header_row_with_ot() {
        let box_chars = BoxChars::unicode();
        let header = build_header_row(true, false, 6, 37, &box_chars);
        assert!(header.contains("OT"));
        assert!(!header.contains("SO"));
    }

    #[test]
    fn test_build_header_row_with_shootout() {
        let box_chars = BoxChars::unicode();
        let header = build_header_row(true, true, 7, 37, &box_chars);
        assert!(header.contains("OT"));
        assert!(header.contains("SO"));
    }

    #[test]
    fn test_build_score_table_no_scores() {
        let box_chars = BoxChars::unicode();
        let table = build_score_table(
            "TOR", "MTL", None, None, false, false, None, None, None, &box_chars,
        );

        // Should contain team names
        assert!(table.contains("TOR"));
        assert!(table.contains("MTL"));

        // Should contain dashes for no scores
        assert!(table.contains('-'));

        // Should have proper box-drawing characters
        assert!(table.contains('╭'));
        assert!(table.contains('╰'));
        assert!(table.contains('│'));
    }

    #[test]
    fn test_build_score_table_with_scores() {
        let away_periods = vec![1, 2, 0];
        let home_periods = vec![0, 1, 2];
        let box_chars = BoxChars::unicode();

        let table = build_score_table(
            "BOS",
            "NYR",
            Some(3),
            Some(3),
            false,
            false,
            Some(&away_periods),
            Some(&home_periods),
            None,
            &box_chars,
        );

        // Should contain team names
        assert!(table.contains("BOS"));
        assert!(table.contains("NYR"));

        // Should contain final scores
        assert!(table.contains('3'));

        // Should have period scores
        assert!(table.contains('1'));
        assert!(table.contains('2'));
        assert!(table.contains('0'));
    }

    #[test]
    fn test_build_score_table_with_overtime() {
        let away_periods = vec![1, 1, 1, 1];
        let home_periods = vec![1, 1, 1, 0];
        let box_chars = BoxChars::unicode();

        let table = build_score_table(
            "EDM",
            "VAN",
            Some(4),
            Some(3),
            true,
            false,
            Some(&away_periods),
            Some(&home_periods),
            None,
            &box_chars,
        );

        // Should contain OT header
        assert!(table.contains("OT"));

        // Should show OT score
        assert!(table.contains('4')); // Away total
        assert!(table.contains('3')); // Home total
    }

    #[test]
    fn test_period_scores_struct() {
        let scores = PeriodScores {
            away_periods: vec![1, 2, 3],
            home_periods: vec![0, 1, 2],
            has_ot: false,
            has_so: false,
        };

        assert_eq!(scores.away_periods.len(), 3);
        assert_eq!(scores.home_periods.len(), 3);
        assert!(!scores.has_ot);
        assert!(!scores.has_so);
    }

    #[test]
    fn test_format_period_text_regular() {
        assert_eq!(format_period_text(PeriodType::Regulation, 1), "1st Period");
        assert_eq!(format_period_text(PeriodType::Regulation, 2), "2nd Period");
        assert_eq!(format_period_text(PeriodType::Regulation, 3), "3rd Period");
        assert_eq!(format_period_text(PeriodType::Regulation, 4), "4th Period");
    }

    #[test]
    fn test_format_period_text_overtime() {
        assert_eq!(format_period_text(PeriodType::Overtime, 4), "Overtime");
    }

    #[test]
    fn test_format_period_text_shootout() {
        assert_eq!(format_period_text(PeriodType::Shootout, 5), "Shootout");
    }
}
