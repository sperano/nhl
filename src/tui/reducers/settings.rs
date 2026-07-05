use tracing::debug;

use crate::config::Config;
use crate::tui::action::{Action, SettingsAction};
use crate::tui::component::Effect;
use crate::tui::component_store::ComponentStateStore;
use crate::tui::constants::SETTINGS_TAB_PATH;
use crate::tui::state::AppState;
use crate::tui::types::SettingsCategory;

pub fn reduce_settings(
    state: AppState,
    action: SettingsAction,
    component_states: &mut ComponentStateStore,
) -> (AppState, Effect) {
    match action {
        SettingsAction::NavigateCategoryLeft => {
            let new_category = match state.ui.settings.selected_category {
                SettingsCategory::Logging => SettingsCategory::Data,
                SettingsCategory::Display => SettingsCategory::Logging,
                SettingsCategory::Data => SettingsCategory::Display,
            };
            navigate_category(state, component_states, new_category)
        }

        SettingsAction::NavigateCategoryRight => {
            let new_category = match state.ui.settings.selected_category {
                SettingsCategory::Logging => SettingsCategory::Display,
                SettingsCategory::Display => SettingsCategory::Data,
                SettingsCategory::Data => SettingsCategory::Logging,
            };
            navigate_category(state, component_states, new_category)
        }

        SettingsAction::ToggleBoolean(key) => {
            debug!("SETTINGS: Toggling boolean setting: {}", key);
            let mut new_state = state;
            match key.as_str() {
                "use_unicode" => {
                    new_state.system.config.display.use_unicode =
                        !new_state.system.config.display.use_unicode;
                    new_state.system.config.display.box_chars =
                        crate::formatting::BoxChars::from_use_unicode(
                            new_state.system.config.display.use_unicode,
                        );
                }
                "western_teams_first" => {
                    new_state.system.config.display_standings_western_first =
                        !new_state.system.config.display_standings_western_first;
                    // Rebuild standings focusable metadata so team selection uses the new order
                    let config = new_state.system.config.clone();
                    let save_effect = save_config_effect(config);
                    let rebuild_effect = Effect::Action(Action::RebuildStandingsFocusable);
                    return (new_state, Effect::Batch(vec![save_effect, rebuild_effect]));
                }
                _ => {
                    debug!("SETTINGS: Unknown boolean setting: {}", key);
                }
            }
            let config = new_state.system.config.clone();
            let effect = save_config_effect(config);
            (new_state, effect)
        }

        SettingsAction::UpdateSetting { key, value } => {
            debug!("SETTINGS: Updating setting: {} = {}", key, value);
            let mut new_state = state;
            match key.as_str() {
                "log_level" => {
                    new_state.system.config.log_level = value;
                }
                "theme" => {
                    if value == "none" {
                        new_state.system.config.display.theme_name = None;
                        new_state.system.config.display.theme = None;
                    } else {
                        use crate::config::THEMES;
                        let theme = THEMES.get(value.as_str()).map(|t| (*t).clone());
                        new_state.system.config.display.theme_name = Some(value);
                        new_state.system.config.display.theme = theme;
                    }
                }
                _ => {
                    debug!("SETTINGS: Unknown setting key: {}", key);
                }
            }
            let config = new_state.system.config.clone();
            let effect = save_config_effect(config);
            (new_state, effect)
        }

        SettingsAction::UpdateConfig(config) => {
            debug!("SETTINGS: Updating config");
            let mut new_state = state;
            new_state.system.config = *config;
            (new_state, Effect::None)
        }
    }
}

/// Switch the selected Settings category and rebuild the new category's focus metadata
/// (shared by NavigateCategoryLeft/Right, which only differ in the direction the category cycles).
fn navigate_category(
    state: AppState,
    component_states: &mut ComponentStateStore,
    new_category: SettingsCategory,
) -> (AppState, Effect) {
    use crate::tui::components::{SettingsDocument, SettingsTabState};
    use crate::tui::document::Document;

    let mut new_state = state;
    new_state.ui.settings.selected_category = new_category;

    if let Some(settings_state) = component_states.get_mut::<SettingsTabState>(SETTINGS_TAB_PATH) {
        let doc = SettingsDocument::new(new_category, new_state.system.config.clone());
        settings_state.doc_nav = Default::default();
        settings_state.doc_nav.focusable_positions = doc.focusable_positions();
        settings_state.doc_nav.focusable_ids = doc.focusable_ids();
        settings_state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
    }

    (new_state, Effect::None)
}

