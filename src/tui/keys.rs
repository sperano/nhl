/// Keyboard event to action mapping
///
/// This module handles converting crossterm KeyEvents into framework Actions.
/// It contains all the keyboard navigation logic for the TUI.
use crossterm::event::{KeyCode, KeyEvent};
use tracing::{debug, trace};

use crossterm::event::KeyModifiers;

use super::action::{Action, SettingsAction};
use super::component_store::ComponentStateStore;
use super::components::scores_tab::ScoresTabMsg;
use super::components::scores_tab::ScoresTabState;
use super::components::standings_tab::StandingsTabMsg;
use super::components::standings_tab::StandingsTabState;
#[cfg(feature = "development")]
use super::constants::DEMO_TAB_PATH;
use super::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use super::state::AppState;
use super::tab_component::TabState;
use super::types::Tab;

/// Helper to check if scores tab is in browse mode (box selection)
fn is_scores_browse_mode_active(component_states: &ComponentStateStore) -> bool {
    component_states
        .get::<ScoresTabState>(SCORES_TAB_PATH)
        .map(|s| s.is_browse_mode())
        .unwrap_or(false)
}

/// Helper to check if standings tab is in browse mode
fn is_standings_browse_mode_active(component_states: &ComponentStateStore) -> bool {
    component_states
        .get::<StandingsTabState>(STANDINGS_TAB_PATH)
        .map(|s| s.is_browse_mode())
        .unwrap_or(false)
}

/// Helper to check if settings tab has modal open
fn is_settings_modal_open(component_states: &ComponentStateStore) -> bool {
    use super::components::settings_tab::SettingsTabState;
    component_states
        .get::<SettingsTabState>(SETTINGS_TAB_PATH)
        .map(|s| s.modal.is_some())
        .unwrap_or(false)
}

/// Handle global keys that work regardless of tab or focus state
fn handle_global_keys(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Action::Quit),
        KeyCode::Char('/') => Some(Action::ToggleCommandPalette),
        _ => None,
    }
}

/// Handle ESC key with priority-based navigation up through focus hierarchy
fn handle_esc_key(state: &AppState, component_states: &ComponentStateStore) -> Option<Action> {
    use crate::tui::components::settings_tab::{ModalMsg, SettingsTabMsg};

    // Priority 1: If there's a document on the stack, close it
    if !state.navigation.document_stack.is_empty() {
        debug!("KEY: ESC pressed with document open - popping document");
        return Some(Action::PopDocument);
    }

    // Priority 2: If settings modal is open, close it
    if is_settings_modal_open(component_states) {
        debug!("KEY: ESC pressed with settings modal open - closing modal");
        return Some(Action::ComponentMessage {
            path: SETTINGS_TAB_PATH.to_string(),
            message: Box::new(SettingsTabMsg::Modal(ModalMsg::Cancel)),
        });
    }

    // Priority 3: If in box selection mode on Scores tab, exit to date subtabs
    if is_scores_browse_mode_active(component_states) {
        debug!("KEY: ESC pressed in box selection - exiting to date subtabs");
        return Some(Action::ComponentMessage {
            path: SCORES_TAB_PATH.to_string(),
            message: Box::new(ScoresTabMsg::ExitBoxSelection),
        });
    }

    // Priority 4: If in browse mode on Standings tab, exit to view subtabs
    if is_standings_browse_mode_active(component_states) {
        debug!("KEY: ESC pressed in browse mode - exiting to view subtabs");
        return Some(Action::ComponentMessage {
            path: STANDINGS_TAB_PATH.to_string(),
            message: Box::new(StandingsTabMsg::ExitBrowseMode),
        });
    }

    // Priority 5: If content is focused, return to tab bar
    if state.navigation.content_focused {
        debug!("KEY: ESC pressed in content - returning to tab bar");
        return Some(Action::ExitContentFocus);
    }

    // Priority 6: At top level (tab bar), do nothing - use 'q' to quit
    debug!("KEY: ESC pressed at tab bar - ignoring (use 'q' to quit)");
    None
}

