# Refactoring Status: TabbedPanel Implementation

## Goal
Replace the tab/subtab architecture with composable TabbedPanel components (like React Bootstrap Tabs)

## Progress

### ✅ Step 1: Create TabbedPanel Component (COMPLETE)

**Files Created:**
- `src/tui/components/tabbed_panel.rs` - Full implementation with tests

**What was built:**
- `TabbedPanel` component - Renders tab bar + active content
- `TabItem` struct - Associates tab label with content
- `TabbedPanelProps` - Props with active_key and tabs list
- `TabBarWidget` - Private widget for rendering tab labels
- 5 unit tests - All passing ✅

**Key Features:**
- Tabs contain their own content (like React Bootstrap)
- Support for nested TabbedPanels (tab content can be another TabbedPanel)
- Simple API: `active_key` + list of `TabItem`s
- No subtab_focused hack needed

**Architecture:**
```rust
TabbedPanel {
  active_key: "tab1",
  tabs: [
    TabItem { key: "tab1", title: "Tab 1", content: Element },
    TabItem { key: "tab2", title: "Tab 2", content: Element },
  ]
}
```

**Tests passing:**
```
test tui::components::tabbed_panel::tests::test_tab_item_builder ... ok
test tui::components::tabbed_panel::tests::test_empty_tabs_shows_nothing ... ok
test tui::components::tabbed_panel::tests::test_nonexistent_active_key_shows_none ... ok
test tui::components::tabbed_panel::tests::test_tabbed_panel_shows_active_content ... ok
test tui::components::tabbed_panel::tests::test_tabbed_panel_renders_container ... ok
```

## Next Steps

### Step 2: Update App Component (PENDING)
Replace manual tab switching with TabbedPanel

**Changes needed:**
- Refactor `App.view()` to use TabbedPanel
- Create TabItems for each main tab (Scores, Standings, Stats, etc.)
- Remove manual match on current_tab

### Step 3: Add Nested Tabs for Scores (PENDING)
Replace date selector with nested TabbedPanel

**Changes needed:**
- Create TabItems for each date in the 5-date window
- Nest TabbedPanel inside Scores tab content
- Update date navigation to change active_key

### Step 4: Add Nested Tabs for Standings (PENDING)
Replace view selector with nested TabbedPanel

**Changes needed:**
- Create TabItems for Division/Conference/League
- Nest TabbedPanel inside Standings tab content
- Update view navigation to change active_key

### Step 5: Clean Up State (PENDING)
Remove subtab_focused and related code

**Files to modify:**
- `src/tui/framework/state.rs` - Remove subtab_focused field
- `src/tui/framework/action.rs` - Remove subtab actions
- `src/tui/framework/reducer.rs` - Remove subtab logic

### Step 6: Update Key Handlers (PENDING)
Adjust keyboard navigation for nested tabs

**Changes needed:**
- Tab navigation should work recursively
- Down/Up for entering/exiting nested tabs
- Left/Right for switching within tab group

### Step 7: Integration Testing (PENDING)
- Run full test suite
- Manual TUI testing
- Verify all navigation works

### Step 8: Documentation (PENDING)
- Update CLAUDE.md
- Update COMPONENT_EXAMPLE.md with TabbedPanel examples
- Remove old subtab documentation

## Design Decisions Made

1. **Content lives with tabs** - Each TabItem owns its content Element
2. **Composable nesting** - TabbedPanel content can be another TabbedPanel
3. **Simple state model** - Just track active_key per panel
4. **No special subtab mode** - Nesting replaces the subtab_focused hack

## Files Modified So Far

- ✅ Created: `src/tui/components/tabbed_panel.rs`
- ✅ Modified: `src/tui/components/mod.rs` (added exports)

## Pause Points for Testing

After each major step, we should:
1. Run `cargo test`
2. Run `NHL_EXPERIMENTAL=1 cargo run` to test TUI
3. Verify navigation works as expected
4. Commit changes to git

## Current Status
**✅ Steps 2-4 COMPLETE - Full TabbedPanel Migration!**

### Step 2: App uses TabbedPanel ✅
- App.view() now returns 2 children (TabbedPanel + StatusBar) instead of 3
- Tab bar and content unified in TabbedPanel

### Step 3: Scores uses nested TabbedPanel ✅
- Date navigation now uses TabbedPanel with 5 date tabs
- Removed old DateSelectorWidget
- Each date tab contains the game list for that date

### Step 4: Standings uses nested TabbedPanel ✅
- View selection (Division/Conference/League) now uses TabbedPanel
- Removed old ViewSelectorWidget
- Each view tab contains the standings table for that view

