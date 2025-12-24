//! Core type definitions used across the framework
//!
//! This module contains fundamental types that are used throughout
//! the TUI framework, particularly for navigation and categorization.

/// Tab enum for main navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Scores,
    Standings,
    Settings,
    #[cfg(feature = "development")]
    Demo,
}

/// Document types for drill-down views (pushed onto document stack)
#[derive(Debug, Clone)]
pub enum StackedDocument {
    Boxscore {
        game_id: i64,
        away_abbrev: String,
        home_abbrev: String,
        away_score: i32,
        home_score: i32,
        /// Game date formatted for breadcrumb display (e.g., "12/24")
        game_date: String,
    },
    TeamDetail {
        abbrev: String,
    },
    PlayerDetail {
        player_id: i64,
        /// Player jersey number (e.g., 87)
        sweater_number: Option<i32>,
        /// Player last name (e.g., "Crosby")
        last_name: String,
    },
}

/// Settings category enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    #[default]
    Logging,
    Display,
    Data,
}

impl StackedDocument {
    /// Get the display label for this document (for breadcrumbs)
    pub fn label(&self) -> String {
        match self {
            Self::Boxscore {
                away_abbrev,
                home_abbrev,
                away_score,
                home_score,
                ..
            } => format!(
                "{}:{}-{}:{}",
                away_abbrev, away_score, home_abbrev, home_score
            ),
            Self::TeamDetail { abbrev } => abbrev.clone(),
            Self::PlayerDetail {
                sweater_number,
                last_name,
                ..
            } => {
                if let Some(num) = sweater_number {
                    format!("#{} {}", num, last_name)
                } else {
                    last_name.clone()
                }
            }
        }
    }
}
