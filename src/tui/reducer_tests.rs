use super::*;
use crate::tui::action::SettingsAction;
use crate::tui::types::Tab;

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
    assert!(!new_state.navigation.focus_in_content);
}

#[test]
fn test_document_stack_actions_are_handled() {
    let state = AppState::default();
    let doc = super::super::types::StackedDocument::TeamDetail {
        abbrev: "BOS".to_string(),
        season: None,
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
        StackedDocument::Boxscore { game_id, .. } => {
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
fn test_tick_always_advances_animation_frame() {
    let mut state = AppState::default();
    state.system.animation_frame = 2;

    let (new_state, _) = test_reduce(state, Action::Tick);

    assert_eq!(new_state.system.animation_frame, 3);
}

#[test]
fn test_tick_wraps_animation_frame_at_four() {
    let mut state = AppState::default();
    state.system.animation_frame = 3;

    let (new_state, _) = test_reduce(state, Action::Tick);

    assert_eq!(new_state.system.animation_frame, 0);
}

#[test]
fn test_tick_does_not_refresh_before_interval_elapses() {
    let mut state = AppState::default();
    state.system.config.refresh_interval = 60;
    state.system.last_refresh = Some(SystemTime::now() - Duration::from_secs(5));
    let last_refresh_before = state.system.last_refresh;

    let (new_state, effect) = test_reduce(state, Action::Tick);

    assert!(matches!(effect, Effect::None));
    assert_eq!(new_state.system.last_refresh, last_refresh_before);
}

#[test]
fn test_tick_triggers_refresh_data_once_interval_elapses() {
    let mut state = AppState::default();
    state.system.config.refresh_interval = 30;
    state.system.last_refresh = Some(SystemTime::now() - Duration::from_secs(31));
    let last_refresh_before = state.system.last_refresh;

    let (new_state, effect) = test_reduce(state, Action::Tick);

    assert!(matches!(effect, Effect::Action(Action::RefreshData)));
    // The timestamp should be bumped immediately so back-to-back ticks
    // (before the RefreshData round-trip lands) don't re-trigger.
    assert!(new_state.system.last_refresh > last_refresh_before);
}

#[test]
fn test_tick_does_not_refresh_when_no_prior_refresh_recorded() {
    let mut state = AppState::default();
    state.system.config.refresh_interval = 30;
    state.system.last_refresh = None;

    let (new_state, effect) = test_reduce(state, Action::Tick);

    assert!(matches!(effect, Effect::None));
    assert!(new_state.system.last_refresh.is_none());
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
fn test_unknown_action_does_nothing() {
    let state = AppState::default();
    // FocusNext has no handler in any sub-reducer, so it falls through to the
    // final catch-all arm in `reduce`.
    let action = Action::FocusNext;

    let (new_state, effect) = test_reduce(state.clone(), action);

    // State should remain unchanged
    assert_eq!(
        new_state.navigation.current_tab,
        state.navigation.current_tab
    );
    assert!(matches!(effect, Effect::None));
}

// Settings reducer tests
//
// Category-navigation tests used to live here (against
// `Action::SettingsAction(NavigateCategoryLeft/Right)` and global
// `state.ui.settings.selected_category`) but moved to
// `components/settings_tab.rs`'s test module along with the behavior
// itself now that category selection is component-local.

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
    let action = Action::SettingsAction(SettingsAction::ToggleBoolean("use_unicode".to_string()));

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

    let action = Action::SettingsAction(SettingsAction::ToggleBoolean("use_unicode".to_string()));

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
