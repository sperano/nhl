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
#[path = "tail_tests.rs"]
mod tests;
