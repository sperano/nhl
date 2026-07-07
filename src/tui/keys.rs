/// Keyboard event to action mapping
///
/// This module handles converting crossterm KeyEvents into framework Actions.
/// It contains all the keyboard navigation logic for the TUI.
use crossterm::event::{KeyCode, KeyEvent};
use tracing::{debug, trace};

use super::action::Action;
use super::component_store::ComponentStateStore;
#[cfg(feature = "development")]
use super::components::demo_tab::DemoTabMsg;
use super::components::scores_tab::ScoresTabMsg;
use super::components::scores_tab::ScoresTabState;
use super::components::standings_tab::StandingsTabMsg;
use super::components::standings_tab::StandingsTabState;
#[cfg(feature = "development")]
use super::constants::DEMO_TAB_PATH;
use super::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use super::nav_handler::key_to_nav_msg;
use super::state::AppState;
use super::tab_component::TabState;
use super::types::Tab;

/// Helper to check if scores tab has an item focused (box selection).
///
/// Gated on the Scores tab being current: another tab's stale focus state must
/// never influence key routing (cross-tab focus bleed).
fn has_scores_item_focus(state: &AppState, component_states: &ComponentStateStore) -> bool {
    state.navigation.current_tab == Tab::Scores
        && component_states
            .get::<ScoresTabState>(SCORES_TAB_PATH)
            .map(|s| s.has_item_focus())
            .unwrap_or(false)
}

/// Helper to check if standings tab has an item focused (gated on current tab)
fn has_standings_item_focus(state: &AppState, component_states: &ComponentStateStore) -> bool {
    state.navigation.current_tab == Tab::Standings
        && component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .map(|s| s.has_item_focus())
            .unwrap_or(false)
}

/// Helper to check if settings tab has modal open (gated on current tab)
fn is_settings_modal_open(state: &AppState, component_states: &ComponentStateStore) -> bool {
    use super::components::settings_tab::SettingsTabState;
    state.navigation.current_tab == Tab::Settings
        && component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .map(|s| s.modal.is_some())
            .unwrap_or(false)
}

/// Helper to check if settings tab has an item focused (gated on current tab)
fn has_settings_item_focus(state: &AppState, component_states: &ComponentStateStore) -> bool {
    use super::components::settings_tab::SettingsTabState;
    state.navigation.current_tab == Tab::Settings
        && component_states
            .get::<SettingsTabState>(SETTINGS_TAB_PATH)
            .map(|s| s.has_item_focus())
            .unwrap_or(false)
}

/// Handle global keys that work regardless of tab or focus state
fn handle_global_keys(key_code: KeyCode) -> Option<Action> {
    match key_code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Action::Quit),
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

