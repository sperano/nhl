use crate::commands::parse_game_date;
use crate::config::{Config, DisplayConfig};
use crate::data_provider::NHLDataProvider;
use crate::formatting::format_header;
use crate::tui::helpers::StandingsSorting;
use anyhow::{Context, Result};
use nhl_api::Standing;
use std::collections::BTreeMap;

// Layout Constants
/// Width of standings table column (for two-column layout)
const STANDINGS_COLUMN_WIDTH: usize = 46;

/// Width of team name column
const TEAM_NAME_COL_WIDTH: usize = 25;

/// Width of games played column
const GP_COL_WIDTH: usize = 3;

/// Width of wins column
const W_COL_WIDTH: usize = 3;

/// Width of losses column
const L_COL_WIDTH: usize = 3;

/// Width of OT losses column
const OT_COL_WIDTH: usize = 3;

/// Width of points column
const PTS_COL_WIDTH: usize = 4;

/// Spacing between columns in two-column layout
const COLUMN_SPACING: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupBy {
    Division,
    Conference,
    League,
    Wildcard,
}

impl GroupBy {
    pub fn name(&self) -> &str {
        match self {
            Self::Division => "Division",
            Self::Conference => "Conference",
            Self::League => "League",
            Self::Wildcard => "Wildcard",
        }
    }

    pub fn all() -> [Self; 4] {
        [
            Self::Wildcard,
            Self::Division,
            Self::Conference,
            Self::League,
        ]
    }

    /// Get the next view in the cycle (Wildcard → Division → Conference → League → Wildcard)
    pub fn next(&self) -> Self {
        match self {
            Self::Wildcard => Self::Division,
            Self::Division => Self::Conference,
            Self::Conference => Self::League,
            Self::League => Self::Wildcard,
        }
    }

    /// Get the previous view in the cycle (Wildcard → League → Conference → Division → Wildcard)
    pub fn prev(&self) -> Self {
        match self {
            Self::Wildcard => Self::League,
            Self::Division => Self::Wildcard,
            Self::Conference => Self::Division,
            Self::League => Self::Conference,
        }
    }
}

pub fn format_standings_table(standings: &[Standing], display: &DisplayConfig) -> String {
    let mut output = String::new();

    // Print table header
    output.push_str(&format!(
        "{:<team_width$} {:>gp_width$} {:>w_width$} {:>l_width$} {:>ot_width$} {:>pts_width$}\n",
        "Team",
        "GP",
        "W",
        "L",
        "OT",
        "PTS",
        team_width = TEAM_NAME_COL_WIDTH,
        gp_width = GP_COL_WIDTH,
        w_width = W_COL_WIDTH,
        l_width = L_COL_WIDTH,
        ot_width = OT_COL_WIDTH,
        pts_width = PTS_COL_WIDTH
    ));
    output.push_str(&format!(
        "{}\n",
        display.box_chars.horizontal.repeat(STANDINGS_COLUMN_WIDTH)
    ));

    // Print each team's stats
    for standing in standings {
        let team_name = &standing.team_common_name.default;
        output.push_str(&format!(
            "{:<team_width$} {:>gp_width$} {:>w_width$} {:>l_width$} {:>ot_width$} {:>pts_width$}\n",
            team_name,
            standing.games_played(),
            standing.wins,
            standing.losses,
            standing.ot_losses,
            standing.points,
            team_width = TEAM_NAME_COL_WIDTH,
            gp_width = GP_COL_WIDTH,
            w_width = W_COL_WIDTH,
            l_width = L_COL_WIDTH,
            ot_width = OT_COL_WIDTH,
            pts_width = PTS_COL_WIDTH
        ));
    }

    output
}

fn format_group_with_header(
    name: &str,
    teams: &[Standing],
    display: &DisplayConfig,
) -> Vec<String> {
    let mut lines = Vec::new();
    let header = format_header(name, true, display);
    lines.extend(header.lines().map(|s| s.to_string()));
    lines.push(String::new()); // Empty line between header and table

    // Add table rows
    let table = format_standings_table(teams, display);
    lines.extend(table.lines().map(|s| s.to_string()));

    lines
}

/// Formats a column of divisions by stacking them vertically with blank lines between each division.
///
/// # Arguments
/// * `divisions` - A slice of tuples containing division name and team standings
///
/// # Returns
/// A vector of formatted strings representing the division column
fn format_division_column(
    divisions: &[(String, Vec<Standing>)],
    display: &DisplayConfig,
) -> Vec<String> {
    let mut lines = Vec::new();

    for (div_name, teams) in divisions {
        if !lines.is_empty() {
            lines.push(String::new()); // Add blank line between divisions
        }
        lines.extend(format_group_with_header(div_name, teams, display));
    }

    lines
}