fn save_config_effect(config: Config) -> Effect {
    Effect::Async(Box::pin(async move {
        match crate::config::write(&config) {
            Ok(_) => {
                debug!("CONFIG: Successfully saved to disk");
                Action::SetStatusMessage {
                    message: "Configuration saved".to_string(),
                    is_error: false,
                }
            }
            Err(e) => {
                debug!("CONFIG: Failed to save: {}", e);
                Action::SetStatusMessage {
                    message: format!("Failed to save config: {}", e),
                    is_error: true,
                }
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::THEMES;
    use crate::tui::components::{SettingsDocument, SettingsTabState};
    use crate::tui::document::Document;
    use crate::tui::document_nav::DocumentNavState;

    /// `Config`/`Theme` don't derive `PartialEq`, so tests compare via `Debug`
    /// formatting as a practical equality check.
    fn config_debug_eq(a: &Config, b: &Config) -> bool {
        format!("{:?}", a) == format!("{:?}", b)
    }

    /// Builds a `SettingsTabState` with a `doc_nav` that is deliberately "stale"
    /// (as if it were left over from a previously focused category), so tests can
    /// confirm that `navigate_category` actually replaces it rather than merely
    /// leaving it untouched.
    fn stale_settings_tab_state() -> SettingsTabState {
        SettingsTabState {
            doc_nav: DocumentNavState {
                focus_index: Some(3),
                scroll_offset: 7,
                viewport_height: 20,
                focusable_positions: vec![99],
                ..Default::default()
            },
            modal: None,
        }
    }

    // --- navigate_category() -------------------------------------------------

    #[test]
    fn navigate_category_updates_selected_category_with_no_registered_component() {
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();

        let (new_state, effect) =
            navigate_category(state, &mut component_states, SettingsCategory::Data);

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Data
        );
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn navigate_category_rebuilds_doc_nav_focus_metadata_when_component_registered() {
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();
        component_states.insert(SETTINGS_TAB_PATH.to_string(), stale_settings_tab_state());

        let (new_state, _effect) =
            navigate_category(state, &mut component_states, SettingsCategory::Display);

        let expected_doc =
            SettingsDocument::new(SettingsCategory::Display, new_state.system.config.clone());
        let settings_state = component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .expect("settings tab state should still be registered");

        // Stale focus/scroll position must be cleared, not carried over into the new category.
        assert_eq!(settings_state.doc_nav.focus_index, None);
        assert_eq!(settings_state.doc_nav.scroll_offset, 0);
        assert_eq!(settings_state.doc_nav.viewport_height, 0);
        assert_eq!(
            settings_state.doc_nav.focusable_positions,
            expected_doc.focusable_positions()
        );
        assert_eq!(
            settings_state.doc_nav.focusable_ids,
            expected_doc.focusable_ids()
        );
        assert_eq!(
            settings_state.doc_nav.focusable_row_positions,
            expected_doc.focusable_row_positions()
        );
        // Sanity check: Display category actually has focusable settings, so this
        // test would fail loudly (rather than vacuously) if rebuilding broke.
        assert!(!settings_state.doc_nav.focusable_positions.is_empty());
    }

    #[test]
    fn navigate_category_returns_none_effect() {
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();

        let (_new_state, effect) =
            navigate_category(state, &mut component_states, SettingsCategory::Logging);

        assert!(matches!(effect, Effect::None));
    }

    // --- SettingsAction::NavigateCategoryLeft/Right (wrap-around edges) -----

    #[test]
    fn navigate_category_left_wraps_from_first_variant_to_last_and_rebuilds_focus() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Logging;
        let mut component_states = ComponentStateStore::new();
        component_states.insert(SETTINGS_TAB_PATH.to_string(), stale_settings_tab_state());

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::NavigateCategoryLeft,
            &mut component_states,
        );

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Data
        );
        assert!(matches!(effect, Effect::None));
        let settings_state = component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .unwrap();
        assert_eq!(settings_state.doc_nav.focus_index, None);
        assert!(!settings_state.doc_nav.focusable_positions.is_empty());
    }

    #[test]
    fn navigate_category_right_wraps_from_last_variant_to_first_and_rebuilds_focus() {
        let mut state = AppState::default();
        state.ui.settings.selected_category = SettingsCategory::Data;
        let mut component_states = ComponentStateStore::new();
        component_states.insert(SETTINGS_TAB_PATH.to_string(), stale_settings_tab_state());

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::NavigateCategoryRight,
            &mut component_states,
        );

        assert_eq!(
            new_state.ui.settings.selected_category,
            SettingsCategory::Logging
        );
        assert!(matches!(effect, Effect::None));
        let settings_state = component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .unwrap();
        assert_eq!(settings_state.doc_nav.focus_index, None);
        assert!(!settings_state.doc_nav.focusable_positions.is_empty());
    }

    // --- SettingsAction::ToggleBoolean ---------------------------------------

    #[test]
    fn toggle_boolean_use_unicode_false_to_true_sets_unicode_box_chars() {
        let mut state = AppState::default();
        state.system.config.display.use_unicode = false;
        let mut component_states = ComponentStateStore::new();

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::ToggleBoolean("use_unicode".to_string()),
            &mut component_states,
        );

        assert!(new_state.system.config.display.use_unicode);
        assert_eq!(
            new_state.system.config.display.box_chars,
            crate::formatting::BoxChars::unicode()
        );
    }

    #[test]
    fn toggle_boolean_western_teams_first_returns_batch_with_save_and_rebuild_effects() {
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();

        let (_new_state, effect) = reduce_settings(
            state,
            SettingsAction::ToggleBoolean("western_teams_first".to_string()),
            &mut component_states,
        );

        match effect {
            Effect::Batch(effects) => {
                assert_eq!(effects.len(), 2);
                assert!(matches!(effects[0], Effect::Async(_)));
                assert!(matches!(
                    effects[1],
                    Effect::Action(Action::RebuildStandingsFocusable)
                ));
            }
            other => panic!("expected Effect::Batch, got {:?}", other),
        }
    }

    #[test]
    fn toggle_boolean_unknown_key_leaves_config_unchanged_but_still_saves() {
        let state = AppState::default();
        let original_config = state.system.config.clone();
        let mut component_states = ComponentStateStore::new();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::ToggleBoolean("does_not_exist".to_string()),
            &mut component_states,
        );

        assert!(config_debug_eq(&new_state.system.config, &original_config));
        // Falls through to the shared save path even though nothing changed.
        assert!(matches!(effect, Effect::Async(_)));
    }

    // --- SettingsAction::UpdateSetting ---------------------------------------

    #[test]
    fn update_setting_log_level_updates_config_and_saves() {
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "log_level".to_string(),
                value: "debug".to_string(),
            },
            &mut component_states,
        );

        assert_eq!(new_state.system.config.log_level, "debug");
        assert!(matches!(effect, Effect::Async(_)));
    }

    #[test]
    fn update_setting_theme_to_known_theme_sets_name_and_resolved_theme() {
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "theme".to_string(),
                value: "blue".to_string(),
            },
            &mut component_states,
        );

        assert_eq!(
            new_state.system.config.display.theme_name,
            Some("blue".to_string())
        );
        assert_eq!(
            format!("{:?}", new_state.system.config.display.theme),
            format!("{:?}", THEMES.get("blue").map(|t| (*t).clone()))
        );
    }

    #[test]
    fn update_setting_theme_none_clears_theme_name_and_theme() {
        let mut state = AppState::default();
        state.system.config.display.theme_name = Some("blue".to_string());
        state.system.config.display.theme = THEMES.get("blue").map(|t| (*t).clone());
        let mut component_states = ComponentStateStore::new();

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "theme".to_string(),
                value: "none".to_string(),
            },
            &mut component_states,
        );

        assert_eq!(new_state.system.config.display.theme_name, None);
        assert!(new_state.system.config.display.theme.is_none());
    }

    #[test]
    fn update_setting_theme_unknown_value_sets_name_but_resolves_no_theme() {
        // Latent inconsistency: an unrecognized theme name is still stored in
        // `theme_name`, even though `theme` resolves to None. This leaves the
        // config in a state where `theme_name` no longer corresponds to any
        // actual applied theme.
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "theme".to_string(),
                value: "not_a_real_theme".to_string(),
            },
            &mut component_states,
        );

        assert_eq!(
            new_state.system.config.display.theme_name,
            Some("not_a_real_theme".to_string())
        );
        assert!(new_state.system.config.display.theme.is_none());
    }

    #[test]
    fn update_setting_unknown_key_leaves_config_unchanged_but_still_saves() {
        let state = AppState::default();
        let original_config = state.system.config.clone();
        let mut component_states = ComponentStateStore::new();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "does_not_exist".to_string(),
                value: "irrelevant".to_string(),
            },
            &mut component_states,
        );

        assert!(config_debug_eq(&new_state.system.config, &original_config));
        assert!(matches!(effect, Effect::Async(_)));
    }

    // --- SettingsAction::UpdateConfig ----------------------------------------

    #[test]
    fn update_config_replaces_entire_config_without_persisting() {
        let state = AppState::default();
        let mut replacement_config = state.system.config.clone();
        replacement_config.log_level = "trace".to_string();
        replacement_config.display_standings_western_first = true;
        let mut component_states = ComponentStateStore::new();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateConfig(Box::new(replacement_config.clone())),
            &mut component_states,
        );

        assert!(config_debug_eq(
            &new_state.system.config,
            &replacement_config
        ));
        // Unlike ToggleBoolean/UpdateSetting, UpdateConfig does not trigger a save.
        assert!(matches!(effect, Effect::None));
    }
}
