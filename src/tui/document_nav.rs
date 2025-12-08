//! Generic document navigation logic
//!
//! This module provides reusable navigation logic for components that display
//! scrollable, focusable document-like content (e.g., StandingsTab, DemoTab).

use crate::tui::component::Effect;
use crate::tui::document::{FocusableId, LinkTarget, RowPosition};

/// Minimum viewport height - if smaller than this, autoscroll may behave oddly
const MIN_VIEWPORT_HEIGHT: u16 = 5;

/// Padding lines above/below focused element for autoscroll
const AUTOSCROLL_PADDING: u16 = 3;

/// Minimum page size for page up/down operations
const MIN_PAGE_SIZE: u16 = 10;

/// Direction for finding sibling in row navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowDirection {
    Left,
    Right,
}

/// Document navigation state
///
/// Components can embed this struct to get document navigation behavior.
/// All navigation functions in this module operate on this state.
#[derive(Debug, Clone, Default)]
pub struct DocumentNavState {
    pub focus_index: Option<usize>,
    pub scroll_offset: u16,
    pub viewport_height: u16,
    pub focusable_positions: Vec<u16>,
    pub focusable_heights: Vec<u16>,
    pub focusable_ids: Vec<FocusableId>,
    pub focusable_row_positions: Vec<Option<RowPosition>>,
    pub link_targets: Vec<Option<LinkTarget>>,
}

impl DocumentNavState {
    /// Get the link target at the current focus, if any
    pub fn focused_link_target(&self) -> Option<&LinkTarget> {
        let focus_idx = self.focus_index?;
        self.link_targets.get(focus_idx)?.as_ref()
    }
}

/// Document navigation messages
///
/// Components that use DocumentNavState can include these messages in their
/// own message enum to handle document navigation.
#[derive(Clone, Debug, PartialEq)]
pub enum DocumentNavMsg {
    FocusNext,
    FocusPrev,
    FocusLeft,
    FocusRight,
    ScrollUp(u16),
    ScrollDown(u16),
    ScrollToTop,
    ScrollToBottom,
    PageUp,
    PageDown,
    UpdateViewportHeight(u16),
}

/// Handle a document navigation message
///
/// Call this from your component's update() method to handle document navigation.
/// Returns Effect::None (navigation doesn't trigger side effects).
pub fn handle_message(state: &mut DocumentNavState, msg: &DocumentNavMsg) -> Effect {
    match msg {
        DocumentNavMsg::FocusNext => {
            let _wrapped = focus_next(state);
            autoscroll_to_focus(state);
        }
        DocumentNavMsg::FocusPrev => {
            let _wrapped = focus_prev(state);
            autoscroll_to_focus(state);
        }
        DocumentNavMsg::FocusLeft => {
            if let Some(new_idx) = find_row_sibling(state, RowDirection::Left) {
                state.focus_index = Some(new_idx);
                autoscroll_to_focus(state);
            }
        }
        DocumentNavMsg::FocusRight => {
            if let Some(new_idx) = find_row_sibling(state, RowDirection::Right) {
                state.focus_index = Some(new_idx);
                autoscroll_to_focus(state);
            }
        }
        DocumentNavMsg::ScrollUp(lines) => {
            scroll_up(state, *lines);
        }
        DocumentNavMsg::ScrollDown(lines) => {
            scroll_down(state, *lines);
        }
        DocumentNavMsg::ScrollToTop => {
            scroll_to_top(state);
        }
        DocumentNavMsg::ScrollToBottom => {
            scroll_to_bottom(state);
        }
        DocumentNavMsg::PageUp => {
            page_up(state);
        }
        DocumentNavMsg::PageDown => {
            page_down(state);
        }
        DocumentNavMsg::UpdateViewportHeight(height) => {
            state.viewport_height = *height;
        }
    }
    Effect::None
}

// ============================================================================
// Focus Navigation
// ============================================================================

/// Move focus to next element, wrapping from last to first
/// Returns true if wrapped around
pub fn focus_next(state: &mut DocumentNavState) -> bool {
    let focusable_count = state.focusable_positions.len();
    if focusable_count == 0 {
        return false;
    }

    match state.focus_index {
        None => {
            state.focus_index = Some(0);
            false // didn't wrap
        }
        Some(idx) if idx + 1 >= focusable_count => {
            state.focus_index = Some(0);
            state.scroll_offset = 0;
            true // wrapped
        }
        Some(idx) => {
            state.focus_index = Some(idx + 1);
            false
        }
    }
}

