# How Scrolling Works in Ratatui

## Core Concept: Ratatui Doesn't Manage Scroll State

**Key insight**: Ratatui widgets are **stateless**. They don't track scroll position themselves. Instead, **YOU** manage the scroll state in your application state, and pass it to the widget during rendering.

## The Two Main Approaches

### 1. Manual Scrolling (What You're Currently Using)

This is what your codebase does - you manage a `scroll_offset` in your state and use it to "window" your data before rendering.

**Example from your code:**

```rust
// In your state
pub struct PanelState {
    pub scroll_offset: usize,  // ← YOU track this
    pub selected_index: Option<usize>,
}

// In your reducer (when user presses PageDown)
fn scroll_panel_down(state: AppState, amount: usize) -> (AppState, Effect) {
    let mut new_state = state;
    if let Some(panel) = new_state.navigation.panel_stack.last_mut() {
        panel.scroll_offset = panel.scroll_offset.saturating_add(amount);
    }
    (new_state, Effect::None)
}

// In your rendering code
fn render(&self, area: Rect, buf: &mut Buffer) {
    let total_items = season_stats.len();
    let available_height = area.height as usize;

    // Calculate visible window using YOUR scroll_offset
    let visible_start = self.scroll_offset;
    let visible_end = (self.scroll_offset + available_height).min(total_items);

    // Slice the data to show only visible items
    let windowed_items = &season_stats[visible_start..visible_end];

    // Render only the windowed data
    render_table(windowed_items, area, buf);
}
```

**Pros:**
- Full control over scrolling behavior
- Works with any data structure
- You can implement custom scroll logic (auto-scroll to selection, etc.)

**Cons:**
- More code to write
- You handle all edge cases (bounds checking, etc.)

---

### 2. Using Ratatui's Built-in Scroll Support

Some ratatui widgets have built-in scroll support via `.scroll()` method:
- `Paragraph` - for text
- `List` - for lists (limited)
- `Table` - NO built-in scroll support (you must use manual approach)

**Example with Paragraph:**

```rust
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarState};

// In your state
pub struct TextViewState {
    pub scroll_offset: u16,  // ← YOU still track this
    pub total_lines: u16,
}

// In your rendering
impl RenderableWidget for TextViewWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Full text content (not windowed)
        let text = "Line 1\nLine 2\nLine 3\n....\nLine 100";

        // Create paragraph with YOUR scroll offset
        let paragraph = Paragraph::new(text)
            .scroll((self.scroll_offset, 0));  // ← Pass YOUR state here
                                                // (vertical_offset, horizontal_offset)

        paragraph.render(area, buf);
    }
}
```

**How `.scroll()` works internally:**
- Ratatui's `Paragraph` widget calculates which lines to display based on the scroll offset
- It skips the first N lines and renders starting from line `scroll_offset`
- The widget handles line wrapping and text layout for you

**Important:** Even with `.scroll()`, YOU still:
- Track `scroll_offset` in your state
- Update it when user presses PageUp/PageDown
- Pass it to the widget on each render

---

## 3. Scrollbar Widget (Visual Indicator Only)

Ratatui provides a `Scrollbar` widget to show scroll position, but it's **purely visual** - it doesn't handle input or update state.

```rust
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

// In your state
pub struct MyState {
    pub scroll_offset: usize,
    pub scrollbar_state: ScrollbarState,  // ← Ratatui provides this
}

// When scrolling changes
fn update_scrollbar(state: &mut MyState, total_items: usize, visible_items: usize) {
    state.scrollbar_state = state.scrollbar_state
        .content_length(total_items)       // Total items
        .viewport_content_length(visible_items)  // Visible items
        .position(state.scroll_offset);    // Current position
}

// In rendering
fn render(&self, area: Rect, buf: &mut Buffer) {
    // Render your content (with windowing)
    let content_area = area;  // or area.clip_right(1) if using scrollbar
    render_content(content_area, buf);

    // Render scrollbar on the right
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    let scrollbar_area = Rect::new(
        area.right().saturating_sub(1),
        area.y,
        1,
        area.height
    );

    scrollbar.render(scrollbar_area, buf, &mut self.scrollbar_state);
}
```

---

## Complete Example: Scrollable List

Here's a complete example combining all concepts:

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
    style::{Style, Color},
};

#[derive(Clone)]
pub struct ScrollableListWidget {
    items: Vec<String>,
    scroll_offset: usize,
    visible_items: usize,
}

impl ScrollableListWidget {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            scroll_offset: 0,
            visible_items: 0,
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.items.len().saturating_sub(self.visible_items);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }
}

impl RenderableWidget for ScrollableListWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Calculate visible window
        let visible_height = area.height.saturating_sub(2) as usize; // -2 for borders
        let visible_start = self.scroll_offset;
        let visible_end = (visible_start + visible_height).min(self.items.len());

        // Window the data
        let visible_items: Vec<ListItem> = self.items[visible_start..visible_end]
            .iter()
            .map(|s| ListItem::new(s.clone()))
            .collect();

        // Render the list
        let list = List::new(visible_items)
            .block(Block::default().borders(Borders::ALL).title("Items"));

        let list_area = area.clip_right(1); // Leave room for scrollbar
        list.render(list_area, buf);

        // Render scrollbar
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(self.items.len())
            .viewport_content_length(visible_height)
            .position(self.scroll_offset);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let scrollbar_area = Rect::new(
            area.right().saturating_sub(1),
            area.y + 1,  // +1 to account for top border
            1,
            area.height.saturating_sub(2), // -2 for borders
        );
        scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);
    }
}
```

---

## Key Patterns in Your Codebase

### Pattern 1: Auto-scroll to Keep Selection Visible

```rust
// When selection changes, adjust scroll to keep it visible
fn ensure_selection_visible(
    selected_index: usize,
    scroll_offset: usize,
    visible_height: usize,
    total_items: usize,
) -> usize {
    // If selection is above visible area, scroll up
    if selected_index < scroll_offset {
        return selected_index;
    }

    // If selection is below visible area, scroll down
    if selected_index >= scroll_offset + visible_height {
        return selected_index.saturating_sub(visible_height - 1);
    }

    // Selection is visible, don't change scroll
    scroll_offset
}
```

### Pattern 2: Bounds Checking

```rust
fn clamp_scroll_offset(
    scroll_offset: usize,
    total_items: usize,
    visible_items: usize,
) -> usize {
    let max_offset = total_items.saturating_sub(visible_items);
    scroll_offset.min(max_offset)
}
```

---

## Summary

**Ratatui scrolling is simple but manual:**

1. **You own the scroll state** - Track `scroll_offset` in your application state
2. **You handle input** - Update `scroll_offset` when user presses PageUp/PageDown/etc.
3. **You calculate the visible window** - Slice your data: `data[scroll_offset..scroll_offset+visible_height]`
4. **You render only visible data** - Pass windowed data to widgets
5. **Optional: Add visual scrollbar** - Use `Scrollbar` widget to show position

**Why this design?**
- Ratatui is a low-level rendering library, not a full UI framework
- It gives you complete control over behavior
- Works with any data structure or layout
- You can implement custom scroll logic (smooth scrolling, snap-to-item, etc.)

**What widgets support `.scroll()`?**
- `Paragraph` - for text content
- Some other text-based widgets

**What doesn't support `.scroll()`?**
- `Table` - you must manually window the data
- `List` - limited support, manual windowing is better
- Most other widgets - use manual approach
