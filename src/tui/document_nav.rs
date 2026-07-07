//! Generic document navigation logic
//!
//! This module provides reusable navigation logic for components that display
//! scrollable, focusable document-like content (e.g., StandingsTab, DemoTab).

use std::collections::HashMap;

use crate::tui::component::Effect;
use crate::tui::document::{Document, FocusContext, FocusableElement, LinkTarget};

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
    /// All focusable elements in document order, carrying position, height,
    /// ID, row membership, and link target together. Replaces five
    /// index-aligned Vecs that used to be synced by hand at every call
    /// site -- a site that forgets a field is no longer representable.
    pub focusables: Vec<FocusableElement>,
    /// Active tab selections for Tabs elements (tabs_id -> active_index)
    pub doc_tab_selections: HashMap<String, usize>,
}

impl DocumentNavState {
    /// Rebuild `focusables` from `doc`, built with `ctx`.
    ///
    /// This is the single sync path: every call site that needs to refresh
    /// navigation metadata (after data loads, a view/category changes, or
    /// the layout width changes) goes through this one function instead of
    /// hand-copying individual fields.
    pub fn sync_focusables(&mut self, doc: &dyn Document, ctx: &FocusContext) {
        self.focusables = doc.focusables(ctx);
    }

    /// Get the link target at the current focus, if any
    pub fn focused_link_target(&self) -> Option<&LinkTarget> {
        let focus_idx = self.focus_index?;
        self.focusables.get(focus_idx)?.link_target.as_ref()
    }

    /// Get the active tab index for a tabs element
    pub fn get_tab_selection(&self, tabs_id: &str) -> usize {
        self.doc_tab_selections.get(tabs_id).copied().unwrap_or(0)
    }

    /// Set the active tab index for a tabs element
    pub fn set_tab_selection(&mut self, tabs_id: impl Into<String>, index: usize) {
        self.doc_tab_selections.insert(tabs_id.into(), index);
    }

    /// Switch to the next tab for a tabs element
    /// Returns true if switched (for potential state invalidation)
    pub fn next_tab(&mut self, tabs_id: &str, tab_count: usize) -> bool {
        if tab_count == 0 {
            return false;
        }
        let current = self.get_tab_selection(tabs_id);
        let next = if current + 1 >= tab_count {
            0
        } else {
            current + 1
        };
        if next != current {
            self.doc_tab_selections.insert(tabs_id.to_string(), next);
            true
        } else {
            false
        }
    }

    /// Switch to the previous tab for a tabs element
    /// Returns true if switched (for potential state invalidation)
    pub fn prev_tab(&mut self, tabs_id: &str, tab_count: usize) -> bool {
        if tab_count == 0 {
            return false;
        }
        let current = self.get_tab_selection(tabs_id);
        let prev = if current == 0 {
            tab_count.saturating_sub(1)
        } else {
            current - 1
        };
        if prev != current {
            self.doc_tab_selections.insert(tabs_id.to_string(), prev);
            true
        } else {
            false
        }
    }

    /// Focus the first focusable item
    pub fn focus_first_item(&mut self) {
        if !self.focusables.is_empty() {
            self.focus_index = Some(0);
        }
    }

