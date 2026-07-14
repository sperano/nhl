//! Scores tab key handling (date-navigation mode vs box-selection mode).

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::action::Action;
use crate::tui::component_store::ComponentStateStore;
use crate::tui::components::scores_tab::ScoresTabMsg;
use crate::tui::constants::SCORES_TAB_PATH;
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::AppState;

use super::focus::has_scores_item_focus;

/// Handle Scores tab navigation (box selection mode vs date mode)
///
/// Box-selection mode matches the canonical `key_to_nav_msg` mapping exactly
/// (Tab/BackTab focus cycling, PageUp/PageDown, Home/End, Shift+arrow
/// scrolling all apply here); only `Enter` is special-cased, since it
/// activates the focused game rather than emitting a `DocumentNavMsg`.
/// Date-navigation mode (no item focus) is untouched - its `Left`/`Right` are
/// date semantics, not document navigation.
pub fn handle_scores_tab_keys(
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
