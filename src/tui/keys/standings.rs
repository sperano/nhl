//! Standings tab key handling (view-selection mode vs browse/document mode).

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::action::Action;
use crate::tui::components::standings_tab::StandingsTabMsg;
use crate::tui::constants::STANDINGS_TAB_PATH;
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::AppState;

/// Handle League standings navigation with document system
///
/// Delegates the arrow/Tab/page-navigation mapping to the canonical
/// `nav_handler::key_to_nav_msg`, which matches this context's mapping exactly
/// (including Shift+Left/Right scrolling instead of row navigation). Enter is
/// handled separately since it activates the focused team rather than emitting
/// a `DocumentNavMsg`.
pub fn handle_standings_league_keys(key: KeyEvent, _state: &AppState) -> Option<Action> {
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
pub fn handle_standings_tab_keys(key_code: KeyCode, _state: &AppState) -> Option<Action> {
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