### Architecture Now:
```
App (TabbedPanel)
├─ Scores Tab (TabbedPanel)
│  ├─ 11/08 Tab → Game List
│  ├─ 11/09 Tab → Game List
│  ├─ 11/10 Tab → Game List (active)
│  ├─ 11/11 Tab → Game List
│  └─ 11/12 Tab → Game List
├─ Standings Tab (TabbedPanel)
│  ├─ Division Tab → Standings Table (active)
│  ├─ Conference Tab → Standings Table
│  └─ League Tab → Standings Table
└─ Settings Tab
```

### Files Modified:
- `src/tui/components/app.rs` - Uses TabbedPanel for main tabs
- `src/tui/components/scores_tab.rs` - Uses nested TabbedPanel for dates
- `src/tui/components/standings_tab.rs` - Uses nested TabbedPanel for views
- Test files - Updated expectations

### All Tests Passing: ✅ 607 tests

**Step 5: ✅ COMPLETE - Removed subtab_focused**

### What was done:
- ✅ Removed `subtab_focused` field from `NavigationState`
- ✅ Removed `subtab_focused` from `ScoresTabProps` and `StandingsTabProps`
- ✅ Updated `App` component to not pass `subtab_focused`
- ✅ Updated reducer to remove `subtab_focused` assignments and deprecated `EnterSubtabMode`/`ExitSubtabMode` actions
- ✅ Simplified `keys.rs` - removed subtab mode concept, implemented context-sensitive key handling
- ✅ Fixed all test compilation errors (removed `subtab_focused` from test props and assertions)
- ✅ Updated experimental tests to reflect new context-sensitive behavior
- ✅ **All 605 tests passing** ✅

### Key Changes:

**Context-Sensitive Arrow Keys (no mode switching needed):**
- **Scores tab**: Left/Right navigate dates, Down enters box selection
- **Standings tab**: Left/Right cycle views, Down enters team mode
- **Other tabs**: Left/Right navigate main tabs

**Deprecated Actions:**
- `Action::EnterSubtabMode` - deprecated (logged as no-op)
- `Action::ExitSubtabMode` - deprecated (logged as no-op)

**Files Modified:**
- `src/tui/framework/state.rs` - Removed field
- `src/tui/framework/reducer.rs` - Removed logic
- `src/tui/framework/keys.rs` - Simplified to context-sensitive handling
- `src/tui/framework/effects.rs` - Fixed test helper
- `src/tui/framework/experimental_tests.rs` - Updated tests
- `src/tui/components/scores_tab.rs` - Removed from props and tests
- `src/tui/components/standings_tab.rs` - Removed from props and tests

**Step 6: ✅ COMPLETE - Restored Two-Level Focus**

### What was done:
- ✅ Added `content_focused: bool` to `NavigationState`
- ✅ Renamed actions: `EnterContentFocus` / `ExitContentFocus` (with deprecated aliases)
- ✅ Updated reducer to handle focus transitions
- ✅ Updated `keys.rs` to implement two-level focus behavior
- ✅ Updated tests to reflect new focus model
- ✅ **All 605 tests passing** ✅

### Two-Level Focus Behavior:

**Level 1: Tab Bar Focused (default state)**
- **Left/Right**: Navigate between main tabs
- **Down**: Enter content focus mode
- **Number keys (1-6)**: Direct tab switching (works regardless of focus)

**Level 2: Content Focused**
- **Up**: Return to tab bar (unless in nested mode like box selection)
- **Arrows**: Context-sensitive based on current tab:
  - **Scores tab**: Left/Right navigate dates, Down enters box selection
  - **Standings tab**: Left/Right cycle views, Down enters team mode
  - **Other tabs**: No special navigation yet

**Focus Transitions:**
- Switching tabs → returns focus to tab bar
- Entering content → `content_focused = true`
- Exiting content → `content_focused = false` and clears nested modes (box_selection, team_mode)

**Files Modified:**
- `src/tui/framework/state.rs` - Added `content_focused` field
- `src/tui/framework/action.rs` - Added `EnterContentFocus` / `ExitContentFocus`
- `src/tui/framework/reducer.rs` - Handle focus transitions, clear on tab switch
- `src/tui/framework/keys.rs` - Two-level focus logic
- `src/tui/framework/effects.rs` - Fixed test helper
- `src/tui/framework/experimental_tests.rs` - Updated tests for focus behavior

**Step 7: Documentation (PENDING)**

Last updated: 2025-11-13
