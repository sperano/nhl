//! Common navigation handler for document-based components
//!
//! This module provides shared logic for converting key events into DocumentNavMsg
//! to eliminate duplication across components and document handlers.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::document_nav::DocumentNavMsg;

/// Convert a KeyEvent to a DocumentNavMsg for standard document navigation
///
/// This handles the common navigation patterns used across:
/// - Stacked document handlers (Boxscore, TeamDetail, PlayerDetail)
/// - Browse mode in StandingsTab and SettingsTab
///
/// Returns Some(DocumentNavMsg) if the key should be handled as navigation,
/// None if the key is not recognized as a navigation key.
///
/// # Navigation Keys (without Shift)
/// - Tab: FocusNext
/// - BackTab: FocusPrev
/// - Up: FocusPrev
/// - Down: FocusNext
/// - Left: FocusLeft (row navigation)
/// - Right: FocusRight (row navigation)
///
/// # Scrolling Keys (with Shift modifier)
/// - Shift+Up: ScrollUp(1)
/// - Shift+Down: ScrollDown(1)
/// - Shift+Left: ScrollUp(1)
/// - Shift+Right: ScrollDown(1)
///
/// # Page Navigation
/// - PageUp: PageUp
/// - PageDown: PageDown
/// - Home: ScrollToTop
/// - End: ScrollToBottom
///
/// # Non-navigation Keys
/// - Enter, Esc, and other keys return None
pub fn key_to_nav_msg(key: KeyEvent) -> Option<DocumentNavMsg> {
    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        // Tab key for focus navigation
        KeyCode::Tab => {
            if has_shift {
                Some(DocumentNavMsg::FocusPrev)
            } else {
                Some(DocumentNavMsg::FocusNext)
            }
        }
        KeyCode::BackTab => Some(DocumentNavMsg::FocusPrev),

        // Up/Down arrows - focus when no shift, scroll when shift
        KeyCode::Up => {
            if has_shift {
                Some(DocumentNavMsg::ScrollUp(1))
            } else {
                Some(DocumentNavMsg::FocusPrev)
            }
        }
        KeyCode::Down => {
            if has_shift {
                Some(DocumentNavMsg::ScrollDown(1))
            } else {
                Some(DocumentNavMsg::FocusNext)
            }
        }

        // Left/Right arrows - row navigation when no shift, scroll when shift
        KeyCode::Left => {
            if has_shift {
                Some(DocumentNavMsg::ScrollUp(1))
            } else {
                Some(DocumentNavMsg::FocusLeft)
            }
        }
        KeyCode::Right => {
            if has_shift {
                Some(DocumentNavMsg::ScrollDown(1))
            } else {
                Some(DocumentNavMsg::FocusRight)
            }
        }

        // Page navigation
        KeyCode::PageUp => Some(DocumentNavMsg::PageUp),
        KeyCode::PageDown => Some(DocumentNavMsg::PageDown),
        KeyCode::Home => Some(DocumentNavMsg::ScrollToTop),
        KeyCode::End => Some(DocumentNavMsg::ScrollToBottom),

        // Not a navigation key
        _ => None,
    }
}

