//! Tail command - stream play-by-play events like Unix tail
//!
//! Shows recent plays from a game with optional follow mode for live updates.

use crate::data_provider::NHLDataProvider;
use anyhow::{Context, Result};
use nhl_api::{PlayByPlay, PlayEvent, PlayEventType, PlayerId, RosterSpot};
use std::collections::HashSet;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

/// ANSI color codes
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const BLUE: &str = "\x1b[34m";
    pub const GRAY: &str = "\x1b[90m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BOLD: &str = "\x1b[1m";
}

/// Filter options for which events to show
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub goals: bool,
    pub penalties: bool,
    pub shots: bool,
}

impl EventFilter {
    /// Returns true if no specific filters are set (show all)
    pub fn show_all(&self) -> bool {
        !self.goals && !self.penalties && !self.shots
    }

    /// Check if an event should be shown based on filters
    pub fn should_show(&self, event_type: &PlayEventType) -> bool {
        if self.show_all() {
            return true;
        }

        match event_type {
            PlayEventType::Goal => self.goals || self.shots,
            PlayEventType::Penalty => self.penalties,
            PlayEventType::ShotOnGoal | PlayEventType::MissedShot | PlayEventType::BlockedShot => {
                self.shots
            }
            _ => false,
        }
    }
}

/// Run the tail command (one-shot mode)
pub async fn run(
    client: &dyn NHLDataProvider,
    game_id: i64,
    count: usize,
    filter: &EventFilter,
    verbose: bool,
) -> Result<()> {
    let pbp = client
        .play_by_play(game_id.into())
        .await
        .context("Failed to fetch play-by-play data")?;

    print_header(&pbp);

    let plays = get_filtered_plays(&pbp, filter);
    let recent: Vec<_> = plays.into_iter().rev().take(count).collect();

    // Print in chronological order (oldest first)
    for play in recent.into_iter().rev() {
        print_play(&pbp, play, verbose);
    }

    Ok(())
}

/// Run the tail command in follow mode
pub async fn follow(
    client: &dyn NHLDataProvider,
    game_id: i64,
    count: usize,
    interval_secs: u64,
    filter: &EventFilter,
    verbose: bool,
) -> Result<()> {
    let mut seen_event_ids: HashSet<i64> = HashSet::new();
    let mut first_fetch = true;

    loop {
        let pbp = client
            .play_by_play(game_id.into())
            .await
            .context("Failed to fetch play-by-play data")?;

        if first_fetch {
            print_header(&pbp);

            // Show initial plays
            let plays = get_filtered_plays(&pbp, filter);
            let recent: Vec<_> = plays.into_iter().rev().take(count).collect();

            for play in recent.into_iter().rev() {
                seen_event_ids.insert(play.event_id);
                print_play(&pbp, play, verbose);
            }

            first_fetch = false;
        } else {
            // Show only new plays
            let plays = get_filtered_plays(&pbp, filter);
            let mut new_plays: Vec<_> = plays
                .into_iter()
                .filter(|p| !seen_event_ids.contains(&p.event_id))
                .collect();

            // Sort by event_id to show in order
            new_plays.sort_by_key(|p| p.event_id);

            for play in &new_plays {
                seen_event_ids.insert(play.event_id);
                print_play(&pbp, play, verbose);
            }
        }

        // Flush stdout to ensure output appears immediately
        io::stdout().flush().ok();

        // Check if game is over
        if pbp.game_state.is_final() {
            println!(
                "\n{}Game ended: {} {} - {} {}{}",
                colors::BOLD,
                pbp.away_team.abbrev,
                pbp.away_team.score,
                pbp.home_team.abbrev,
                pbp.home_team.score,
                colors::RESET
            );
            break;
        }

        sleep(Duration::from_secs(interval_secs)).await;
    }

    Ok(())
}

