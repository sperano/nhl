Debug state flow issues in the TUI.

Ask the user:
1. **What's the symptom?** (e.g., "pressing Left doesn't change selection", "data not updating", "wrong tab focused")
2. **Which tab/panel?** (Scores, Standings, Settings, or a specific panel)
3. **What action triggered it?** (key press, data load, startup)

**Step 1: Trace the action path**

Identify the key → action → reducer → state chain:

```bash
# Find key binding
grep -n "KeyCode::{key}" src/tui/keys.rs

# Find action handler
grep -rn "Action::{ActionName}" src/tui/reducers/

# Check state field
grep -n "{state_field}" src/tui/state.rs
```

**Step 2: Add diagnostic logging**

Temporarily add tracing to the reducer:
```rust
tracing::debug!(
    "Action: {:?}, Before: {:?}, After: {:?}",
    action,
    state.{relevant_field},
    new_state.{relevant_field}
);
```

**Step 3: Check common issues**

- [ ] **Action not dispatched**: Key handler returns `None`?
- [ ] **Wrong reducer handles it**: Check reducer dispatch order in `reducer.rs`
- [ ] **State not cloned**: Forgot `state.clone()` before mutation?
- [ ] **Effect not processed**: Async effect not awaited?
- [ ] **Wrong state field**: Reading from wrong nested field?
- [ ] **Focus state wrong**: `content_focused` or `panel_stack` blocking input?

**Step 4: Verify state structure**

Print relevant state sections:
```
NavigationState:
  current_tab: {tab}
  content_focused: {bool}
  panel_stack: [{panels}]

UiState.{tab}:
  selected_index: {n}
  scroll_offset: {n}
  {other_fields}

DataState:
  {relevant_data}: {loaded/loading/error}
```

**Step 5: Test in isolation**

Create a minimal test case:
```rust
#[test]
fn test_debug_{issue}() {
    let mut state = AppState::default();
    // Setup state to match problematic scenario
    state.navigation.current_tab = Tab::{Tab};
    state.navigation.content_focused = {bool};

    let action = Action::{Action};
    let (new_state, effect) = reduce(state, action);

    // Assert expected behavior
    dbg!(&new_state.{field});
    assert_eq!(new_state.{field}, {expected});
}
```

```bash
cargo test --lib test_debug_{issue} -- --nocapture
```

**Step 6: Common fixes**

**Focus not working:**
```rust
// Check focus hierarchy in keys.rs
if state.navigation.panel_stack.is_empty() {
    // Handle at tab level
} else {
    // Delegate to panel
}
```

**Data not updating:**
```rust
// Ensure Effect::Async returns the right action
Effect::Async(Box::pin(async move {
    Action::{DataLoaded}(result)  // <- Is this the right action?
}))
```

**Selection out of bounds:**
```rust
// Clamp selection to valid range
new_state.ui.{tab}.selected_index =
    new_state.ui.{tab}.selected_index.min(max_valid_index);
```

**Report:**
- Issue: {description}
- Root cause: {explanation}
- Fix applied: {what was changed}
- Test added: `test_{scenario}`
