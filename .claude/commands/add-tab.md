Add a new tab to the TUI.

Ask the user:
1. **Tab name** (e.g., "Stats", "Players", "Teams")
2. **Tab position** (which number key 1-6)
3. **What will it display?** (brief description)

**Step 1: Add Tab enum variant**

In `src/tui/types.rs`:
```rust
pub enum Tab {
    Scores,
    Standings,
    // Add new tab
    {TabName},
    Settings,
}
```

**Step 2: Create tab-specific state**

In `src/tui/state.rs`:
```rust
#[derive(Default, Clone)]
pub struct {TabName}State {
    pub selected_index: usize,
    pub scroll_offset: usize,
    // Add tab-specific fields
}

pub struct UiState {
    pub scores: ScoresState,
    pub standings: StandingsState,
    pub {tab_name}: {TabName}State,  // Add here
    pub settings: SettingsState,
}
```

**Step 3: Create tab-specific actions (optional)**

In `src/tui/action.rs`:
```rust
#[derive(Clone, Debug)]
pub enum {TabName}Action {
    NavigateUp,
    NavigateDown,
    Select,
    // Add tab-specific actions
}

pub enum Action {
    // ... existing
    {TabName}Action({TabName}Action),
}
```

**Step 4: Create the reducer**

Create `src/tui/reducers/{tab_name}.rs`:
```rust
use crate::tui::action::{Action, {TabName}Action};
use crate::tui::effects::Effect;
use crate::tui::state::AppState;

pub fn reduce_{tab_name}(state: &AppState, action: &Action) -> Option<(AppState, Effect)> {
    match action {
        Action::{TabName}Action(tab_action) => {
            let mut new_state = state.clone();
            match tab_action {
                {TabName}Action::NavigateUp => {
                    // Handle up navigation
                }
                {TabName}Action::NavigateDown => {
                    // Handle down navigation
                }
                {TabName}Action::Select => {
                    // Handle selection
                }
            }
            Some((new_state, Effect::None))
        }
        _ => None,
    }
}
```

Add to `src/tui/reducers/mod.rs`:
```rust
mod {tab_name};
pub use {tab_name}::reduce_{tab_name};
```

Wire up in `src/tui/reducer.rs`:
```rust
reduce_{tab_name}(state, action)
    .or_else(|| reduce_navigation(state, action))
    // ... etc
```

**Step 5: Create the component**

Create `src/tui/components/{tab_name}_tab.rs`:
```rust
use crate::tui::component::{Component, Element, vertical};
use crate::tui::state::AppState;

pub struct {TabName}Tab;

#[derive(Clone, Default)]
pub struct {TabName}TabProps {
    // Props from AppState
}

impl Component for {TabName}Tab {
    type Props = {TabName}TabProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        vertical(vec![
            // Tab content
        ])
    }
}
```

Add to `src/tui/components/mod.rs`:
```rust
mod {tab_name}_tab;
pub use {tab_name}_tab::{TabName}Tab, {TabName}TabProps};
```

**Step 6: Wire up in TabbedPanel**

In `src/tui/components/tabbed_panel.rs`, add to tab rendering:
```rust
Tab::{TabName} => {
    let props = {TabName}TabProps {
        // Extract from app_state
    };
    let component = {TabName}Tab;
    component.view(&props, &())
}
```

**Step 7: Add key handling**

In `src/tui/keys.rs`, add to `key_to_action_for_{tab_name}`:
```rust
fn key_to_action_for_{tab_name}(key: KeyCode, state: &AppState) -> Option<Action> {
    match key {
        KeyCode::Up => Some(Action::{TabName}Action({TabName}Action::NavigateUp)),
        KeyCode::Down => Some(Action::{TabName}Action({TabName}Action::NavigateDown)),
        KeyCode::Enter => Some(Action::{TabName}Action({TabName}Action::Select)),
        _ => None,
    }
}
```

Add to main key handler dispatch:
```rust
Tab::{TabName} if state.navigation.content_focused => {
    key_to_action_for_{tab_name}(key, state)
}
```

**Step 8: Add number key shortcut**

```rust
KeyCode::Char('{n}') => Some(Action::NavigateTab(Tab::{TabName})),
```

**Step 9: Add tests**

```rust
#[test]
fn test_{tab_name}_tab_renders() {
    let component = {TabName}Tab;
    let props = {TabName}TabProps::default();
    let element = component.view(&props, &());

    let config = test_config();
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    render_element(&element, buf.area, &mut buf, &config);

    assert_buffer(&buf, &[
        // Expected output
    ]);
}
```

**Step 10: Verify**
```bash
cargo test --lib tui -- --nocapture
cargo check
```

**Report:**
- Tab enum: `Tab::{TabName}`
- State: `UiState.{tab_name}: {TabName}State`
- Component: `src/tui/components/{tab_name}_tab.rs`
- Reducer: `src/tui/reducers/{tab_name}.rs`
- Key shortcut: `{n}`
