use super::*;
use crate::fixtures;
use nhl_api::{LocalizedString, ZoneCode};

/// Live game 2024020002 (TOR @ OTT, period 1) with a fully populated roster and a
/// mix of faceoff/shot/hit/penalty/goal/takeaway/giveaway plays.
fn sample_pbp() -> PlayByPlay {
    fixtures::create_mock_play_by_play(2024020002)
}

fn play(pbp: &PlayByPlay, event_id: i64) -> PlayEvent {
    pbp.plays
        .iter()
        .find(|p| p.event_id == event_id)
        .cloned()
        .unwrap_or_else(|| panic!("no play with event_id {event_id} in fixture"))
}

fn with_situation_code(mut event: PlayEvent, code: &str) -> PlayEvent {
    event.situation_code = code.to_string();
    event
}

// --- EventFilter ---

#[test]
fn test_show_all_true_when_no_filters_set() {
    assert!(EventFilter::default().show_all());
}

#[test]
fn test_show_all_false_when_any_filter_set() {
    assert!(!EventFilter {
        goals: true,
        ..Default::default()
    }
    .show_all());
    assert!(!EventFilter {
        penalties: true,
        ..Default::default()
    }
    .show_all());
    assert!(!EventFilter {
        shots: true,
        ..Default::default()
    }
    .show_all());
}

#[test]
fn test_should_show_returns_true_for_everything_when_show_all() {
    let filter = EventFilter::default();
    assert!(filter.should_show(&PlayEventType::Goal));
    assert!(filter.should_show(&PlayEventType::Hit));
    assert!(filter.should_show(&PlayEventType::Stoppage));
}

#[test]
fn test_should_show_goals_only_filter() {
    let filter = EventFilter {
        goals: true,
        ..Default::default()
    };
    assert!(filter.should_show(&PlayEventType::Goal));
    assert!(!filter.should_show(&PlayEventType::Penalty));
    assert!(!filter.should_show(&PlayEventType::ShotOnGoal));
    assert!(!filter.should_show(&PlayEventType::Hit));
}

#[test]
fn test_should_show_shots_filter_includes_goals_and_all_shot_types() {
    let filter = EventFilter {
        shots: true,
        ..Default::default()
    };
    assert!(filter.should_show(&PlayEventType::Goal));
    assert!(filter.should_show(&PlayEventType::ShotOnGoal));
    assert!(filter.should_show(&PlayEventType::MissedShot));
    assert!(filter.should_show(&PlayEventType::BlockedShot));
    assert!(!filter.should_show(&PlayEventType::Penalty));
}

#[test]
fn test_should_show_penalties_only_filter() {
    let filter = EventFilter {
        penalties: true,
        ..Default::default()
    };
    assert!(filter.should_show(&PlayEventType::Penalty));
    assert!(!filter.should_show(&PlayEventType::Goal));
    assert!(!filter.should_show(&PlayEventType::ShotOnGoal));
}

// --- get_filtered_plays ---

#[test]
fn test_get_filtered_plays_default_filter_returns_all_plays_in_order() {
    let pbp = sample_pbp();
    let filtered = get_filtered_plays(&pbp, &EventFilter::default());
    assert_eq!(filtered.len(), pbp.plays.len());
    for (a, b) in filtered.iter().zip(pbp.plays.iter()) {
        assert_eq!(a.event_id, b.event_id);
    }
}

#[test]
fn test_get_filtered_plays_goals_only_returns_just_the_goals_in_order() {
    let pbp = sample_pbp();
    let filter = EventFilter {
        goals: true,
        ..Default::default()
    };
    let filtered = get_filtered_plays(&pbp, &filter);
    let event_ids: Vec<i64> = filtered.iter().map(|p| p.event_id).collect();
    assert_eq!(event_ids, vec![105, 108]);
}

// --- get_event_color_and_label ---

