use tracing::{debug, trace};

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::component_store::ComponentStateStore;
#[cfg(feature = "development")]
use crate::tui::components::demo_tab::DemoTabMsg;
use crate::tui::components::scores_tab::ScoresTabState;
use crate::tui::components::settings_tab::SettingsTabState;
use crate::tui::components::standings_tab::StandingsTabState;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
#[cfg(feature = "development")]
use crate::tui::document_nav::DocumentNavState;
use crate::tui::state::AppState;
use crate::tui::tab_component::TabState;
use crate::tui::types::Tab;

/// Handle all navigation-related actions
///
/// Returns Ok((new_state, effect)) if the action was handled,
/// or Err(state) to pass ownership back to the caller.
#[allow(clippy::result_large_err)]
pub fn reduce_navigation(
    state: AppState,
    action: &Action,
    component_states: &mut ComponentStateStore,
) -> Result<(AppState, Effect), AppState> {
    match action {
        Action::NavigateTab(tab) => Ok(navigate_to_tab(state, *tab, component_states)),
        Action::NavigateTabLeft => Ok(navigate_tab_left(state, component_states)),
        Action::NavigateTabRight => Ok(navigate_tab_right(state, component_states)),
        Action::EnterContentFocus => Ok(enter_content_focus(state)),
        Action::ExitContentFocus => Ok(exit_content_focus(state)),
        _ => Err(state),
    }
}

/// Clear every tab's item focus (and any open settings modal).
///
/// Tab switches return focus to the tab bar, so no tab may keep inner item
/// focus behind: stale focus on a non-current tab both renders a phantom
/// highlight and used to misroute Up/ESC handling in keys.rs (cross-tab focus
/// bleed).
fn clear_all_tab_item_focus(component_states: &mut ComponentStateStore) {
    if let Some(s) = component_states.get_mut::<ScoresTabState>(SCORES_TAB_PATH) {
        s.clear_item_focus();
    }
    if let Some(s) = component_states.get_mut::<StandingsTabState>(STANDINGS_TAB_PATH) {
        s.clear_item_focus();
    }
    if let Some(s) = component_states.get_mut::<SettingsTabState>(SETTINGS_TAB_PATH) {
        s.clear_item_focus();
        s.modal = None;
    }
    #[cfg(feature = "development")]
    if let Some(s) = component_states.get_mut::<DocumentNavState>(DEMO_TAB_PATH) {
        s.clear_item_focus();
    }
}

fn navigate_to_tab(
    state: AppState,
    tab: Tab,
    component_states: &mut ComponentStateStore,
) -> (AppState, Effect) {
    trace!("Navigating to tab: {:?}", tab);
    let mut new_state = state;
    new_state.navigation.current_tab = tab;
    new_state.navigation.document_stack.clear();
    new_state.navigation.focus_in_content = false; // Return focus to tab bar
    clear_all_tab_item_focus(component_states);
    trace!("  Cleared document stack, item focus, and returned focus to tab bar");
    (new_state, Effect::None)
}

fn navigate_tab_left(
    state: AppState,
    component_states: &mut ComponentStateStore,
) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.navigation.current_tab = match new_state.navigation.current_tab {
        #[cfg(feature = "development")]
        Tab::Scores => Tab::Demo,
        #[cfg(not(feature = "development"))]
        Tab::Scores => Tab::Settings,
        Tab::Standings => Tab::Scores,
        Tab::Settings => Tab::Standings,
        #[cfg(feature = "development")]
        Tab::Demo => Tab::Settings,
    };
    new_state.navigation.document_stack.clear();
    new_state.navigation.focus_in_content = false; // Return focus to tab bar
    clear_all_tab_item_focus(component_states);
    (new_state, Effect::None)
}

fn navigate_tab_right(
    state: AppState,
    component_states: &mut ComponentStateStore,
) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.navigation.current_tab = match new_state.navigation.current_tab {
        Tab::Scores => Tab::Standings,
        Tab::Standings => Tab::Settings,
        #[cfg(feature = "development")]
        Tab::Settings => Tab::Demo,
        #[cfg(not(feature = "development"))]
        Tab::Settings => Tab::Scores,
        #[cfg(feature = "development")]
        Tab::Demo => Tab::Scores,
    };
    new_state.navigation.document_stack.clear();
    new_state.navigation.focus_in_content = false; // Return focus to tab bar
    clear_all_tab_item_focus(component_states);
    (new_state, Effect::None)
}

fn enter_content_focus(state: AppState) -> (AppState, Effect) {
    debug!("FOCUS: Entering content focus (Down key from tab bar)");
    let mut new_state = state;
    new_state.navigation.focus_in_content = true;

    // Set tab-specific status message and initialize focus for Demo tab
    #[cfg(feature = "development")]
    if new_state.navigation.current_tab == Tab::Demo {
        new_state
            .system
            .set_status_message("↑↓: move selection  Shift+↑↓: scroll  Esc: go back".to_string());

        // Send EnterFocus to Demo tab component to focus first item
        // This happens AFTER focus_in_content is set, avoiding visual flash
        return (
            new_state,
            Effect::Action(Action::ComponentMessage {
                path: DEMO_TAB_PATH.to_string(),
                message: Box::new(DemoTabMsg::EnterFocus),
            }),
        );
    }

    (new_state, Effect::None)
}

