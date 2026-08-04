//! Generic document navigation logic
//!
//! This module provides reusable navigation logic for components that display
//! scrollable, focusable document-like content (e.g., StandingsTab, DemoTab).

use std::collections::HashMap;

use crate::tui::component::Effect;
use crate::tui::document::{
    Document, FocusContext, FocusableElement, FocusableId, LinkTarget, RowPosition,
};

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

    /// Resolve `focus_index` into the focused element's ID, if any.
    ///
    /// This is what the render path consumes ([`DocumentView::focus_id`]):
    /// `focus_index` is only meaningful relative to `focusables`, and both
    /// live here, so the index never has to leave this struct.
    ///
    /// [`DocumentView::focus_id`]: crate::tui::document::DocumentView::focus_id
    pub fn focused_id(&self) -> Option<FocusableId> {
        Some(self.focusables.get(self.focus_index?)?.id.clone())
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

    let target_child_idx = row_sibling_child_idx(state, current_row, direction)?;

    row_element_at_child_idx(state, current_row, target_child_idx)
}

/// Determine which child (column) in the row to move focus to, wrapping
/// from the last child back to the first (or vice versa).
fn row_sibling_child_idx(
    state: &DocumentNavState,
    current_row: RowPosition,
    direction: RowDirection,
) -> Option<usize> {
    let row_child_indices = || {
        state
            .focusables
            .iter()
            .filter_map(|f| f.row_position)
            .filter(|r| r.row_y == current_row.row_y)
            .map(|r| r.child_idx)
    };

    match direction {
        RowDirection::Left => {
            if current_row.child_idx == 0 {
                row_child_indices().max() // Wrap to rightmost child
            } else {
                Some(current_row.child_idx - 1)
            }
        }
        RowDirection::Right => {
            let max_child_idx = row_child_indices().max()?;
            Some(if current_row.child_idx >= max_child_idx {
                0 // Wrap to leftmost child
            } else {
                current_row.child_idx + 1
            })
        }
    }
}

/// Find the focusable element in `target_child_idx`'s column of the row,
/// preferring an exact `idx_within_child` match and falling back to the
/// closest one (columns can have different element counts, e.g. team
/// rosters with different player counts).
fn row_element_at_child_idx(
    state: &DocumentNavState,
    current_row: RowPosition,
    target_child_idx: usize,
) -> Option<usize> {
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
#[path = "document_nav_tests.rs"]
mod tests;