#[test]
fn test_get_event_color_and_label_known_variants() {
    assert_eq!(
        get_event_color_and_label(&PlayEventType::Goal),
        (colors::GREEN, "GOAL")
    );
    assert_eq!(
        get_event_color_and_label(&PlayEventType::Penalty),
        (colors::RED, "PENALTY")
    );
    assert_eq!(
        get_event_color_and_label(&PlayEventType::ShotOnGoal),
        (colors::BLUE, "SHOT")
    );
    assert_eq!(
        get_event_color_and_label(&PlayEventType::Faceoff),
        (colors::GRAY, "FACEOFF")
    );
    assert_eq!(
        get_event_color_and_label(&PlayEventType::Giveaway),
        (colors::YELLOW, "GIVEAWAY")
    );
}

#[test]
fn test_get_event_color_and_label_falls_back_to_event_for_unmapped_types() {
    assert_eq!(
        get_event_color_and_label(&PlayEventType::DelayedPenalty),
        (colors::GRAY, "EVENT")
    );
    assert_eq!(
        get_event_color_and_label(&PlayEventType::Unknown),
        (colors::GRAY, "EVENT")
    );
}

// --- format_period_event ---

#[test]
fn test_format_period_event_boundary_types() {
    assert_eq!(
        format_period_event(&PlayEventType::PeriodStart),
        "Period started"
    );
    assert_eq!(
        format_period_event(&PlayEventType::PeriodEnd),
        "Period ended"
    );
    assert_eq!(
        format_period_event(&PlayEventType::GameStart),
        "Game started"
    );
    assert_eq!(format_period_event(&PlayEventType::GameEnd), "Game ended");
}

#[test]
fn test_format_period_event_non_boundary_type_is_empty() {
    assert_eq!(format_period_event(&PlayEventType::Hit), "");
}

// --- capitalize / capitalize_penalty ---

#[test]
fn test_capitalize_uppercases_first_letter_only() {
    assert_eq!(capitalize("wrist"), "Wrist");
    assert_eq!(capitalize("Wrist"), "Wrist");
}

#[test]
fn test_capitalize_empty_string() {
    assert_eq!(capitalize(""), "");
}

#[test]
fn test_capitalize_is_unicode_aware() {
    assert_eq!(capitalize("élan"), "Élan");
}

#[test]
fn test_capitalize_penalty_kebab_case() {
    assert_eq!(capitalize_penalty("cross-checking"), "Cross Checking");
}

#[test]
fn test_capitalize_penalty_single_word() {
    assert_eq!(capitalize_penalty("tripping"), "Tripping");
}

// --- get_player_name / format_player_name ---

#[test]
fn test_get_player_name_known_player_returns_last_name() {
    let pbp = sample_pbp();
    assert_eq!(get_player_name(&pbp, Some(8478483.into())), "Matthews");
}

#[test]
fn test_get_player_name_unknown_id_formats_as_hash_number() {
    let pbp = sample_pbp();
    assert_eq!(get_player_name(&pbp, Some(999999.into())), "#999999");
}

#[test]
fn test_get_player_name_none_returns_unknown() {
    let pbp = sample_pbp();
    assert_eq!(get_player_name(&pbp, None), "Unknown");
}

#[test]
fn test_format_player_name_uses_last_name_only() {
    let roster_spot = RosterSpot {
        team_id: 10.into(),
        player_id: 1.into(),
        first_name: LocalizedString {
            default: "Auston".to_string(),
        },
        last_name: LocalizedString {
            default: "Matthews".to_string(),
        },
        sweater_number: 34,
        position: None,
        headshot: String::new(),
    };
    assert_eq!(format_player_name(&roster_spot), "Matthews");
}

// --- format_assists_compact / format_assists_verbose ---

#[test]
fn test_format_assists_compact_with_two_assists() {
    let pbp = sample_pbp();
    let goal = play(&pbp, 105);
    let details = goal.details.as_ref().unwrap();
    assert_eq!(format_assists_compact(&pbp, details), "Marner, Nylander");
}