/// Convert a KeyEvent to a DocumentNavMsg for simple up/down navigation
///
/// This is a simplified version used by stacked document handlers that only
/// support up/down navigation (no shift-scroll, no page keys).
///
/// Returns Some(DocumentNavMsg) for:
/// - Up: FocusPrev
/// - Down: FocusNext
///
/// Returns None for all other keys (including Enter, which is handled separately).
#[cfg(test)]
fn key_to_simple_nav_msg(key: KeyEvent) -> Option<DocumentNavMsg> {
    match key.code {
        KeyCode::Up => Some(DocumentNavMsg::FocusPrev),
        KeyCode::Down => Some(DocumentNavMsg::FocusNext),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_nav_msg_tab() {
        let key = KeyEvent::from(KeyCode::Tab);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusNext));
    }

    #[test]
    fn test_key_to_nav_msg_shift_tab() {
        let mut key = KeyEvent::from(KeyCode::Tab);
        key.modifiers = KeyModifiers::SHIFT;
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusPrev));
    }

    #[test]
    fn test_key_to_nav_msg_backtab() {
        let key = KeyEvent::from(KeyCode::BackTab);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusPrev));
    }

    #[test]
    fn test_key_to_nav_msg_up() {
        let key = KeyEvent::from(KeyCode::Up);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusPrev));
    }

    #[test]
    fn test_key_to_nav_msg_down() {
        let key = KeyEvent::from(KeyCode::Down);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusNext));
    }

    #[test]
    fn test_key_to_nav_msg_shift_up() {
        let mut key = KeyEvent::from(KeyCode::Up);
        key.modifiers = KeyModifiers::SHIFT;
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::ScrollUp(1)));
    }

    #[test]
    fn test_key_to_nav_msg_shift_down() {
        let mut key = KeyEvent::from(KeyCode::Down);
        key.modifiers = KeyModifiers::SHIFT;
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::ScrollDown(1)));
    }

    #[test]
    fn test_key_to_nav_msg_left() {
        let key = KeyEvent::from(KeyCode::Left);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusLeft));
    }

    #[test]
    fn test_key_to_nav_msg_right() {
        let key = KeyEvent::from(KeyCode::Right);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::FocusRight));
    }

    #[test]
    fn test_key_to_nav_msg_shift_left() {
        let mut key = KeyEvent::from(KeyCode::Left);
        key.modifiers = KeyModifiers::SHIFT;
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::ScrollUp(1)));
    }

    #[test]
    fn test_key_to_nav_msg_shift_right() {
        let mut key = KeyEvent::from(KeyCode::Right);
        key.modifiers = KeyModifiers::SHIFT;
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::ScrollDown(1)));
    }

    #[test]
    fn test_key_to_nav_msg_pageup() {
        let key = KeyEvent::from(KeyCode::PageUp);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::PageUp));
    }

    #[test]
    fn test_key_to_nav_msg_pagedown() {
        let key = KeyEvent::from(KeyCode::PageDown);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::PageDown));
    }

    #[test]
    fn test_key_to_nav_msg_home() {
        let key = KeyEvent::from(KeyCode::Home);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::ScrollToTop));
    }

    #[test]
    fn test_key_to_nav_msg_end() {
        let key = KeyEvent::from(KeyCode::End);
        assert_eq!(key_to_nav_msg(key), Some(DocumentNavMsg::ScrollToBottom));
    }

    #[test]
    fn test_key_to_nav_msg_enter_returns_none() {
        let key = KeyEvent::from(KeyCode::Enter);
        assert_eq!(key_to_nav_msg(key), None);
    }

    #[test]
    fn test_key_to_nav_msg_esc_returns_none() {
        let key = KeyEvent::from(KeyCode::Esc);
        assert_eq!(key_to_nav_msg(key), None);
    }

    #[test]
    fn test_key_to_simple_nav_msg_up() {
        let key = KeyEvent::from(KeyCode::Up);
        assert_eq!(key_to_simple_nav_msg(key), Some(DocumentNavMsg::FocusPrev));
    }

    #[test]
    fn test_key_to_simple_nav_msg_down() {
        let key = KeyEvent::from(KeyCode::Down);
        assert_eq!(key_to_simple_nav_msg(key), Some(DocumentNavMsg::FocusNext));
    }

    #[test]
    fn test_key_to_simple_nav_msg_enter_returns_none() {
        let key = KeyEvent::from(KeyCode::Enter);
        assert_eq!(key_to_simple_nav_msg(key), None);
    }

    #[test]
    fn test_key_to_simple_nav_msg_left_returns_none() {
        let key = KeyEvent::from(KeyCode::Left);
        assert_eq!(key_to_simple_nav_msg(key), None);
    }

    #[test]
    fn test_key_to_simple_nav_msg_tab_returns_none() {
        let key = KeyEvent::from(KeyCode::Tab);
        assert_eq!(key_to_simple_nav_msg(key), None);
    }
}
