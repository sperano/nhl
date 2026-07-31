//! Tail command - stream play-by-play events like Unix tail
//!
//! Shows recent plays from a game with optional follow mode for live updates.

use crate::data_provider::NHLDataProvider;
use anyhow::{Context, Result};
use nhl_api::{PlayByPlay, PlayEvent, PlayEventType};
use std::collections::HashSet;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

#[path = "tail_format.rs"]
mod tail_format;
pub(crate) use tail_format::{
    colors, format_play_description_compact, format_play_description_verbose,
    get_event_color_and_label,
};
// Re-exported so tail_tests.rs (which does `use super::*;`) keeps seeing these
// names at their original `tail::` paths; unused outside test builds since
// tail.rs itself only calls into tail_format through the names re-exported above.
#[cfg(test)]
pub(crate) use tail_format::{
    capitalize, capitalize_penalty, format_assists_compact, format_assists_verbose,
    format_period_event, format_player_name, format_situation_tag, format_situation_verbose,
    get_player_name,
};
// `RosterSpot` isn't used by tail.rs itself, but tail_tests.rs constructs one
// directly; only nhl_api::PlayEventType and PlayByPlay/PlayEvent are needed here.
#[cfg(test)]
use nhl_api::RosterSpot;

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

#[cfg(test)]
#[path = "tail_tests.rs"]
mod tests;
