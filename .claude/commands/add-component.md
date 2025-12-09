Create a new TUI component following the Component trait pattern.

Ask the user:
1. **Component name** (e.g., "PlayerCard", "GamePreview", "TeamHeader")
2. **What does it display?** (brief description)
3. **Does it need local state?** (yes/no - most don't)
4. **What props does it receive?** (data from AppState)

**Step 1: Create the component file**

Create `src/tui/components/{name_snake}.rs`:

```rust
use crate::tui::component::{Component, Element, vertical, horizontal};
use crate::tui::state::AppState;
use crate::tui::action::Action;
use crate::tui::effects::Effect;
use ratatui::prelude::*;

/// {Brief description of what this component displays}
pub struct {ComponentName};

/// Props passed to {ComponentName}
#[derive(Clone, Default)]
pub struct {ComponentName}Props {
    // Add fields based on user's answer
}

impl Component for {ComponentName} {
    type Props = {ComponentName}Props;
    type State = ();  // Use () if no local state needed
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        // Build the element tree
        vertical(vec![
            // Add child elements here
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::*;
    use crate::tui::renderer::render_element;

    #[test]
    fn test_{name_snake}_renders() {
        let component = {ComponentName};
        let props = {ComponentName}Props::default();
        let state = ();

        let element = component.view(&props, &state);

        let config = test_config();
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 10));
        render_element(&element, buf.area, &mut buf, &config);

        assert_buffer(&buf, &[
            // Expected output lines
        ]);
    }
}
```

**Step 2: Export from mod.rs**

Add to `src/tui/components/mod.rs`:
```rust
mod {name_snake};
pub use {name_snake}::{ComponentName, {ComponentName}Props};
```

**Step 3: Integrate into parent component**

Show how to use the new component:
```rust
use crate::tui::components::{ComponentName, {ComponentName}Props};

// In parent's view() method:
let props = {ComponentName}Props {
    // Fill from AppState
};
let component = {ComponentName};
let element = component.view(&props, &());
```

**Step 4: Run tests**
```bash
cargo test --lib tui::components::{name_snake} -- --nocapture
```

**Step 5: Verify compilation**
```bash
cargo check --lib
```

**Report:**
- Created: `src/tui/components/{name_snake}.rs`
- Exported from: `src/tui/components/mod.rs`
- Props struct: `{ComponentName}Props`
- Test: `test_{name_snake}_renders`
