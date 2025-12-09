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
    Demo,
}

/// Document types for drill-down views (pushed onto document stack)
#[derive(Debug, Clone)]
pub enum StackedDocument {
    Boxscore { game_id: i64 },
    TeamDetail { abbrev: String },
    PlayerDetail { player_id: i64 },
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
            Self::Boxscore { game_id } => format!("Game {}", game_id),
            Self::TeamDetail { abbrev } => abbrev.clone(),
            Self::PlayerDetail { player_id } => format!("Player {}", player_id),
        }
    }
}
