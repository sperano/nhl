//! Focus/modal predicates shared across the per-tab key handlers.

use crate::tui::component_store::ComponentStateStore;
use crate::tui::components::scores_tab::ScoresTabState;
use crate::tui::components::standings_tab::StandingsTabState;
use crate::tui::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::state::AppState;
use crate::tui::tab_component::TabState;
use crate::tui::types::Tab;

/// Helper to check if scores tab has an item focused (box selection).
///
/// Gated on the Scores tab being current: another tab's stale focus state must
/// never influence key routing (cross-tab focus bleed).
pub fn has_scores_item_focus(state: &AppState, component_states: &ComponentStateStore) -> bool {
    state.navigation.current_tab == Tab::Scores
        && component_states
            .get::<ScoresTabState>(SCORES_TAB_PATH)
            .map(|s| s.has_item_focus())
            .unwrap_or(false)
}

/// Helper to check if standings tab has an item focused (gated on current tab)
pub fn has_standings_item_focus(state: &AppState, component_states: &ComponentStateStore) -> bool {
    state.navigation.current_tab == Tab::Standings
        && component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .map(|s| s.has_item_focus())
            .unwrap_or(false)
}

/// Helper to check if settings tab has modal open (gated on current tab)
pub fn is_settings_modal_open(state: &AppState, component_states: &ComponentStateStore) -> bool {
    use crate::tui::components::settings_tab::SettingsTabState;
    state.navigation.current_tab == Tab::Settings
        && component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .map(|s| s.modal.is_some())
            .unwrap_or(false)
}

/// Helper to check if settings tab has an item focused (gated on current tab)
pub fn has_settings_item_focus(state: &AppState, component_states: &ComponentStateStore) -> bool {
    use crate::tui::components::settings_tab::SettingsTabState;
    state.navigation.current_tab == Tab::Settings
        && component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .map(|s| s.has_item_focus())
            .unwrap_or(false)
}
