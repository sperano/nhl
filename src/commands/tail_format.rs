//! Formatting helpers for the tail command: ANSI colors, event labels, and
//! play-description text (compact and verbose).

use nhl_api::{PlayByPlay, PlayEvent, PlayEventDetails, PlayEventType, PlayerId, RosterSpot};

/// ANSI color codes
pub(crate) mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const BLUE: &str = "\x1b[34m";
    pub const GRAY: &str = "\x1b[90m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BOLD: &str = "\x1b[1m";
}

/// Get color and label for an event type
pub(crate) fn get_event_color_and_label(
    event_type: &PlayEventType,
) -> (&'static str, &'static str) {
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
pub(crate) fn format_play_description_compact(pbp: &PlayByPlay, play: &PlayEvent) -> String {
    let details = match &play.details {
        Some(d) => d,
        None => return format_period_event(&play.type_desc_key),
    };

    match play.type_desc_key {
        PlayEventType::Goal => format_goal_compact(pbp, play, details),
        PlayEventType::Penalty => format_penalty_compact(pbp, details),
        PlayEventType::ShotOnGoal => format_shot_compact(pbp, details),
        PlayEventType::BlockedShot => format_blocked_shot_compact(pbp, details),
        PlayEventType::Hit => format_hit_compact(pbp, details),
        PlayEventType::Faceoff => format_faceoff_compact(pbp, details),
        PlayEventType::Giveaway | PlayEventType::Takeaway => {
            format_giveaway_takeaway_compact(pbp, details)
        }
        _ => String::new(),
    }
}

fn format_goal_compact(pbp: &PlayByPlay, play: &PlayEvent, details: &PlayEventDetails) -> String {
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

fn format_penalty_compact(pbp: &PlayByPlay, details: &PlayEventDetails) -> String {
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

fn format_shot_compact(pbp: &PlayByPlay, details: &PlayEventDetails) -> String {
    let shooter = get_player_name(pbp, details.shooting_player_id);
    let shot_type = details
        .shot_type
        .as_ref()
        .map(|s| format!(" - {}", capitalize(s)))
        .unwrap_or_default();

    format!("{}{}", shooter, shot_type)
}

fn format_blocked_shot_compact(pbp: &PlayByPlay, details: &PlayEventDetails) -> String {
    let shooter = get_player_name(pbp, details.shooting_player_id);
    let blocker = get_player_name(pbp, details.blocking_player_id);

    format!("{}, blocked by {}", shooter, blocker)
}

fn format_hit_compact(pbp: &PlayByPlay, details: &PlayEventDetails) -> String {
    let hitter = get_player_name(pbp, details.hitting_player_id);
    let hittee = get_player_name(pbp, details.hittee_player_id);

    format!("{} on {}", hitter, hittee)
}

fn format_faceoff_compact(pbp: &PlayByPlay, details: &PlayEventDetails) -> String {
    let winner = get_player_name(pbp, details.winning_player_id);
    let loser = get_player_name(pbp, details.losing_player_id);
    let zone = details
        .zone_code
        .as_ref()
        .map(|z| format!(" ({})", z))
        .unwrap_or_default();

    format!("{} won vs {}{}", winner, loser, zone)
}

fn format_giveaway_takeaway_compact(pbp: &PlayByPlay, details: &PlayEventDetails) -> String {
    let player = get_player_name(pbp, details.player_id);
    let zone = details
        .zone_code
        .as_ref()
        .map(|z| format!(" ({})", z))
        .unwrap_or_default();

    format!("{}{}", player, zone)
}

/// Format play description for verbose output, returns (main line, detail lines)
pub(crate) fn format_play_description_verbose(
    pbp: &PlayByPlay,
    play: &PlayEvent,
) -> (String, Vec<String>) {
    let details = match &play.details {
        Some(d) => d,
        None => return (format_period_event(&play.type_desc_key), vec![]),
    };

    match play.type_desc_key {
        PlayEventType::Goal => format_goal_verbose(pbp, play, details),
        PlayEventType::Penalty => format_penalty_verbose(pbp, details),
        _ => (format_play_description_compact(pbp, play), vec![]),
    }
}

fn format_goal_verbose(
    pbp: &PlayByPlay,
    play: &PlayEvent,
    details: &PlayEventDetails,
) -> (String, Vec<String>) {
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

fn format_penalty_verbose(pbp: &PlayByPlay, details: &PlayEventDetails) -> (String, Vec<String>) {
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

/// Format assists in compact form
pub(crate) fn format_assists_compact(
    pbp: &PlayByPlay,
    details: &nhl_api::PlayEventDetails,
) -> String {
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
pub(crate) fn format_assists_verbose(
    pbp: &PlayByPlay,
    details: &nhl_api::PlayEventDetails,
) -> String {
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
pub(crate) fn format_situation_tag(play: &PlayEvent) -> String {
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
pub(crate) fn format_situation_verbose(play: &PlayEvent) -> Option<String> {
    play.situation().map(|s| format!("Situation: {}", s))
}

/// Format period boundary events
pub(crate) fn format_period_event(event_type: &PlayEventType) -> String {
    match event_type {
        PlayEventType::PeriodStart => "Period started".to_string(),
        PlayEventType::PeriodEnd => "Period ended".to_string(),
        PlayEventType::GameStart => "Game started".to_string(),
        PlayEventType::GameEnd => "Game ended".to_string(),
        _ => String::new(),
    }
}

/// Get player name from roster
pub(crate) fn get_player_name(pbp: &PlayByPlay, player_id: Option<PlayerId>) -> String {
    match player_id {
        Some(id) => pbp
            .get_player(id)
            .map(format_player_name)
            .unwrap_or_else(|| format!("#{}", id)),
        None => "Unknown".to_string(),
    }
}

/// Format player name (Last name only for compact display)
pub(crate) fn format_player_name(player: &RosterSpot) -> String {
    player.last_name.default.clone()
}

/// Capitalize first letter
pub(crate) fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Capitalize penalty type (handle kebab-case)
pub(crate) fn capitalize_penalty(s: &str) -> String {
    s.split('-').map(capitalize).collect::<Vec<_>>().join(" ")
}