#[test]
fn test_format_assists_compact_with_no_assists_is_empty() {
    let pbp = sample_pbp();
    let mut goal = play(&pbp, 105);
    let mut details = goal.details.take().unwrap();
    details.assist1_player_id = None;
    details.assist2_player_id = None;
    assert_eq!(format_assists_compact(&pbp, &details), "");
}

#[test]
fn test_format_assists_verbose_includes_season_totals() {
    let pbp = sample_pbp();
    let goal = play(&pbp, 105);
    let details = goal.details.as_ref().unwrap();
    assert_eq!(
        format_assists_verbose(&pbp, details),
        "Marner (25), Nylander (18)"
    );
}

// --- format_situation_tag / format_situation_verbose ---

#[test]
fn test_format_situation_tag_even_strength_is_empty() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "1551");
    assert_eq!(format_situation_tag(&event), "");
}

#[test]
fn test_format_situation_tag_away_power_play() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "1541");
    assert_eq!(format_situation_tag(&event), " [PPG]");
}

#[test]
fn test_format_situation_tag_home_power_play() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "1451");
    assert_eq!(format_situation_tag(&event), " [PPG]");
}

#[test]
fn test_format_situation_tag_empty_net() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "1550");
    assert_eq!(format_situation_tag(&event), " [EN]");
}

#[test]
fn test_format_situation_tag_invalid_code_is_empty() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "??");
    assert_eq!(format_situation_tag(&event), "");
}

#[test]
fn test_format_situation_verbose_valid_code() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "1551");
    assert_eq!(
        format_situation_verbose(&event),
        Some("Situation: 5v5".to_string())
    );
}

#[test]
fn test_format_situation_verbose_invalid_code_is_none() {
    let pbp = sample_pbp();
    let event = with_situation_code(play(&pbp, 105), "invalid");
    assert_eq!(format_situation_verbose(&event), None);
}

// --- format_play_description_compact ---

#[test]
fn test_format_compact_period_boundary_with_no_details() {
    let pbp = sample_pbp();
    let event = play(&pbp, 100); // PeriodStart
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Period started"
    );
}

#[test]
fn test_format_compact_faceoff() {
    let pbp = sample_pbp();
    let event = play(&pbp, 101);
    // The fixture's faceoff loser id (8478469) doesn't match any roster spot
    // (Stutzle's real id is 8479469), so the unknown-player "#id" fallback applies.
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Matthews won vs #8478469 (N)"
    );
}

#[test]
fn test_format_compact_shot_on_goal_includes_shot_type() {
    let pbp = sample_pbp();
    let event = play(&pbp, 102);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Matthews - Wrist"
    );
}

#[test]
fn test_format_compact_blocked_shot() {
    let pbp = sample_pbp();
    let event = play(&pbp, 107);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Stutzle, blocked by Rielly"
    );
}

#[test]
fn test_format_compact_hit() {
    let pbp = sample_pbp();
    let event = play(&pbp, 103);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Chabot on Matthews"
    );
}

#[test]
fn test_format_compact_penalty_with_drawn_by() {
    let pbp = sample_pbp();
    let event = play(&pbp, 104);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Chabot - Tripping 2:00 (drawn by Matthews)"
    );
}

#[test]
fn test_format_compact_goal_with_assists_and_score() {
    let pbp = sample_pbp();
    let event = play(&pbp, 105);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Matthews (15) from Marner, Nylander [TOR 1 - OTT 0]"
    );
}

#[test]
fn test_format_compact_goal_with_no_assists_omits_from_clause() {
    let pbp = sample_pbp();
    let mut event = play(&pbp, 105);
    let mut details = event.details.take().unwrap();
    details.assist1_player_id = None;
    details.assist2_player_id = None;
    event.details = Some(details);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Matthews (15) [TOR 1 - OTT 0]"
    );
}

