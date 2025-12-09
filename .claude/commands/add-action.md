Add a new Redux-like action to the TUI system.

Ask the user:
1. **Action name** (e.g., "SelectPlayer", "ToggleFilter", "LoadTeamStats")
2. **Which domain?** (navigation, scores, standings, settings, data)
3. **What data does it carry?** (e.g., "player_id: u32", "filter: FilterType", none)

**Step 1: Determine action location**
- Navigation actions → `Action` enum directly
- Tab-specific → Nested in `ScoresAction`, `StandingsAction`, or `SettingsAction`
- Data loading → `Action` enum with `Loaded` suffix pattern

**Step 2: Add the action variant**

For top-level action in `src/tui/action.rs`:
```rust
pub enum Action {
    // ... existing actions
    {ActionName}({payload_type}),  // or just {ActionName}, if no payload
}
```

For nested action (e.g., ScoresAction):
```rust
pub enum ScoresAction {
    // ... existing actions
    {ActionName}({payload_type}),
}
```

**Step 3: Create the reducer handler**

Determine which reducer file handles this:
- `src/tui/reducers/navigation.rs` - Tab/panel navigation
- `src/tui/reducers/panels.rs` - Panel stack management
- `src/tui/reducers/scores.rs` - Scores tab logic
- `src/tui/reducers/standings.rs` - Standings tab logic
- `src/tui/reducers/data_loading.rs` - API data arrival

Add handler:
```rust
Action::{ActionName}(payload) => {
    let mut new_state = state.clone();
    // Update state based on action
    new_state.{field} = {new_value};

    // Determine effect
    let effect = Effect::None;  // or Effect::Action(...) or Effect::Async(...)

    Some((new_state, effect))
}
```

**Step 4: Wire up key binding (if user-triggered)**

In `src/tui/keys.rs`, add to appropriate handler:
```rust
KeyCode::Char('{key}') => Some(Action::{ActionName}({payload})),
```

**Step 5: Add tests**

```rust
#[test]
fn test_{action_name_snake}() {
    let mut state = AppState::default();
    // Setup initial state

    let action = Action::{ActionName}({payload});
    let (new_state, effect) = reduce(state, action);

    assert_eq!(new_state.{field}, {expected});
    assert!(matches!(effect, Effect::None));
}
```

**Step 6: Verify**
```bash
cargo test --lib tui::reducers -- --nocapture
cargo test --lib tui::action -- --nocapture
```

**Report:**
- Action variant: `Action::{ActionName}`
- Reducer: `src/tui/reducers/{file}.rs`
- Key binding: `{key}` (if applicable)
- Test: `test_{action_name_snake}`
