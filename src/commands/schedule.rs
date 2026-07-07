use crate::commands::{format_local_time, parse_game_date};
use crate::config;
use crate::data_provider::NHLDataProvider;
use crate::layout_constants::{SCHEDULE_BOX_CONTENT_WIDTH, SCHEDULE_BOX_TOTAL_WIDTH};
use anyhow::{Context, Result};
use nhl_api::DailySchedule;

pub fn format_schedule(schedule: &DailySchedule, time_format: &str) -> String {
    let mut output = String::new();

    // Display schedule header
    output.push_str(&format!("\nNHL Games - {}\n", schedule.date));
    output.push_str(&format!("{}\n\n", "═".repeat(80)));

    if schedule.number_of_games == 0 {
        output.push_str("No games scheduled for today.\n");
    } else {
        // Display each game in a box
        for (i, game) in schedule.games.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&format!(
                "┌{:─<width$}┐\n",
                "",
                width = SCHEDULE_BOX_TOTAL_WIDTH
            ));
            let team_line = format!("{} @ {}", game.away_team.abbrev, game.home_team.abbrev);
            output.push_str(&format!(
                "│ {:<width$} │\n",
                team_line,
                width = SCHEDULE_BOX_CONTENT_WIDTH
            ));
            let id_line = format!("Game ID: {}", game.id);
            output.push_str(&format!(
                "│ {:<width$} │\n",
                id_line,
                width = SCHEDULE_BOX_CONTENT_WIDTH
            ));
            output.push_str(&format!(
                "├{:─<width$}┤\n",
                "",
                width = SCHEDULE_BOX_TOTAL_WIDTH
            ));
            let status_line = format!("Status: {}", game.game_state);
            output.push_str(&format!(
                "│ {:<width$} │\n",
                status_line,
                width = SCHEDULE_BOX_CONTENT_WIDTH
            ));

            let time_display = format_local_time(&game.start_time_utc, time_format);
            let time_line = format!("Time: {}", time_display);
            output.push_str(&format!(
                "│ {:<width$} │\n",
                time_line,
                width = SCHEDULE_BOX_CONTENT_WIDTH
            ));
            if let (Some(away_score), Some(home_score)) =
                (game.away_team.score, game.home_team.score)
            {
                output.push_str(&format!(
                    "├{:─<width$}┤\n",
                    "",
                    width = SCHEDULE_BOX_TOTAL_WIDTH
                ));
                let left_side = format!("{:<23} {:>2}", game.away_team.abbrev, away_score);
                let right_side = format!("{:<2} {:>26}", home_score, game.home_team.abbrev);
                let score_line = format!("{}  -  {}", left_side, right_side);
                output.push_str(&format!("│ {} │\n", score_line));
            } else {
                output.push_str(&format!(
                    "│ {:<width$} │\n",
                    "Game not started",
                    width = SCHEDULE_BOX_CONTENT_WIDTH
                ));
            }
            output.push_str(&format!(
                "└{:─<width$}┘\n",
                "",
                width = SCHEDULE_BOX_TOTAL_WIDTH
            ));
        }
    }
    output
}

pub async fn run(client: &dyn NHLDataProvider, date: Option<String>) -> Result<()> {
    let game_date = parse_game_date(date)?;
    let config = config::read();
    let schedule = client
        .daily_schedule(Some(game_date))
        .await
        .context("Failed to fetch schedule")?;

    print!("{}", format_schedule(&schedule, &config.time_format));
    display_navigation(&schedule);
    Ok(())
}

fn display_navigation(schedule: &DailySchedule) {
    if let Some(prev) = &schedule.previous_start_date {
        println!("Previous date with games: {}", prev);
    }
    if let Some(next) = &schedule.next_start_date {
        println!("Next date with games: {}", next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nhl_api::{DailySchedule, GameState, ScheduleGame, ScheduleTeam};

    fn create_test_game(
        away_abbrev: &str,
        home_abbrev: &str,
        game_state: GameState,
        start_time_utc: &str,
        away_score: Option<i32>,
        home_score: Option<i32>,
    ) -> ScheduleGame {
        ScheduleGame {
            id: 2024020001.into(),
            game_type: nhl_api::GameType::RegularSeason,
            game_date: Some("2024-11-03".to_string()),
            start_time_utc: start_time_utc.to_string(),
            game_state,
            away_team: ScheduleTeam {
                id: 1.into(),
                abbrev: away_abbrev.to_string(),
                place_name: None,
                logo: "".to_string(),
                score: away_score,
            },
            home_team: ScheduleTeam {
                id: 2.into(),
                abbrev: home_abbrev.to_string(),
                place_name: None,
                logo: "".to_string(),
                score: home_score,
            },
        }
    }

    #[test]
    fn test_game_box_output() {
        let schedule = DailySchedule {
            date: "2024-11-03".to_string(),
            number_of_games: 1,
            previous_start_date: None,
            next_start_date: None,
            games: vec![create_test_game(
                "CHI",
                "SEA",
                GameState::Live,
                "2024-11-04T03:00:00Z",
                Some(0),
                Some(0),
            )],
        };

        let output = format_schedule(&schedule, "%I:%M %p");
        let lines: Vec<&str> = output.lines().skip(4).take(9).collect();
        assert_eq!(lines.len(), 9, "Should be 9 lines of output");
        assert_eq!(
            lines[0], "┌──────────────────────────────────────────────────────────────┐",
            "Top border line"
        );
        assert_eq!(
            lines[1], "│ CHI @ SEA                                                    │",
            "Team line"
        );
        assert_eq!(
            lines[2], "│ Game ID: 2024020001                                          │",
            "Game ID line"
        );
        assert_eq!(
            lines[3], "├──────────────────────────────────────────────────────────────┤",
            "Middle border line"
        );
        assert_eq!(
            lines[4], "│ Status: LIVE                                                 │",
            "Status line"
        );
        // Time varies by timezone, so just check the format
        assert!(
            lines[5].starts_with("│ Time: ") && lines[5].ends_with(" │"),
            "Time line should have correct format, got: {}",
            lines[5]
        );
        // Verify it contains a time pattern like "HH:MM AM/PM"
        assert!(
            lines[5].contains(":00 AM") || lines[5].contains(":00 PM"),
            "Time line should contain a time, got: {}",
            lines[5]
        );
        assert_eq!(
            lines[6], "├──────────────────────────────────────────────────────────────┤",
            "Score border line"
        );
        assert_eq!(
            lines[7], "│ CHI                      0  -  0                         SEA │",
            "Score line"
        );
        assert_eq!(
            lines[8], "└──────────────────────────────────────────────────────────────┘",
            "Bottom border line"
        );
    }

    #[test]
    fn test_game_box_output_respects_custom_time_format() {
        let schedule = DailySchedule {
            date: "2024-11-03".to_string(),
            number_of_games: 1,
            previous_start_date: None,
            next_start_date: None,
            games: vec![create_test_game(
                "CHI",
                "SEA",
                GameState::Live,
                "2024-11-04T03:00:00Z",
                Some(0),
                Some(0),
            )],
        };

        let output = format_schedule(&schedule, "%H:%M:%S");
        let time_line = output
            .lines()
            .find(|line| line.contains("Time:"))
            .expect("output should contain a Time line");

        // 24-hour format should never contain AM/PM markers
        assert!(!time_line.contains("AM") && !time_line.contains("PM"));
        // "Time: HH:MM:SS" has 3 colons total: one from the "Time:" label and two from HH:MM:SS
        assert_eq!(time_line.matches(':').count(), 3);
    }
}