#[test]
fn test_format_compact_giveaway() {
    let pbp = sample_pbp();
    let event = play(&pbp, 110);
    assert_eq!(format_play_description_compact(&pbp, &event), "Stutzle (D)");
}

#[test]
fn test_format_compact_takeaway() {
    let pbp = sample_pbp();
    let event = play(&pbp, 109);
    assert_eq!(
        format_play_description_compact(&pbp, &event),
        "Matthews (N)"
    );
}

#[test]
fn test_format_compact_unmatched_type_with_details_is_empty() {
    let pbp = sample_pbp();
    let mut event = play(&pbp, 105);
    event.type_desc_key = PlayEventType::Stoppage;
    // Stoppage isn't handled explicitly and falls through to the catch-all branch.
    assert_eq!(format_play_description_compact(&pbp, &event), "");
}

// --- format_play_description_verbose ---

#[test]
fn test_format_verbose_period_boundary_has_no_detail_lines() {
    let pbp = sample_pbp();
    let event = play(&pbp, 100); // PeriodStart
    let (main, details) = format_play_description_verbose(&pbp, &event);
    assert_eq!(main, "Period started");
    assert!(details.is_empty());
}

#[test]
fn test_format_verbose_goal_includes_assists_score_situation_shot_and_location() {
    let pbp = sample_pbp();
    let event = play(&pbp, 105);
    let (main, details) = format_play_description_verbose(&pbp, &event);
    assert_eq!(main, "Matthews (15)");
    assert_eq!(
        details,
        vec![
            "Assists: Marner (25), Nylander (18)",
            "TOR 1 - OTT 0",
            "Situation: 5v5",
            "Shot: Slap",
            "Location: (80, 5)",
        ]
    );
}

#[test]
fn test_format_verbose_goal_with_no_assists_omits_assist_line() {
    let pbp = sample_pbp();
    let mut event = play(&pbp, 105);
    let mut details = event.details.take().unwrap();
    details.assist1_player_id = None;
    details.assist2_player_id = None;
    event.details = Some(details);
    let (_, detail_lines) = format_play_description_verbose(&pbp, &event);
    assert!(!detail_lines.iter().any(|l| l.starts_with("Assists:")));
}

#[test]
fn test_format_verbose_penalty_with_drawn_by() {
    let pbp = sample_pbp();
    let event = play(&pbp, 104);
    let (main, details) = format_play_description_verbose(&pbp, &event);
    assert_eq!(main, "Chabot - Tripping (2:00)");
    assert_eq!(details, vec!["Drawn by: Matthews"]);
}

#[test]
fn test_format_verbose_penalty_without_drawn_by_has_no_detail_lines() {
    let pbp = sample_pbp();
    let mut event = play(&pbp, 104);
    let mut details = event.details.take().unwrap();
    details.drawn_by_player_id = None;
    event.details = Some(details);
    let (_, detail_lines) = format_play_description_verbose(&pbp, &event);
    assert!(detail_lines.is_empty());
}

#[test]
fn test_format_verbose_falls_back_to_compact_for_other_event_types() {
    let pbp = sample_pbp();
    let event = play(&pbp, 103); // Hit
    let (main, details) = format_play_description_verbose(&pbp, &event);
    assert_eq!(main, format_play_description_compact(&pbp, &event));
    assert!(details.is_empty());
}

// Sanity check that the fixture's Faceoff details are wired the way these tests expect,
// so failures above point at formatting logic rather than an unexpectedly-changed
// fixture. Note: the fixture's `losing_player_id` (8478469) does not actually match
// Stutzle's roster id (8479469) — see `test_format_compact_faceoff`.
#[test]
fn test_fixture_faceoff_details_sanity_check() {
    let pbp = sample_pbp();
    let event = play(&pbp, 101);
    let details = event.details.as_ref().unwrap();
    assert_eq!(details.winning_player_id, Some(8478483.into()));
    assert_eq!(details.losing_player_id, Some(8478469.into()));
    assert_eq!(details.zone_code, Some(ZoneCode::Neutral));
}
