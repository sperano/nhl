//! Settings tab key handling (modal navigation vs normal category/document navigation).

use crossterm::event::{KeyCode, KeyEvent};
use tracing::debug;

use crate::tui::action::Action;
use crate::tui::component_store::ComponentStateStore;
use crate::tui::constants::SETTINGS_TAB_PATH;
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::AppState;

use super::focus::is_settings_modal_open;

/// Handle Settings tab navigation
pub fn handle_settings_tab_keys(
    key: KeyEvent,
    state: &AppState,
    component_states: &ComponentStateStore,
) -> Option<Action> {
    use crate::tui::components::settings_tab::{ModalMsg, SettingsTabMsg};

    // Check if modal is open - if so, handle modal navigation first
    if is_settings_modal_open(state, component_states) {
        return match key.code {
            KeyCode::Up => Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::Modal(ModalMsg::Up)),
            }),
            KeyCode::Down => Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::Modal(ModalMsg::Down)),
            }),
            KeyCode::Enter => Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::Modal(ModalMsg::Confirm)),
            }),
            // Note: no Esc arm here - ESC is intercepted globally by handle_esc_key's
            // priority-2 check (settings modal open) before key_to_action ever
            // reaches this function, so an Esc arm here would be dead code.
            _ => None,
        };
    }

    // No modal open - handle normal navigation
    // Left/Right always navigate categories. Category selection is
    // component-local (SettingsTabState::selected_category), so the message
    // carries the Config that `update()` needs to rebuild doc_nav for the new
    // category - the same pattern ActivateSetting below already uses.
    match key.code {
        KeyCode::Left => {
            return Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::NavigateCategoryLeft(
                    state.system.config.clone(),
                )),
            })
        }
        KeyCode::Right => {
            return Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::NavigateCategoryRight(
                    state.system.config.clone(),
                )),
            })
        }
        _ => {}
    }

    // Enter key activates the focused setting; not a nav message.
    if key.code == KeyCode::Enter {
        debug!("KEY: Enter in Settings tab - activate focused setting");
        return Some(Action::ComponentMessage {
            path: SETTINGS_TAB_PATH.to_string(),
            message: Box::new(SettingsTabMsg::ActivateSetting(state.system.config.clone())),
        });
    }

    // Handle document navigation within the current category via the canonical
    // mapping (Tab/arrows/Shift+arrows/Page/Home/End match this context exactly).
    let nav_msg = key_to_nav_msg(key)?;

    Some(Action::ComponentMessage {
        path: SETTINGS_TAB_PATH.to_string(),
        message: Box::new(SettingsTabMsg::DocNav(nav_msg)),
    })
}
