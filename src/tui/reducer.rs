use tracing::debug;

use super::action::Action;
use super::component::Effect;
use super::component_store::ComponentStateStore;
use super::state::AppState;
use super::types::StackedDocument;

// Import sub-reducers from the parent framework module
use crate::tui::reducers::{
    reduce_data_loading, reduce_document_stack, reduce_navigation, reduce_settings,
    rebuild_standings_focusable_metadata,
};

/// Pure state reducer - like Redux reducer
///
/// Takes current state, component state store, and an action, returns new state and optional effect.
/// This function is PURE - no side effects, no I/O, no async (except for component state updates).
/// All side effects are returned as `Effect` to be executed separately.
///
/// Note: component_states is passed as &mut to allow ComponentMessage actions to update
/// component-local state, but the reducer itself doesn't create side effects through this.
///
/// Ownership is passed through the sub-reducer chain to avoid cloning:
/// - Each sub-reducer returns Ok((state, effect)) if it handled the action
/// - Or Err(state) to pass ownership back for the next reducer to try
pub fn reduce(
    state: AppState,
    action: Action,
    component_states: &mut ComponentStateStore,
) -> (AppState, Effect) {
    // Component message dispatch (React-like component system)
    if let Action::ComponentMessage { path, message } = &action {
        if let Some(component_state) = component_states.get_mut_any(path) {
            debug!("COMPONENT: Dispatching message to {}: {:?}", path, message);
            let effect = message.apply(component_state);
            return (state, effect);
        } else {
            debug!("COMPONENT: No state found for path: {}", path);
            return (state, Effect::None);
        }
    }

    // Chain sub-reducers, passing ownership through
    // Each returns Ok((state, effect)) if handled, Err(state) to continue

    // Navigation actions
    let state = match reduce_navigation(state, &action) {
        Ok(result) => return result,
        Err(state) => state,
    };

    // Document stack management actions
    let state = match reduce_document_stack(state, &action) {
        Ok(result) => return result,
        Err(state) => state,
    };

    // Data loading actions
    let state = match reduce_data_loading(state, &action, component_states) {
        Ok(result) => return result,
        Err(state) => state,
    };

    // Tab-specific action delegation
    match action {
        Action::SettingsAction(settings_action) => reduce_settings(state, settings_action, component_states),

        // Scores: SelectGame pushes boxscore document onto stack
        Action::SelectGame(game_id) => {
            reduce_document_stack(state, &Action::PushDocument(StackedDocument::Boxscore { game_id }))
                .unwrap_or_else(|s| (s, Effect::None))
        }

        // Standings: Rebuild focusable metadata after view change
        Action::RebuildStandingsFocusable => {
            rebuild_standings_focusable_metadata(&state, component_states);
            (state, Effect::None)
        }

        Action::SetStatusMessage { message, is_error } => {
            let mut new_state = state;
            if is_error {
                new_state.system.set_status_error_message(message);
            } else {
                new_state.system.set_status_message(message);
            }
            (new_state, Effect::None)
        }

        Action::UpdateTerminalWidth(width) => {
            let mut new_state = state;
            new_state.system.terminal_width = width;
            (new_state, Effect::None)
        }

        Action::Quit | Action::Error(_) => (state, Effect::None),

        _ => (state, Effect::None),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::action::SettingsAction;
    use crate::tui::types::{SettingsCategory, Tab};

    // Test helper that creates a ComponentStateStore for each test
    fn test_reduce(state: AppState, action: Action) -> (AppState, Effect) {
        let mut component_states = ComponentStateStore::new();
        reduce(state, action, &mut component_states)
    }

    #[test]
    fn test_navigation_actions_are_handled() {
        let state = AppState::default();
        let action = Action::NavigateTab(Tab::Settings);

        let (new_state, _) = test_reduce(state, action);

        assert_eq!(new_state.navigation.current_tab, Tab::Settings);
        assert!(new_state.navigation.document_stack.is_empty());
        assert!(!new_state.navigation.content_focused);
    }

    #[test]
    fn test_document_stack_actions_are_handled() {
        let state = AppState::default();
        let doc = super::super::types::StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
        };
        let action = Action::PushDocument(doc.clone());

        let (new_state, _) = test_reduce(state, action);

        assert_eq!(new_state.navigation.document_stack.len(), 1);
    }

    #[test]
    fn test_select_game_pushes_boxscore_document_and_fetches() {
        let state = AppState::default();
        let action = Action::SelectGame(12345);
        let (new_state, effect) = test_reduce(state, action);

        // Should push boxscore document onto stack
        assert_eq!(new_state.navigation.document_stack.len(), 1);
        match &new_state.navigation.document_stack[0].document {
            StackedDocument::Boxscore { game_id } => {
                assert_eq!(*game_id, 12345);
            }
            _ => panic!("Expected Boxscore document"),
        }
        // Should return fetch effect to load the boxscore data
        assert!(matches!(effect, Effect::FetchBoxscore(12345)));
    }

    #[test]
    fn test_rebuild_standings_focusable_returns_none() {
        let state = AppState::default();
        let action = Action::RebuildStandingsFocusable;
        let (new_state, effect) = test_reduce(state.clone(), action);

        // State should not be modified (focusable metadata is in component state)
        assert_eq!(new_state.data.standings, state.data.standings);
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_data_loading_actions_are_handled() {
        let state = AppState::default();
        let action = Action::RefreshData;

        let (new_state, _) = test_reduce(state, action);

        assert!(new_state.system.last_refresh.is_some());
    }

    #[test]
    fn test_quit_action_does_nothing_to_state() {
        let state = AppState::default();
        let action = Action::Quit;

        let (new_state, effect) = test_reduce(state.clone(), action);

        // State should remain unchanged
        assert_eq!(
            new_state.navigation.current_tab,
            state.navigation.current_tab
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_error_action_does_nothing_to_state() {
        let state = AppState::default();
        let action = Action::Error("test error".to_string());

        let (new_state, effect) = test_reduce(state.clone(), action);

        // State should remain unchanged
        assert_eq!(
            new_state.navigation.current_tab,
            state.navigation.current_tab
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let state = AppState::default();
        let action = Action::ToggleCommandPalette;

        let (new_state, effect) = test_reduce(state.clone(), action);

        // State should remain unchanged
        assert_eq!(
            new_state.navigation.current_tab,
            state.navigation.current_tab
        );
        assert!(matches!(effect, Effect::None));
    }

    // Settings reducer tests
    #[test]
    fn test_settings_navigate_category_left_from_logging() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Logging;

        let action = Action::SettingsAction(SettingsAction::NavigateCategoryLeft);
        let (new_state, _effect) = test_reduce(state, action);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Data
        );
    }

    #[test]
    fn test_settings_navigate_category_left_from_display() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Display;

        let action = Action::SettingsAction(SettingsAction::NavigateCategoryLeft);
        let (new_state, _) = test_reduce(state, action);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Logging
        );
    }

    #[test]
    fn test_settings_navigate_category_left_from_data() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Data;

        let action = Action::SettingsAction(SettingsAction::NavigateCategoryLeft);
        let (new_state, _) = test_reduce(state, action);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Display
        );
    }

    #[test]
    fn test_settings_navigate_category_right_from_logging() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Logging;

        let action = Action::SettingsAction(SettingsAction::NavigateCategoryRight);
        let (new_state, _effect) = test_reduce(state, action);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Display
        );
    }

    #[test]
    fn test_settings_navigate_category_right_from_display() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Display;

        let action = Action::SettingsAction(SettingsAction::NavigateCategoryRight);
        let (new_state, _) = test_reduce(state, action);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Data
        );
    }

    #[test]
    fn test_settings_navigate_category_right_from_data() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Data;

        let action = Action::SettingsAction(SettingsAction::NavigateCategoryRight);
        let (new_state, _) = test_reduce(state, action);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Logging
        );
    }

    #[test]
    fn test_set_status_message_with_error() {
        let state = AppState::default();
        let action = Action::SetStatusMessage {
            message: "Test error message".to_string(),
            is_error: true,
        };

        let (new_state, effect) = test_reduce(state, action);

        assert_eq!(
            new_state.system.status_message,
            Some("Test error message".to_string())
        );
        assert!(new_state.system.status_is_error);
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_set_status_message_without_error() {
        let state = AppState::default();
        let action = Action::SetStatusMessage {
            message: "Configuration saved".to_string(),
            is_error: false,
        };

        let (new_state, effect) = test_reduce(state, action);

        assert_eq!(
            new_state.system.status_message,
            Some("Configuration saved".to_string())
        );
        assert!(!new_state.system.status_is_error);
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_toggle_boolean_returns_save_effect() {
        let state = AppState::default();
        let action =
            Action::SettingsAction(SettingsAction::ToggleBoolean("use_unicode".to_string()));

        let (new_state, effect) = test_reduce(state.clone(), action);

        // Config should be toggled
        assert_eq!(
            new_state.system.config.display.use_unicode,
            !state.system.config.display.use_unicode
        );

        // Should return an Async effect (save_config_effect)
        assert!(matches!(effect, Effect::Async(_)));
    }

    #[test]
    fn test_toggle_boolean_use_unicode_updates_box_chars() {
        let mut state = AppState::default();
        state.system.config.display.use_unicode = true;

        let action =
            Action::SettingsAction(SettingsAction::ToggleBoolean("use_unicode".to_string()));

        let (new_state, _) = test_reduce(state, action);

        // Should toggle to false
        assert!(!new_state.system.config.display.use_unicode);
        // box_chars should be updated to ASCII
        assert_eq!(
            new_state.system.config.display.box_chars,
            crate::formatting::BoxChars::ascii()
        );
    }

    #[test]
    fn test_toggle_boolean_western_teams_first() {
        let state = AppState::default();
        let action = Action::SettingsAction(SettingsAction::ToggleBoolean(
            "western_teams_first".to_string(),
        ));

        let (new_state, _) = test_reduce(state.clone(), action);

        assert_eq!(
            new_state.system.config.display_standings_western_first,
            !state.system.config.display_standings_western_first
        );
    }

    #[test]
    fn test_toggle_boolean_unknown_setting() {
        let state = AppState::default();
        let action =
            Action::SettingsAction(SettingsAction::ToggleBoolean("unknown_setting".to_string()));

        let (new_state, _) = test_reduce(state.clone(), action);

        // State should not change for unknown settings
        assert_eq!(
            new_state.system.config.display.use_unicode,
            state.system.config.display.use_unicode
        );
        assert_eq!(
            new_state.system.config.display_standings_western_first,
            state.system.config.display_standings_western_first
        );
    }
}