/// Handle direct tab switching via number keys (1-3, or 1-4 with development feature)
fn handle_number_keys(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Char('1') => Some(Action::NavigateTab(Tab::Scores)),
        KeyCode::Char('2') => Some(Action::NavigateTab(Tab::Standings)),
        KeyCode::Char('3') => Some(Action::NavigateTab(Tab::Settings)),
        #[cfg(feature = "development")]
        KeyCode::Char('4') => Some(Action::NavigateTab(Tab::Demo)),
        _ => None,
    }
}

/// Handle navigation when tab bar is focused (Left/Right/Down)
fn handle_tab_bar_navigation(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Left => Some(Action::NavigateTabLeft),
        KeyCode::Right => Some(Action::NavigateTabRight),
        KeyCode::Down => {
            debug!("KEY: Down pressed on tab bar - entering content focus");
            Some(Action::EnterContentFocus)
        }
        _ => None,
    }
}

/// Handle Scores tab navigation (box selection mode vs date mode)
fn handle_scores_tab_keys(
    state: &AppState,
    key_code: KeyCode,
    component_states: &ComponentStateStore,
) -> Option<Action> {
    use crate::tui::document_nav::DocumentNavMsg;

    if is_scores_browse_mode_active(component_states) {
        // Box selection mode - use document navigation
        match key_code {
            KeyCode::Down => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::DocNav(DocumentNavMsg::FocusNext)),
            }),
            KeyCode::Up => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::DocNav(DocumentNavMsg::FocusPrev)),
            }),
            KeyCode::Left => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::DocNav(DocumentNavMsg::FocusLeft)),
            }),
            KeyCode::Right => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::DocNav(DocumentNavMsg::FocusRight)),
            }),
            KeyCode::Enter => {
                // Look up game_id from component state and schedule
                if let Some(scores_state) = component_states.get::<ScoresTabState>(SCORES_TAB_PATH)
                {
                    if let Some(selected_index) = scores_state.doc_nav.focus_index {
                        if let Some(schedule) = state.data.schedule.as_ref().as_ref() {
                            if let Some(game) = schedule.games.get(selected_index) {
                                return Some(Action::SelectGame(game.id));
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    } else {
        // Date navigation mode - arrows navigate dates
        match key_code {
            KeyCode::Left => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::NavigateLeft),
            }),
            KeyCode::Right => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::NavigateRight),
            }),
            KeyCode::Down => Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::EnterBoxSelection),
            }),
            KeyCode::Enter => {
                // Look up game_id from component state and schedule (first game)
                if let Some(schedule) = state.data.schedule.as_ref().as_ref() {
                    if let Some(game) = schedule.games.first() {
                        return Some(Action::SelectGame(game.id));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Handle League standings navigation with document system
fn handle_standings_league_keys(key: KeyEvent, _state: &AppState) -> Option<Action> {
    use crate::tui::document_nav::DocumentNavMsg;

    let nav_msg = match key.code {
        // Tab key for focus navigation
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                DocumentNavMsg::FocusPrev
            } else {
                DocumentNavMsg::FocusNext
            }
        }
        KeyCode::BackTab => DocumentNavMsg::FocusPrev,
        // Enter to activate focused element (push TeamDetail document)
        KeyCode::Enter => {
            return Some(Action::ComponentMessage {
                path: STANDINGS_TAB_PATH.to_string(),
                message: Box::new(StandingsTabMsg::ActivateTeam),
            });
        }
        // Up/Down arrows for focus navigation
        KeyCode::Up if !key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::FocusPrev,
        KeyCode::Down if !key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::FocusNext,
        // Left/Right arrows for Row navigation (Conference view)
        KeyCode::Left if !key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::FocusLeft,
        KeyCode::Right if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            DocumentNavMsg::FocusRight
        }
        // Shift+Arrow keys for scrolling
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
            DocumentNavMsg::ScrollDown(1)
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::ScrollUp(1),
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::ScrollUp(1),
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
            DocumentNavMsg::ScrollDown(1)
        }
        // Page navigation
        KeyCode::PageUp => DocumentNavMsg::PageUp,
        KeyCode::PageDown => DocumentNavMsg::PageDown,
        KeyCode::Home => DocumentNavMsg::ScrollToTop,
        KeyCode::End => DocumentNavMsg::ScrollToBottom,
        _ => return None,
    };

    Some(Action::ComponentMessage {
        path: STANDINGS_TAB_PATH.to_string(),
        message: Box::new(StandingsTabMsg::DocNav(nav_msg)),
    })
}

