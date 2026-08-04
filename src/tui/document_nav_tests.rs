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
fn test_focused_id_resolves_index_into_focusables() {
    let mut state = DocumentNavState {
        focusables: uniform_focusables(&[(0, 1), (5, 1), (10, 1)]),
        ..Default::default()
    };

    // No focus -> no ID.
    assert_eq!(state.focused_id(), None);

    state.focus_index = Some(1);
    assert_eq!(state.focused_id(), Some(FocusableId::link("f1")));

    // A stale index past the end of `focusables` resolves to no focus
    // rather than panicking.
    state.focus_index = Some(99);
    assert_eq!(state.focused_id(), None);
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
        focusables: uniform_focusables(&[(0, 1), (2, 1), (4, 1), (6, 1), (8, 1), (20, 1), (22, 1)]),
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
        focusables: uniform_focusables(&[(0, 1), (2, 1), (4, 1), (6, 1), (8, 1), (20, 1), (22, 1)]),
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
        fn title(&self) -> std::borrow::Cow<'static, str> {
            std::borrow::Cow::Borrowed("fake")
        }
        fn id(&self) -> std::borrow::Cow<'static, str> {
            std::borrow::Cow::Borrowed("fake")
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
