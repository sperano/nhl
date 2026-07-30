use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::fixtures::create_mock_schedule;
use crate::tui::action::Action;
use crate::tui::component_store::ComponentStateStore;
use crate::tui::components::scores_tab::ScoresTabState;
use crate::tui::components::settings_tab::{ModalState, SettingsTabState};
use crate::tui::components::standings_tab::StandingsTabState;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::document_nav::DocumentNavState;
use crate::tui::state::{AppState, DocumentStackEntry};
use crate::tui::types::{StackedDocument, Tab};

use super::key_to_action;

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
            season: None,
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

/// Predicate over the `Action` produced by a `KeyCase`, boxed so each case can
/// carry its own closure.
type ActionCheck = Box<dyn Fn(&Option<Action>) -> bool>;

/// One row of the `key_to_action` behavior table: a named context, an input
/// key event, and a check on the resulting action. Kept as a flat list (rather
/// than nested per-context test functions) so this table can double as a
/// behavior-preservation harness for a future keys.rs refactor.
struct KeyCase {
    description: &'static str,
    state: AppState,
    store: ComponentStateStore,
    key: KeyEvent,
    check: ActionCheck,
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
