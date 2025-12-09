# Document System Refactor Status

## High Priority

- [x] 1. Add `assert_buffer` tests for `DocumentView::render()`
- [x] 2. Use consistent scroll-to-bottom value (`u16::MAX` everywhere)
- [x] 3. Extract theme style helpers to eliminate duplication
- [x] 4. Formalize `DocumentElementWidget` as the standard bridge

## Medium Priority

- [x] 5. Clean up `DemoUiState` - deferred (sync approach needed for reducer autoscroll)
- [x] 6. Extract shared focus navigation patterns
- [x] 7. Add named constants for magic numbers
- [x] 8. Refactor renderer.rs tests to use `assert_buffer`

## Low Priority

- [x] 9. Rename `RenderableWidget` to `SimpleWidget` to avoid confusion
- [x] 10. Custom variant is unused - kept for extensibility
- [x] 11. Add missing derive macros (PartialEq, Eq on FocusContext)

---

## Progress Log

### Completed Changes

#### 1. assert_buffer tests for DocumentView::render()
- Added 5 new tests in `src/tui/document/mod.rs`:
  - `test_document_view_render_basic`
  - `test_document_view_render_with_viewport_offset`
  - `test_document_view_render_scrolled_to_bottom`
  - `test_document_view_render_unfocused_link`
  - `test_document_view_render_focused_link`

#### 2. Consistent scroll-to-bottom value
- Changed `src/tui/reducers/document.rs` lines 93, 99 from `100` to `u16::MAX`

#### 3. Theme style helpers
- Added to `src/config.rs`:
  - `DisplayConfig::text_style()` - returns Style with fg2 color
  - `DisplayConfig::muted_style()` - returns Style with fg3 color
  - `DisplayConfig::heading_style(level)` - returns styled heading
- Updated `src/tui/document/elements.rs` to use these helpers

#### 4. DocumentElementWidget
- Created `src/tui/document/widget.rs` with `DocumentElementWidget` struct
- Implements `ElementWidget` for bridging Documents to the Element tree
- Includes 3 tests with assert_buffer

#### 5. DemoUiState (Deferred)
- Analyzed the state sync pattern between `DemoUiState` and `FocusManager`
- Current approach stores `focusable_positions` in state for reducer autoscroll logic
- Deferred because the sync is architecturally necessary - reducers need position data

#### 6. Extract shared focus navigation patterns
- Created `src/tui/focus_helpers.rs` with reusable functions:
  - `focus_next(current, count) -> Option<usize>` - circular next navigation
  - `focus_prev(current, count) -> Option<usize>` - circular previous navigation
  - `did_wrap_forward(prev, new, count) -> bool` - detect forward wrap
  - `did_wrap_backward(prev, new, count) -> bool` - detect backward wrap
- Updated `FocusManager` in `src/tui/document/focus.rs` to use these helpers
- Added comprehensive tests for all helper functions

#### 7. Named constants for magic numbers
- `src/tui/document/mod.rs`: `PAGE_OVERLAP_LINES = 2`
- `src/tui/document/elements.rs`:
  - `TABLE_HEADER_HEIGHT = 3`
  - `TABLE_COLUMN_HEADER_HEIGHT = 2`
  - `TAB_ORDER_ROW_WEIGHT = 100`
- `src/tui/document/viewport.rs`:
  - `MAX_PADDING_DIVISOR = 4`
  - `SMALL/MEDIUM/LARGE_VIEWPORT_THRESHOLD`
  - `SMALL/MEDIUM/LARGE/VERY_LARGE_VIEWPORT_PADDING`
- `src/tui/reducers/document.rs`:
  - `DEFAULT_VIEWPORT_HEIGHT = 20`
  - `AUTOSCROLL_PADDING = 2`
  - `MIN_PAGE_SIZE = 10`

#### 8. Refactor renderer.rs tests to use assert_buffer
- Updated all tests in `src/tui/renderer.rs` to use `assert_buffer`:
  - `test_render_widget`
  - `test_render_container_vertical`
  - `test_render_container_horizontal`
  - `test_render_fragment`
  - `test_render_overlay`
  - `test_render_none`
  - `test_render_nested_containers`
  - `test_render_with_component`

#### 9. Renamed RenderableWidget to SimpleWidget
- Updated `src/tui/widgets/mod.rs` with new trait name and documentation
- Updated all implementations in:
  - `src/tui/widgets/game_box.rs`
  - `src/tui/widgets/score_table.rs`
  - `src/tui/widgets/testing.rs`
  - `src/tui/widgets/list_modal.rs`
  - `src/tui/widgets/settings_list.rs`
  - `src/tui/components/table.rs`
  - `src/tui/components/scores_tab.rs`
- Updated CLAUDE.md documentation

#### 10. Custom variant (Kept for extensibility)
- Analyzed usage: `DocumentElement::Custom` is not currently used
- Decision: Kept with doc note explaining it's for future extensibility

#### 11. Missing derive macros
- Added `PartialEq, Eq` to `FocusContext` in `src/tui/document/mod.rs`

---

## Summary

All 11 recommendations have been addressed. The refactoring improves:
- **Test coverage**: `assert_buffer` tests throughout
- **Code reuse**: Shared focus navigation helpers
- **Maintainability**: Named constants replace magic numbers
- **API clarity**: `SimpleWidget` vs `ElementWidget` distinction
- **Theme consistency**: Centralized style helpers
