//! Global keys: unconditional quit, the ESC priority hierarchy, direct
//! tab-switch number keys, and tab-bar-focused navigation.

use crossterm::event::KeyCode;
use tracing::debug;

use crate::tui::action::Action;
use crate::tui::component_store::ComponentStateStore;
#[cfg(feature = "development")]
use crate::tui::components::demo_tab::DemoTabMsg;
use crate::tui::components::scores_tab::ScoresTabMsg;
use crate::tui::components::standings_tab::StandingsTabMsg;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::state::AppState;
use crate::tui::types::Tab;

use super::focus::{
    has_scores_item_focus, has_settings_item_focus, has_standings_item_focus,
    is_settings_modal_open,
};

/// Handle global keys that work regardless of tab or focus state
pub fn handle_global_keys(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Action::Quit),
        _ => None,
    }
}

/// Handle ESC key with priority-based navigation up through focus hierarchy
pub fn handle_esc_key(state: &AppState, component_states: &ComponentStateStore) -> Option<Action> {
    use crate::tui::components::settings_tab::{ModalMsg, SettingsTabMsg};

    // Priority 1: If there's a document on the stack, close it
    if !state.navigation.document_stack.is_empty() {
        debug!("KEY: ESC pressed with document open - popping document");
        return Some(Action::PopDocument);
    }

    // Priority 2: If settings modal is open, close it
    if is_settings_modal_open(state, component_states) {
        debug!("KEY: ESC pressed with settings modal open - closing modal");
        return Some(Action::ComponentMessage {
            path: SETTINGS_TAB_PATH.to_string(),
            message: Box::new(SettingsTabMsg::Modal(ModalMsg::Cancel)),
        });
    }

    // Priority 3: If in box selection mode on Scores tab, exit to date subtabs
    if has_scores_item_focus(state, component_states) {
        debug!("KEY: ESC pressed in box selection - exiting to date subtabs");
        return Some(Action::ComponentMessage {
            path: SCORES_TAB_PATH.to_string(),
            message: Box::new(ScoresTabMsg::ExitBoxSelection),
        });
    }

    // Priority 4: If standings tab has item focus, clear it
    if has_standings_item_focus(state, component_states) {
        debug!("KEY: ESC pressed with standings item focus - clearing focus");
        return Some(Action::ComponentMessage {
            path: STANDINGS_TAB_PATH.to_string(),
            message: Box::new(StandingsTabMsg::ExitBrowseMode),
        });
    }

    // Priority 4.5: If settings tab has item focus, clear it
    if has_settings_item_focus(state, component_states) {
        debug!("KEY: ESC pressed with settings item focus - clearing focus");
        return Some(Action::ComponentMessage {
            path: SETTINGS_TAB_PATH.to_string(),
            message: Box::new(SettingsTabMsg::NavigateUp),
        });
    }

    // Priority 4.6: If demo tab has content focus, clear selection and exit
    #[cfg(feature = "development")]
    if state.navigation.current_tab == Tab::Demo && state.navigation.focus_in_content {
        debug!("KEY: ESC pressed with demo content focus - clearing selection");
        return Some(Action::ComponentMessage {
            path: DEMO_TAB_PATH.to_string(),
            message: Box::new(DemoTabMsg::ExitFocus),
        });
    }

    // Priority 5: If content is focused, return to tab bar
    if state.navigation.focus_in_content {
        debug!("KEY: ESC pressed in content - returning to tab bar");
        return Some(Action::ExitContentFocus);
    }

    // Priority 6: At top level (tab bar), do nothing - use 'q' to quit
    debug!("KEY: ESC pressed at tab bar - ignoring (use 'q' to quit)");
    None
}

/// Handle direct tab switching via number keys (1-3, or 1-4 with development feature)
pub fn handle_number_keys(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Char('1') => Some(Action::NavigateTab(Tab::Scores)),
        KeyCode::Char('2') => Some(Action::NavigateTab(Tab::Standings)),
        KeyCode::Char('3') => Some(Action::NavigateTab(Tab::Settings)),
        #[cfg(feature = "development")]
        KeyCode::Char('4') => Some(Action::NavigateTab(Tab::Demo)),
        _ => None,
    }
}

/// Handle navigation when tab bar is focused (Left/Right/Down/Enter)
pub fn handle_tab_bar_navigation(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Left => Some(Action::NavigateTabLeft),
        KeyCode::Right => Some(Action::NavigateTabRight),
        KeyCode::Down | KeyCode::Enter => {
            debug!("KEY: Down/Enter pressed on tab bar - entering content focus");
            Some(Action::EnterContentFocus)
        }
        _ => None,
    }
}
