# Refactoring Plan: Component/Widget Separation

## Current Problem

The codebase has **poor separation** between Components and Widgets:

### Issues Identified

1. **Duplicate implementations**: Same widgets exist in both `components/` and `widgets/`
   - `components/tab_bar.rs` vs `widgets/tab_bar.rs`
   - `components/status_bar.rs` vs `widgets/status_bar.rs`

2. **Thin wrapper anti-pattern**: Most "components" are just trivial wrappers around private widgets
   ```rust
   // components/tab_bar.rs
   impl Component for TabBar {
       fn view(&self, props: &Props, _state: &State) -> Element {
           Element::Widget(Box::new(TabBarWidget { ... }))  // ← Just wraps widget
       }
   }
   struct TabBarWidget { ... }  // ← Private, defined in same file
   ```

3. **No composability**: Components don't compose, they just wrap
   - TabBar → wraps TabBarWidget (no children)
   - StatusBar → wraps StatusBarWidget (no children)
   - BoxscorePanel → wraps BoxscorePanelWidget (no children)

4. **Different RenderableWidget traits**:
   - `framework/component.rs` defines: `fn render(&self, area: Rect, buf: &mut Buffer)`
   - `widgets/mod.rs` defines: `fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig)`
   - These are **incompatible**!

## Architecture Goals

### Component Layer (Virtual/Compositional)
**Purpose**: Compose UI from smaller pieces, manage state flow, layout

**Characteristics**:
- ✅ Composes other components/widgets using `vertical()`, `horizontal()`
- ✅ Builds virtual Element trees
- ✅ Maps state to UI structure
- ✅ Contains business logic for what to show
- ✅ Lives in `src/tui/components/`

**Examples**:
- `App` - composes TabBar, content area, StatusBar
- `ScoresTab` - composes date selector, game list
- `StandingsTab` - composes view selector, standings table

### Widget Layer (Execution/Rendering)
**Purpose**: Actually draw things to the terminal buffer

**Characteristics**:
- ✅ Implements `RenderableWidget` trait
- ✅ Renders directly to ratatui Buffer
- ✅ Self-contained, reusable UI primitives
- ✅ No state management, just rendering
- ✅ Lives in `src/tui/widgets/`
- ✅ Public API for reuse

**Examples**:
- `DateSelector` - renders 5-date sliding window
- `GameList` - renders list of games
- `StandingsTable` - renders standings grid

## Decision Matrix

| Type | Pattern | Location | Example |
|------|---------|----------|---------|
| **Composing UI** | Component with vertical/horizontal layout | `components/` | `App`, `ScoresTab` |
| **Leaf display logic** | Public widget in widgets/ | `widgets/` | `DateSelector`, `GameBox` |
| **Thin wrapper** | **ANTI-PATTERN** - remove it | ~~Delete~~ | ~~`TabBar` component~~ |

## Refactoring Strategy

### Phase 1: Audit and Classify ✅

**Task**: Categorize all files in `components/`

| File | Classification | Action |
|------|----------------|--------|
| `app.rs` | ✅ True component (composes 3 children) | Keep as component |
| `scores_tab.rs` | ✅ True component (composes 2 children) | Keep, extract private widgets |
| `standings_tab.rs` | ✅ True component (composes 2 children) | Keep, extract private widgets |
| `boxscore_panel.rs` | 🟡 Thin wrapper (wraps 1 widget) | Extract widget to widgets/ |
| `tab_bar.rs` | ❌ Thin wrapper (wraps 1 widget) | Delete component, use widget directly |
| `status_bar.rs` | ❌ Thin wrapper (wraps 1 widget) | Delete component, use widget directly |
| `settings_tab.rs` | 🟡 Minimal component | Keep for now (placeholder) |

**Private widgets to extract from components**:
- `DateSelectorWidget` (in scores_tab.rs) → `widgets/date_selector.rs`
- `GameListWidget` (in scores_tab.rs) → `widgets/game_list.rs`
- `ViewSelectorWidget` (in standings_tab.rs) → `widgets/view_selector.rs`
- `StandingsTableWidget` (in standings_tab.rs) → `widgets/standings_table_new.rs`
- `BoxscorePanelWidget` (in boxscore_panel.rs) → `widgets/boxscore_panel.rs`

### Phase 2: Unify RenderableWidget Trait

**Problem**: Two incompatible RenderableWidget definitions

**Solution**:
1. Use the `widgets/mod.rs` version as canonical (includes `DisplayConfig`)
2. Update `framework/component.rs` to match the signature
3. Update Renderer to pass `DisplayConfig` when calling render

**Files to modify**:
- `src/tui/framework/component.rs` - Update RenderableWidget trait
- `src/tui/framework/renderer.rs` - Pass DisplayConfig to render calls
- All widget implementations in components/ - Add config parameter

### Phase 3: Extract Private Widgets

