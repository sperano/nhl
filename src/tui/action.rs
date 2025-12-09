use crossterm::event::KeyEvent;
use nhl_api::{Boxscore, ClubStats, DailySchedule, GameDate, GameMatchup, PlayerLanding, Standing};
use std::any::Any;

use super::component::Effect;
use super::types::{StackedDocument, Tab};

/// Trait for type-erased component messages
///
/// This allows messages to be dispatched to components without knowing
/// their concrete type at the call site. The message carries its own
/// logic for updating component state.
pub trait ComponentMessageTrait: Send + Sync + std::fmt::Debug {
    /// Apply this message to a component state, returning an effect
    fn apply(&self, state: &mut dyn Any) -> Effect;

    /// Clone this message into a Box (for cloning Action enum)
    fn clone_box(&self) -> Box<dyn ComponentMessageTrait>;
}

/// Global actions - like Redux actions
///
/// All state changes in the application happen through actions.
/// Actions are dispatched from:
/// - User input (key events)
/// - Effects (async data loading)
/// - Middleware (logging, side effects)
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Action {
    // Navigation actions
    NavigateTab(Tab),
    NavigateTabLeft,
    NavigateTabRight,
    EnterContentFocus, // Down key: move focus from tab bar to content
    ExitContentFocus,  // Up key: move focus from content back to tab bar
    PushDocument(StackedDocument),
    PopDocument,
    ToggleCommandPalette,

    /// Unified "navigate up" action (ESC key)
    ///
    /// Hierarchical fallthrough:
    /// 1. If document stack not empty → pop document
    /// 2. Send NavigateUpMsg to current tab component
    /// 3. Component returns whether it handled it (closed modal, exited browse mode)
    /// 4. If not handled and content_focused → set content_focused = false
    NavigateUp,

    /// Route key events to stacked documents
    ///
    /// When a document is on the stack, key events are dispatched to the
    /// document's handle_key method for encapsulated navigation handling.
    StackedDocumentKey(KeyEvent),

    // Data actions
    // SelectTeam(String),
    // SelectPlayer(i64),
    RefreshData,
    RefreshSchedule(GameDate),  // Refresh schedule for specific date

    // Data loaded (from effects)
    StandingsLoaded(Result<Vec<Standing>, String>),
    ScheduleLoaded(Result<DailySchedule, String>),
    GameDetailsLoaded(i64, Result<GameMatchup, String>),
    BoxscoreLoaded(i64, Result<Boxscore, String>),
    TeamRosterStatsLoaded(String, Result<ClubStats, String>),
    PlayerStatsLoaded(i64, Result<PlayerLanding, String>),

    // UI actions
    FocusNext,
    FocusPrevious,

    // Component-specific actions
    SettingsAction(SettingsAction),

    // Scores tab actions that modify global state
    SelectGame(i64),

    // Standings tab actions that modify component state directly
    RebuildStandingsFocusable,

    /// Dispatch a message to a specific component
    ///
    /// This is part of the React-like component system refactor.
    /// Instead of global actions for every UI interaction, components
    /// can handle their own messages for local state updates.
    ComponentMessage {
        /// Component path (e.g., "app/scores_tab")
        path: String,
        /// Type-erased message to dispatch
        message: Box<dyn ComponentMessageTrait>,
    },

    // System actions
    Quit,
    Error(String),
    SetStatusMessage { message: String, is_error: bool },
    UpdateTerminalWidth(u16),
    /// Tick action for animation frame advancement
    Tick,
}

/// Tab-specific actions for Settings
#[derive(Debug, Clone)]
pub enum SettingsAction {
    NavigateCategoryLeft,
    NavigateCategoryRight,
    ToggleBoolean(String), // Setting key to toggle
    UpdateSetting { key: String, value: String }, // Update a setting value
    UpdateConfig(Box<crate::config::Config>),
}


impl Clone for Action {
    fn clone(&self) -> Self {
        match self {
            Self::NavigateTab(tab) => Self::NavigateTab(*tab),
            Self::NavigateTabLeft => Self::NavigateTabLeft,
            Self::NavigateTabRight => Self::NavigateTabRight,
            Self::EnterContentFocus => Self::EnterContentFocus,
            Self::ExitContentFocus => Self::ExitContentFocus,
            Self::PushDocument(doc) => Self::PushDocument(doc.clone()),
            Self::PopDocument => Self::PopDocument,
            Self::ToggleCommandPalette => Self::ToggleCommandPalette,
            Self::NavigateUp => Self::NavigateUp,
            Self::StackedDocumentKey(key) => Self::StackedDocumentKey(*key),
            Self::RefreshData => Self::RefreshData,
            Self::RefreshSchedule(date) => Self::RefreshSchedule(date.clone()),
            Self::StandingsLoaded(result) => Self::StandingsLoaded(result.clone()),
            Self::ScheduleLoaded(result) => Self::ScheduleLoaded(result.clone()),
            Self::GameDetailsLoaded(id, result) => Self::GameDetailsLoaded(*id, result.clone()),
            Self::BoxscoreLoaded(id, result) => Self::BoxscoreLoaded(*id, result.clone()),
            Self::TeamRosterStatsLoaded(abbrev, result) => {
                Self::TeamRosterStatsLoaded(abbrev.clone(), result.clone())
            }
            Self::PlayerStatsLoaded(id, result) => Self::PlayerStatsLoaded(*id, result.clone()),
            Self::FocusNext => Self::FocusNext,
            Self::FocusPrevious => Self::FocusPrevious,
            Self::SettingsAction(action) => Self::SettingsAction(action.clone()),
            Self::SelectGame(id) => Self::SelectGame(*id),
            Self::RebuildStandingsFocusable => Self::RebuildStandingsFocusable,
            Self::ComponentMessage { path, message } => Self::ComponentMessage {
                path: path.clone(),
                message: message.clone_box(),
            },
            Self::Quit => Self::Quit,
            Self::Error(msg) => Self::Error(msg.clone()),
            Self::SetStatusMessage { message, is_error } => Self::SetStatusMessage {
                message: message.clone(),
                is_error: *is_error,
            },
            Self::UpdateTerminalWidth(width) => Self::UpdateTerminalWidth(*width),
            Self::Tick => Self::Tick,
        }
    }
}

impl Action {
    /// Returns true if this action should trigger a re-render
    pub fn should_render(&self) -> bool {
        !matches!(self, Self::Error(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_render_returns_true_for_most_actions() {
        assert!(Action::NavigateTabLeft.should_render());
        assert!(Action::NavigateTabRight.should_render());
        assert!(Action::EnterContentFocus.should_render());
        assert!(Action::ExitContentFocus.should_render());
        assert!(Action::RefreshData.should_render());
        assert!(Action::Quit.should_render());
        assert!(Action::ToggleCommandPalette.should_render());
        assert!(Action::PopDocument.should_render());
        assert!(Action::FocusNext.should_render());
        assert!(Action::FocusPrevious.should_render());
        assert!(Action::NavigateUp.should_render());
    }

    #[test]
    fn test_should_render_returns_false_for_error_actions() {
        assert!(!Action::Error("test error".to_string()).should_render());
        assert!(!Action::Error("another error".to_string()).should_render());
        assert!(!Action::Error(String::new()).should_render());
    }
}