fn merge_columns(left_lines: Vec<String>, right_lines: Vec<String>, column_width: usize) -> String {
    let mut output = String::new();
    let max_len = left_lines.len().max(right_lines.len());

    for i in 0..max_len {
        let left = left_lines.get(i).map(|s| s.as_str()).unwrap_or("");
        let right = right_lines.get(i).map(|s| s.as_str()).unwrap_or("");

        // Pad left column to column_width with spacing between columns
        output.push_str(&format!(
            "{:<width$}{}{}\n",
            left,
            " ".repeat(COLUMN_SPACING),
            right,
            width = column_width
        ));
    }

    output
}

/// Formats standings in division view with two-column layout
fn format_division_view(
    sorted_standings: Vec<Standing>,
    western_first: bool,
    display: &DisplayConfig,
) -> String {
    let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for standing in sorted_standings {
        grouped
            .entry(standing.division_name.clone())
            .or_default()
            .push(standing);
    }

    // Separate Eastern and Western divisions
    let mut eastern_divs = Vec::new();
    let mut western_divs = Vec::new();

    for (div_name, teams) in grouped {
        if div_name == "Atlantic" || div_name == "Metropolitan" {
            eastern_divs.push((div_name, teams));
        } else if div_name == "Central" || div_name == "Pacific" {
            western_divs.push((div_name, teams));
        }
    }

    // Sort divisions alphabetically within each conference
    eastern_divs.sort_by(|a, b| a.0.cmp(&b.0));
    western_divs.sort_by(|a, b| a.0.cmp(&b.0));

    // Build column 1 and column 2 based on western_first
    let (col1_divs, col2_divs) = if western_first {
        (western_divs, eastern_divs)
    } else {
        (eastern_divs, western_divs)
    };

    // Format each column as stacked divisions
    let col1_lines = format_division_column(&col1_divs, display);
    let col2_lines = format_division_column(&col2_divs, display);

    let mut output = String::new();
    output.push('\n');
    output.push_str(&merge_columns(
        col1_lines,
        col2_lines,
        STANDINGS_COLUMN_WIDTH,
    ));
    output
}

/// Formats standings in conference view with two-column layout
fn format_conference_view(
    sorted_standings: Vec<Standing>,
    western_first: bool,
    display: &DisplayConfig,
) -> String {
    let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for standing in sorted_standings {
        let conference = standing
            .conference_name
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
        grouped.entry(conference).or_default().push(standing);
    }

    // Convert to vec to split into columns
    let mut groups: Vec<_> = grouped.into_iter().collect();

    // If western_first is true, reverse to show Western first
    if western_first && groups.len() == 2 {
        // BTreeMap gives us Eastern, Western alphabetically
        // Reverse to get Western, Eastern
        groups.reverse();
    }

    let mut output = String::new();
    output.push('\n');

    if groups.len() == 2 {
        let left_lines = format_group_with_header(&groups[0].0, &groups[0].1, display);
        let right_lines = format_group_with_header(&groups[1].0, &groups[1].1, display);
        output.push_str(&merge_columns(
            left_lines,
            right_lines,
            STANDINGS_COLUMN_WIDTH,
        ));
    } else {
        // Fallback to single column if not exactly 2 conferences
        for (conference, teams) in groups {
            output.push_str(&format!("\n{}", format_header(&conference, true, display)));
            output.push_str(&format_standings_table(&teams, display));
        }
    }

    output
}

/// Formats standings in league-wide view (single column, sorted by points)
fn format_league_view(sorted_standings: Vec<Standing>, display: &DisplayConfig) -> String {
    let mut output = String::new();
    output.push('\n');
    output.push_str(&format_standings_table(&sorted_standings, display));
    output
}

/// Format a division's top-3 teams as a header block plus a trailing blank-line
/// separator. Returns an empty vec (no header, no separator) if the division has no
/// teams, matching the original "skip empty divisions entirely" behavior.
fn top3_division_lines(div_name: &str, teams: &[Standing], display: &DisplayConfig) -> Vec<String> {
    let top3: Vec<_> = teams.iter().take(3).cloned().collect();
    if top3.is_empty() {
        return Vec::new();
    }
    let mut lines = format_group_with_header(div_name, &top3, display);
    lines.push(String::new()); // Blank line after division
    lines
}

/// Insert the horizontal playoff-cutoff line after the 2nd wildcard team's row, if
/// there are at least two wildcard teams.
fn insert_playoff_cutoff_line(
    lines: &mut Vec<String>,
    wildcard_team_count: usize,
    display: &DisplayConfig,
) {
    if wildcard_team_count < 2 {
        return;
    }
    // Header has 3 lines (title, underline, blank) + table header (2 lines) + teams
    let cutoff_line_idx = 3 + 2 + 2; // After 2nd team row
    if lines.len() > cutoff_line_idx {
        lines.insert(
            cutoff_line_idx,
            display
                .box_chars
                .horizontal
                .repeat(STANDINGS_COLUMN_WIDTH)
                .to_string(),
        );
    }
}