/// Handle navigation when tab bar is focused (Left/Right/Down/Enter)
fn handle_tab_bar_navigation(key_code: KeyCode) -> Option<Action> {
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

/// Handle Scores tab navigation (box selection mode vs date mode)
///
/// Box-selection mode matches the canonical `key_to_nav_msg` mapping exactly
/// (Tab/BackTab focus cycling, PageUp/PageDown, Home/End, Shift+arrow
/// scrolling all apply here); only `Enter` is special-cased, since it
/// activates the focused game rather than emitting a `DocumentNavMsg`.
/// Date-navigation mode (no item focus) is untouched - its `Left`/`Right` are
/// date semantics, not document navigation.
fn handle_scores_tab_keys(
    state: &AppState,
    key: KeyEvent,
    component_states: &ComponentStateStore,
) -> Option<Action> {
    if has_scores_item_focus(state, component_states) {
        // Box selection mode.
        if key.code == KeyCode::Enter {
            // Delegate to ScoresTabMsg::ActivateGame, which already owns the
            // focus_index -> game_id lookup (via focusable_ids), instead of
            // duplicating that lookup here against state.data.schedule.
            return Some(Action::ComponentMessage {
                path: SCORES_TAB_PATH.to_string(),
                message: Box::new(ScoresTabMsg::ActivateGame),
            });
        }

        let nav_msg = key_to_nav_msg(key)?;
        Some(Action::ComponentMessage {
            path: SCORES_TAB_PATH.to_string(),
            message: Box::new(ScoresTabMsg::DocNav(nav_msg)),
        })
    } else {
        // Date navigation mode - arrows navigate dates
        match key.code {
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
                        return Some(Action::SelectGame(game.id.into()));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Handle League standings navigation with document system
///
/// Delegates the arrow/Tab/page-navigation mapping to the canonical
/// `nav_handler::key_to_nav_msg`, which matches this context's mapping exactly
/// (including Shift+Left/Right scrolling instead of row navigation). Enter is
/// handled separately since it activates the focused team rather than emitting
/// a `DocumentNavMsg`.
fn handle_standings_league_keys(key: KeyEvent, _state: &AppState) -> Option<Action> {
    // Enter activates the focused element (push TeamDetail document)
    if key.code == KeyCode::Enter {
        return Some(Action::ComponentMessage {
            path: STANDINGS_TAB_PATH.to_string(),
            message: Box::new(StandingsTabMsg::ActivateTeam),
        });
    }

    let nav_msg = key_to_nav_msg(key)?;

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

/// Handle Demo tab navigation
#[cfg(feature = "development")]
fn handle_demo_tab_keys(key: KeyEvent, _state: &AppState) -> Option<Action> {
    use crate::tui::components::demo_tab::DemoTabMsg;

    // Enter activates the focused link; not a nav message.
    if key.code == KeyCode::Enter {
        debug!("KEY: Enter in Demo tab - activate focused link");
        return Some(Action::ComponentMessage {
            path: DEMO_TAB_PATH.to_string(),
            message: Box::new(DemoTabMsg::ActivateLink),
        });
    }

    // Matches the canonical mapping exactly, including Shift+Left/Right scroll.
    let nav_msg = key_to_nav_msg(key)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::create_mock_schedule;
    use crate::tui::components::settings_tab::{ModalState, SettingsTabState};
    use crate::tui::document_nav::DocumentNavState;
    use crate::tui::state::DocumentStackEntry;
    use crate::tui::types::StackedDocument;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::sync::Arc;

    // ---------------------------------------------------------------------
    // Construction helpers
    // ---------------------------------------------------------------------

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn shift_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    /// AppState pinned to a tab and focus level, with an empty document stack.
    fn state_for(tab: Tab, content_focused: bool) -> AppState {
        let mut state = AppState::default();
        state.navigation.current_tab = tab;
        state.navigation.focus_in_content = content_focused;
        state
    }

    /// AppState with a document pushed onto the stack (any tab/focus level).
    fn state_with_document_stack(tab: Tab, content_focused: bool) -> AppState {
        let mut state = state_for(tab, content_focused);
        state
            .navigation
            .document_stack
            .push(DocumentStackEntry::new(StackedDocument::TeamDetail {
                abbrev: "TOR".to_string(),
            }));
        state
    }

    /// AppState with a schedule loaded, so Enter-to-select-game paths are exercised.
    fn state_with_schedule(tab: Tab, content_focused: bool) -> AppState {
        let mut state = state_for(tab, content_focused);
        state.data.schedule = Arc::new(Some(create_mock_schedule(None)));
        state
    }

    fn empty_store() -> ComponentStateStore {
        ComponentStateStore::new()
    }

    fn store_with_scores_focus(focus_index: Option<usize>) -> ComponentStateStore {
        let mut store = ComponentStateStore::new();
        store.insert(
            SCORES_TAB_PATH.to_string(),
            ScoresTabState {
                doc_nav: DocumentNavState {
                    focus_index,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        store
    }

    fn store_with_standings_focus(focus_index: Option<usize>) -> ComponentStateStore {
        let mut store = ComponentStateStore::new();
        store.insert(
            STANDINGS_TAB_PATH.to_string(),
            StandingsTabState {
                doc_nav: DocumentNavState {
                    focus_index,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        store
    }

    fn store_with_settings_focus(focus_index: Option<usize>) -> ComponentStateStore {
        let mut store = ComponentStateStore::new();
        store.insert(
            SETTINGS_TAB_PATH.to_string(),
            SettingsTabState {
                doc_nav: DocumentNavState {
                    focus_index,
                    ..Default::default()
                },
                modal: None,
                ..Default::default()
            },
        );
        store
    }

    fn dummy_modal_state() -> ModalState {
        ModalState {
            options: vec![],
            selected_index: 0,
            setting_key: "log_level".to_string(),
            position_x: 0,
            position_y: 0,
        }
    }

    fn store_with_settings_modal_open() -> ComponentStateStore {
        let mut store = ComponentStateStore::new();
        store.insert(
            SETTINGS_TAB_PATH.to_string(),
            SettingsTabState {
                doc_nav: DocumentNavState::default(),
                modal: Some(dummy_modal_state()),
                ..Default::default()
            },
        );
        store
    }

    /// Store with both settings-modal-open AND scores item-focus set, to prove
    /// modal-open (ESC priority 2) wins over scores box-selection (priority 3).
    fn store_with_settings_modal_and_scores_focus() -> ComponentStateStore {
        let mut store = store_with_settings_modal_open();
        store.insert(
            SCORES_TAB_PATH.to_string(),
            ScoresTabState {
                doc_nav: DocumentNavState {
                    focus_index: Some(0),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        store
    }

    /// Store with both scores and standings item-focus set, to prove scores
    /// box-selection (ESC priority 3) wins over standings browse mode (priority 4).
    fn store_with_scores_and_standings_focus() -> ComponentStateStore {
        let mut store = store_with_scores_focus(Some(0));
        store.insert(
            STANDINGS_TAB_PATH.to_string(),
            StandingsTabState {
                doc_nav: DocumentNavState {
                    focus_index: Some(0),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        store
    }

    /// Store with both standings and settings item-focus set, to prove standings
    /// browse mode (ESC priority 4) wins over settings item focus (priority 4.5).
    fn store_with_standings_and_settings_focus() -> ComponentStateStore {
        let mut store = store_with_standings_focus(Some(0));
        store.insert(
            SETTINGS_TAB_PATH.to_string(),
            SettingsTabState {
                doc_nav: DocumentNavState {
                    focus_index: Some(0),
                    ..Default::default()
                },
                modal: None,
                ..Default::default()
            },
        );
        store
    }

    // ---------------------------------------------------------------------
    // Assertion helpers
    // ---------------------------------------------------------------------

    /// Checks that `action` is a `ComponentMessage` to `expected_path` whose boxed
    /// message's `Debug` output exactly equals `expected_debug`. `ComponentMessageTrait`
    /// is type-erased (no `Any`/`PartialEq`), so `Debug` (a supertrait bound already
    /// required on the trait) is the only generic way to assert on message identity
    /// from outside the component that owns the concrete message type.
    fn is_component_message(
        action: &Option<Action>,
        expected_path: &str,
        expected_debug: &str,
    ) -> bool {
        match action {
            Some(Action::ComponentMessage { path, message }) => {
                path == expected_path && format!("{:?}", message) == expected_debug
            }
            _ => false,
        }
    }

    /// Like `is_component_message` but only checks a `Debug` prefix - used for
    /// messages carrying a large embedded value (e.g. the full `Config`) where
    /// pinning the exact `Debug` text would make the test fragile.
    fn is_component_message_prefix(
        action: &Option<Action>,
        expected_path: &str,
        debug_prefix: &str,
    ) -> bool {
        match action {
            Some(Action::ComponentMessage { path, message }) => {
                path == expected_path && format!("{:?}", message).starts_with(debug_prefix)
            }
            _ => false,
        }
    }

    fn is_stacked_document_key(action: &Option<Action>, expected_code: KeyCode) -> bool {
        matches!(action, Some(Action::StackedDocumentKey(k)) if k.code == expected_code)
    }

    fn is_navigate_tab(action: &Option<Action>, expected: Tab) -> bool {
        matches!(action, Some(Action::NavigateTab(tab)) if *tab == expected)
    }

    fn is_select_game(action: &Option<Action>, expected_id: i64) -> bool {
        matches!(action, Some(Action::SelectGame(id)) if *id == expected_id)
    }

    fn is_none(action: &Option<Action>) -> bool {
        action.is_none()
    }

    // ---------------------------------------------------------------------
    // Table-driven cases
    // ---------------------------------------------------------------------

    /// One row of the `key_to_action` behavior table: a named context, an input
    /// key event, and a check on the resulting action. Kept as a flat list (rather
    /// than nested per-context test functions) so this table can double as a
    /// behavior-preservation harness for a future keys.rs refactor.
    struct KeyCase {
        description: &'static str,
        state: AppState,
        store: ComponentStateStore,
        key: KeyEvent,
        check: Box<dyn Fn(&Option<Action>) -> bool>,
    }

    /// Cases that apply regardless of the `development` feature flag.
    fn base_cases() -> Vec<KeyCase> {
        vec![
            // --- Global keys: 'q'/'Q' quit unconditionally, even before ESC/document
            // stack routing is considered. ---
            KeyCase {
                description: "q quits from the tab bar",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('q')),
                check: Box::new(|a| matches!(a, Some(Action::Quit))),
            },
            KeyCase {
                description: "Q (uppercase) quits",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('Q')),
                check: Box::new(|a| matches!(a, Some(Action::Quit))),
            },
            KeyCase {
                description: "q quits even with content focused and box-selection active",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Char('q')),
                check: Box::new(|a| matches!(a, Some(Action::Quit))),
            },
            KeyCase {
                description: "q quits even with a document on the stack",
                state: state_with_document_stack(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('q')),
                check: Box::new(|a| matches!(a, Some(Action::Quit))),
            },
            // --- ESC priority hierarchy (handle_esc_key) ---
            KeyCase {
                description: "ESC pops the document stack (priority 1)",
                state: state_with_document_stack(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| matches!(a, Some(Action::PopDocument))),
            },
            KeyCase {
                description: "ESC prioritizes popping the document over closing the settings modal (priority 1 over 2)",
                state: state_with_document_stack(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| matches!(a, Some(Action::PopDocument))),
            },
            KeyCase {
                description: "ESC closes the settings modal when open (priority 2)",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "Modal(Cancel)")),
            },
            KeyCase {
                description: "ESC closes the settings modal and ignores stale Scores box-selection focus \
                    (priority 2; the scores check is gated on the current tab being Scores)",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_and_scores_focus(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "Modal(Cancel)")),
            },
            KeyCase {
                description: "ESC exits scores box-selection when active (priority 3)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "ExitBoxSelection")),
            },
            KeyCase {
                description: "ESC on the Standings tab exits browse mode and ignores stale Scores box-selection focus \
                    (cross-tab focus bleed regression: only the current tab's focus may drive ESC)",
                state: state_for(Tab::Standings, true),
                store: store_with_scores_and_standings_focus(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "ExitBrowseMode")),
            },
            KeyCase {
                description: "ESC exits standings browse mode when active and scores has no focus (priority 4)",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "ExitBrowseMode")),
            },
            KeyCase {
                description: "ESC on the Settings tab clears settings item focus and ignores stale Standings browse focus \
                    (cross-tab focus bleed regression: only the current tab's focus may drive ESC)",
                state: state_for(Tab::Settings, true),
                store: store_with_standings_and_settings_focus(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "NavigateUp")),
            },
            KeyCase {
                description: "ESC clears settings item focus (priority 4.5)",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_focus(Some(0)),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "NavigateUp")),
            },
            KeyCase {
                description: "ESC returns to the tab bar when content is focused with no nested mode active (priority 5)",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| matches!(a, Some(Action::ExitContentFocus))),
            },
            KeyCase {
                description: "ESC does nothing at the tab bar top level (priority 6, use 'q' to quit)",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Esc),
                check: Box::new(is_none),
            },
            // --- Document stack routing: once a document is open, every key other
            // than 'q'/ESC is routed verbatim to the document, bypassing number-key
            // tab switching and all tab-specific handlers. ---
            KeyCase {
                description: "an open document routes an arbitrary key to StackedDocumentKey",
                state: state_with_document_stack(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Left),
                check: Box::new(|a| is_stacked_document_key(a, KeyCode::Left)),
            },
            KeyCase {
                description: "an open document intercepts number keys, so they do not switch tabs while a document is open",
                state: state_with_document_stack(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('1')),
                check: Box::new(|a| is_stacked_document_key(a, KeyCode::Char('1'))),
            },
            KeyCase {
                description: "an open document intercepts Enter",
                state: state_with_document_stack(Tab::Standings, true),
                store: empty_store(),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_stacked_document_key(a, KeyCode::Enter)),
            },
            // --- Number keys: direct tab switching, independent of focus level and
            // key modifiers (only checked when the document stack is empty). ---
            KeyCase {
                description: "'1' navigates to the Scores tab from the tab bar",
                state: state_for(Tab::Standings, false),
                store: empty_store(),
                key: key(KeyCode::Char('1')),
                check: Box::new(|a| is_navigate_tab(a, Tab::Scores)),
            },
            KeyCase {
                description: "'2' navigates to the Standings tab even while content-focused on another tab",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Char('2')),
                check: Box::new(|a| is_navigate_tab(a, Tab::Standings)),
            },
            KeyCase {
                description: "'3' navigates to the Settings tab",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('3')),
                check: Box::new(|a| is_navigate_tab(a, Tab::Settings)),
            },
            KeyCase {
                description: "number-key tab switching ignores modifiers (Shift+1 still switches tabs)",
                state: state_for(Tab::Standings, false),
                store: empty_store(),
                key: shift_key(KeyCode::Char('1')),
                check: Box::new(|a| is_navigate_tab(a, Tab::Scores)),
            },
            KeyCase {
                description: "'5' is not a mapped tab-switch key and falls through to tab-bar handling (no-op here)",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('5')),
                check: Box::new(is_none),
            },
            // --- Tab bar navigation (content not focused) ---
            KeyCase {
                description: "Left on the tab bar navigates to the previous tab",
                state: state_for(Tab::Standings, false),
                store: empty_store(),
                key: key(KeyCode::Left),
                check: Box::new(|a| matches!(a, Some(Action::NavigateTabLeft))),
            },
            KeyCase {
                description: "Right on the tab bar navigates to the next tab",
                state: state_for(Tab::Standings, false),
                store: empty_store(),
                key: key(KeyCode::Right),
                check: Box::new(|a| matches!(a, Some(Action::NavigateTabRight))),
            },
            KeyCase {
                description: "Down on the tab bar enters content focus",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Down),
                check: Box::new(|a| matches!(a, Some(Action::EnterContentFocus))),
            },
            KeyCase {
                description: "Enter on the tab bar enters content focus",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Enter),
                check: Box::new(|a| matches!(a, Some(Action::EnterContentFocus))),
            },
            KeyCase {
                description: "Up on the tab bar does nothing",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Up),
                check: Box::new(is_none),
            },
            KeyCase {
                description: "an unmapped letter on the tab bar does nothing",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('z')),
                check: Box::new(is_none),
            },
            // --- Scores tab: date-navigation mode (no item focus) ---
            KeyCase {
                description: "Left in date mode navigates to the previous date",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "NavigateLeft")),
            },
            KeyCase {
                description: "Right in date mode navigates to the next date",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "NavigateRight")),
            },
            KeyCase {
                description: "Down in date mode enters box selection",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "EnterBoxSelection")),
            },
            KeyCase {
                description: "Enter in date mode with no schedule loaded does nothing",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Enter),
                check: Box::new(is_none),
            },
            KeyCase {
                description: "Enter in date mode with a schedule loaded selects the first game",
                state: state_with_schedule(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_select_game(a, 2024020001)),
            },
            KeyCase {
                description: "Up in date mode (no nested mode active) returns to the tab bar",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Up),
                check: Box::new(|a| matches!(a, Some(Action::ExitContentFocus))),
            },
            KeyCase {
                description: "an unmapped key in date mode does nothing",
                state: state_for(Tab::Scores, true),
                store: empty_store(),
                key: key(KeyCode::Char('x')),
                check: Box::new(is_none),
            },
            // --- Scores tab: box-selection mode (item focus active) ---
            KeyCase {
                description: "Down in box-selection mode focuses the next game",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Up in box-selection mode focuses the previous game (reached via the \
                    step-6 Up fallthrough into handle_scores_tab_keys, same as Standings/Settings/Demo - \
                    no longer handled directly by key_to_action's own Up special-case, see F6 convergence)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Shift+Up in box-selection mode scrolls up (F6 convergence: was FocusPrev \
                    before key_to_action's Up special-case ignored Shift for scores box-selection)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: shift_key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "Left in box-selection mode focuses left",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusLeft)")),
            },
            KeyCase {
                description: "Right in box-selection mode focuses right",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusRight)")),
            },
            KeyCase {
                description: "Shift+Down in box-selection mode scrolls down (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: shift_key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "Shift+Left in box-selection mode scrolls up, not left (F6 convergence: \
                    newly live, matches canonical mapping where scrolling wins over row navigation)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: shift_key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "Shift+Right in box-selection mode scrolls down (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: shift_key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "Tab in box-selection mode focuses the next game (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Shift+Tab in box-selection mode focuses the previous game (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: shift_key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "BackTab in box-selection mode focuses the previous game (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::BackTab),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "PageUp in box-selection mode pages up (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::PageUp),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(PageUp)")),
            },
            KeyCase {
                description: "PageDown in box-selection mode pages down (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::PageDown),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(PageDown)")),
            },
            KeyCase {
                description: "Home in box-selection mode scrolls to the top (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Home),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(ScrollToTop)")),
            },
            KeyCase {
                description: "End in box-selection mode scrolls to the bottom (F6 convergence: newly live)",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::End),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "DocNav(ScrollToBottom)")),
            },
            KeyCase {
                description: "Enter in box-selection mode delegates to ScoresTabMsg::ActivateGame, \
                    which owns the focus_index -> game_id lookup (was: is_select_game(2024020002) \
                    before A13 routed this through the component message instead of duplicating \
                    the schedule lookup in keys.rs)",
                state: state_with_schedule(Tab::Scores, true),
                store: store_with_scores_focus(Some(1)),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "ActivateGame")),
            },
            KeyCase {
                description: "Enter in box-selection mode with an out-of-range focus index still \
                    delegates to ActivateGame unconditionally (was: is_none, asserted directly by \
                    key_to_action before A13 - keys.rs no longer knows what's in range, so the \
                    focus_index/focusable_ids bounds check that produces 'does nothing' now lives \
                    entirely in ScoresTab::update, one layer down; end-user behavior is unchanged)",
                state: state_with_schedule(Tab::Scores, true),
                store: store_with_scores_focus(Some(99)),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_component_message(a, SCORES_TAB_PATH, "ActivateGame")),
            },
            KeyCase {
                description: "an unmapped key in box-selection mode does nothing",
                state: state_for(Tab::Scores, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Char('x')),
                check: Box::new(is_none),
            },
            // --- Standings tab: view-selection mode (no item focus) ---
            KeyCase {
                description: "Left in view-selection mode cycles the view left",
                state: state_for(Tab::Standings, true),
                store: empty_store(),
                key: key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "CycleViewLeft")),
            },
            KeyCase {
                description: "Right in view-selection mode cycles the view right",
                state: state_for(Tab::Standings, true),
                store: empty_store(),
                key: key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "CycleViewRight")),
            },
            KeyCase {
                description: "Down in view-selection mode enters browse mode",
                state: state_for(Tab::Standings, true),
                store: empty_store(),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "EnterBrowseMode")),
            },
            KeyCase {
                description: "Up in view-selection mode (no item focus) returns to the tab bar",
                state: state_for(Tab::Standings, true),
                store: empty_store(),
                key: key(KeyCode::Up),
                check: Box::new(|a| matches!(a, Some(Action::ExitContentFocus))),
            },
            KeyCase {
                description: "an unmapped key in view-selection mode does nothing",
                state: state_for(Tab::Standings, true),
                store: empty_store(),
                key: key(KeyCode::Char('x')),
                check: Box::new(is_none),
            },
            // --- Standings tab: browse mode (item focus active, document navigation) ---
            KeyCase {
                description: "Tab in browse mode focuses the next element",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Shift+Tab in browse mode focuses the previous element",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: shift_key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "BackTab in browse mode focuses the previous element",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::BackTab),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Enter in browse mode activates the focused team (not wrapped in DocNav)",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "ActivateTeam")),
            },
            KeyCase {
                description: "Up in browse mode focuses the previous element (reached via the Up-key fallthrough)",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Down in browse mode focuses the next element",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Left in browse mode focuses left (row navigation)",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusLeft)")),
            },
            KeyCase {
                description: "Right in browse mode focuses right (row navigation)",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(FocusRight)")),
            },
            KeyCase {
                description: "Shift+Down in browse mode scrolls down",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: shift_key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "Shift+Up in browse mode scrolls up",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: shift_key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "Shift+Left in browse mode scrolls up too, not left (surprising: it is not row navigation)",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: shift_key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "Shift+Right in browse mode scrolls down",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: shift_key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "PageUp in browse mode pages up",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::PageUp),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(PageUp)")),
            },
            KeyCase {
                description: "PageDown in browse mode pages down",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::PageDown),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(PageDown)")),
            },
            KeyCase {
                description: "Home in browse mode scrolls to the top",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Home),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(ScrollToTop)")),
            },
            KeyCase {
                description: "End in browse mode scrolls to the bottom",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::End),
                check: Box::new(|a| is_component_message(a, STANDINGS_TAB_PATH, "DocNav(ScrollToBottom)")),
            },
            KeyCase {
                description: "an unmapped key in browse mode does nothing",
                state: state_for(Tab::Standings, true),
                store: store_with_standings_focus(Some(0)),
                key: key(KeyCode::Char('x')),
                check: Box::new(is_none),
            },
            // --- Settings tab: normal mode (no modal open) ---
            KeyCase {
                description: "Left in Settings always navigates the category left",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Left),
                check: Box::new(|a| {
                    is_component_message_prefix(a, SETTINGS_TAB_PATH, "NavigateCategoryLeft(")
                }),
            },
            KeyCase {
                description: "Right in Settings always navigates the category right",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Right),
                check: Box::new(|a| {
                    is_component_message_prefix(a, SETTINGS_TAB_PATH, "NavigateCategoryRight(")
                }),
            },
            KeyCase {
                description: "Left in Settings navigates category left even with item focus active",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_focus(Some(0)),
                key: key(KeyCode::Left),
                check: Box::new(|a| {
                    is_component_message_prefix(a, SETTINGS_TAB_PATH, "NavigateCategoryLeft(")
                }),
            },
            KeyCase {
                description: "Tab in Settings focuses the next setting",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Shift+Tab in Settings focuses the previous setting",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: shift_key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "BackTab in Settings focuses the previous setting (F6 convergence: was an \
                    explicit no-op guard before key_to_nav_msg delegation, matching Standings/Demo now)",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::BackTab),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Down in Settings focuses the next setting",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Up in Settings focuses the previous setting - note: unlike Scores/Standings, Settings intercepts \
                    Up unconditionally (even with no item focus yet), so plain Up can never return Settings content focus to the tab bar; only ESC does",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Shift+Down in Settings scrolls down",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: shift_key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "Shift+Up in Settings scrolls up",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: shift_key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "PageUp in Settings pages up",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::PageUp),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(PageUp)")),
            },
            KeyCase {
                description: "PageDown in Settings pages down",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::PageDown),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(PageDown)")),
            },
            KeyCase {
                description: "Home in Settings scrolls to the top",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Home),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(ScrollToTop)")),
            },
            KeyCase {
                description: "End in Settings scrolls to the bottom",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::End),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "DocNav(ScrollToBottom)")),
            },
            KeyCase {
                description: "Enter in Settings activates the focused setting, carrying the current config",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_component_message_prefix(a, SETTINGS_TAB_PATH, "ActivateSetting(")),
            },
            KeyCase {
                description: "an unmapped key in Settings normal mode does nothing",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Char('x')),
                check: Box::new(is_none),
            },
            // --- Settings tab: modal open ---
            KeyCase {
                description: "Up while the settings modal is open navigates the modal selection up",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "Modal(Up)")),
            },
            KeyCase {
                description: "Down while the settings modal is open navigates the modal selection down",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "Modal(Down)")),
            },
            KeyCase {
                description: "Enter while the settings modal is open confirms the modal selection",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_component_message(a, SETTINGS_TAB_PATH, "Modal(Confirm)")),
            },
            KeyCase {
                description: "Left while the settings modal is open does nothing (category navigation is suppressed by the modal)",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Left),
                check: Box::new(is_none),
            },
            KeyCase {
                description: "Tab while the settings modal is open does nothing (not handled by the modal branch)",
                state: state_for(Tab::Settings, true),
                store: store_with_settings_modal_open(),
                key: key(KeyCode::Tab),
                check: Box::new(is_none),
            },
            // --- Cross-tab focus bleed regression: the scores-item-focus check used by
            // key_to_action's own Up-key special-case is gated on the current tab
            // being Scores, so stale ScoresTabState focus (which navigate_to_tab now
            // also clears, as defense in depth) can no longer swallow Up on a
            // different tab. ---
            KeyCase {
                description: "Up on the Standings tab returns to the tab bar even with stale Scores box-selection focus present",
                state: state_for(Tab::Standings, true),
                store: store_with_scores_focus(Some(0)),
                key: key(KeyCode::Up),
                check: Box::new(|a| matches!(a, Some(Action::ExitContentFocus))),
            },
        ]
    }

    /// Cases exercising the `#[cfg(feature = "development")]` Demo tab and the
    /// number-4 shortcut, which only exist when the development feature is enabled.
    #[cfg(feature = "development")]
    fn dev_cases() -> Vec<KeyCase> {
        vec![
            KeyCase {
                description: "'4' navigates to the Demo tab from the tab bar",
                state: state_for(Tab::Scores, false),
                store: empty_store(),
                key: key(KeyCode::Char('4')),
                check: Box::new(|a| is_navigate_tab(a, Tab::Demo)),
            },
            KeyCase {
                description: "'4' navigates to the Demo tab even while content-focused on another tab",
                state: state_for(Tab::Settings, true),
                store: empty_store(),
                key: key(KeyCode::Char('4')),
                check: Box::new(|a| is_navigate_tab(a, Tab::Demo)),
            },
            KeyCase {
                description: "ESC exits Demo tab focus when content-focused with no other nested mode active (priority 4.6)",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "ExitFocus")),
            },
            KeyCase {
                description: "ESC on the Demo tab exits Demo focus and ignores stale Settings item focus \
                    (cross-tab focus bleed regression: the settings check is gated on the current tab being Settings)",
                state: state_for(Tab::Demo, true),
                store: store_with_settings_focus(Some(0)),
                key: key(KeyCode::Esc),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "ExitFocus")),
            },
            KeyCase {
                description: "Tab in the Demo tab focuses the next element",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Shift+Tab in the Demo tab focuses the previous element",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: shift_key(KeyCode::Tab),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "BackTab in the Demo tab focuses the previous element",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::BackTab),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Enter in the Demo tab activates the focused link",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Enter),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "ActivateLink")),
            },
            KeyCase {
                description: "Left in the Demo tab focuses left",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusLeft)")),
            },
            KeyCase {
                description: "Right in the Demo tab focuses right",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusRight)")),
            },
            KeyCase {
                description: "Shift+Left in the Demo tab scrolls up, not left (F6 convergence: was an \
                    unconditional row-navigation override before key_to_nav_msg delegation, matching \
                    Standings browse mode now - scrolling wins over row navigation)",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: shift_key(KeyCode::Left),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "Shift+Right in the Demo tab scrolls down (F6 convergence: was an \
                    unconditional row-navigation override before key_to_nav_msg delegation)",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: shift_key(KeyCode::Right),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "Up in the Demo tab focuses the previous element (reached via the Up-key fallthrough)",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusPrev)")),
            },
            KeyCase {
                description: "Down in the Demo tab focuses the next element",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(FocusNext)")),
            },
            KeyCase {
                description: "Shift+Down in the Demo tab scrolls down",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: shift_key(KeyCode::Down),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(ScrollDown(1))")),
            },
            KeyCase {
                description: "Shift+Up in the Demo tab scrolls up",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: shift_key(KeyCode::Up),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(ScrollUp(1))")),
            },
            KeyCase {
                description: "PageUp in the Demo tab pages up",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::PageUp),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(PageUp)")),
            },
            KeyCase {
                description: "PageDown in the Demo tab pages down",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::PageDown),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(PageDown)")),
            },
            KeyCase {
                description: "Home in the Demo tab scrolls to the top",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Home),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(ScrollToTop)")),
            },
            KeyCase {
                description: "End in the Demo tab scrolls to the bottom",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::End),
                check: Box::new(|a| is_component_message(a, DEMO_TAB_PATH, "DocNav(ScrollToBottom)")),
            },
            KeyCase {
                description: "an unmapped key in the Demo tab does nothing",
                state: state_for(Tab::Demo, true),
                store: empty_store(),
                key: key(KeyCode::Char('x')),
                check: Box::new(is_none),
            },
        ]
    }

    #[test]
    fn test_key_to_action_table() {
        #[allow(unused_mut)]
        let mut cases = base_cases();
        #[cfg(feature = "development")]
        cases.extend(dev_cases());

        for case in cases {
            let action = key_to_action(case.key, &case.state, &case.store);
            assert!(
                (case.check)(&action),
                "case '{}' failed: got {:?}",
                case.description,
                action
            );
        }
    }

    // ---------------------------------------------------------------------
    // Standalone tests for behavior that doesn't fit the flat table above
    // ---------------------------------------------------------------------

    /// Without the development feature, '4' is not a mapped tab-switch key (the
    /// `Tab::Demo` variant and its number-key mapping don't exist), so it falls
    /// through to whatever the current focus-level handler does with an
    /// unrecognized character - a no-op from the tab bar.
    #[cfg(not(feature = "development"))]
    #[test]
    fn test_number_key_four_does_nothing_without_dev_feature() {
        let state = state_for(Tab::Scores, false);
        let store = empty_store();

        let action = key_to_action(key(KeyCode::Char('4')), &state, &store);

        assert!(action.is_none());
    }

    // Note: the two characterization tests that used to live here documented
    // dead duplicated match arms - `handle_settings_tab_keys`'s own
    // `KeyCode::Esc => ModalMsg::Cancel` arm and `handle_scores_tab_keys`'s
    // box-selection `KeyCode::Up` arm - both unreachable from `key_to_action`.
    // Those arms were deleted as part of the A13/A14 keys.rs refactor, so the
    // tests documenting them were removed too.
    //
    // Update (F6 convergence): step 6's Up special-case no longer intercepts
    // scores box-selection directly, so `Up` (plain and Shift) now reaches
    // `handle_scores_tab_keys` again - but via the canonical `key_to_nav_msg`
    // delegation, not a reinstated explicit `KeyCode::Up` match arm. See the
    // "Up in box-selection mode" cases in `base_cases()` above.
}