    /// Clear the current item focus
    pub fn clear_item_focus(&mut self) {
        self.focus_index = None;
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
    let focusable_count = state.focusables.len();
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
    let focusable_count = state.focusables.len();
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
    let current_row = state.focusables.get(focus_idx)?.row_position?;

    // Find element in same row at same idx_within_child but different child_idx
    let target_child_idx = match direction {
        RowDirection::Left => {
            if current_row.child_idx == 0 {
                // Wrap to rightmost child
                state
                    .focusables
                    .iter()
                    .filter_map(|f| f.row_position)
                    .filter(|r| r.row_y == current_row.row_y)
                    .map(|r| r.child_idx)
                    .max()?
            } else {
                current_row.child_idx - 1
            }
        }
        RowDirection::Right => {
            let max_child_idx = state
                .focusables
                .iter()
                .filter_map(|f| f.row_position)
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
    let candidates: Vec<_> = state
        .focusables
        .iter()
        .enumerate()
        .filter_map(|(idx, f)| {
            f.row_position.and_then(|row| {
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

    let Some(focused) = state.focusables.get(focus_idx) else {
        return;
    };
    let focused_y = focused.y;
    let focused_height = focused.height;

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
    use crate::tui::document::FocusableId;

    /// Build a `focusables` Vec from `(y, height)` pairs, with synthetic
    /// index-based IDs and no row positions -- enough for the navigation
    /// math tests below, which only look at position/height/count.
    fn uniform_focusables(entries: &[(u16, u16)]) -> Vec<FocusableElement> {
        entries
            .iter()
            .enumerate()
            .map(|(i, &(y, height))| {
                FocusableElement::at(y, height, FocusableId::link(format!("f{i}")))
            })
            .collect()
    }

    #[test]
    fn test_focus_next_no_focusables_is_noop() {
        // Ported from the deleted DocumentView::focus_next (Engine A) test of the same
        // edge case: navigating an empty document must not panic or set a focus index.
        let mut state = DocumentNavState::default();

        let wrapped = focus_next(&mut state);

        assert!(!wrapped);
        assert_eq!(state.focus_index, None);
    }

    #[test]
    fn test_focus_prev_no_focusables_is_noop() {
        // Ported from the deleted DocumentView::focus_prev (Engine A) test of the same
        // edge case.
        let mut state = DocumentNavState::default();

        let wrapped = focus_prev(&mut state);

        assert!(!wrapped);
        assert_eq!(state.focus_index, None);
    }

    #[test]
    fn test_focus_next_wraps_around() {
        let mut state = DocumentNavState {
            focus_index: Some(2),
            scroll_offset: 0,
            viewport_height: 20,
            focusables: uniform_focusables(&[(0, 1), (5, 1), (10, 1)]),
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
            focusables: uniform_focusables(&[(0, 1), (5, 1), (10, 1)]),
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
            focusables: uniform_focusables(&[(0, 1), (5, 1), (10, 1)]),
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
            focusables: uniform_focusables(&[(0, 1), (5, 1), (10, 1)]),
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
            // Element 5 is at y=20
            focusables: uniform_focusables(&[
                (0, 1),
                (2, 1),
                (4, 1),
                (6, 1),
                (8, 1),
                (20, 1),
                (22, 1),
            ]),
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
            // Element 0 is at y=0
            focusables: uniform_focusables(&[
                (0, 1),
                (2, 1),
                (4, 1),
                (6, 1),
                (8, 1),
                (20, 1),
                (22, 1),
            ]),
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
            // Element 2 is at y=5
            focusables: uniform_focusables(&[(0, 1), (2, 1), (5, 1), (8, 1), (15, 1)]),
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
            // Element 3 is at y=8
            focusables: uniform_focusables(&[(0, 1), (2, 1), (5, 1), (8, 1), (15, 1)]),
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
            // Element 4 is at y=10
            focusables: uniform_focusables(&[(0, 1), (2, 1), (5, 1), (8, 1), (10, 1)]),
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
            // Row 1 at y=0, Row 2 at y=7, Row 3 at y=14 (3 games per row), GameBox height = 7
            focusables: uniform_focusables(&[
                (0, 7),
                (0, 7),
                (0, 7),
                (7, 7),
                (7, 7),
                (7, 7),
                (14, 7),
                (14, 7),
                (14, 7),
            ]),
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
            focusables: uniform_focusables(&[(0, 7), (7, 7), (14, 7)]),
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
            focusables: uniform_focusables(&[(0, 7), (7, 7), (14, 7)]),
            ..Default::default()
        };

        autoscroll_to_focus(&mut state);
        // Element at y=14, height=7, element_bottom=21
        // With viewport_height=18 and padding=3:
        // new_offset = 21 + 3 - 18 = 6
        assert_eq!(state.scroll_offset, 6);
    }

    #[test]
    fn test_find_row_sibling_moves_right_within_row() {
        use crate::tui::document::RowPosition;

        // Two rows of 3 columns each, all sharing row_y per row.
        let focusables = vec![
            FocusableElement::at(0, 1, FocusableId::link("r0c0")).with_row_position(RowPosition {
                row_y: 0,
                child_idx: 0,
                idx_within_child: 0,
            }),
            FocusableElement::at(0, 1, FocusableId::link("r0c1")).with_row_position(RowPosition {
                row_y: 0,
                child_idx: 1,
                idx_within_child: 0,
            }),
            FocusableElement::at(0, 1, FocusableId::link("r0c2")).with_row_position(RowPosition {
                row_y: 0,
                child_idx: 2,
                idx_within_child: 0,
            }),
        ];
        let state = DocumentNavState {
            focus_index: Some(0),
            focusables,
            ..Default::default()
        };

        assert_eq!(find_row_sibling(&state, RowDirection::Right), Some(1));
    }

    #[test]
    fn test_find_row_sibling_wraps_left_to_rightmost_column() {
        use crate::tui::document::RowPosition;

        let focusables = vec![
            FocusableElement::at(0, 1, FocusableId::link("r0c0")).with_row_position(RowPosition {
                row_y: 0,
                child_idx: 0,
                idx_within_child: 0,
            }),
            FocusableElement::at(0, 1, FocusableId::link("r0c1")).with_row_position(RowPosition {
                row_y: 0,
                child_idx: 1,
                idx_within_child: 0,
            }),
        ];
        let state = DocumentNavState {
            focus_index: Some(0),
            focusables,
            ..Default::default()
        };

        assert_eq!(find_row_sibling(&state, RowDirection::Left), Some(1));
    }

    #[test]
    fn test_find_row_sibling_none_when_no_row_position() {
        let state = DocumentNavState {
            focus_index: Some(0),
            focusables: uniform_focusables(&[(0, 1)]),
            ..Default::default()
        };

        assert_eq!(find_row_sibling(&state, RowDirection::Right), None);
    }

    #[test]
    fn test_sync_focusables_rebuilds_from_document() {
        use crate::tui::document::{Document, DocumentElement, FocusContext};

        struct FakeDoc;
        impl Document for FakeDoc {
            fn build(&self, _focus: &FocusContext) -> Vec<DocumentElement> {
                vec![DocumentElement::link(
                    "a",
                    "A",
                    LinkTarget::Anchor("a".to_string()),
                )]
            }
            fn title(&self) -> String {
                "fake".to_string()
            }
            fn id(&self) -> String {
                "fake".to_string()
            }
        }

        // Start with stale metadata that must be fully replaced, not merged.
        let mut state = DocumentNavState {
            focusables: uniform_focusables(&[(0, 1), (5, 1)]),
            ..Default::default()
        };

        state.sync_focusables(&FakeDoc, &FocusContext::default());

        assert_eq!(state.focusables.len(), 1);
        assert_eq!(state.focusables[0].id, FocusableId::link("a"));
        assert_eq!(
            state.focusables[0].link_target,
            Some(LinkTarget::Anchor("a".to_string()))
        );
    }
}
