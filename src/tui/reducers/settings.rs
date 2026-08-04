use tracing::{debug, warn};

use crate::config::Config;
use crate::tui::action::{Action, SettingsAction};
use crate::tui::component::Effect;
use crate::tui::state::AppState;

/// Handles the remaining global `SettingsAction` variants (modal confirm
/// flows that mutate and persist `Config`). Category navigation used to live
/// here too, but it - along with all other Settings UI state - is now
/// component-local (see `SettingsTabMsg::NavigateCategoryLeft/Right` in
/// `settings_tab.rs`), so this reducer no longer needs `ComponentStateStore`
/// access.
pub fn reduce_settings(state: AppState, action: SettingsAction) -> (AppState, Effect) {
    match action {
        SettingsAction::ToggleBoolean(key) => handle_toggle_boolean(state, key),
        SettingsAction::UpdateSetting { key, value } => handle_update_setting(state, key, value),
        SettingsAction::UpdateConfig(config) => handle_update_config(state, config),
    }
}

fn handle_toggle_boolean(state: AppState, key: String) -> (AppState, Effect) {
    debug!("SETTINGS: Toggling boolean setting: {}", key);
    let mut new_state = state;
    let effect = match key.as_str() {
        "use_unicode" => {
            new_state.system.config.display.use_unicode =
                !new_state.system.config.display.use_unicode;
            new_state.system.config.display.box_chars = crate::formatting::BoxChars::from_use_unicode(
                new_state.system.config.display.use_unicode,
            );
            save_config_effect(new_state.system.config.clone())
        }
        "western_teams_first" => {
            new_state.system.config.display_standings_western_first =
                !new_state.system.config.display_standings_western_first;
            // Rebuild standings focusable metadata so team selection uses the new order
            let save_effect = save_config_effect(new_state.system.config.clone());
            let rebuild_effect = Effect::Action(Action::RebuildStandingsFocusable);
            Effect::Batch(vec![save_effect, rebuild_effect])
        }
        _ => {
            // Since F3, activation flows through typed `LinkTarget::ToggleSetting(key)`
            // values that originate only from `settings_document.rs`, so an unrecognized
            // key here is a programming error, not user input. No-op rather than save.
            warn!(
                "SETTINGS: ToggleBoolean received unrecognized key {:?}; ignoring",
                key
            );
            Effect::None
        }
    };
    (new_state, effect)
}

fn handle_update_setting(state: AppState, key: String, value: String) -> (AppState, Effect) {
    debug!("SETTINGS: Updating setting: {} = {}", key, value);
    let mut new_state = state;
    let effect = match key.as_str() {
        "log_level" => {
            new_state.system.config.log_level = value;
            save_config_effect(new_state.system.config.clone())
        }
        "theme" => update_theme_setting(&mut new_state, value),
        _ => {
            // Same reasoning as ToggleBoolean above: an unrecognized key is a
            // programming error, not user input. No-op rather than save.
            warn!(
                "SETTINGS: UpdateSetting received unrecognized key {:?}; ignoring",
                key
            );
            Effect::None
        }
    };
    (new_state, effect)
}

fn handle_update_config(state: AppState, config: Box<Config>) -> (AppState, Effect) {
    debug!("SETTINGS: Updating config");
    let mut new_state = state;
    new_state.system.config = *config;
    (new_state, Effect::None)
}

