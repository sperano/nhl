use crate::commands::parse_game_date;
use crate::config::Config;
use crate::data_provider::NHLDataProvider;
use crate::formatting::BoxChars;
use anyhow::{bail, Context, Result};
use chrono::Datelike;
use nhl_api::{GameDate, GameLog, GameState, GameType, PlayerSearchResult, SkaterStats};

/// Run the player-stats command
///
/// Searches for a player by name and displays their stats from today's game (or specified date).
pub async fn run(
    client: &dyn NHLDataProvider,
    player_name: &str,
    date: Option<String>,
    config: &Config,
) -> Result<()> {
    // Parse the date
    let game_date = parse_game_date(date)?;
    let date_str = game_date.to_api_string();
    let season = get_season_for_date(&game_date);

    // Search for the player
    let search_results = client
        .search_player(player_name, Some(10))
        .await
        .context("Failed to search for player")?;

    if search_results.is_empty() {
        bail!("No players found matching '{}'", player_name);
    }

    // Filter results to those whose name contains all search terms and have a team
    let matching = filter_matching_players(&search_results, player_name);

    if matching.is_empty() {
        bail!("No players found matching '{}'", player_name);
    }

    let player = match resolve_single_match(&matching, player_name) {
        Some(player) => player,
        None => std::process::exit(1),
    };
    let player_id = player.player_id;

    // Get the player's game log for the season
    let game_log = client
        .player_game_log(player_id, season, GameType::RegularSeason)
        .await
        .context("Failed to fetch player game log")?;

    // Find the game for the specified date
    let game_entry = game_log.game_log.iter().find(|g| g.game_date == date_str);

    let log_stats = match game_entry {
        Some(stats) => stats,
        None => {
            println!("No data for {} on {}", player.name, date_str);
            return Ok(());
        }
    };

    let boxscore_stats = fetch_boxscore_stats_for_player(client, &game_date, player).await?;

    print_player_game_stats(player, log_stats, boxscore_stats.as_ref(), config);

    Ok(())
}

/// If there is exactly one matching player, return it. If there are several,
/// print them (so the caller can be more specific) and return `None`.
///
/// Extracted from `run` to keep the ambiguous-match printing separate from
/// the main control flow.
fn resolve_single_match<'a>(
    matching: &[&'a PlayerSearchResult],
    player_name: &str,
) -> Option<&'a PlayerSearchResult> {
    if matching.len() > 1 {
        println!("Multiple players found matching '{}':", player_name);
        println!();
        for player in matching {
            let team = player.team_abbrev.as_deref().unwrap_or("N/A");
            let number = player
                .sweater_number
                .map(|n| format!("#{}", n))
                .unwrap_or_default();
            println!(
                "  {} {} - {} {}",
                player.name,
                number,
                team,
                player.position.map_or("N/A", |p| p.code())
            );
        }
        println!();
        println!("Please be more specific.");
        return None;
    }

    Some(matching[0])
}

/// Fetch the player's boxscore-derived stats (hits, blocks, FO%, etc.) for the
/// game on `game_date`, if their team has a game that has started.
///
/// Extracted from `run` to isolate the schedule/boxscore lookup chain.
async fn fetch_boxscore_stats_for_player(
    client: &dyn NHLDataProvider,
    game_date: &GameDate,
    player: &PlayerSearchResult,
) -> Result<Option<SkaterStats>> {
    let player_team = player.team_abbrev.as_deref().unwrap_or("");

    let schedule = client
        .daily_schedule(Some(game_date.clone()))
        .await
        .context("Failed to fetch schedule")?;

    let team_game = schedule
        .games
        .iter()
        .find(|g| g.away_team.abbrev == player_team || g.home_team.abbrev == player_team);

    let Some(game) = team_game else {
        return Ok(None);
    };

    // Only fetch boxscore if the game has started
    if game.game_state == GameState::Future || game.game_state == GameState::PreGame {
        return Ok(None);
    }

    let boxscore = client.boxscore(game.id).await.ok();
    Ok(boxscore.and_then(|bs| {
        let team_stats = if bs.away_team.abbrev == player_team {
            &bs.player_by_game_stats.away_team
        } else {
            &bs.player_by_game_stats.home_team
        };

        // Find player in forwards or defense
        team_stats
            .forwards
            .iter()
            .find(|s| s.player_id == player.player_id)
            .or_else(|| {
                team_stats
                    .defense
                    .iter()
                    .find(|s| s.player_id == player.player_id)
            })
            .cloned()
    }))
}

/// Filter player search results to those whose name contains every whitespace-separated
/// term in `player_name` (case-insensitive) and that have a known current team. Players
/// without a team (e.g. free agents) can't be matched to a game, so they're excluded.
fn filter_matching_players<'a>(
    search_results: &'a [PlayerSearchResult],
    player_name: &str,
) -> Vec<&'a PlayerSearchResult> {
    let search_lower = player_name.to_lowercase();
    let search_terms: Vec<&str> = search_lower.split_whitespace().collect();

    search_results
        .iter()
        .filter(|p| {
            let name_lower = p.name.to_lowercase();
            let matches_name = search_terms.iter().all(|term| name_lower.contains(term));
            let has_team = p.team_abbrev.is_some();
            matches_name && has_team
        })
        .collect()
}

