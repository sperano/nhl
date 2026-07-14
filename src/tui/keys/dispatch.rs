//! Top-level `key_to_action` dispatcher: routes a `KeyEvent` to the global,
//! document-stack, or per-tab handlers based on current focus state.

use crossterm::event::{KeyCode, KeyEvent};
use tracing::{debug, trace};

use crate::tui::action::Action;
use crate::tui::state::AppState;
use crate::tui::types::Tab;

#[cfg(feature = "development")]
use super::demo::handle_demo_tab_keys;
use super::focus::{has_scores_item_focus, has_standings_item_focus};
use super::global::{
    handle_esc_key, handle_global_keys, handle_number_keys, handle_tab_bar_navigation,
};
use super::scores::handle_scores_tab_keys;
use super::settings::handle_settings_tab_keys;
use super::standings::{handle_standings_league_keys, handle_standings_tab_keys};

/// Convert a KeyEvent into an Action based on current application state
///
/// This function implements all keyboard navigation:
/// - Global keys (q, /, ESC)
/// - Tab bar focus: Left/Right navigate tabs, Down enters content
/// - Content focus: Context-sensitive navigation, Up returns to tab bar
/// - Document stack navigation (ESC to close)
///
/// Convert a KeyEvent into an Action based on current application state
///
/// Reads from component state for component-specific checks (e.g., browse mode active).
pub fn key_to_action(
    key: KeyEvent,
    state: &AppState,
    component_states: &crate::tui::component_store::ComponentStateStore,
) -> Option<Action> {
    // Get current tab and focus state
    let current_tab = state.navigation.current_tab;
    let content_focused = state.navigation.focus_in_content;

    trace!(
        "KEY: {:?} (tab={:?}, content_focused={}, document_stack_len={})",
        key.code,
        current_tab,
        content_focused,
        state.navigation.document_stack.len()
    );

    // 1. Check global keys (q/Q, /)
    if let Some(action) = handle_global_keys(key.code) {
        return Some(action);
    }

    // 2. Check ESC key (7-priority hierarchy)
    if key.code == KeyCode::Esc {
        return handle_esc_key(state, component_states);
    }

    // 3. Route key events to stacked documents (when stacked document is open)
    if !state.navigation.document_stack.is_empty() {
        // Delegate key handling to the stacked document handler
        return Some(Action::StackedDocumentKey(key));
    }

    // 4. Check number keys for direct tab switching
    if let Some(action) = handle_number_keys(key.code) {
        return Some(action);
    }

    // 5. Handle navigation based on focus level
    if !content_focused {
        // TAB BAR FOCUSED: delegate to tab bar handler
        // Note: Demo tab focus initialization is handled by enter_content_focus reducer
        let action = handle_tab_bar_navigation(key.code);
        if action.is_some() {
            debug!("KEY: Tab bar navigation: {:?}", action);
        }
        return action;
    }

    // CONTENT FOCUSED: context-sensitive navigation

    // 6. Handle Up key with special logic (returns to tab bar unless in nested mode)
    if key.code == KeyCode::Up {
        // Check if we're in a nested mode first
        #[cfg(feature = "development")]
        let in_demo_tab = current_tab == Tab::Demo;
        #[cfg(not(feature = "development"))]
        let in_demo_tab = false;

        if in_demo_tab {
            // Demo tab - Up handled by handle_demo_tab_keys (both plain and Shift)
        } else if current_tab == Tab::Settings {
            // Settings tab - Up handled by handle_settings_tab_keys (both plain and Shift)
        } else if current_tab == Tab::Standings && has_standings_item_focus(state, component_states)
        {
            // Standings browse mode - Up handled by handle_standings_league_keys (both plain and Shift)
        } else if current_tab == Tab::Scores && has_scores_item_focus(state, component_states) {
            // Scores box-selection - Up handled by handle_scores_tab_keys (both plain and Shift)
        } else {
            // Not in nested mode - Up returns to tab bar
            debug!("KEY: Up pressed in content - returning to tab bar");
            return Some(Action::ExitContentFocus);
        }
    }

    // 6b. Handle Down key for Demo tab - delegated to handle_demo_tab_keys
    // (Both plain Down for focus navigation and Shift+Down for scrolling)

    // 6c. Handle Down key for standings browse mode - delegated to handle_standings_league_keys
    // (Both plain Down for focus navigation and Shift+Down for scrolling)

    // 7. Delegate to tab-specific handlers
    match current_tab {
        Tab::Scores => handle_scores_tab_keys(state, key, component_states),
        Tab::Standings => {
            // All standings views use document navigation in browse mode
            if has_standings_item_focus(state, component_states) {
                handle_standings_league_keys(key, state)
            } else {
                handle_standings_tab_keys(key.code, state)
            }
        }
        Tab::Settings => handle_settings_tab_keys(key, state, component_states),
        #[cfg(feature = "development")]
        Tab::Demo => handle_demo_tab_keys(key, state),
    }
}
