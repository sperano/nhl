# TUI Architecture Simplification Plan

## Summary

Simplify the TUI by consolidating navigation state, moving key handling to components, and adding `handle_key()` to stacked documents. The goal is reducing complexity while preserving the solid unidirectional data flow.

## Key Decisions

1. **Key dispatch**: Move tab-specific handling to components; keys.rs becomes thin dispatcher
2. **Stacked documents**: Keep as Documents but add `handle_key()` method for encapsulation
3. **State refactor**: Consolidate - embed `DocumentNavState` inside `StackedDocumentEntry`

---

## Phase 1: Consolidate Navigation State Types

**Goal**: Single canonical navigation state, eliminate duplication.

### Changes

1. **Simplify `StackedDocumentEntry`** in `src/tui/state.rs`:
```rust
// Before: 6 separate fields duplicating DocumentNavState
pub struct DocumentStackEntry {
    pub document: StackedDocument,
    pub selected_index: Option<usize>,
    pub scroll_offset: u16,
    pub focusable_positions: Vec<u16>,
    pub focusable_heights: Vec<u16>,
    pub viewport_height: u16,
}

// After: embedded DocumentNavState
pub struct StackedDocumentEntry {
    pub document: StackedDocument,
    pub nav: DocumentNavState,
}
```

2. **Delete unused `DocumentState`** from `src/tui/state.rs` if present

3. **Update all stack access** to use `entry.nav.focus_index` instead of `entry.selected_index`, etc.

### Files
- `src/tui/state.rs` - struct change
- `src/tui/reducers/document_stack.rs` - update field access
- `src/tui/runtime.rs` - update viewport height code
- `src/tui/keys.rs` - update stack navigation helpers

---

## Phase 2: Add NavigateUp Action

**Goal**: Unified "go back" semantics instead of scattered ESC handling.

### Changes

1. **Add action** in `src/tui/action.rs`:
```rust
pub enum Action {
    // ...existing...
    NavigateUp,  // Unified ESC behavior
}
```

2. **Reducer logic** (hierarchical fallthrough):
   - If document stack not empty → pop document
   - Else send `NavigateUpMsg` to current tab component
   - Component returns whether it handled it (closed modal, exited browse mode)
   - If not handled and content_focused → set content_focused = false

3. **Replace `handle_esc_key()`** with single `Action::NavigateUp` dispatch

### Files
- `src/tui/action.rs` - add action
- `src/tui/reducer.rs` - add NavigateUp handler
- `src/tui/keys.rs` - simplify ESC handling

---

## Phase 3: Component Key Handling

**Goal**: Each tab handles its own keys; keys.rs becomes dispatcher.

### New Pattern

Each tab component gets a `Key(KeyEvent)` message variant:

```rust
pub enum ScoresTabMsg {
    Key(KeyEvent),  // New: raw key when tab is focused
    DocNav(DocumentNavMsg),
    // ...existing...
}
```

Component handles its own modes internally:

```rust
impl ScoresTab {
    fn handle_key(key: KeyEvent, state: &mut ScoresTabState) -> Effect {
        if state.doc_nav.is_browsing() {
            // Game selection mode
            match key.code {
                KeyCode::Up => /* FocusPrev */,
                KeyCode::Down => /* FocusNext */,
                KeyCode::Enter => /* activate game */,
                _ => Effect::None,
            }
        } else {
            // Date navigation mode
            match key.code {
                KeyCode::Left => /* prev date */,
                KeyCode::Right => /* next date */,
                KeyCode::Down => /* enter browse mode */,
                _ => Effect::None,
            }
        }
    }
}
```

### Simplified keys.rs (~200 lines target)

```rust
pub fn key_to_action(key: KeyEvent, state: &AppState, _: &ComponentStateStore) -> Option<Action> {
    // 1. Global keys (q, /, 1-4)
    if let Some(action) = handle_global_keys(key.code) {
        return Some(action);
    }

    // 2. ESC → unified navigate up
    if key.code == KeyCode::Esc {
        return Some(Action::NavigateUp);
    }

    // 3. Document stack open → route to stacked document
    if !state.navigation.document_stack.is_empty() {
        return Some(Action::StackedDocumentKey(key));
    }

    // 4. Tab bar focused → tab navigation
    if !state.navigation.content_focused {
        return handle_tab_bar_keys(key.code);
    }

    // 5. Content focused → dispatch to current tab
    Some(Action::ComponentMessage {
        path: tab_path(state.navigation.current_tab),
        message: Box::new(TabKeyMsg(key)),
    })
}
```

### Files
- `src/tui/keys.rs` - major simplification
- `src/tui/components/scores_tab.rs` - add Key handling
- `src/tui/components/standings_tab.rs` - add Key handling
- `src/tui/components/settings_tab.rs` - add Key handling
- `src/tui/components/demo_tab.rs` - add Key handling

---

## Phase 4: Stacked Document Key Handling

**Goal**: Stacked documents handle their own navigation via `handle_key()`.

### Add trait method

```rust
pub trait StackedDocumentHandler {
    fn handle_key(
        &self,
        key: KeyEvent,
        nav: &mut DocumentNavState,
        data: &DataState,
    ) -> Effect;
}
```

### Implement for each stacked document type

```rust
impl StackedDocumentHandler for BoxscoreDocument {
    fn handle_key(&self, key: KeyEvent, nav: &mut DocumentNavState, data: &DataState) -> Effect {
        match key.code {
            KeyCode::Up => { nav.focus_prev(); Effect::None }
            KeyCode::Down => { nav.focus_next(); Effect::None }
            KeyCode::Enter => self.activate_focused(nav),
            _ => Effect::None,
        }
    }
}
```

### Reducer dispatches to handler

```rust
Action::StackedDocumentKey(key) => {
    if let Some(entry) = state.navigation.document_stack.last_mut() {
        let handler = get_handler(&entry.document);
        handler.handle_key(key, &mut entry.nav, &state.data)
    } else {
        Effect::None
    }
}
```

### Files
- `src/tui/document/mod.rs` - add trait
- `src/tui/components/boxscore_document.rs` - implement handler
- `src/tui/components/team_detail_document.rs` - implement handler
- `src/tui/components/player_detail_document.rs` - implement handler
- `src/tui/reducer.rs` - add StackedDocumentKey handling

---

## Phase 5: Cleanup

1. Remove unused action variants (`DocumentSelectNext`, `DocumentSelectPrev`, etc.)
2. Remove helper functions from keys.rs (`is_scores_browse_mode_active`, etc.)
3. Remove dead reducer code
4. Update tests
5. Update documentation

---

## Expected Outcomes

| Metric | Before | After |
|--------|--------|-------|
| keys.rs lines | ~565 | ~200 |
| Navigation state types | 3 overlapping | 1 canonical (DocumentNavState) |
| Tab key handlers in keys.rs | 6 functions | 1 dispatcher |
| Mode-check helper functions | 4 | 0 |

## Risk Mitigation

- Each phase is independently testable
- Phase 3 can be done one tab at a time
- All changes preserve the action→reducer→state→view flow
- Existing tests catch regressions

## Critical Files

| File | Changes |
|------|---------|
| `src/tui/keys.rs` | Major reduction, thin dispatcher |
| `src/tui/state.rs` | Consolidate StackedDocumentEntry |
| `src/tui/document_nav.rs` | Canonical nav state |
| `src/tui/components/scores_tab.rs` | Add Key message handling |
| `src/tui/components/standings_tab.rs` | Add Key message handling |
| `src/tui/reducer.rs` | NavigateUp, StackedDocumentKey |