/// Apply a `theme` setting update, saving on success.
///
/// Rejects unrecognized theme values instead of applying them: the modal that
/// drives this action (see `settings_helpers::get_setting_modal_options`) only
/// ever offers `"none"` or a known theme id, so an unrecognized value here
/// would only occur via a hand-edited config or a future bug. Applying it
/// anyway would leave `theme_name` naming a theme that doesn't resolve to an
/// actual `theme` — and persist that inconsistency to disk.
fn update_theme_setting(state: &mut AppState, value: String) -> Effect {
    if value == "none" {
        state.system.config.display.theme_name = None;
        state.system.config.display.theme = None;
        return save_config_effect(state.system.config.clone());
    }

    use crate::config::THEMES;
    match THEMES.get(value.as_str()) {
        Some(theme) => {
            state.system.config.display.theme_name = Some(value);
            state.system.config.display.theme = Some((*theme).clone());
            save_config_effect(state.system.config.clone())
        }
        None => {
            warn!(
                "SETTINGS: Unknown theme value {:?}; keeping previous theme",
                value
            );
            state
                .system
                .set_status_error_message(format!("Unknown theme: {}", value));
            Effect::None
        }
    }
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

    /// `Config`/`Theme` don't derive `PartialEq`, so tests compare via `Debug`
    /// formatting as a practical equality check.
    fn config_debug_eq(a: &Config, b: &Config) -> bool {
        format!("{:?}", a) == format!("{:?}", b)
    }

    // --- SettingsAction::ToggleBoolean ---------------------------------------
    //
    // Category-navigation tests used to live here (as `navigate_category` /
    // `NavigateCategoryLeft/Right` tests) but moved to
    // `components/settings_tab.rs`'s test module along with the behavior
    // itself now that category selection is component-local.

    #[test]
    fn toggle_boolean_use_unicode_false_to_true_sets_unicode_box_chars() {
        let mut state = AppState::default();
        state.system.config.display.use_unicode = false;

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::ToggleBoolean("use_unicode".to_string()),
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

        let (_new_state, effect) = reduce_settings(
            state,
            SettingsAction::ToggleBoolean("western_teams_first".to_string()),
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
    fn toggle_boolean_unknown_key_leaves_config_unchanged_and_does_not_save() {
        // Unrecognized keys are a programming error (activation is typed at the
        // source via `LinkTarget::ToggleSetting`), so this must be a no-op:
        // no config mutation and no disk write.
        let state = AppState::default();
        let original_config = state.system.config.clone();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::ToggleBoolean("does_not_exist".to_string()),
        );

        assert!(config_debug_eq(&new_state.system.config, &original_config));
        assert!(matches!(effect, Effect::None));
    }

    // --- SettingsAction::UpdateSetting ---------------------------------------

    #[test]
    fn update_setting_log_level_updates_config_and_saves() {
        let state = AppState::default();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "log_level".to_string(),
                value: "debug".to_string(),
            },
        );

        assert_eq!(new_state.system.config.log_level, "debug");
        assert!(matches!(effect, Effect::Async(_)));
    }

    #[test]
    fn update_setting_theme_to_known_theme_sets_name_and_resolved_theme() {
        let state = AppState::default();

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "theme".to_string(),
                value: "blue".to_string(),
            },
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

        let (new_state, _effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "theme".to_string(),
                value: "none".to_string(),
            },
        );

        assert_eq!(new_state.system.config.display.theme_name, None);
        assert!(new_state.system.config.display.theme.is_none());
    }

    #[test]
    fn update_setting_theme_unknown_value_rejected_keeps_previous_theme_and_reports_error() {
        // An unrecognized theme value must not be applied: it would leave
        // `theme_name` naming a theme that `theme` doesn't resolve to, and that
        // inconsistency would get persisted to disk. The previous theme stays
        // in place, a no-op effect is returned (no save), and the user sees an
        // error status message.
        let mut state = AppState::default();
        state.system.config.display.theme_name = Some("blue".to_string());
        state.system.config.display.theme = THEMES.get("blue").map(|t| (*t).clone());

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "theme".to_string(),
                value: "not_a_real_theme".to_string(),
            },
        );

        assert_eq!(
            new_state.system.config.display.theme_name,
            Some("blue".to_string())
        );
        assert_eq!(
            format!("{:?}", new_state.system.config.display.theme),
            format!("{:?}", THEMES.get("blue").map(|t| (*t).clone()))
        );
        assert_eq!(
            new_state.system.status_message,
            Some("Unknown theme: not_a_real_theme".to_string())
        );
        assert!(new_state.system.status_is_error);
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn update_setting_unknown_key_leaves_config_unchanged_and_does_not_save() {
        // Unrecognized keys are a programming error (activation is typed at the
        // source via `LinkTarget::EditSetting`), so this must be a no-op: no
        // config mutation and no disk write.
        let state = AppState::default();
        let original_config = state.system.config.clone();

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateSetting {
                key: "does_not_exist".to_string(),
                value: "irrelevant".to_string(),
            },
        );

        assert!(config_debug_eq(&new_state.system.config, &original_config));
        assert!(matches!(effect, Effect::None));
    }

    // --- SettingsAction::UpdateConfig ----------------------------------------

    #[test]
    fn update_config_replaces_entire_config_without_persisting() {
        let state = AppState::default();
        let mut replacement_config = state.system.config.clone();
        replacement_config.log_level = "trace".to_string();
        replacement_config.display_standings_western_first = true;

        let (new_state, effect) = reduce_settings(
            state,
            SettingsAction::UpdateConfig(Box::new(replacement_config.clone())),
        );

        assert!(config_debug_eq(
            &new_state.system.config,
            &replacement_config
        ));
        // Unlike ToggleBoolean/UpdateSetting, UpdateConfig does not trigger a save.
        assert!(matches!(effect, Effect::None));
    }
}