/// Print the game header
fn print_header(pbp: &PlayByPlay) {
    let status = if pbp.game_state.is_live() {
        format!(
            "P{} {}",
            pbp.period_descriptor.number, pbp.clock.time_remaining
        )
    } else if pbp.game_state.is_final() {
        "Final".to_string()
    } else if pbp.game_state.is_scheduled() {
        "Scheduled".to_string()
    } else {
        pbp.game_state.to_string()
    };

    println!(
        "{}{} @ {} | {} | {} {} - {} {}{}",
        colors::BOLD,
        pbp.away_team.abbrev,
        pbp.home_team.abbrev,
        status,
        pbp.away_team.abbrev,
        pbp.away_team.score,
        pbp.home_team.abbrev,
        pbp.home_team.score,
        colors::RESET
    );
    println!();
}

/// Get plays filtered by the event filter
fn get_filtered_plays<'a>(pbp: &'a PlayByPlay, filter: &EventFilter) -> Vec<&'a PlayEvent> {
    pbp.plays
        .iter()
        .filter(|p| filter.should_show(&p.type_desc_key))
        .collect()
}

/// Print a single play event
fn print_play(pbp: &PlayByPlay, play: &PlayEvent, verbose: bool) {
    if verbose {
        print_play_verbose(pbp, play);
    } else {
        print_play_compact(pbp, play);
    }
}

/// Print play in compact format (single line)
fn print_play_compact(pbp: &PlayByPlay, play: &PlayEvent) {
    let (color, event_label) = get_event_color_and_label(&play.type_desc_key);
    let period = format!("P{}", play.period_descriptor.number);
    let time = &play.time_in_period;

    let description = format_play_description_compact(pbp, play);

    println!(
        "{}{} {:>5}  {:<10}{} {}",
        color,
        period,
        time,
        event_label,
        colors::RESET,
        description
    );
}

/// Print play in verbose format (multi-line with details)
fn print_play_verbose(pbp: &PlayByPlay, play: &PlayEvent) {
    let (color, event_label) = get_event_color_and_label(&play.type_desc_key);
    let period = format!("P{}", play.period_descriptor.number);
    let time = &play.time_in_period;

    let (main_line, detail_lines) = format_play_description_verbose(pbp, play);

    println!(
        "{}{} {:>5}  {:<10}{} {}",
        color,
        period,
        time,
        event_label,
        colors::RESET,
        main_line
    );

    for detail in detail_lines {
        println!(
            "                    {}{}{}",
            colors::GRAY,
            detail,
            colors::RESET
        );
    }
}

/// Get color and label for an event type
fn get_event_color_and_label(event_type: &PlayEventType) -> (&'static str, &'static str) {
    match event_type {
        PlayEventType::Goal => (colors::GREEN, "GOAL"),
        PlayEventType::Penalty => (colors::RED, "PENALTY"),
        PlayEventType::ShotOnGoal => (colors::BLUE, "SHOT"),
        PlayEventType::MissedShot => (colors::GRAY, "MISS"),
        PlayEventType::BlockedShot => (colors::GRAY, "BLOCKED"),
        PlayEventType::Hit => (colors::GRAY, "HIT"),
        PlayEventType::Faceoff => (colors::GRAY, "FACEOFF"),
        PlayEventType::Giveaway => (colors::YELLOW, "GIVEAWAY"),
        PlayEventType::Takeaway => (colors::YELLOW, "TAKEAWAY"),
        PlayEventType::PeriodStart => (colors::GRAY, "PERIOD"),
        PlayEventType::PeriodEnd => (colors::GRAY, "PERIOD"),
        PlayEventType::GameStart => (colors::GRAY, "GAME"),
        PlayEventType::GameEnd => (colors::GRAY, "GAME"),
        PlayEventType::Stoppage => (colors::GRAY, "STOPPAGE"),
        _ => (colors::GRAY, "EVENT"),
    }
}