**For each private widget in components/**:

1. **Create new file in widgets/**
   ```rust
   // src/tui/widgets/date_selector.rs
   pub struct DateSelector {
       pub game_date: GameDate,
       pub selected_index: usize,
       pub focused: bool,
   }

   impl RenderableWidget for DateSelector {
       fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
           // Move rendering logic here
       }
       fn clone_box(&self) -> Box<dyn RenderableWidget> { ... }
   }
   ```

2. **Export from widgets/mod.rs**
   ```rust
   pub mod date_selector;
   pub use date_selector::DateSelector;
   ```

3. **Update component to use public widget**
   ```rust
   // src/tui/components/scores_tab.rs
   use crate::tui::widgets::DateSelector;

   fn render_date_selector(&self, props: &ScoresTabProps) -> Element {
       Element::Widget(Box::new(DateSelector {
           game_date: props.game_date.clone(),
           selected_index: props.selected_index,
           focused: props.subtab_focused,
       }))
   }
   ```

4. **Delete private widget from component file**

### Phase 4: Remove Duplicate Widgets

**Duplicates to resolve**:

1. **TabBar**
   - Keep: `widgets/tab_bar.rs` (old, has full implementation)
   - Delete: `components/tab_bar.rs` (new, thin wrapper)
   - Update: `components/app.rs` to use widget directly

2. **StatusBar**
   - Keep: `widgets/status_bar.rs` (old, has full implementation)
   - Delete: `components/status_bar.rs` (new, thin wrapper)
   - Update: `components/app.rs` to use widget directly

**Note**: The old widgets use the correct RenderableWidget signature with DisplayConfig!

### Phase 5: Remove Thin Wrapper Components

**For TabBar and StatusBar in components/app.rs**:

Before:
```rust
vec![
    TabBar.view(&props.navigation.current_tab, &()),
    self.render_content(props),
    StatusBar.view(&props.system, &()),
]
```

After:
```rust
use crate::tui::widgets::{TabBar, StatusBar};

vec![
    Element::Widget(Box::new(TabBar {
        tabs: vec![
            Tab::new("Scores", Some('1')),
            Tab::new("Standings", Some('2')),
            // ...
        ],
        current_tab: tab_index_from_state(props),
        focused: false,
    })),
    self.render_content(props),
    Element::Widget(Box::new(StatusBar {
        key_hints: vec![/* ... */],
        style: KeyHintStyle::default(),
    })),
]
```

**Helper function needed**:
```rust
fn tab_index_from_state(state: &AppState) -> usize {
    match state.navigation.current_tab {
        Tab::Scores => 0,
        Tab::Standings => 1,
        // ...
    }
}
```

### Phase 6: Update Tests

**For each modified file**:
1. Run existing tests
2. Update tests to use new public widgets
3. Add tests for newly extracted widgets

### Phase 7: Documentation Update

**Update files**:
- `CLAUDE.md` - Document component vs widget separation
- `COMPONENT_EXAMPLE.md` - Update examples with correct patterns
- Add inline docs explaining when to use Component vs Widget

## Implementation Order

### Step 1: Unify RenderableWidget Trait (CRITICAL - DO FIRST)
- [ ] Update `framework/component.rs` RenderableWidget signature
- [ ] Update `framework/renderer.rs` to pass DisplayConfig
- [ ] Run tests, fix compilation errors

### Step 2: Extract Widgets (Can be done in parallel)
- [ ] Extract DateSelectorWidget → widgets/date_selector.rs
- [ ] Extract GameListWidget → widgets/game_list.rs
- [ ] Extract ViewSelectorWidget → widgets/view_selector.rs
- [ ] Extract StandingsTableWidget → widgets/standings_table_new.rs
- [ ] Extract BoxscorePanelWidget → widgets/boxscore_panel.rs

### Step 3: Remove Duplicates
- [ ] Delete components/tab_bar.rs
- [ ] Delete components/status_bar.rs
- [ ] Update app.rs to use widgets directly

### Step 4: Clean Up and Test
- [ ] Run full test suite
- [ ] Update documentation
- [ ] Remove dead code

## Success Criteria

✅ No duplicate widgets (no component + widget with same name)
✅ Components compose (use vertical/horizontal), don't just wrap
✅ All widgets in `widgets/` are public and reusable
✅ Single RenderableWidget trait definition
✅ All tests passing
✅ Clear documentation on when to use Component vs Widget

## Risks and Mitigations

### Risk 1: Breaking existing functionality
**Mitigation**: Run tests after each step, use git commits

### Risk 2: Old widgets may be incompatible
**Mitigation**: Check old widget implementations before deleting new ones

### Risk 3: DisplayConfig not available in all contexts
**Mitigation**: Pass DisplayConfig through renderer, add to AppState if needed

## Files to Modify Summary

### Create (5 files)
- `src/tui/widgets/date_selector.rs`
- `src/tui/widgets/game_list.rs`
- `src/tui/widgets/view_selector.rs`
- `src/tui/widgets/standings_table_new.rs` (temp name to avoid conflict)
- `src/tui/widgets/boxscore_panel.rs`

### Delete (2 files)
- `src/tui/components/tab_bar.rs`
- `src/tui/components/status_bar.rs`

### Modify (8+ files)
- `src/tui/framework/component.rs` (trait signature)
- `src/tui/framework/renderer.rs` (pass DisplayConfig)
- `src/tui/components/app.rs` (use widgets directly)
- `src/tui/components/scores_tab.rs` (use extracted widgets)
- `src/tui/components/standings_tab.rs` (use extracted widgets)
- `src/tui/components/boxscore_panel.rs` (delete, was thin wrapper)
- `src/tui/widgets/mod.rs` (add new exports)
- Tests for all modified components

## Estimated Effort

- **Step 1 (Unify trait)**: 30 minutes (critical, affects everything)
- **Step 2 (Extract widgets)**: 2 hours (5 widgets × 20-30 mins each)
- **Step 3 (Remove duplicates)**: 30 minutes
- **Step 4 (Testing/cleanup)**: 1 hour

**Total**: ~4 hours

## Next Actions

1. Get approval on this plan
2. Start with Step 1 (unify RenderableWidget trait)
3. Proceed incrementally, testing after each change
4. Update documentation as we go
