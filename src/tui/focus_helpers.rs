//! Shared focus navigation helpers
//!
//! This module provides common focus navigation patterns that can be reused
//! across different components and reducers.

/// Navigate to the next item in a circular list
///
/// Returns the new index after navigation. If `current` is None, returns 0 (first item).
/// Wraps from the last item back to the first.
///
/// # Arguments
/// * `current` - The current focus index (None = no focus)
/// * `count` - The total number of focusable items
///
/// # Returns
/// * `Some(index)` - The new focus index
/// * `None` - If count is 0 (no items to focus)
///
/// # Example
/// ```ignore
/// let new_idx = focus_next(Some(2), 5); // Returns Some(3)
/// let new_idx = focus_next(Some(4), 5); // Returns Some(0) - wrapped
/// let new_idx = focus_next(None, 5);    // Returns Some(0) - first item
/// ```
pub fn focus_next(current: Option<usize>, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match current {
        None => Some(0),
        Some(idx) => Some((idx + 1) % count),
    }
}

/// Navigate to the previous item in a circular list
///
/// Returns the new index after navigation. If `current` is None, returns the last item.
/// Wraps from the first item back to the last.
///
/// # Arguments
/// * `current` - The current focus index (None = no focus)
/// * `count` - The total number of focusable items
///
/// # Returns
/// * `Some(index)` - The new focus index
/// * `None` - If count is 0 (no items to focus)
///
/// # Example
/// ```ignore
/// let new_idx = focus_prev(Some(2), 5); // Returns Some(1)
/// let new_idx = focus_prev(Some(0), 5); // Returns Some(4) - wrapped
/// let new_idx = focus_prev(None, 5);    // Returns Some(4) - last item
/// ```
pub fn focus_prev(current: Option<usize>, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match current {
        None => Some(count - 1),
        Some(0) => Some(count - 1),
        Some(idx) => Some(idx - 1),
    }
}

/// Check if navigation wrapped from last to first (forward wrap)
///
/// # Arguments
/// * `prev_index` - The index before navigation (None if no previous focus)
/// * `new_index` - The index after navigation
/// * `count` - The total number of focusable items
///
/// # Returns
/// * `true` if we wrapped from the last item to the first
pub fn did_wrap_forward(prev_index: Option<usize>, new_index: Option<usize>, count: usize) -> bool {
    match (prev_index, new_index) {
        (Some(prev), Some(0)) if prev == count - 1 => true,
        (None, Some(0)) => true, // Initial focus counts as wrap for scroll behavior
        _ => false,
    }
}

/// Check if navigation wrapped from first to last (backward wrap)
///
/// # Arguments
/// * `prev_index` - The index before navigation (None if no previous focus)
/// * `new_index` - The index after navigation
/// * `count` - The total number of focusable items
///
/// # Returns
/// * `true` if we wrapped from the first item to the last
pub fn did_wrap_backward(
    prev_index: Option<usize>,
    new_index: Option<usize>,
    count: usize,
) -> bool {
    match (prev_index, new_index) {
        (Some(0), Some(last)) if last == count - 1 => true,
        (None, Some(last)) if last == count - 1 => true, // Initial focus counts as wrap
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === focus_next tests ===

    #[test]
    fn test_focus_next_empty() {
        assert_eq!(focus_next(None, 0), None);
        assert_eq!(focus_next(Some(0), 0), None);
    }

    #[test]
    fn test_focus_next_from_none() {
        assert_eq!(focus_next(None, 5), Some(0));
    }

    #[test]
    fn test_focus_next_increments() {
        assert_eq!(focus_next(Some(0), 5), Some(1));
        assert_eq!(focus_next(Some(1), 5), Some(2));
        assert_eq!(focus_next(Some(3), 5), Some(4));
    }

    #[test]
    fn test_focus_next_wraps() {
        assert_eq!(focus_next(Some(4), 5), Some(0));
    }

    // === focus_prev tests ===

    #[test]
    fn test_focus_prev_empty() {
        assert_eq!(focus_prev(None, 0), None);
        assert_eq!(focus_prev(Some(0), 0), None);
    }

    #[test]
    fn test_focus_prev_from_none() {
        assert_eq!(focus_prev(None, 5), Some(4));
    }

    #[test]
    fn test_focus_prev_decrements() {
        assert_eq!(focus_prev(Some(4), 5), Some(3));
        assert_eq!(focus_prev(Some(2), 5), Some(1));
        assert_eq!(focus_prev(Some(1), 5), Some(0));
    }

    #[test]
    fn test_focus_prev_wraps() {
        assert_eq!(focus_prev(Some(0), 5), Some(4));
    }

    // === did_wrap_forward tests ===

    #[test]
    fn test_did_wrap_forward_yes() {
        assert!(did_wrap_forward(Some(4), Some(0), 5));
    }

    #[test]
    fn test_did_wrap_forward_no() {
        assert!(!did_wrap_forward(Some(2), Some(3), 5));
        assert!(!did_wrap_forward(Some(0), Some(1), 5));
    }

    #[test]
    fn test_did_wrap_forward_initial_focus() {
        assert!(did_wrap_forward(None, Some(0), 5));
    }

    // === did_wrap_backward tests ===

    #[test]
    fn test_did_wrap_backward_yes() {
        assert!(did_wrap_backward(Some(0), Some(4), 5));
    }

    #[test]
    fn test_did_wrap_backward_no() {
        assert!(!did_wrap_backward(Some(3), Some(2), 5));
        assert!(!did_wrap_backward(Some(1), Some(0), 5));
    }

    #[test]
    fn test_did_wrap_backward_initial_focus() {
        assert!(did_wrap_backward(None, Some(4), 5));
    }
}