/// Format play description for compact output
fn format_play_description_compact(pbp: &PlayByPlay, play: &PlayEvent) -> String {
    let details = match &play.details {
        Some(d) => d,
        None => return format_period_event(&play.type_desc_key),
    };

    match play.type_desc_key {
        PlayEventType::Goal => {
            let scorer = get_player_name(pbp, details.scoring_player_id);
            let goal_num = details.scoring_player_total.unwrap_or(0);
            let assists = format_assists_compact(pbp, details);
            let score = format!(
                "{} {} - {} {}",
                pbp.away_team.abbrev,
                details.away_score.unwrap_or(0),
                pbp.home_team.abbrev,
                details.home_score.unwrap_or(0)
            );
            let situation = format_situation_tag(play);

            if assists.is_empty() {
                format!("{} ({}) [{}]{}", scorer, goal_num, score, situation)
            } else {
                format!(
                    "{} ({}) from {} [{}]{}",
                    scorer, goal_num, assists, score, situation
                )
            }
        }
        PlayEventType::Penalty => {
            let player = get_player_name(pbp, details.committed_by_player_id);
            let penalty_type = details
                .desc_key
                .as_ref()
                .map(|s| capitalize_penalty(s))
                .unwrap_or_else(|| "Penalty".to_string());
            let duration = details.duration.unwrap_or(2);
            let drawn_by = details
                .drawn_by_player_id
                .map(|id| format!(" (drawn by {})", get_player_name(pbp, Some(id))))
                .unwrap_or_default();

            format!("{} - {} {}:00{}", player, penalty_type, duration, drawn_by)
        }
        PlayEventType::ShotOnGoal => {
            let shooter = get_player_name(pbp, details.shooting_player_id);
            let shot_type = details
                .shot_type
                .as_ref()
                .map(|s| format!(" - {}", capitalize(s)))
                .unwrap_or_default();

            format!("{}{}", shooter, shot_type)
        }
        PlayEventType::BlockedShot => {
            let shooter = get_player_name(pbp, details.shooting_player_id);
            let blocker = get_player_name(pbp, details.blocking_player_id);

            format!("{}, blocked by {}", shooter, blocker)
        }
        PlayEventType::Hit => {
            let hitter = get_player_name(pbp, details.hitting_player_id);
            let hittee = get_player_name(pbp, details.hittee_player_id);

            format!("{} on {}", hitter, hittee)
        }
        PlayEventType::Faceoff => {
            let winner = get_player_name(pbp, details.winning_player_id);
            let loser = get_player_name(pbp, details.losing_player_id);
            let zone = details
                .zone_code
                .as_ref()
                .map(|z| format!(" ({})", z))
                .unwrap_or_default();

            format!("{} won vs {}{}", winner, loser, zone)
        }
        PlayEventType::Giveaway | PlayEventType::Takeaway => {
            let player = get_player_name(pbp, details.player_id);
            let zone = details
                .zone_code
                .as_ref()
                .map(|z| format!(" ({})", z))
                .unwrap_or_default();

            format!("{}{}", player, zone)
        }
        _ => String::new(),
    }
}

