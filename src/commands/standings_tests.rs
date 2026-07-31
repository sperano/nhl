use super::*;

#[test]
fn test_groupby_name() {
    assert_eq!(GroupBy::Division.name(), "Division");
    assert_eq!(GroupBy::Conference.name(), "Conference");
    assert_eq!(GroupBy::League.name(), "League");
}

#[test]
fn test_groupby_all() {
    let all = GroupBy::all();
    assert_eq!(all.len(), 4);
    assert_eq!(all[0], GroupBy::Wildcard);
    assert_eq!(all[1], GroupBy::Division);
    assert_eq!(all[2], GroupBy::Conference);
    assert_eq!(all[3], GroupBy::League);
}

#[test]
fn test_groupby_next_full_cycle() {
    // Test full cycle: Wildcard → Division → Conference → League → Wildcard
    let wildcard = GroupBy::Wildcard;
    let division = wildcard.next();
    assert_eq!(division, GroupBy::Division);

    let conference = division.next();
    assert_eq!(conference, GroupBy::Conference);

    let league = conference.next();
    assert_eq!(league, GroupBy::League);

    let back_to_wildcard = league.next();
    assert_eq!(back_to_wildcard, GroupBy::Wildcard);
}

#[test]
fn test_groupby_prev_full_cycle() {
    // Test full cycle: Wildcard → League → Conference → Division → Wildcard
    let wildcard = GroupBy::Wildcard;
    let league = wildcard.prev();
    assert_eq!(league, GroupBy::League);

    let conference = league.prev();
    assert_eq!(conference, GroupBy::Conference);

    let division = conference.prev();
    assert_eq!(division, GroupBy::Division);

    let back_to_wildcard = division.prev();
    assert_eq!(back_to_wildcard, GroupBy::Wildcard);
}

#[test]
fn test_groupby_next_from_each_variant() {
    assert_eq!(GroupBy::Wildcard.next(), GroupBy::Division);
    assert_eq!(GroupBy::Division.next(), GroupBy::Conference);
    assert_eq!(GroupBy::Conference.next(), GroupBy::League);
    assert_eq!(GroupBy::League.next(), GroupBy::Wildcard);
}

#[test]
fn test_groupby_prev_from_each_variant() {
    assert_eq!(GroupBy::Wildcard.prev(), GroupBy::League);
    assert_eq!(GroupBy::Division.prev(), GroupBy::Wildcard);
    assert_eq!(GroupBy::Conference.prev(), GroupBy::Division);
    assert_eq!(GroupBy::League.prev(), GroupBy::Conference);
}

#[test]
fn test_groupby_name_all_variants() {
    assert_eq!(GroupBy::Wildcard.name(), "Wildcard");
    assert_eq!(GroupBy::Division.name(), "Division");
    assert_eq!(GroupBy::Conference.name(), "Conference");
    assert_eq!(GroupBy::League.name(), "League");
}

#[test]
fn test_format_standings_by_group_empty() {
    let display = DisplayConfig::default();
    let standings = vec![];
    let output = format_standings_by_group(&standings, GroupBy::Division, false, &display);
    assert_eq!(output, "Loading standings...");
}

#[test]
fn test_merge_columns_equal_length() {
    let left = vec!["Left1".to_string(), "Left2".to_string()];
    let right = vec!["Right1".to_string(), "Right2".to_string()];

    let output = merge_columns(left, right, 10);

    // Should have both columns
    assert!(output.contains("Left1"));
    assert!(output.contains("Right1"));
    assert!(output.contains("Left2"));
    assert!(output.contains("Right2"));
}

#[test]
fn test_merge_columns_unequal_length() {
    let left = vec![
        "Left1".to_string(),
        "Left2".to_string(),
        "Left3".to_string(),
    ];
    let right = vec!["Right1".to_string()];

    let output = merge_columns(left, right, 10);

    // Should have all left items
    assert!(output.contains("Left1"));
    assert!(output.contains("Left2"));
    assert!(output.contains("Left3"));

    // Should have right item
    assert!(output.contains("Right1"));
}
