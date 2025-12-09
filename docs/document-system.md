# Document System

The document system (`src/tui/document/`) provides scrollable, focusable content views for content that exceeds viewport height.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Document Trait                          │
│  - build(focus) -> Vec<DocumentElement>                     │
│  - focusable_positions() -> Vec<u16>                        │
│  - focusable_ids() -> Vec<FocusableId>                      │
│  - focusable_row_positions() -> Vec<Option<RowPosition>>    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DocumentView                             │
│  - Holds Arc<dyn Document>                                  │
│  - Manages Viewport (scroll offset, height)                 │
│  - Manages FocusManager (focus state, navigation)           │
│  - Renders visible portion to Buffer                        │
└─────────────────────────────────────────────────────────────┘
```

## Document Trait

```rust
pub trait Document: Send + Sync {
    /// Build the element tree (called on each render)
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement>;

    /// Document title for navigation/history
    fn title(&self) -> String;

    /// Unique document ID
    fn id(&self) -> String;

    // Default implementations provided:
    fn calculate_height(&self) -> u16;
    fn focusable_positions(&self) -> Vec<u16>;
    fn focusable_ids() -> Vec<FocusableId>;
    fn focusable_row_positions(&self) -> Vec<Option<RowPosition>>;
}
```

## DocumentElement Types

```rust
pub enum DocumentElement {
    Heading { level: u8, text: String },
    Text(String),
    Link { id: String, text: String, target: LinkTarget, focused: bool },
    Separator,
    Blank,
    Group { children: Vec<DocumentElement> },
    Table { widget: TableWidget, name: String },
    Row { children: Vec<DocumentElement>, gap: u16 },  // Horizontal layout
    Custom { widget: Box<dyn ElementWidget>, height: u16 },
}
```

**Key types:**
- **Link**: Focusable element with `LinkTarget` for activation
- **Table**: Embeds `TableWidget` with focusable cells
- **Row**: Horizontal layout enabling left/right navigation
- **Group**: Vertical container for nested elements

## Row Navigation (Left/Right)

The `Row` element enables horizontal navigation between side-by-side content:

```rust
DocumentBuilder::new()
    .row(vec![
        DocumentElement::table(left_table, "left"),
        DocumentElement::table(right_table, "right"),
    ])
```

### How it works

Each element in a Row gets a `RowPosition`:

```rust
pub struct RowPosition {
    pub row_y: u16,           // Y position identifying the Row
    pub child_idx: usize,     // 0 = leftmost, 1 = next, etc.
    pub idx_within_child: usize, // Position within that child
}
```

Left/Right arrows find the element with matching `row_y` and `idx_within_child` in the adjacent `child_idx`.

**Wrapping**: Left at leftmost wraps to rightmost; Right at rightmost wraps to leftmost.

**Example:**
```
Row with two tables (5 rows each):
┌─────────────┐  ┌─────────────┐
│ Table Left  │  │ Table Right │
├─────────────┤  ├─────────────┤
│ Row 0 ◄─────┼──┼─► Row 0     │  ← Left/Right moves between tables
│ Row 1 ◄─────┼──┼─► Row 1     │     preserving row position
│ Row 2       │  │   Row 2     │
└─────────────┘  └─────────────┘
```

## Focus Navigation

**Up/Down:**
- Cycles through all focusable elements in document order
- Wraps from last to first (Down) or first to last (Up)
- Autoscrolls viewport to keep focused element visible

**Left/Right (within Rows):**
- Only works when focused element is inside a Row
- Moves to same relative position in adjacent child
- Wraps around at edges

**Enter:**
- Activates the focused element
- Returns `LinkTarget` for navigation actions

## DocumentBuilder

Declarative API for constructing documents:

```rust
let doc = DocumentBuilder::new()
    .heading(1, "Player Stats")
    .blank()
    .text("Top scorers this season:")
    .table(stats_table, "scorers")
    .separator()
    .link("back", "← Back", LinkTarget::Action("go_back".into()))
    .when(show_details, |b| b.text("Additional details..."))
    .for_each(players, |b, player| b.text(format!("- {}", player.name)))
    .build();
```

## Generic Document Navigation (document_nav.rs)

Reusable navigation module that eliminates duplication across components.

### DocumentNavState

```rust
#[derive(Debug, Clone, Default)]
pub struct DocumentNavState {
    pub focus_index: Option<usize>,
    pub scroll_offset: u16,
    pub viewport_height: u16,
    pub focusable_positions: Vec<u16>,
    pub focusable_heights: Vec<u16>,
    pub focusable_ids: Vec<FocusableId>,
    pub focusable_row_positions: Vec<Option<RowPosition>>,
    pub link_targets: Vec<Option<LinkTarget>>,
}
```

### DocumentNavMsg

```rust
pub enum DocumentNavMsg {
    FocusNext,
    FocusPrev,
    FocusLeft,
    FocusRight,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    ScrollToTop,
    ScrollToBottom,
    UpdateViewportHeight(u16),
}
```

### Usage

1. Embed `DocumentNavState` in component state
2. Wrap `DocumentNavMsg` in component messages
3. Delegate to `document_nav::handle_message()` in update

```rust
impl Component for MyTab {
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            MyTabMsg::DocNav(nav_msg) => {
                document_nav::handle_message(&nav_msg, &mut state.doc_nav)
                    .unwrap_or(Effect::None)
            }
            // ...
        }
    }
}
```

## Creating a New Document

1. **Define the document struct:**
   ```rust
   pub struct MyDocument {
       data: Vec<MyData>,
   }
   ```

2. **Implement Document trait:**
   ```rust
   impl Document for MyDocument {
       fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
           DocumentBuilder::new()
               .heading(1, "My Document")
               .for_each(&self.data, |b, item| {
                   b.link(&item.id, &item.name, LinkTarget::Document(...))
               })
               .build()
       }

       fn title(&self) -> String { "My Document".into() }
       fn id(&self) -> String { "my_doc".into() }
   }
   ```

3. **Store focusable metadata in state** when data changes

4. **Handle navigation** via `DocumentNavMsg` in component