/// Format play description for verbose output, returns (main line, detail lines)
fn format_play_description_verbose(pbp: &PlayByPlay, play: &PlayEvent) -> (String, Vec<String>) {
    let details = match &play.details {
        Some(d) => d,
        None => return (format_period_event(&play.type_desc_key), vec![]),
    };

    match play.type_desc_key {
        PlayEventType::Goal => {
            let scorer = get_player_name(pbp, details.scoring_player_id);
            let goal_num = details.scoring_player_total.unwrap_or(0);
            let assists = format_assists_verbose(pbp, details);

            let main = format!("{} ({})", scorer, goal_num);

            let mut detail_lines = vec![];

            if !assists.is_empty() {
                detail_lines.push(format!("Assists: {}", assists));
            }

            let score = format!(
                "{} {} - {} {}",
                pbp.away_team.abbrev,
                details.away_score.unwrap_or(0),
                pbp.home_team.abbrev,
                details.home_score.unwrap_or(0)
            );
            detail_lines.push(score);

            if let Some(situation) = format_situation_verbose(play) {
                detail_lines.push(situation);
            }

            if let Some(shot_type) = &details.shot_type {
                detail_lines.push(format!("Shot: {}", capitalize(shot_type)));
            }

            if let (Some(x), Some(y)) = (details.x_coord, details.y_coord) {
                detail_lines.push(format!("Location: ({}, {})", x, y));
            }

            (main, detail_lines)
        }
        PlayEventType::Penalty => {
            let player = get_player_name(pbp, details.committed_by_player_id);
            let penalty_type = details
                .desc_key
                .as_ref()
                .map(|s| capitalize_penalty(s))
                .unwrap_or_else(|| "Penalty".to_string());
            let duration = details.duration.unwrap_or(2);

            let main = format!("{} - {} ({}:00)", player, penalty_type, duration);
            let mut detail_lines = vec![];

            if let Some(drawn_by_id) = details.drawn_by_player_id {
                detail_lines.push(format!(
                    "Drawn by: {}",
                    get_player_name(pbp, Some(drawn_by_id))
                ));
            }

            (main, detail_lines)
        }
        _ => (format_play_description_compact(pbp, play), vec![]),
    }
}

/// Format assists in compact form
fn format_assists_compact(pbp: &PlayByPlay, details: &nhl_api::PlayEventDetails) -> String {
    let mut assists = vec![];

    if let Some(id) = details.assist1_player_id {
        assists.push(get_player_name(pbp, Some(id)));
    }
    if let Some(id) = details.assist2_player_id {
        assists.push(get_player_name(pbp, Some(id)));
    }

    assists.join(", ")
}

/// Format assists in verbose form (with totals)
fn format_assists_verbose(pbp: &PlayByPlay, details: &nhl_api::PlayEventDetails) -> String {
    let mut assists = vec![];

    if let Some(id) = details.assist1_player_id {
        let name = get_player_name(pbp, Some(id));
        let total = details.assist1_player_total.unwrap_or(0);
        assists.push(format!("{} ({})", name, total));
    }
    if let Some(id) = details.assist2_player_id {
        let name = get_player_name(pbp, Some(id));
        let total = details.assist2_player_total.unwrap_or(0);
        assists.push(format!("{} ({})", name, total));
    }

    assists.join(", ")
}

/// Format situation tag (PPG, SHG, EN)
fn format_situation_tag(play: &PlayEvent) -> String {
    if let Some(situation) = play.situation() {
        if situation.is_away_power_play() || situation.is_home_power_play() {
            return " [PPG]".to_string();
        }
        if situation.is_empty_net() {
            return " [EN]".to_string();
        }
    }
    String::new()
}

/// Format situation in verbose mode
fn format_situation_verbose(play: &PlayEvent) -> Option<String> {
    play.situation().map(|s| format!("Situation: {}", s))
}

/// Format period boundary events
fn format_period_event(event_type: &PlayEventType) -> String {
    match event_type {
        PlayEventType::PeriodStart => "Period started".to_string(),
        PlayEventType::PeriodEnd => "Period ended".to_string(),
        PlayEventType::GameStart => "Game started".to_string(),
        PlayEventType::GameEnd => "Game ended".to_string(),
        _ => String::new(),
    }
}

/// Get player name from roster
fn get_player_name(pbp: &PlayByPlay, player_id: Option<PlayerId>) -> String {
    match player_id {
        Some(id) => pbp
            .get_player(id)
            .map(format_player_name)
            .unwrap_or_else(|| format!("#{}", id)),
        None => "Unknown".to_string(),
    }
}

/// Format player name (Last name only for compact display)
fn format_player_name(player: &RosterSpot) -> String {
    player.last_name.default.clone()
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Capitalize penalty type (handle kebab-case)
fn capitalize_penalty(s: &str) -> String {
    s.split('-').map(capitalize).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
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
}