/// Helper to format wildcard groups for a conference
fn format_wildcard_conference(
    div1_name: &str,
    div1_teams: &[Standing],
    div2_name: &str,
    div2_teams: &[Standing],
    display: &DisplayConfig,
) -> Vec<String> {
    let mut lines = Vec::new();

    lines.extend(top3_division_lines(div1_name, div1_teams, display));
    lines.extend(top3_division_lines(div2_name, div2_teams, display));

    // Remaining teams (wildcards and out of playoffs) - sorted by points
    let div1_remaining: Vec<_> = div1_teams.iter().skip(3).cloned().collect();
    let div2_remaining: Vec<_> = div2_teams.iter().skip(3).cloned().collect();

    let mut wildcard_teams: Vec<_> = div1_remaining.into_iter().chain(div2_remaining).collect();
    wildcard_teams.sort_by_points_desc();

    if !wildcard_teams.is_empty() {
        lines.extend(format_group_with_header(
            "Wildcard",
            &wildcard_teams,
            display,
        ));
        insert_playoff_cutoff_line(&mut lines, wildcard_teams.len(), display);
    }

    lines
}

/// Filter standings to one division and sort by points, descending.
fn division_teams_sorted(standings: &[Standing], division_name: &str) -> Vec<Standing> {
    let mut teams: Vec<_> = standings
        .iter()
        .filter(|s| s.division_name == division_name)
        .cloned()
        .collect();
    teams.sort_by_points_desc();
    teams
}

/// Formats standings in wildcard view with two-column layout
fn format_wildcard_view(
    sorted_standings: Vec<Standing>,
    western_first: bool,
    display: &DisplayConfig,
) -> String {
    // Group teams by division
    let atlantic = division_teams_sorted(&sorted_standings, "Atlantic");
    let metropolitan = division_teams_sorted(&sorted_standings, "Metropolitan");
    let central = division_teams_sorted(&sorted_standings, "Central");
    let pacific = division_teams_sorted(&sorted_standings, "Pacific");

    // Build Eastern Conference wildcard groups
    let eastern_lines = format_wildcard_conference(
        "Atlantic",
        &atlantic,
        "Metropolitan",
        &metropolitan,
        display,
    );

    // Build Western Conference wildcard groups
    let western_lines =
        format_wildcard_conference("Central", &central, "Pacific", &pacific, display);

    let (col1_lines, col2_lines) = if western_first {
        (western_lines, eastern_lines)
    } else {
        (eastern_lines, western_lines)
    };

    let mut output = String::new();
    output.push('\n');
    output.push_str(&merge_columns(
        col1_lines,
        col2_lines,
        STANDINGS_COLUMN_WIDTH,
    ));
    output
}

pub fn format_standings_by_group(
    standings: &[Standing],
    by: GroupBy,
    western_first: bool,
    display: &DisplayConfig,
) -> String {
    if standings.is_empty() {
        return "Loading standings...".to_string();
    }

    let mut sorted_standings = standings.to_vec();
    sorted_standings.sort_by_points_desc();

    match by {
        GroupBy::Division => format_division_view(sorted_standings, western_first, display),
        GroupBy::Conference => format_conference_view(sorted_standings, western_first, display),
        GroupBy::League => format_league_view(sorted_standings, display),
        GroupBy::Wildcard => format_wildcard_view(sorted_standings, western_first, display),
    }
}

pub async fn run(
    client: &dyn NHLDataProvider,
    season: Option<i64>,
    date: Option<String>,
    by: GroupBy,
    config: &Config,
) -> Result<()> {
    let standings = if date.is_some() {
        // Parse date string and get standings for that date
        let game_date = parse_game_date(date)?;
        client
            .league_standings_for_date(&game_date)
            .await
            .context("Failed to fetch standings for date")?
    } else if let Some(season_year) = season {
        // Get standings for specific season
        client
            .league_standings_for_season(season_year)
            .await
            .context("Failed to fetch standings for season")?
    } else {
        // Get current standings
        client
            .current_league_standings()
            .await
            .context("Failed to fetch current standings")?
    };

    // Use the shared formatting function, honoring the user's configured column order
    let output = format_standings_by_group(
        &standings,
        by,
        config.display_standings_western_first,
        &config.display,
    );
    print!("{}", output);

    Ok(())
}

#[cfg(test)]
#[path = "standings_tests.rs"]
mod tests;