/// Handle Standings tab view selection mode (cycling between Division/Conference/League/Wildcard)
/// Note: Browse mode is handled by handle_standings_league_keys via document navigation
fn handle_standings_tab_keys(key_code: KeyCode, _state: &AppState) -> Option<Action> {
    // View selection mode - arrows navigate views
    match key_code {
        KeyCode::Left => Some(Action::ComponentMessage {
            path: STANDINGS_TAB_PATH.to_string(),
            message: Box::new(StandingsTabMsg::CycleViewLeft),
        }),
        KeyCode::Right => Some(Action::ComponentMessage {
            path: STANDINGS_TAB_PATH.to_string(),
            message: Box::new(StandingsTabMsg::CycleViewRight),
        }),
        KeyCode::Down => Some(Action::ComponentMessage {
            path: STANDINGS_TAB_PATH.to_string(),
            message: Box::new(StandingsTabMsg::EnterBrowseMode),
        }),
        _ => None,
    }
}

/// Handle Settings tab navigation
fn handle_settings_tab_keys(
    key: KeyEvent,
    state: &AppState,
    component_states: &ComponentStateStore,
) -> Option<Action> {
    use crate::tui::components::settings_tab::{ModalMsg, SettingsTabMsg};
    use crate::tui::document_nav::DocumentNavMsg;

    // Check if modal is open - if so, handle modal navigation first
    if is_settings_modal_open(component_states) {
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
            KeyCode::Esc => Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::Modal(ModalMsg::Cancel)),
            }),
            _ => None,
        };
    }

    // No modal open - handle normal navigation
    // Left/Right always navigate categories
    match key.code {
        KeyCode::Left => return Some(Action::SettingsAction(SettingsAction::NavigateCategoryLeft)),
        KeyCode::Right => {
            return Some(Action::SettingsAction(
                SettingsAction::NavigateCategoryRight,
            ))
        }
        _ => {}
    }

    // Handle document navigation within the current category
    let nav_msg = match key.code {
        // Tab key for focus navigation
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                debug!("KEY: Shift-Tab in Settings tab - focus previous");
                DocumentNavMsg::FocusPrev
            } else {
                debug!("KEY: Tab in Settings tab - focus next");
                DocumentNavMsg::FocusNext
            }
        }
        // Arrow keys for focus navigation
        KeyCode::Up if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            debug!("KEY: Up in Settings tab - focus previous");
            DocumentNavMsg::FocusPrev
        }
        KeyCode::Down if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            debug!("KEY: Down in Settings tab - focus next");
            DocumentNavMsg::FocusNext
        }
        // Shift+Arrow keys for scrolling
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
            DocumentNavMsg::ScrollDown(1)
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::ScrollUp(1),
        // Page navigation
        KeyCode::PageUp => DocumentNavMsg::PageUp,
        KeyCode::PageDown => DocumentNavMsg::PageDown,
        KeyCode::Home => DocumentNavMsg::ScrollToTop,
        KeyCode::End => DocumentNavMsg::ScrollToBottom,
        // Enter key activates the focused setting
        KeyCode::Enter => {
            debug!("KEY: Enter in Settings tab - activate focused setting");
            return Some(Action::ComponentMessage {
                path: SETTINGS_TAB_PATH.to_string(),
                message: Box::new(SettingsTabMsg::ActivateSetting(state.system.config.clone())),
            });
        }
        _ => return None,
    };

    Some(Action::ComponentMessage {
        path: SETTINGS_TAB_PATH.to_string(),
        message: Box::new(SettingsTabMsg::DocNav(nav_msg)),
    })
}

