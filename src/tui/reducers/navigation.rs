use tracing::{debug, trace};

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::state::AppState;
use crate::tui::types::Tab;

/// Handle all navigation-related actions
///
/// Returns Ok((new_state, effect)) if the action was handled,
/// or Err(state) to pass ownership back to the caller.
#[allow(clippy::result_large_err)]
pub fn reduce_navigation(state: AppState, action: &Action) -> Result<(AppState, Effect), AppState> {
    match action {
        Action::NavigateTab(tab) => Ok(navigate_to_tab(state, *tab)),
        Action::NavigateTabLeft => Ok(navigate_tab_left(state)),
        Action::NavigateTabRight => Ok(navigate_tab_right(state)),
        Action::EnterContentFocus => Ok(enter_content_focus(state)),
        Action::ExitContentFocus => Ok(exit_content_focus(state)),
        Action::NavigateUp => Ok(navigate_up(state)),
        Action::ToggleCommandPalette => Ok((state, Effect::None)),
        _ => Err(state),
    }
}

fn navigate_to_tab(state: AppState, tab: Tab) -> (AppState, Effect) {
    trace!("Navigating to tab: {:?}", tab);
    let mut new_state = state;
    new_state.navigation.current_tab = tab;
    new_state.navigation.document_stack.clear();
    new_state.navigation.content_focused = false; // Return focus to tab bar
    trace!("  Cleared document stack and returned focus to tab bar");
    (new_state, Effect::None)
}

fn navigate_tab_left(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.navigation.current_tab = match new_state.navigation.current_tab {
        Tab::Scores => Tab::Demo,
        Tab::Standings => Tab::Scores,
        Tab::Settings => Tab::Standings,
        Tab::Demo => Tab::Settings,
    };
    new_state.navigation.document_stack.clear();
    new_state.navigation.content_focused = false; // Return focus to tab bar
    (new_state, Effect::None)
}

fn navigate_tab_right(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;
    new_state.navigation.current_tab = match new_state.navigation.current_tab {
        Tab::Scores => Tab::Standings,
        Tab::Standings => Tab::Settings,
        Tab::Settings => Tab::Demo,
        Tab::Demo => Tab::Scores,
    };
    new_state.navigation.document_stack.clear();
    new_state.navigation.content_focused = false; // Return focus to tab bar
    (new_state, Effect::None)
}

fn enter_content_focus(state: AppState) -> (AppState, Effect) {
    debug!("FOCUS: Entering content focus (Down key from tab bar)");
    let mut new_state = state;
    new_state.navigation.content_focused = true;

    // Set tab-specific status message and initialize focus for Demo tab
    if new_state.navigation.current_tab == Tab::Demo {
        new_state
            .system
            .set_status_message("↑↓: move selection  Shift+↑↓: scroll  Esc: go back".to_string());

        // Demo tab focus is managed by component state
        // Component initializes focus to first element when needed
    }

    (new_state, Effect::None)
}

fn exit_content_focus(state: AppState) -> (AppState, Effect) {
    debug!("FOCUS: Exiting content focus (Up key to tab bar)");
    let mut new_state = state;

    // Reset status message if exiting from Demo tab
    if new_state.navigation.current_tab == Tab::Demo {
        new_state.system.reset_status_message();
    }

    new_state.navigation.content_focused = false;

    (new_state, Effect::None)
}

/// Unified "navigate up" action (ESC key)
///
/// Hierarchical fallthrough:
/// 1. If document stack not empty → pop document
/// 2. If content_focused → set content_focused = false
/// 3. Otherwise do nothing (already at top level)
///
/// Components may intercept NavigateUp to handle their own modes (e.g., exit browse mode,
/// close modal) before falling through to exit_content_focus.
fn navigate_up(state: AppState) -> (AppState, Effect) {
    let mut new_state = state;

    // 1. If document stack not empty → pop document
    if !new_state.navigation.document_stack.is_empty() {
        debug!("NAVIGATE_UP: Popping document from stack");
        new_state.navigation.document_stack.pop();
        return (new_state, Effect::None);
    }

    // 2. If content_focused → exit content focus
    if new_state.navigation.content_focused {
        debug!("NAVIGATE_UP: Exiting content focus");

        // Reset status message if exiting from Demo tab
        if new_state.navigation.current_tab == Tab::Demo {
            new_state.system.reset_status_message();
        }

        new_state.navigation.content_focused = false;
        return (new_state, Effect::None);
    }

    // 3. At top level, do nothing
    debug!("NAVIGATE_UP: Already at top level, ignoring");
    (new_state, Effect::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigate_to_tab() {
        let state = AppState::default();
        let (new_state, _) = navigate_to_tab(state, Tab::Settings);

        assert_eq!(new_state.navigation.current_tab, Tab::Settings);
        assert!(new_state.navigation.document_stack.is_empty());
        assert!(!new_state.navigation.content_focused);
    }

    #[test]
    fn test_tab_left_navigation_cycles() {
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Scores;

        let (state, _) = navigate_tab_left(state);
        assert_eq!(state.navigation.current_tab, Tab::Demo);

        let (state, _) = navigate_tab_left(state);
        assert_eq!(state.navigation.current_tab, Tab::Settings);
    }

    #[test]
    fn test_tab_right_navigation_cycles() {
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Demo;

        let (state, _) = navigate_tab_right(state);
        assert_eq!(state.navigation.current_tab, Tab::Scores);

        let (state, _) = navigate_tab_right(state);
        assert_eq!(state.navigation.current_tab, Tab::Standings);
    }

    #[test]
    fn test_navigate_up_pops_document_stack() {
        use crate::tui::state::DocumentStackEntry;
        use crate::tui::types::StackedDocument;

        let mut state = AppState::default();
        state.navigation.content_focused = true;
        state
            .navigation
            .document_stack
            .push(DocumentStackEntry::new(StackedDocument::TeamDetail {
                abbrev: "BOS".to_string(),
            }));

        let (new_state, _) = navigate_up(state);

        // Should pop the document, not exit content focus
        assert!(new_state.navigation.document_stack.is_empty());
        assert!(new_state.navigation.content_focused);
    }

    #[test]
    fn test_navigate_up_exits_content_focus_when_stack_empty() {
        let mut state = AppState::default();
        state.navigation.content_focused = true;

        let (new_state, _) = navigate_up(state);

        assert!(!new_state.navigation.content_focused);
    }

    #[test]
    fn test_navigate_up_does_nothing_at_top_level() {
        let state = AppState::default();
        assert!(!state.navigation.content_focused);
        assert!(state.navigation.document_stack.is_empty());

        let (new_state, _) = navigate_up(state.clone());

        // Should be unchanged
        assert!(!new_state.navigation.content_focused);
        assert!(new_state.navigation.document_stack.is_empty());
    }
}