/// Move focus to previous element, wrapping from first to last
/// Returns true if wrapped around
pub fn focus_prev(state: &mut DocumentNavState) -> bool {
    let focusable_count = state.focusable_positions.len();
    if focusable_count == 0 {
        return false;
    }

    match state.focus_index {
        None => {
            state.focus_index = Some(focusable_count - 1);
            state.scroll_offset = u16::MAX;
            true // wrapped
        }
        Some(0) => {
            state.focus_index = Some(focusable_count - 1);
            state.scroll_offset = u16::MAX;
            true // wrapped
        }
        Some(idx) => {
            state.focus_index = Some(idx - 1);
            false
        }
    }
}

/// Find sibling element in the same row (left or right)
pub fn find_row_sibling(state: &DocumentNavState, direction: RowDirection) -> Option<usize> {
    let focus_idx = state.focus_index?;
    let row_positions = &state.focusable_row_positions;
    let current_row = row_positions.get(focus_idx)?.as_ref()?;

    // Find element in same row at same idx_within_child but different child_idx
    let target_child_idx = match direction {
        RowDirection::Left => {
            if current_row.child_idx == 0 {
                // Wrap to rightmost child
                row_positions
                    .iter()
                    .filter_map(|r| r.as_ref())
                    .filter(|r| r.row_y == current_row.row_y)
                    .map(|r| r.child_idx)
                    .max()?
            } else {
                current_row.child_idx - 1
            }
        }
        RowDirection::Right => {
            let max_child_idx = row_positions
                .iter()
                .filter_map(|r| r.as_ref())
                .filter(|r| r.row_y == current_row.row_y)
                .map(|r| r.child_idx)
                .max()?;

            if current_row.child_idx >= max_child_idx {
                // Wrap to leftmost child
                0
            } else {
                current_row.child_idx + 1
            }
        }
    };

    // Find element with matching row_y and target child_idx
    // Try exact idx_within_child match first, then fall back to closest
    let candidates: Vec<_> = row_positions
        .iter()
        .enumerate()
        .filter_map(|(idx, r)| {
            r.as_ref().and_then(|row| {
                if row.row_y == current_row.row_y && row.child_idx == target_child_idx {
                    Some((idx, row.idx_within_child))
                } else {
                    None
                }
            })
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Try exact match first
    if let Some((idx, _)) = candidates
        .iter()
        .find(|(_, inner_idx)| *inner_idx == current_row.idx_within_child)
    {
        return Some(*idx);
    }

    // Fall back to closest idx_within_child (handles different player counts)
    candidates
        .iter()
        .min_by_key(|(_, inner_idx)| {
            (*inner_idx as i32 - current_row.idx_within_child as i32).abs()
        })
        .map(|(idx, _)| *idx)
}

// ============================================================================
// Scrolling
// ============================================================================

/// Scroll up by N lines
pub fn scroll_up(state: &mut DocumentNavState, lines: u16) {
    state.scroll_offset = state.scroll_offset.saturating_sub(lines);
}

/// Scroll down by N lines
pub fn scroll_down(state: &mut DocumentNavState, lines: u16) {
    state.scroll_offset = state.scroll_offset.saturating_add(lines);
}

/// Scroll to top
pub fn scroll_to_top(state: &mut DocumentNavState) {
    state.scroll_offset = 0;
}

/// Scroll to bottom
pub fn scroll_to_bottom(state: &mut DocumentNavState) {
    state.scroll_offset = u16::MAX;
}

/// Page up (scroll by viewport height)
pub fn page_up(state: &mut DocumentNavState) {
    let page_size = state.viewport_height.max(MIN_PAGE_SIZE);
    state.scroll_offset = state.scroll_offset.saturating_sub(page_size);
}

/// Page down (scroll by viewport height)
pub fn page_down(state: &mut DocumentNavState) {
    let page_size = state.viewport_height.max(MIN_PAGE_SIZE);
    state.scroll_offset = state.scroll_offset.saturating_add(page_size);
}

/// Autoscroll to keep focused element visible
///
/// This function ensures the ENTIRE focused element is visible, not just its top.
/// For tall elements (like GameBox with height=7), we scroll enough to show the
/// full element including its bottom edge.
pub fn autoscroll_to_focus(state: &mut DocumentNavState) {
    let focus_idx = match state.focus_index {
        Some(idx) => idx,
        None => return,
    };

    let focused_y = match state.focusable_positions.get(focus_idx) {
        Some(&y) => y,
        None => return,
    };

    // Get element height (default to 1 for backwards compatibility)
    let focused_height = state
        .focusable_heights
        .get(focus_idx)
        .copied()
        .unwrap_or(1);

    let viewport_height = state.viewport_height.max(MIN_VIEWPORT_HEIGHT);
    let scroll_offset = state.scroll_offset;

    // Calculate viewport bounds
    let viewport_top = scroll_offset;
    let viewport_bottom = scroll_offset.saturating_add(viewport_height);

    // Calculate element bounds
    let element_top = focused_y;
    let element_bottom = focused_y.saturating_add(focused_height);

    // Only scroll if element is actually outside the viewport
    if element_top < viewport_top {
        // Element top is above viewport - scroll up to show it with padding
        let new_offset = element_top.saturating_sub(AUTOSCROLL_PADDING);
        state.scroll_offset = new_offset;
    } else if element_bottom > viewport_bottom {
        // Element bottom is below viewport - scroll down to show entire element
        // We want element_bottom to be at (viewport_bottom - padding)
        // So: new_offset + viewport_height - padding = element_bottom
        // Thus: new_offset = element_bottom - viewport_height + padding
        let new_offset = element_bottom
            .saturating_add(AUTOSCROLL_PADDING)
            .saturating_sub(viewport_height);
        state.scroll_offset = new_offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_next_wraps_around() {
        let mut state = DocumentNavState {
            focus_index: Some(2),
            scroll_offset: 0,
            viewport_height: 20,
            focusable_positions: vec![0, 5, 10],
            focusable_heights: vec![1, 1, 1],
            focusable_row_positions: vec![None, None, None],
            ..Default::default()
        };

        let wrapped = focus_next(&mut state);
        assert!(wrapped);
        assert_eq!(state.focus_index, Some(0));
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_focus_next_advances() {
        let mut state = DocumentNavState {
            focus_index: Some(0),
            scroll_offset: 0,
            viewport_height: 20,
            focusable_positions: vec![0, 5, 10],
            focusable_heights: vec![1, 1, 1],
            focusable_row_positions: vec![None, None, None],
            ..Default::default()
        };

        let wrapped = focus_next(&mut state);
        assert!(!wrapped);
        assert_eq!(state.focus_index, Some(1));
    }

    #[test]
    fn test_focus_prev_wraps_around() {
        let mut state = DocumentNavState {
            focus_index: Some(0),
            scroll_offset: 5,
            viewport_height: 20,
            focusable_positions: vec![0, 5, 10],
            focusable_heights: vec![1, 1, 1],
            focusable_row_positions: vec![None, None, None],
            ..Default::default()
        };

        let wrapped = focus_prev(&mut state);
        assert!(wrapped);
        assert_eq!(state.focus_index, Some(2));
        assert_eq!(state.scroll_offset, u16::MAX);
    }

    #[test]
    fn test_focus_prev_from_none() {
        let mut state = DocumentNavState {
            focus_index: None,
            scroll_offset: 0,
            viewport_height: 20,
            focusable_positions: vec![0, 5, 10],
            focusable_heights: vec![1, 1, 1],
            focusable_row_positions: vec![None, None, None],
            ..Default::default()
        };

        let wrapped = focus_prev(&mut state);
        assert!(wrapped);
        assert_eq!(state.focus_index, Some(2));
    }

    #[test]
    fn test_scroll_up() {
        let mut state = DocumentNavState {
            focus_index: None,
            scroll_offset: 10,
            viewport_height: 20,
            ..Default::default()
        };

        scroll_up(&mut state, 5);
        assert_eq!(state.scroll_offset, 5);

        scroll_up(&mut state, 10); // Should saturate at 0
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = DocumentNavState {
            scroll_offset: 10,
            viewport_height: 20,
            ..Default::default()
        };

        scroll_down(&mut state, 5);
        assert_eq!(state.scroll_offset, 15);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut state = DocumentNavState {
            scroll_offset: 100,
            viewport_height: 20,
            ..Default::default()
        };

        scroll_to_top(&mut state);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut state = DocumentNavState {
            scroll_offset: 0,
            viewport_height: 20,
            ..Default::default()
        };

        scroll_to_bottom(&mut state);
        assert_eq!(state.scroll_offset, u16::MAX);
    }

    #[test]
    fn test_page_up() {
        let mut state = DocumentNavState {
            scroll_offset: 50,
            viewport_height: 20,
            ..Default::default()
        };

        page_up(&mut state);
        assert_eq!(state.scroll_offset, 30);
    }

    #[test]
    fn test_page_down() {
        let mut state = DocumentNavState {
            scroll_offset: 30,
            viewport_height: 20,
            ..Default::default()
        };

        page_down(&mut state);
        assert_eq!(state.scroll_offset, 50);
    }

    #[test]
    fn test_autoscroll_to_focus_scrolls_down() {
        let mut state = DocumentNavState {
            focus_index: Some(5),
            scroll_offset: 0,
            viewport_height: 10,
            focusable_positions: vec![0, 2, 4, 6, 8, 20, 22], // Element 5 is at y=20
            focusable_heights: vec![1, 1, 1, 1, 1, 1, 1],
            focusable_row_positions: vec![None; 7],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Element at y=20, height=1, viewport=10, padding=3
        // element_bottom = 20 + 1 = 21
        // new_offset = 21 + 3 - 10 = 14
        assert_eq!(state.scroll_offset, 14);
    }

    #[test]
    fn test_autoscroll_to_focus_scrolls_up() {
        let mut state = DocumentNavState {
            focus_index: Some(0),
            scroll_offset: 10,
            viewport_height: 10,
            focusable_positions: vec![0, 2, 4, 6, 8, 20, 22], // Element 0 is at y=0
            focusable_heights: vec![1, 1, 1, 1, 1, 1, 1],
            focusable_row_positions: vec![None; 7],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Should scroll so element at y=0 is visible (with padding=3, new_offset = 0 - 3 saturates to 0)
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_autoscroll_no_scroll_when_visible() {
        // Element at y=5 is within viewport [0, 10)
        let mut state = DocumentNavState {
            focus_index: Some(2),
            scroll_offset: 0,
            viewport_height: 10,
            focusable_positions: vec![0, 2, 5, 8, 15], // Element 2 is at y=5
            focusable_heights: vec![1, 1, 1, 1, 1],
            focusable_row_positions: vec![None; 5],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Should NOT scroll - element is visible
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_autoscroll_no_scroll_when_near_bottom_but_visible() {
        // Regression test: element at y=8, height=1 is within viewport [0, 10)
        // element_bottom = 8 + 1 = 9, which is < viewport_bottom = 10
        let mut state = DocumentNavState {
            focus_index: Some(3),
            scroll_offset: 0,
            viewport_height: 10,
            focusable_positions: vec![0, 2, 5, 8, 15], // Element 3 is at y=8
            focusable_heights: vec![1, 1, 1, 1, 1],
            focusable_row_positions: vec![None; 5],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Should NOT scroll - element at y=8, height=1 fits in viewport [0, 10)
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_autoscroll_correct_offset_for_element_just_outside() {
        // Element at y=10, height=1 is just outside viewport [0, 10)
        let mut state = DocumentNavState {
            focus_index: Some(4),
            scroll_offset: 0,
            viewport_height: 10,
            focusable_positions: vec![0, 2, 5, 8, 10], // Element 4 is at y=10
            focusable_heights: vec![1, 1, 1, 1, 1],
            focusable_row_positions: vec![None; 5],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // element_bottom = 10 + 1 = 11, padding = 3
        // new_offset = 11 + 3 - 10 = 4
        // After scroll: viewport is [4, 14), element at y=10 with height=1 is visible
        assert_eq!(state.scroll_offset, 4);
    }

    #[test]
    fn test_autoscroll_tall_element_gamebox() {
        // GameBox elements are 7 lines tall. When navigating to the third row (y=14),
        // we need to scroll enough to show the ENTIRE element (y=14 through y=20).
        // This is a regression test for the bug where only element top was considered,
        // causing the bottom of tall elements to be cut off.
        let mut state = DocumentNavState {
            focus_index: Some(6), // Third row, first game
            scroll_offset: 0,
            viewport_height: 20,
            // Row 1 at y=0, Row 2 at y=7, Row 3 at y=14 (3 games per row)
            focusable_positions: vec![0, 0, 0, 7, 7, 7, 14, 14, 14],
            focusable_heights: vec![7, 7, 7, 7, 7, 7, 7, 7, 7], // GameBox height = 7
            focusable_row_positions: vec![None; 9],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Element at y=14, height=7
        // element_bottom = 14 + 7 = 21
        // With viewport_height=20 and padding=3:
        // new_offset = element_bottom + padding - viewport_height = 21 + 3 - 20 = 4
        // After scroll: viewport is [4, 24), element at y=14..21 is fully visible
        assert_eq!(state.scroll_offset, 4);
    }

    #[test]
    fn test_autoscroll_tall_element_already_visible() {
        // When a tall element (height=7) is already fully visible, don't scroll
        let mut state = DocumentNavState {
            focus_index: Some(0),
            scroll_offset: 0,
            viewport_height: 20,
            focusable_positions: vec![0, 7, 14],
            focusable_heights: vec![7, 7, 7],
            focusable_row_positions: vec![None; 3],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Element at y=0, height=7, element_bottom=7
        // viewport is [0, 20), element is at [0, 7)
        // Element is fully visible, no scroll needed
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_autoscroll_tall_element_partially_visible_bottom_cut() {
        // When bottom of tall element is cut off, scroll to show full element
        let mut state = DocumentNavState {
            focus_index: Some(2), // Element at y=14
            scroll_offset: 0,
            viewport_height: 18, // viewport ends at y=18, but element bottom is at y=21
            focusable_positions: vec![0, 7, 14],
            focusable_heights: vec![7, 7, 7],
            focusable_row_positions: vec![None; 3],
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Element at y=14, height=7, element_bottom=21
        // With viewport_height=18 and padding=3:
        // new_offset = 21 + 3 - 18 = 6
        assert_eq!(state.scroll_offset, 6);
    }
}