/// Handle Demo tab navigation
#[cfg(feature = "development")]
fn handle_demo_tab_keys(key: KeyEvent, _state: &AppState) -> Option<Action> {
    use crate::tui::components::demo_tab::DemoTabMsg;
    use crate::tui::document_nav::DocumentNavMsg;

    let nav_msg = match key.code {
        // Tab key for focus navigation
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                debug!("KEY: Shift-Tab in Demo tab - focus previous");
                DocumentNavMsg::FocusPrev
            } else {
                debug!("KEY: Tab in Demo tab - focus next");
                DocumentNavMsg::FocusNext
            }
        }
        KeyCode::BackTab => {
            debug!("KEY: BackTab in Demo tab - focus previous");
            DocumentNavMsg::FocusPrev
        }
        // Enter to activate focused element (push team/player document)
        KeyCode::Enter => {
            debug!("KEY: Enter in Demo tab - activate focused link");
            return Some(Action::ComponentMessage {
                path: DEMO_TAB_PATH.to_string(),
                message: Box::new(DemoTabMsg::ActivateLink),
            });
        }
        // Left/Right arrows for row navigation (jump between side-by-side elements)
        KeyCode::Left => {
            debug!("KEY: Left in Demo tab - focus left in row");
            DocumentNavMsg::FocusLeft
        }
        KeyCode::Right => {
            debug!("KEY: Right in Demo tab - focus right in row");
            DocumentNavMsg::FocusRight
        }
        // Up/Down arrows for focus navigation
        KeyCode::Up if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            debug!("KEY: Up in Demo tab - focus previous");
            DocumentNavMsg::FocusPrev
        }
        KeyCode::Down if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            debug!("KEY: Down in Demo tab - focus next");
            DocumentNavMsg::FocusNext
        }
        // Shift+Arrow keys for scrolling
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
            DocumentNavMsg::ScrollDown(1)
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => DocumentNavMsg::ScrollUp(1),
        // Page navigation
        KeyCode::PageUp => DocumentNavMsg::PageUp,
        KeyCode::PageDown => DocumentNavMsg::PageDown,
        KeyCode::Home => DocumentNavMsg::ScrollToTop,
        KeyCode::End => DocumentNavMsg::ScrollToBottom,
        _ => return None,
    };

    Some(Action::ComponentMessage {
        path: DEMO_TAB_PATH.to_string(),
        message: Box::new(DemoTabMsg::DocNav(nav_msg)),
    })
}

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
    let content_focused = state.navigation.content_focused;

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
        if is_scores_browse_mode_active(component_states) {
            // In box selection - Up uses document navigation
            use crate::tui::document_nav::DocumentNavMsg;
            return Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::DocNav(DocumentNavMsg::FocusPrev)),
            });
        }
        #[cfg(feature = "development")]
        if current_tab == Tab::Demo {
            // Demo tab - Up handled by handle_demo_tab_keys (both plain and Shift)
        } else if current_tab == Tab::Settings {
            // Settings tab - Up handled by handle_settings_tab_keys (both plain and Shift)
        } else if current_tab == Tab::Standings && is_standings_browse_mode_active(component_states)
        {
            // Standings browse mode - Up handled by handle_standings_league_keys (both plain and Shift)
        } else {
            // Not in nested mode - Up returns to tab bar
            debug!("KEY: Up pressed in content - returning to tab bar");
            return Some(Action::ExitContentFocus);
        }
        #[cfg(not(feature = "development"))]
        if current_tab == Tab::Settings {
            // Settings tab - Up handled by handle_settings_tab_keys (both plain and Shift)
        } else if current_tab == Tab::Standings && is_standings_browse_mode_active(component_states)
        {
            // Standings browse mode - Up handled by handle_standings_league_keys (both plain and Shift)
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
        Tab::Scores => handle_scores_tab_keys(state, key.code, component_states),
        Tab::Standings => {
            // All standings views use document navigation in browse mode
            if is_standings_browse_mode_active(component_states) {
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