fn exit_content_focus(state: AppState) -> (AppState, Effect) {
    debug!("FOCUS: Exiting content focus (Up key to tab bar)");
    let mut new_state = state;

    // Reset status message if exiting from Demo tab
    #[cfg(feature = "development")]
    if new_state.navigation.current_tab == Tab::Demo {
        new_state.system.reset_status_message();
    }

    new_state.navigation.focus_in_content = false;

    (new_state, Effect::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigate_to_tab() {
        let state = AppState::default();
        let (new_state, _) = navigate_to_tab(state, Tab::Settings, &mut ComponentStateStore::new());

        assert_eq!(new_state.navigation.current_tab, Tab::Settings);
        assert!(new_state.navigation.document_stack.is_empty());
        assert!(!new_state.navigation.focus_in_content);
    }

    #[test]
    #[cfg(feature = "development")]
    fn test_tab_left_navigation_cycles_with_demo() {
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Scores;
        let mut store = ComponentStateStore::new();

        let (state, _) = navigate_tab_left(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Demo);

        let (state, _) = navigate_tab_left(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Settings);
    }

    #[test]
    #[cfg(not(feature = "development"))]
    fn test_tab_left_navigation_cycles() {
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Scores;
        let mut store = ComponentStateStore::new();

        let (state, _) = navigate_tab_left(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Settings);

        let (state, _) = navigate_tab_left(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Standings);
    }

    #[test]
    #[cfg(feature = "development")]
    fn test_tab_right_navigation_cycles_with_demo() {
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Demo;
        let mut store = ComponentStateStore::new();

        let (state, _) = navigate_tab_right(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Scores);

        let (state, _) = navigate_tab_right(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Standings);
    }

    #[test]
    #[cfg(not(feature = "development"))]
    fn test_tab_right_navigation_cycles() {
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Settings;
        let mut store = ComponentStateStore::new();

        let (state, _) = navigate_tab_right(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Scores);

        let (state, _) = navigate_tab_right(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Standings);
    }

    /// Regression test for the cross-tab focus bleed: switching tabs must clear
    /// every tab's item focus (and any open settings modal), so stale focus
    /// can't misroute Up/ESC handling in keys.rs or render phantom highlights.
    #[test]
    fn test_tab_switch_clears_all_item_focus_and_modal() {
        use crate::tui::components::settings_tab::{ModalState, SettingsTabState};

        let mut store = ComponentStateStore::new();

        let mut scores = ScoresTabState::default();
        scores.doc_nav_mut().focus_index = Some(1);
        scores.doc_nav_mut().scroll_offset = 3;
        store.insert(SCORES_TAB_PATH.to_string(), scores);

        let mut standings = StandingsTabState::default();
        standings.doc_nav_mut().focus_index = Some(2);
        store.insert(STANDINGS_TAB_PATH.to_string(), standings);

        let mut settings = SettingsTabState::default();
        settings.doc_nav_mut().focus_index = Some(0);
        settings.modal = Some(ModalState {
            options: Vec::new(),
            selected_index: 0,
            setting_key: String::new(),
            position_x: 0,
            position_y: 0,
        });
        store.insert(SETTINGS_TAB_PATH.to_string(), settings);

        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Scores;
        state.navigation.focus_in_content = true;

        let (state, _) = navigate_to_tab(state, Tab::Standings, &mut store);

        assert_eq!(state.navigation.current_tab, Tab::Standings);
        assert!(!state.navigation.focus_in_content);
        let scores = store.get::<ScoresTabState>(SCORES_TAB_PATH).unwrap();
        assert_eq!(scores.doc_nav().focus_index, None);
        assert_eq!(scores.doc_nav().scroll_offset, 0);
        let standings = store.get::<StandingsTabState>(STANDINGS_TAB_PATH).unwrap();
        assert_eq!(standings.doc_nav().focus_index, None);
        let settings = store.get::<SettingsTabState>(SETTINGS_TAB_PATH).unwrap();
        assert_eq!(settings.doc_nav().focus_index, None);
        assert!(settings.modal.is_none());
    }

    /// Same clearing must happen for Left/Right tab cycling, not just direct switches.
    #[test]
    fn test_tab_cycling_clears_item_focus() {
        let mut store = ComponentStateStore::new();
        let mut scores = ScoresTabState::default();
        scores.doc_nav_mut().focus_index = Some(1);
        store.insert(SCORES_TAB_PATH.to_string(), scores);

        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Scores;
        let (state, _) = navigate_tab_right(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Standings);
        assert_eq!(
            store
                .get::<ScoresTabState>(SCORES_TAB_PATH)
                .unwrap()
                .doc_nav()
                .focus_index,
            None
        );

        let mut scores = ScoresTabState::default();
        scores.doc_nav_mut().focus_index = Some(0);
        store.insert(SCORES_TAB_PATH.to_string(), scores);
        let (state, _) = navigate_tab_left(state, &mut store);
        assert_eq!(state.navigation.current_tab, Tab::Scores);
        assert_eq!(
            store
                .get::<ScoresTabState>(SCORES_TAB_PATH)
                .unwrap()
                .doc_nav()
                .focus_index,
            None
        );
    }
}