/// Get the NHL season for a given date.
/// NHL seasons run from October to June, so:
/// - Oct-Dec of year Y = season YYYYYYYY+1 (e.g., Oct 2024 = 20242025)
/// - Jan-Sep of year Y = season YYYY-1YYYY (e.g., Apr 2025 = 20242025)
fn get_season_for_date(date: &GameDate) -> i32 {
    let naive_date = match date {
        GameDate::Now => chrono::Local::now().date_naive(),
        GameDate::Date(d) => *d,
    };

    let year = naive_date.year();
    let month = naive_date.month();

    if month >= 10 {
        // October-December: season starts this year
        year * 10000 + (year + 1)
    } else {
        // January-September: season started last year
        (year - 1) * 10000 + year
    }
}

fn print_player_game_stats(
    player: &PlayerSearchResult,
    log_stats: &GameLog,
    boxscore_stats: Option<&SkaterStats>,
    config: &Config,
) {
    print!(
        "{}",
        format_player_game_stats(player, log_stats, boxscore_stats, &config.display.box_chars)
    );
}

/// Build the player header box, matchup line, and stats table as a single string.
///
/// Extracted from `print_player_game_stats` so the formatting logic can be
/// unit-tested without capturing stdout.
fn format_player_game_stats(
    player: &PlayerSearchResult,
    log_stats: &GameLog,
    boxscore_stats: Option<&SkaterStats>,
    box_chars: &BoxChars,
) -> String {
    let box_width = 50;

    let team = player.team_abbrev.as_deref().unwrap_or("N/A");
    let number = player
        .sweater_number
        .map(|n| format!("#{}", n))
        .unwrap_or_default();
    let content = format!(
        "{} {} - {} {}",
        player.name,
        number,
        team,
        player.position.map_or("N/A", |p| p.code())
    );

    let mut output = String::new();

    // Player header
    output.push_str(&format!(
        "\n{}{}{}\n",
        box_chars.top_left,
        box_chars.horizontal.repeat(box_width),
        box_chars.top_right
    ));
    output.push_str(&format!(
        "{} {:<width$} {}\n",
        box_chars.vertical,
        content,
        box_chars.vertical,
        width = box_width - 2
    ));
    output.push_str(&format!(
        "{}{}{}\n",
        box_chars.bottom_left,
        box_chars.horizontal.repeat(box_width),
        box_chars.bottom_right
    ));

    // Game info
    let opponent = if log_stats.home_road_flag == nhl_api::HomeRoad::Home {
        format!("vs {}", log_stats.opponent_abbrev)
    } else {
        format!("@ {}", log_stats.opponent_abbrev)
    };
    output.push_str(&format!("\n{} - {}\n", log_stats.game_date, opponent));

    // Stats table
    output.push('\n');
    output.push_str(&format_skater_stats_table(
        log_stats,
        boxscore_stats,
        box_chars,
    ));

    output
}

/// Build the skater stats table (header row, separator, and value row) as a string.
///
/// Extracted from `print_skater_stats` so the two formatting branches (with and
/// without boxscore-derived stats) can be unit-tested without capturing stdout.
fn format_skater_stats_table(
    log: &GameLog,
    boxscore: Option<&SkaterStats>,
    box_chars: &BoxChars,
) -> String {
    let pim = log.pim.unwrap_or(0);
    let mut output = String::new();

    if let Some(bs) = boxscore {
        // Full stats with boxscore data
        output.push_str(&format!(
            "{:>2} {:>2} {:>2} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>4} {:>2} {:>2} {:>6}\n",
            "G", "A", "P", "+/-", "PIM", "SOG", "PPP", "HIT", "BLK", "FO%", "GV", "TK", "TOI"
        ));
        output.push_str(&format!("{}\n", box_chars.horizontal.repeat(52)));
        output.push_str(&format!(
            "{:>2} {:>2} {:>2} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>4.0} {:>2} {:>2} {:>6}\n",
            log.goals,
            log.assists,
            log.points,
            format_plus_minus(log.plus_minus),
            pim,
            log.shots,
            log.power_play_points,
            bs.hits,
            bs.blocked_shots,
            bs.faceoff_winning_pctg,
            bs.giveaways,
            bs.takeaways,
            log.toi
        ));
    } else {
        // Basic stats without boxscore
        output.push_str(&format!(
            "{:>3} {:>3} {:>3} {:>4} {:>4} {:>4} {:>4} {:>7}\n",
            "G", "A", "P", "+/-", "PIM", "SOG", "PPP", "TOI"
        ));
        output.push_str(&format!("{}\n", box_chars.horizontal.repeat(42)));
        output.push_str(&format!(
            "{:>3} {:>3} {:>3} {:>4} {:>4} {:>4} {:>4} {:>7}\n",
            log.goals,
            log.assists,
            log.points,
            format_plus_minus(log.plus_minus),
            pim,
            log.shots,
            log.power_play_points,
            log.toi
        ));
    }

    output
}

fn format_plus_minus(pm: i32) -> String {
    if pm > 0 {
        format!("+{}", pm)
    } else {
        pm.to_string()
    }
}

#[cfg(test)]
#[path = "player_stats_tests.rs"]
mod tests;
