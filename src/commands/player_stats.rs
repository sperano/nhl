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
    let search_lower = player_name.to_lowercase();
    let search_terms: Vec<&str> = search_lower.split_whitespace().collect();

    let matching: Vec<&PlayerSearchResult> = search_results
        .iter()
        .filter(|p| {
            let name_lower = p.name.to_lowercase();
            let matches_name = search_terms.iter().all(|term| name_lower.contains(term));
            let has_team = p.team_abbrev.is_some();
            matches_name && has_team
        })
        .collect();

    if matching.is_empty() {
        bail!("No players found matching '{}'", player_name);
    }

    // If multiple exact matches, print them and exit
    if matching.len() > 1 {
        println!("Multiple players found matching '{}':", player_name);
        println!();
        for player in &matching {
            let team = player.team_abbrev.as_deref().unwrap_or("N/A");
            let number = player
                .sweater_number
                .map(|n| format!("#{}", n))
                .unwrap_or_default();
            println!(
                "  {} {} - {} {}",
                player.name, number, team, player.position
            );
        }
        println!();
        println!("Please be more specific.");
        std::process::exit(1);
    }

    let player = matching[0];
    let player_id: i64 = player.player_id.parse().context("Invalid player ID")?;
    let player_team = player.team_abbrev.as_deref().unwrap_or("");

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

    // Now fetch the boxscore for additional stats (hits, blocks, FO%, etc.)
    // First, find the game from the schedule
    let schedule = client
        .daily_schedule(Some(game_date.clone()))
        .await
        .context("Failed to fetch schedule")?;

    let team_game = schedule
        .games
        .iter()
        .find(|g| g.away_team.abbrev == player_team || g.home_team.abbrev == player_team);

    let boxscore_stats = if let Some(game) = team_game {
        // Only fetch boxscore if game has started
        if game.game_state != GameState::Future && game.game_state != GameState::PreGame {
            let boxscore = client.boxscore(game.id).await.ok();
            boxscore.and_then(|bs| {
                let team_stats = if bs.away_team.abbrev == player_team {
                    &bs.player_by_game_stats.away_team
                } else {
                    &bs.player_by_game_stats.home_team
                };

                // Find player in forwards or defense
                team_stats
                    .forwards
                    .iter()
                    .find(|s| s.player_id == player_id)
                    .or_else(|| team_stats.defense.iter().find(|s| s.player_id == player_id))
                    .cloned()
            })
        } else {
            None
        }
    } else {
        None
    };

    print_player_game_stats(player, log_stats, boxscore_stats.as_ref(), config);

    Ok(())
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
    let box_chars = &config.display.box_chars;
    let box_width = 50;

    let team = player.team_abbrev.as_deref().unwrap_or("N/A");
    let number = player
        .sweater_number
        .map(|n| format!("#{}", n))
        .unwrap_or_default();
    let content = format!("{} {} - {} {}", player.name, number, team, player.position);

    // Player header
    println!(
        "\n{}{}{}",
        box_chars.top_left,
        box_chars.horizontal.repeat(box_width),
        box_chars.top_right
    );
    println!(
        "{} {:<width$} {}",
        box_chars.vertical,
        content,
        box_chars.vertical,
        width = box_width - 2
    );
    println!(
        "{}{}{}",
        box_chars.bottom_left,
        box_chars.horizontal.repeat(box_width),
        box_chars.bottom_right
    );

    // Game info
    let opponent = if log_stats.home_road_flag == nhl_api::HomeRoad::Home {
        format!("vs {}", log_stats.opponent_abbrev)
    } else {
        format!("@ {}", log_stats.opponent_abbrev)
    };
    println!("\n{} - {}", log_stats.game_date, opponent);

    // Stats table
    println!();
    print_skater_stats(log_stats, boxscore_stats, box_chars);
}

fn print_skater_stats(log: &GameLog, boxscore: Option<&SkaterStats>, box_chars: &BoxChars) {
    let pim = log.pim.unwrap_or(0);

    if let Some(bs) = boxscore {
        // Full stats with boxscore data
        println!(
            "{:>2} {:>2} {:>2} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>4} {:>2} {:>2} {:>6}",
            "G", "A", "P", "+/-", "PIM", "SOG", "PPP", "HIT", "BLK", "FO%", "GV", "TK", "TOI"
        );
        println!("{}", box_chars.horizontal.repeat(52));
        println!(
            "{:>2} {:>2} {:>2} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>4.0} {:>2} {:>2} {:>6}",
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
        );
    } else {
        // Basic stats without boxscore
        println!(
            "{:>3} {:>3} {:>3} {:>4} {:>4} {:>4} {:>4} {:>7}",
            "G", "A", "P", "+/-", "PIM", "SOG", "PPP", "TOI"
        );
        println!("{}", box_chars.horizontal.repeat(42));
        println!(
            "{:>3} {:>3} {:>3} {:>4} {:>4} {:>4} {:>4} {:>7}",
            log.goals,
            log.assists,
            log.points,
            format_plus_minus(log.plus_minus),
            pim,
            log.shots,
            log.power_play_points,
            log.toi
        );
    }
}

fn format_plus_minus(pm: i32) -> String {
    if pm > 0 {
        format!("+{}", pm)
    } else {
        pm.to_string()
    }
}
