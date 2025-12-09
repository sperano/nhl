Create a new low-level RenderableWidget for direct buffer rendering.

Ask the user:
1. **Widget name** (e.g., "TeamLogo", "StatBar", "MiniScoreboard")
2. **What does it render?** (brief description)
3. **Fixed or dynamic size?** (specify dimensions or "dynamic")
4. **What data does it need?** (fields for the struct)

**Step 1: Create the widget file**

Create `src/tui/widgets/{name_snake}.rs`:

```rust
use crate::tui::widgets::RenderableWidget;
use crate::config::DisplayConfig;
use ratatui::prelude::*;

/// {Brief description}
#[derive(Clone)]
pub struct {WidgetName} {
    // Fields based on user's answer
}

impl {WidgetName} {
    pub fn new(/* params */) -> Self {
        Self {
            // Initialize fields
        }
    }
}

impl RenderableWidget for {WidgetName} {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Direct buffer rendering
        // Use buf.set_string(), buf.set_style(), etc.
    }

    fn clone_box(&self) -> Box<dyn RenderableWidget> {
        Box::new(self.clone())
    }

    fn preferred_height(&self) -> Option<u16> {
        // Return Some(n) for fixed height, None for flexible
        None
    }

    fn preferred_width(&self) -> Option<u16> {
        // Return Some(n) for fixed width, None for flexible
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::*;
    use crate::tui::widgets::testing::render_widget;

    #[test]
    fn test_{name_snake}_renders() {
        let widget = {WidgetName}::new(/* test data */);
        let config = test_config();

        let buf = render_widget(&widget, 20, 5, &config);

        assert_buffer(&buf, &[
            // Expected output lines
        ]);
    }

    #[test]
    fn test_{name_snake}_empty() {
        let widget = {WidgetName}::new(/* empty data */);
        let config = test_config();

        let buf = render_widget(&widget, 20, 5, &config);

        assert_buffer(&buf, &[
            // Expected empty state
        ]);
    }
}
```

**Step 2: Export from mod.rs**

Add to `src/tui/widgets/mod.rs`:
```rust
mod {name_snake};
pub use {name_snake}::{WidgetName};
```

**Step 3: Show usage in a component**

```rust
use crate::tui::widgets::{WidgetName};
use crate::tui::component::Element;

// In a component's view() method:
let widget = {WidgetName}::new(/* data */);
Element::Widget(Box::new(widget))
```

**Step 4: Run tests**
```bash
cargo test --lib tui::widgets::{name_snake} -- --nocapture
```

**Widget vs Component Decision Guide:**
- Use **Widget** when: Direct buffer control needed, performance critical, self-contained visual element
- Use **Component** when: Composing other elements, needs lifecycle/state, part of larger UI hierarchy

**Report:**
- Created: `src/tui/widgets/{name_snake}.rs`
- Exported from: `src/tui/widgets/mod.rs`
- Tests: `test_{name_snake}_renders`, `test_{name_snake}_empty`
