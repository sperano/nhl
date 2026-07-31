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
    assert_eq!(
        format_period_text(Some(PeriodType::Regulation), 1),
        "1st Period"
    );
    assert_eq!(
        format_period_text(Some(PeriodType::Regulation), 2),
        "2nd Period"
    );
    assert_eq!(
        format_period_text(Some(PeriodType::Regulation), 3),
        "3rd Period"
    );
    assert_eq!(
        format_period_text(Some(PeriodType::Regulation), 4),
        "4th Period"
    );
    assert_eq!(format_period_text(None, 1), "1st Period");
}

#[test]
fn test_format_period_text_overtime() {
    assert_eq!(
        format_period_text(Some(PeriodType::Overtime), 4),
        "Overtime"
    );
}

#[test]
fn test_format_period_text_shootout() {
    assert_eq!(
        format_period_text(Some(PeriodType::Shootout), 5),
        "Shootout"
    );
}
