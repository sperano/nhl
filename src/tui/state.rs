use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;

use nhl_api::{Boxscore, ClubStats, DailySchedule, GameDate, GameMatchup, PlayerLanding, Standing};

use crate::commands::scores_format::PeriodScores;
use crate::config::Config;

use super::document_nav::DocumentNavState;
use super::types::{SettingsCategory, StackedDocument, Tab};

/// Root application state - single source of truth
///
/// This is the entire application state in one place.
/// All state changes happen through the reducer.
/// Components receive slices of this state as props.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// Navigation state (which tab, panel stack)
    pub navigation: NavigationState,

    /// Application data (from API)
    pub data: DataState,

    /// UI state per tab
    pub ui: UiState,

    /// System state
    pub system: SystemState,
}

#[derive(Debug, Clone)]
pub struct NavigationState {
    pub current_tab: Tab,
    pub document_stack: Vec<DocumentStackEntry>,
    /// Whether focus is in content area (true) or on tab bar (false)
    pub focus_in_content: bool,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            current_tab: Tab::Scores,
            document_stack: Vec::new(),
            focus_in_content: false, // Start with tab bar focused
        }
    }
}

/// Default viewport height for stacked documents before terminal size is known
const DEFAULT_VIEWPORT_HEIGHT: u16 = 30;

/// Entry in the document stack
///
/// Each stacked document (boxscore, team detail, player detail) has its own
/// navigation state embedded via `DocumentNavState`.
#[derive(Debug, Clone)]
pub struct DocumentStackEntry {
    pub document: StackedDocument,
    /// Navigation state for this stacked document (focus, scroll, focusable metadata)
    pub nav: DocumentNavState,
}

impl DocumentStackEntry {
    /// Create a new document stack entry with default values
    pub fn new(document: StackedDocument) -> Self {
        Self {
            document,
            nav: DocumentNavState {
                focus_index: Some(0),
                viewport_height: DEFAULT_VIEWPORT_HEIGHT,
                ..Default::default()
            },
        }
    }

    /// Create a new document stack entry with a specific selected index
    pub fn with_selection(document: StackedDocument, selected_index: Option<usize>) -> Self {
        Self {
            document,
            nav: DocumentNavState {
                focus_index: selected_index,
                viewport_height: DEFAULT_VIEWPORT_HEIGHT,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DataState {
    // API data - wrapped in Arc to avoid deep clones on every reducer call
    pub standings: Arc<Option<Vec<Standing>>>,
    pub schedule: Arc<Option<DailySchedule>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,
    pub period_scores: Arc<HashMap<i64, PeriodScores>>,
    pub boxscores: Arc<HashMap<i64, Boxscore>>,
    pub team_roster_stats: Arc<HashMap<String, ClubStats>>,
    pub player_data: Arc<HashMap<i64, PlayerLanding>>,

    // Loading states
    pub loading: HashSet<LoadingKey>,

    // Errors
    pub errors: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoadingKey {
    Standings,
    Schedule(String), // GameDate formatted as string
    GameDetails(i64),
    Boxscore(i64),
    TeamRosterStats(String), // Team abbreviation
    PlayerStats(i64),
}

#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub scores: ScoresUiState,
    pub settings: SettingsUiState,
}

/// UI state for Scores tab (minimal - most state in component-local ScoresTabState)
///
/// `game_date` is kept in global state for the effects system (timer-based refreshes).
/// Component-local ScoresTabState manages all UI state (date navigation, game selection, browse mode).
///
/// Note: game_date is duplicated between global and component state by design:
/// - Global: What schedule data is loaded (for effects system)
/// - Component: What date UI is viewing (for rendering)
///
/// These are kept in sync via RefreshSchedule action.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoresUiState {
    pub game_date: GameDate,
}

impl Default for ScoresUiState {
    fn default() -> Self {
        Self {
            game_date: GameDate::today(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SettingsUiState {
    pub selected_category: SettingsCategory,
}

/// Default help message shown in the status bar
pub const DEFAULT_STATUS_MESSAGE: &str =
    "Keys: ←→ navigate | ↓ enter | ↑/ESC back | q quit | 1-6 jump to tab | / command palette";

#[derive(Debug, Clone, Default)]
pub struct SystemState {
    pub last_refresh: Option<SystemTime>,
    pub config: Config,
    pub status_message: Option<String>,
    pub status_is_error: bool,
    /// Cached terminal width for calculating game grid layout
    /// Updated during render, used by key handlers
    pub terminal_width: u16,
    /// Animation frame counter for loading animations (0-8, wraps)
    pub animation_frame: u8,
}

impl SystemState {
    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_is_error = false;
    }

    pub fn set_status_error_message(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_is_error = true;
    }

    pub fn reset_status_message(&mut self) {
        self.status_message = Some(DEFAULT_STATUS_MESSAGE.to_string());
        self.status_is_error = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_status_message() {
        let mut state = SystemState::default();

        state.set_status_message("Test message".to_string());

        assert_eq!(state.status_message, Some("Test message".to_string()));
        assert!(!state.status_is_error);
    }

    #[test]
    fn test_set_status_error_message() {
        let mut state = SystemState::default();

        state.set_status_error_message("Error message".to_string());

        assert_eq!(state.status_message, Some("Error message".to_string()));
        assert!(state.status_is_error);
    }

    #[test]
    fn test_set_status_message_overwrites_error_flag() {
        let mut state = SystemState::default();

        // First set an error
        state.set_status_error_message("Error".to_string());
        assert!(state.status_is_error);

        // Then set a normal message - should clear error flag
        state.set_status_message("Normal message".to_string());
        assert_eq!(state.status_message, Some("Normal message".to_string()));
        assert!(!state.status_is_error);
    }

    #[test]
    fn test_set_status_error_message_overwrites_normal_message() {
        let mut state = SystemState::default();

        // First set a normal message
        state.set_status_message("Normal".to_string());
        assert!(!state.status_is_error);

        // Then set an error - should set error flag
        state.set_status_error_message("Error message".to_string());
        assert_eq!(state.status_message, Some("Error message".to_string()));
        assert!(state.status_is_error);
    }

    #[test]
    fn test_reset_status_message() {
        let mut state = SystemState::default();

        // Set a custom message
        state.set_status_message("Custom message".to_string());
        assert_eq!(state.status_message, Some("Custom message".to_string()));

        // Reset should restore default
        state.reset_status_message();
        assert_eq!(
            state.status_message,
            Some(DEFAULT_STATUS_MESSAGE.to_string())
        );
        assert!(!state.status_is_error);
    }

    #[test]
    fn test_reset_status_message_clears_error_flag() {
        let mut state = SystemState::default();

        // Set an error message
        state.set_status_error_message("Error".to_string());
        assert!(state.status_is_error);

        // Reset should clear error flag
        state.reset_status_message();
        assert!(!state.status_is_error);
    }
}
