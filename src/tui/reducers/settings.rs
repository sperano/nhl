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
            use crate::tui::components::{SettingsDocument, SettingsTabState};
            use crate::tui::document::Document;
            let mut new_state = state;
            let old_category = new_state.ui.settings.selected_category;
            new_state.ui.settings.selected_category = match old_category {
                SettingsCategory::Logging => SettingsCategory::Data,
                SettingsCategory::Display => SettingsCategory::Logging,
                SettingsCategory::Data => SettingsCategory::Display,
            };

            if let Some(settings_state) =
                component_states.get_mut::<SettingsTabState>(SETTINGS_TAB_PATH)
            {
                let doc = SettingsDocument::new(
                    new_state.ui.settings.selected_category,
                    new_state.system.config.clone(),
                );
                settings_state.doc_nav = Default::default();
                settings_state.doc_nav.focusable_positions = doc.focusable_positions();
                settings_state.doc_nav.focusable_ids = doc.focusable_ids();
                settings_state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
            }

            (new_state, Effect::None)
        }

        SettingsAction::NavigateCategoryRight => {
            use crate::tui::components::{SettingsDocument, SettingsTabState};
            use crate::tui::document::Document;
            let mut new_state = state;
            let old_category = new_state.ui.settings.selected_category;
            new_state.ui.settings.selected_category = match old_category {
                SettingsCategory::Logging => SettingsCategory::Display,
                SettingsCategory::Display => SettingsCategory::Data,
                SettingsCategory::Data => SettingsCategory::Logging,
            };

            if let Some(settings_state) =
                component_states.get_mut::<SettingsTabState>(SETTINGS_TAB_PATH)
            {
                let doc = SettingsDocument::new(
                    new_state.ui.settings.selected_category,
                    new_state.system.config.clone(),
                );
                settings_state.doc_nav = Default::default();
                settings_state.doc_nav.focusable_positions = doc.focusable_positions();
                settings_state.doc_nav.focusable_ids = doc.focusable_ids();
                settings_state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
            }

            (new_state, Effect::None)
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
                        use crate::config::{
                            THEME_BLUE, THEME_CYAN, THEME_GREEN, THEME_ORANGE, THEME_PURPLE,
                            THEME_RED, THEME_WHITE, THEME_YELLOW,
                        };
                        let theme = match value.as_str() {
                            "orange" => Some(THEME_ORANGE.clone()),
                            "green" => Some(THEME_GREEN.clone()),
                            "blue" => Some(THEME_BLUE.clone()),
                            "purple" => Some(THEME_PURPLE.clone()),
                            "white" => Some(THEME_WHITE.clone()),
                            "red" => Some(THEME_RED.clone()),
                            "yellow" => Some(THEME_YELLOW.clone()),
                            "cyan" => Some(THEME_CYAN.clone()),
                            _ => None,
                        };
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
