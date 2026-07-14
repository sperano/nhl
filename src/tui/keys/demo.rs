//! Demo tab key handling (development feature only).

use crossterm::event::{KeyCode, KeyEvent};
use tracing::debug;

use crate::tui::action::Action;
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::AppState;

/// Handle Demo tab navigation
pub fn handle_demo_tab_keys(key: KeyEvent, _state: &AppState) -> Option<Action> {
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
