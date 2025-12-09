# Comprehensive Document System Implementation Plan

## Implementation Status: ✅ CORE COMPLETE + STANDINGS DEMO

**Last Updated:** 2024-11-21

### What's Implemented:
- ✅ `src/tui/document/mod.rs` - Document trait and DocumentView container
- ✅ `src/tui/document/viewport.rs` - Viewport management with `ensure_visible_with_padding()`
- ✅ `src/tui/document/focus.rs` - FocusManager with Tab/Shift-Tab navigation and wrap detection
- ✅ `src/tui/document/elements.rs` - DocumentElement enum (Text, Heading, Link, Separator, Spacer, Group, Custom)
- ✅ `src/tui/document/link.rs` - LinkTarget, DocumentLink, DocumentType, LinkParams
- ✅ `src/tui/document/builder.rs` - DocumentBuilder fluent API
- ✅ Demo tab showcasing document system (`src/tui/components/demo_tab.rs`)
- ✅ Browser tab renamed to Demo
- ✅ `DocumentAction` enum in `action.rs` (FocusNext, FocusPrev, ActivateFocused, ScrollUp/Down, etc.)
- ✅ `DemoUiState` in AppState (focus_index, scroll_offset, viewport_height)
- ✅ Document reducer (`reducers/document.rs`) with full Tab/Shift-Tab handling
- ✅ Key bindings in `keys.rs` for Demo tab (Tab, Shift-Tab, Enter, arrows, Page keys)
- ✅ **Standings display in Demo tab** - League standings rendered at natural height with focusable team links
- ✅ **Dynamic focusable count** - Reducer calculates focusable elements based on loaded standings data

### Standings in Demo Tab:
The Demo tab now displays the full league standings as a demonstration of the document system:
- Standings data is passed from AppState via `DemoTabProps.standings`
- Each team is rendered as a focusable link (`link_with_id`)
- Teams are sorted by points (highest first)
- Display format: `Rank  Team                     GP   W   L  OT  PTS`
- Focus navigation cycles through all standings rows + the 4 example links
- Autoscrolling keeps focused team visible in viewport

### What's NOT Implemented:
- ❌ `document/renderer.rs` - Document-specific rendering logic (rendering is inline in mod.rs)
- ❌ `document/cache.rs` - Render caching for performance
- ❌ `document/tests/` - Separate test directory (tests are inline in each module)
- ❌ Full StandingsDocument as separate component (partial impl in DemoDocument)
- ❌ TableWidget `natural_height()` mode and `get_link_cells()`
- ❌ Migration of standings tab to use DocumentView

### Additional Changes Made:
- Renamed `RenderableWidget` trait in `component.rs` to `ElementWidget` to avoid confusion with `widgets::RenderableWidget`

---

## 1. Overview

The document system will enable unbounded content rendering with viewport-based scrolling. Components will render at their full natural height, and users will navigate through focusable elements using Tab/Shift-Tab, with automatic viewport scrolling to keep focused elements visible.

## 2. Module Structure

### New modules to create:

```
src/tui/document/
├── mod.rs              # Document trait, DocumentView container (ALREADY STARTED)
├── viewport.rs         # Viewport management (scrolling, visible range)
├── focus.rs            # Focus management (Tab navigation, focus tracking)
├── elements.rs         # DocumentElement types (Text, Table, Link, etc.)
├── link.rs             # Link handling and navigation
├── builder.rs          # Document builder utilities
├── renderer.rs         # Document-specific rendering logic
├── cache.rs            # Render caching for performance
└── tests/
    ├── mod.rs          # Test module entry
    ├── viewport_tests.rs
    ├── focus_tests.rs
    ├── document_tests.rs
    └── integration_tests.rs
```

### Modified modules:

```
src/tui/
├── action.rs           # Add DocumentAction variants
├── reducer.rs          # Add document reducer
├── reducers/
│   └── document.rs     # New reducer for document actions
├── state.rs            # Add DocumentState to AppState
└── components/
    └── standings_tab.rs # Migrate to use DocumentView
```

## 3. Core Traits and Structs

### 3.1 Document Trait (in `document/mod.rs`)

```rust
pub trait Document: Send + Sync {
    /// Build the document's element tree
    fn build(&self) -> Vec<DocumentElement>;

    /// Get the document's title for navigation/history
    fn title(&self) -> String;

    /// Get the document's unique ID
    fn id(&self) -> String;

    /// Calculate the total height needed to render all elements
    fn calculate_height(&self) -> u16 {
        self.build()
            .iter()
            .map(|elem| elem.height())
            .sum()
    }

    /// Render the document to a buffer at full height
    fn render_full(&self, width: u16, config: &DisplayConfig) -> (Buffer, u16);
}
```

### 3.2 Viewport (in `document/viewport.rs`)

```rust
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Current scroll offset from top
    offset: u16,
    /// Height of the viewport (visible area)
    height: u16,
    /// Total height of the content
    content_height: u16,
}

impl Viewport {
    pub fn new(offset: u16, height: u16, content_height: u16) -> Self {
        Self {
            offset: offset.min(content_height.saturating_sub(height)),
            height,
            content_height,
        }
    }

    /// Get the range of visible lines
    pub fn visible_range(&self) -> std::ops::Range<u16> {
        self.offset..self.offset.saturating_add(self.height).min(self.content_height)
    }

    /// Check if a rectangle is at least partially visible
    pub fn is_rect_visible(&self, rect: &Rect) -> bool {
        let visible = self.visible_range();
        rect.y < visible.end && rect.y + rect.height > visible.start
    }

    /// Ensure a line or region is visible, scrolling if necessary
    pub fn ensure_visible(&mut self, y: u16, height: u16) {
        let bottom = y + height;

        // If above viewport, scroll up
        if y < self.offset {
            self.offset = y;
        }
        // If below viewport, scroll down
        else if bottom > self.offset + self.height {
            self.offset = bottom.saturating_sub(self.height);
        }
    }

    /// Scroll operations
    pub fn scroll_up(&mut self, lines: u16) {
        self.offset = self.offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: u16) {
        let max_offset = self.content_height.saturating_sub(self.height);
        self.offset = (self.offset + lines).min(max_offset);
    }

    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.content_height.saturating_sub(self.height);
    }

    // Getters
    pub fn offset(&self) -> u16 { self.offset }
    pub fn height(&self) -> u16 { self.height }
    pub fn content_height(&self) -> u16 { self.content_height }

    // Setters
    pub fn set_height(&mut self, height: u16) {
        self.height = height;
        // Adjust offset if necessary
        let max_offset = self.content_height.saturating_sub(height);
        self.offset = self.offset.min(max_offset);
    }

    pub fn set_content_height(&mut self, height: u16) {
        self.content_height = height;
        // Adjust offset if necessary
        let max_offset = height.saturating_sub(self.height);
        self.offset = self.offset.min(max_offset);
    }
}
```

### 3.3 Focus Management (in `document/focus.rs`)

```rust
use super::link::LinkTarget;

#[derive(Debug, Clone)]
pub struct FocusableElement {
    /// ID for this focusable element
    pub id: String,
    /// Position in the document (y coordinate)
    pub y: u16,
    /// Height of the element
    pub height: u16,
    /// Rectangle of the focusable area
    pub rect: Rect,
    /// Optional link target if this is a link
    pub link_target: Option<LinkTarget>,
    /// Tab order (lower numbers get focus first)
    pub tab_order: i32,
}

#[derive(Debug, Clone)]
pub struct FocusManager {
    /// All focusable elements in document order
    elements: Vec<FocusableElement>,
    /// Currently focused element index
    current_focus: Option<usize>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            current_focus: None,
        }
    }

    /// Build from document elements
    pub fn from_elements(elements: &[DocumentElement]) -> Self {
        let mut focusable = Vec::new();
        let mut y_offset = 0u16;

        for element in elements {
            element.collect_focusable(&mut focusable, y_offset);
            y_offset += element.height();
        }

        // Sort by tab order, then by y position
        focusable.sort_by(|a, b| {
            a.tab_order.cmp(&b.tab_order)
                .then(a.y.cmp(&b.y))
        });

        Self {
            elements: focusable,
            current_focus: None,
        }
    }

    /// Navigate to next focusable element (Tab)
    pub fn focus_next(&mut self) -> bool {
        if self.elements.is_empty() {
            return false;
        }

        self.current_focus = match self.current_focus {
            None => Some(0),
            Some(idx) => Some((idx + 1) % self.elements.len()),
        };
        true
    }

    /// Navigate to previous focusable element (Shift-Tab)
    pub fn focus_prev(&mut self) -> bool {
        if self.elements.is_empty() {
            return false;
        }

        self.current_focus = match self.current_focus {
            None => Some(self.elements.len() - 1),
            Some(0) => Some(self.elements.len() - 1),
            Some(idx) => Some(idx - 1),
        };
        true
    }

    /// Get the currently focused element's position
    pub fn get_focused_position(&self) -> Option<u16> {
        self.current_focus.map(|idx| self.elements[idx].y)
    }

    /// Get the currently focused element's rectangle
    pub fn get_focused_rect(&self) -> Option<Rect> {
        self.current_focus.map(|idx| self.elements[idx].rect)
    }

    /// Activate the currently focused element
    pub fn activate_current(&self) -> Option<LinkTarget> {
        self.current_focus.and_then(|idx| {
            self.elements[idx].link_target.clone()
        })
    }

    /// Get the current link target without activating
    pub fn get_current_link(&self) -> Option<&LinkTarget> {
        self.current_focus.and_then(|idx| {
            self.elements[idx].link_target.as_ref()
        })
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        self.current_focus = None;
    }

    /// Focus a specific element by ID
    pub fn focus_by_id(&mut self, id: &str) -> bool {
        self.current_focus = self.elements.iter()
            .position(|e| e.id == id);
        self.current_focus.is_some()
    }
}
```

### 3.4 Document Elements (in `document/elements.rs`)

```rust
use super::focus::FocusableElement;
use super::link::LinkTarget;
use crate::tui::widgets::RenderableWidget;
use crate::tui::components::TableWidget;

/// Elements that can be part of a document
#[derive(Clone)]
pub enum DocumentElement {
    /// Plain text paragraph
    Text {
        content: String,
        style: Option<Style>,
    },

    /// Heading (different levels)
    Heading {
        level: u8, // 1-6
        content: String,
    },

    /// A table that renders all rows
    Table {
        widget: Box<TableWidget>,
        focusable_cells: Vec<(usize, usize, LinkTarget)>, // (row, col, target)
    },

    /// A link that can be focused and activated
    Link {
        display: String,
        target: LinkTarget,
        id: String,
    },

    /// Horizontal separator
    Separator,

    /// Vertical spacing
    Spacer {
        height: u16,
    },

    /// Custom widget
    Widget {
        widget: Box<dyn RenderableWidget>,
        height: u16,
    },

    /// Container for grouping elements
    Group {
        children: Vec<DocumentElement>,
        style: Option<Style>,
    },
}

impl DocumentElement {
    /// Calculate the height this element needs
    pub fn height(&self) -> u16 {
        match self {
            Self::Text { content, .. } => {
                // Count lines in text
                content.lines().count() as u16
            },
            Self::Heading { .. } => 2, // Heading + underline
            Self::Table { widget, .. } => {
                widget.natural_height()
            },
            Self::Link { .. } => 1,
            Self::Separator => 1,
            Self::Spacer { height } => *height,
            Self::Widget { height, .. } => *height,
            Self::Group { children, .. } => {
                children.iter().map(|c| c.height()).sum()
            },
        }
    }

    /// Collect focusable elements
    pub fn collect_focusable(&self, out: &mut Vec<FocusableElement>, y_offset: u16) {
        match self {
            Self::Link { display, target, id } => {
                out.push(FocusableElement {
                    id: id.clone(),
                    y: y_offset,
                    height: 1,
                    rect: Rect::new(0, y_offset, display.len() as u16, 1),
                    link_target: Some(target.clone()),
                    tab_order: 0,
                });
            },
            Self::Table { widget, focusable_cells } => {
                for (row, col, target) in focusable_cells {
                    let rect = widget.get_cell_rect(*row, *col);
                    out.push(FocusableElement {
                        id: format!("table_{}_{}", row, col),
                        y: y_offset + rect.y,
                        height: rect.height,
                        rect: Rect::new(rect.x, y_offset + rect.y, rect.width, rect.height),
                        link_target: Some(target.clone()),
                        tab_order: (*row * 100 + *col) as i32,
                    });
                }
            },
            Self::Group { children, .. } => {
                let mut child_offset = y_offset;
                for child in children {
                    child.collect_focusable(out, child_offset);
                    child_offset += child.height();
                }
            },
            _ => {}
        }
    }

    /// Render this element to a buffer
    pub fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        match self {
            Self::Text { content, style } => {
                // Render text line by line
                for (i, line) in content.lines().enumerate() {
                    if i as u16 >= area.height {
                        break;
                    }
                    let y = area.y + i as u16;
                    for (x, ch) in line.chars().enumerate() {
                        if x as u16 >= area.width {
                            break;
                        }
                        let idx = (y * buf.area.width + area.x + x as u16) as usize;
                        if idx < buf.content.len() {
                            buf.content[idx].set_char(ch);
                            if let Some(s) = style {
                                buf.content[idx].set_style(*s);
                            }
                        }
                    }
                }
            },
            Self::Heading { level, content } => {
                // Render heading with underline
                let style = match level {
                    1 => Style::default().bold().underlined(),
                    2 => Style::default().bold(),
                    _ => Style::default().underlined(),
                };
                // Render heading text
                for (x, ch) in content.chars().enumerate() {
                    if x as u16 >= area.width {
                        break;
                    }
                    let idx = (area.y * buf.area.width + area.x + x as u16) as usize;
                    if idx < buf.content.len() {
                        buf.content[idx].set_char(ch);
                        buf.content[idx].set_style(style);
                    }
                }
                // Render underline for level 1
                if *level == 1 && area.height > 1 {
                    for x in 0..area.width.min(content.len() as u16) {
                        let idx = ((area.y + 1) * buf.area.width + area.x + x) as usize;
                        if idx < buf.content.len() {
                            buf.content[idx].set_char('═');
                        }
                    }
                }
            },
            Self::Table { widget, .. } => {
                widget.render(area, buf, config);
            },
            Self::Link { display, .. } => {
                let style = Style::default().underlined().fg(Color::Blue);
                for (x, ch) in display.chars().enumerate() {
                    if x as u16 >= area.width {
                        break;
                    }
                    let idx = (area.y * buf.area.width + area.x + x as u16) as usize;
                    if idx < buf.content.len() {
                        buf.content[idx].set_char(ch);
                        buf.content[idx].set_style(style);
                    }
                }
            },
            Self::Separator => {
                for x in 0..area.width {
                    let idx = (area.y * buf.area.width + area.x + x) as usize;
                    if idx < buf.content.len() {
                        buf.content[idx].set_char('─');
                    }
                }
            },
            Self::Spacer { .. } => {
                // Just empty space
            },
            Self::Widget { widget, .. } => {
                widget.render(area, buf, config);
            },
            Self::Group { children, style } => {
                let mut y_offset = 0;
                for child in children {
                    let child_height = child.height();
                    if y_offset >= area.height {
                        break;
                    }
                    let child_area = Rect::new(
                        area.x,
                        area.y + y_offset,
                        area.width,
                        child_height.min(area.height - y_offset),
                    );
                    child.render(child_area, buf, config);
                    y_offset += child_height;
                }
                // Apply group style if any
                if let Some(s) = style {
                    for y in area.y..area.y + area.height {
                        for x in area.x..area.x + area.width {
                            let idx = (y * buf.area.width + x) as usize;
                            if idx < buf.content.len() {
                                let existing = buf.content[idx].style();
                                buf.content[idx].set_style(existing.patch(*s));
                            }
                        }
                    }
                }
            },
        }
    }
}
```

### 3.5 Link System (in `document/link.rs`)

```rust
/// Target of a document link
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Navigate to another document
    Document(DocumentLink),

    /// Navigate to a specific position in current document
    Anchor(String),

    /// External action (e.g., open modal, trigger command)
    Action(String),
}

/// Link to another document
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentLink {
    /// Document type
    pub doc_type: DocumentType,
    /// Parameters for the document
    pub params: LinkParams,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentType {
    Team,
    Player,
    Game,
    Standings,
    Schedule,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkParams {
    Team { abbrev: String },
    Player { id: i64 },
    Game { id: i64 },
    StandingsView { group_by: GroupBy },
    ScheduleDate { date: GameDate },
}
```

### 3.6 Document Builder (in `document/builder.rs`)

```rust
use super::elements::DocumentElement;

/// Builder for constructing documents
pub struct DocumentBuilder {
    elements: Vec<DocumentElement>,
}

impl DocumentBuilder {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn heading(mut self, level: u8, content: impl Into<String>) -> Self {
        self.elements.push(DocumentElement::Heading {
            level,
            content: content.into(),
        });
        self
    }

    pub fn text(mut self, content: impl Into<String>) -> Self {
        self.elements.push(DocumentElement::Text {
            content: content.into(),
            style: None,
        });
        self
    }

    pub fn styled_text(mut self, content: impl Into<String>, style: Style) -> Self {
        self.elements.push(DocumentElement::Text {
            content: content.into(),
            style: Some(style),
        });
        self
    }

    pub fn link(mut self, display: impl Into<String>, target: LinkTarget) -> Self {
        let display = display.into();
        self.elements.push(DocumentElement::Link {
            id: format!("link_{}", self.elements.len()),
            display,
            target,
        });
        self
    }

    pub fn table(mut self, widget: TableWidget) -> Self {
        // Extract focusable cells from the table
        let focusable_cells = widget.get_link_cells();

        self.elements.push(DocumentElement::Table {
            widget: Box::new(widget),
            focusable_cells,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.elements.push(DocumentElement::Separator);
        self
    }

    pub fn spacer(mut self, height: u16) -> Self {
        self.elements.push(DocumentElement::Spacer { height });
        self
    }

    pub fn widget(mut self, widget: Box<dyn RenderableWidget>, height: u16) -> Self {
        self.elements.push(DocumentElement::Widget { widget, height });
        self
    }

    pub fn group<F>(mut self, f: F) -> Self
    where
        F: FnOnce(DocumentBuilder) -> DocumentBuilder,
    {
        let group_builder = DocumentBuilder::new();
        let group_builder = f(group_builder);
        self.elements.push(DocumentElement::Group {
            children: group_builder.elements,
            style: None,
        });
        self
    }

    pub fn build(self) -> Vec<DocumentElement> {
        self.elements
    }
}
```

## 4. Integration with Component System

### 4.1 Action Updates (in `action.rs`)

```rust
#[derive(Clone, Debug)]
pub enum Action {
    // ... existing actions ...

    /// Document-related actions
    DocumentAction(DocumentAction),
}

#[derive(Clone, Debug)]
pub enum DocumentAction {
    /// Navigate to a document
    NavigateToDocument(DocumentLink),

    /// Navigate within document (Tab/Shift-Tab)
    FocusNext,
    FocusPrev,

    /// Activate focused element (Enter)
    ActivateFocused,

    /// Scroll viewport
    ScrollUp(u16),
    ScrollDown(u16),
    ScrollToTop,
    ScrollToBottom,
    PageUp,
    PageDown,

    /// Go back in document history
    NavigateBack,
}
```

### 4.2 State Updates (in `state.rs`)

```rust
#[derive(Debug, Clone, Default)]
pub struct AppState {
    // ... existing fields ...

    /// Document state
    pub document: DocumentState,
}

#[derive(Debug, Clone)]
pub struct DocumentState {
    /// Current document being viewed (if any)
    pub current_document: Option<Arc<dyn Document>>,

    /// Document view (manages viewport and focus)
    pub document_view: Option<DocumentView>,

    /// Navigation history
    pub history: Vec<DocumentLink>,

    /// Whether document mode is active
    pub active: bool,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            current_document: None,
            document_view: None,
            history: Vec::new(),
            active: false,
        }
    }
}
```

### 4.3 Document Reducer (in `reducers/document.rs`)

```rust
use crate::tui::state::AppState;
use crate::tui::action::{Action, DocumentAction};
use crate::tui::component::Effect;
use crate::tui::document::{DocumentView, StandingsDocument};

pub fn reduce_document(state: AppState, action: &Action) -> Option<(AppState, Effect)> {
    match action {
        Action::DocumentAction(doc_action) => {
            handle_document_action(state, doc_action)
        },
        _ => None,
    }
}

fn handle_document_action(mut state: AppState, action: &DocumentAction) -> Option<(AppState, Effect)> {
    match action {
        DocumentAction::NavigateToDocument(link) => {
            // Create the appropriate document
            let document = create_document_from_link(&link, &state);

            // Add to history
            if state.document.current_document.is_some() {
                state.document.history.push(link.clone());
            }

            // Create document view
            let viewport_height = 20; // This should come from terminal size
            let document_view = DocumentView::new(document.clone(), viewport_height);

            state.document.current_document = Some(document);
            state.document.document_view = Some(document_view);
            state.document.active = true;

            Some((state, Effect::None))
        },

        DocumentAction::FocusNext => {
            if let Some(view) = &mut state.document.document_view {
                view.focus_next();
            }
            Some((state, Effect::None))
        },

        DocumentAction::FocusPrev => {
            if let Some(view) = &mut state.document.document_view {
                view.focus_prev();
            }
            Some((state, Effect::None))
        },

        DocumentAction::ActivateFocused => {
            if let Some(view) = &state.document.document_view {
                if let Some(target) = view.activate_focused() {
                    match target {
                        LinkTarget::Document(link) => {
                            return Some((state, Effect::Action(
                                Action::DocumentAction(DocumentAction::NavigateToDocument(link))
                            )));
                        },
                        LinkTarget::Action(action_str) => {
                            // Handle custom actions
                            log::info!("Action triggered: {}", action_str);
                        },
                        LinkTarget::Anchor(anchor) => {
                            // Scroll to anchor in current document
                            log::info!("Scroll to anchor: {}", anchor);
                        },
                    }
                }
            }
            Some((state, Effect::None))
        },

        DocumentAction::ScrollDown(lines) => {
            if let Some(view) = &mut state.document.document_view {
                view.scroll_down(*lines);
            }
            Some((state, Effect::None))
        },

        DocumentAction::ScrollUp(lines) => {
            if let Some(view) = &mut state.document.document_view {
                view.scroll_up(*lines);
            }
            Some((state, Effect::None))
        },

        DocumentAction::PageDown => {
            if let Some(view) = &mut state.document.document_view {
                view.scroll_down(10); // Or viewport height - 1
            }
            Some((state, Effect::None))
        },

        DocumentAction::PageUp => {
            if let Some(view) = &mut state.document.document_view {
                view.scroll_up(10); // Or viewport height - 1
            }
            Some((state, Effect::None))
        },

        DocumentAction::ScrollToTop => {
            if let Some(view) = &mut state.document.document_view {
                view.scroll_to_top();
            }
            Some((state, Effect::None))
        },

        DocumentAction::ScrollToBottom => {
            if let Some(view) = &mut state.document.document_view {
                view.scroll_to_bottom();
            }
            Some((state, Effect::None))
        },

        DocumentAction::NavigateBack => {
            if let Some(link) = state.document.history.pop() {
                return Some((state, Effect::Action(
                    Action::DocumentAction(DocumentAction::NavigateToDocument(link))
                )));
            }
            Some((state, Effect::None))
        },
    }
}

fn create_document_from_link(link: &DocumentLink, state: &AppState) -> Arc<dyn Document> {
    match &link.params {
        LinkParams::StandingsView { group_by } => {
            Arc::new(StandingsDocument::new(
                state.data.standings.clone(),
                *group_by,
                state.system.config.clone(),
            ))
        },
        // Add other document types
        _ => todo!("Implement other document types"),
    }
}
```

## 5. Standings Document Implementation

### 5.1 StandingsDocument (in `document/standings_document.rs`)

```rust
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, LinkTarget, DocumentLink, LinkParams};
use crate::commands::standings::GroupBy;
use nhl_api::Standing;
use std::sync::Arc;

pub struct StandingsDocument {
    standings: Arc<Option<Vec<Standing>>>,
    group_by: GroupBy,
    config: Config,
}

impl StandingsDocument {
    pub fn new(standings: Arc<Option<Vec<Standing>>>, group_by: GroupBy, config: Config) -> Self {
        Self {
            standings,
            group_by,
            config,
        }
    }
}

impl Document for StandingsDocument {
    fn title(&self) -> String {
        format!("NHL Standings - {}", match self.group_by {
            GroupBy::Division => "Division",
            GroupBy::Conference => "Conference",
            GroupBy::League => "League",
        })
    }

    fn id(&self) -> String {
        format!("standings_{:?}", self.group_by)
    }

    fn build(&self) -> Vec<DocumentElement> {
        let mut builder = DocumentBuilder::new();

        builder = builder.heading(1, self.title());

        if let Some(standings) = self.standings.as_ref() {
            match self.group_by {
                GroupBy::League => {
                    // Single table with all teams
                    builder = builder.spacer(1);

                    let table = create_standings_table(standings, &self.config);
                    builder = builder.table(table);
                },
                GroupBy::Conference => {
                    // Group by conference
                    let mut eastern = Vec::new();
                    let mut western = Vec::new();

                    for standing in standings {
                        if let Some(conf) = &standing.conference_name {
                            if conf.contains("Eastern") {
                                eastern.push(standing.clone());
                            } else {
                                western.push(standing.clone());
                            }
                        }
                    }

                    builder = builder.spacer(1)
                        .heading(2, "Eastern Conference")
                        .table(create_standings_table(&eastern, &self.config))
                        .spacer(2)
                        .heading(2, "Western Conference")
                        .table(create_standings_table(&western, &self.config));
                },
                GroupBy::Division => {
                    // Group by division
                    let mut divisions = std::collections::BTreeMap::new();

                    for standing in standings {
                        if let Some(div) = &standing.division_name {
                            divisions.entry(div.clone())
                                .or_insert_with(Vec::new)
                                .push(standing.clone());
                        }
                    }

                    for (i, (division, teams)) in divisions.into_iter().enumerate() {
                        if i > 0 {
                            builder = builder.spacer(2);
                        }
                        builder = builder
                            .heading(2, &division)
                            .table(create_standings_table(&teams, &self.config));
                    }
                },
            }
        } else {
            builder = builder.text("Loading standings data...");
        }

        builder.build()
    }

    fn render_full(&self, width: u16, config: &DisplayConfig) -> (Buffer, u16) {
        let elements = self.build();
        let height = elements.iter().map(|e| e.height()).sum();

        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        let mut y_offset = 0;

        for element in elements {
            let element_height = element.height();
            let area = Rect::new(0, y_offset, width, element_height);
            element.render(area, &mut buffer, config);
            y_offset += element_height;
        }

        (buffer, height)
    }
}

fn create_standings_table(standings: &[Standing], config: &Config) -> TableWidget {
    let columns = vec![
        ColumnDef::new("Team", 26, Alignment::Left, |s: &Standing| {
            CellValue::TeamLink {
                display: s.team_common_name.default.clone(),
                team_abbrev: s.team_abbrev.default.clone(),
            }
        }),
        ColumnDef::new("GP", 4, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.games_played().to_string())
        }),
        ColumnDef::new("W", 4, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.wins.to_string())
        }),
        ColumnDef::new("L", 3, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.losses.to_string())
        }),
        ColumnDef::new("OT", 3, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.ot_losses.to_string())
        }),
        ColumnDef::new("PTS", 5, Alignment::Right, |s: &Standing| {
            CellValue::Text(s.points.to_string())
        }),
    ];

    TableWidget::from_data(&columns, standings.to_vec())
        .with_natural_height()  // This makes the table render ALL rows
        .with_margin(0)
}
```

## 6. Testing Strategy

### 6.1 Test Structure

Each module should have comprehensive unit tests achieving 100% coverage:

```rust
// In document/tests/viewport_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::assert_buffer;

    #[test]
    fn test_viewport_visible_range() {
        let viewport = Viewport::new(10, 20, 100);
        assert_eq!(viewport.visible_range(), 10..30);
    }

    #[test]
    fn test_viewport_scroll_down() {
        let mut viewport = Viewport::new(0, 20, 100);
        viewport.scroll_down(5);
        assert_eq!(viewport.offset(), 5);
        assert_eq!(viewport.visible_range(), 5..25);
    }

    #[test]
    fn test_viewport_scroll_down_clamps_at_bottom() {
        let mut viewport = Viewport::new(85, 20, 100);
        viewport.scroll_down(20);
        assert_eq!(viewport.offset(), 80); // 100 - 20 = 80
        assert_eq!(viewport.visible_range(), 80..100);
    }

    #[test]
    fn test_ensure_visible_scrolls_up() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible(15, 2);
        assert_eq!(viewport.offset(), 15);
    }

    #[test]
    fn test_ensure_visible_scrolls_down() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible(35, 2);
        assert_eq!(viewport.offset(), 27); // 35 + 2 - 10 = 27
    }

    #[test]
    fn test_is_rect_visible() {
        let viewport = Viewport::new(20, 10, 100);

        // Fully visible
        assert!(viewport.is_rect_visible(&Rect::new(0, 22, 10, 3)));

        // Partially visible (top cut off)
        assert!(viewport.is_rect_visible(&Rect::new(0, 18, 10, 5)));

        // Partially visible (bottom cut off)
        assert!(viewport.is_rect_visible(&Rect::new(0, 28, 10, 5)));

        // Not visible (above)
        assert!(!viewport.is_rect_visible(&Rect::new(0, 10, 10, 5)));

        // Not visible (below)
        assert!(!viewport.is_rect_visible(&Rect::new(0, 35, 10, 5)));
    }
}
```

### 6.2 Focus Manager Tests

```rust
// In document/tests/focus_tests.rs

#[test]
fn test_focus_navigation_wraps() {
    let mut focus = FocusManager::new();
    focus.elements = vec![
        FocusableElement {
            id: "link1".to_string(),
            y: 0,
            height: 1,
            rect: Rect::new(0, 0, 10, 1),
            link_target: Some(LinkTarget::Action("test".to_string())),
            tab_order: 0,
        },
        FocusableElement {
            id: "link2".to_string(),
            y: 5,
            height: 1,
            rect: Rect::new(0, 5, 10, 1),
            link_target: Some(LinkTarget::Action("test2".to_string())),
            tab_order: 1,
        },
    ];

    // Start with no focus
    assert_eq!(focus.current_focus, None);

    // Tab focuses first element
    assert!(focus.focus_next());
    assert_eq!(focus.current_focus, Some(0));

    // Tab again focuses second element
    assert!(focus.focus_next());
    assert_eq!(focus.current_focus, Some(1));

    // Tab again wraps to first
    assert!(focus.focus_next());
    assert_eq!(focus.current_focus, Some(0));

    // Shift-Tab goes backward
    assert!(focus.focus_prev());
    assert_eq!(focus.current_focus, Some(1));
}

#[test]
fn test_activate_returns_link_target() {
    let mut focus = FocusManager::new();
    let target = LinkTarget::Document(DocumentLink {
        doc_type: DocumentType::Team,
        params: LinkParams::Team { abbrev: "TOR".to_string() },
    });

    focus.elements = vec![
        FocusableElement {
            id: "team_link".to_string(),
            y: 0,
            height: 1,
            rect: Rect::new(0, 0, 10, 1),
            link_target: Some(target.clone()),
            tab_order: 0,
        },
    ];

    focus.focus_next();

    let activated = focus.activate_current();
    assert_eq!(activated, Some(target));
}
```

### 6.3 Document Rendering Tests

```rust
// In document/tests/document_tests.rs

#[test]
fn test_document_view_renders_visible_portion() {
    // Create a simple test document
    let doc = Arc::new(TestDocument::new(100)); // 100 lines tall
    let mut view = DocumentView::new(doc, 20); // 20 lines visible

    // Render at top
    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 20));
    let config = DisplayConfig::default();
    view.render(Rect::new(0, 0, 80, 20), &mut buffer, &config);

    // Check first line is visible
    assert_buffer(&buffer, &[
        "Line 0                                                                          ",
        "Line 1                                                                          ",
        // ... 18 more lines
    ]);

    // Scroll down 10 lines
    view.scroll_down(10);
    buffer = Buffer::empty(Rect::new(0, 0, 80, 20));
    view.render(Rect::new(0, 0, 80, 20), &mut buffer, &config);

    // Check lines 10-29 are visible
    assert_buffer(&buffer, &[
        "Line 10                                                                         ",
        "Line 11                                                                         ",
        // ... 18 more lines
    ]);
}

#[test]
fn test_focus_highlighting() {
    let doc = Arc::new(DocumentWithLinks::new());
    let mut view = DocumentView::new(doc, 20);

    // Focus first link
    view.focus_next();

    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 20));
    let config = DisplayConfig::default();
    view.render(Rect::new(0, 0, 80, 20), &mut buffer, &config);

    // Check that the focused link has selection style
    // This requires checking buffer styles, not just text
    let line_with_link = 5; // Assuming link is on line 5
    for x in 0..10 { // Link is 10 chars wide
        let idx = (line_with_link * 80 + x) as usize;
        assert_eq!(buffer.content[idx].style(), config.selection_style());
    }
}
```

### 6.4 Integration Tests

```rust
// In document/tests/integration_tests.rs

#[test]
fn test_standings_document_full_render() {
    let standings = create_test_standings(); // From tui::testing
    let doc = StandingsDocument::new(
        Arc::new(Some(standings)),
        GroupBy::League,
        Config::default(),
    );

    let (buffer, height) = doc.render_full(100, &DisplayConfig::default());

    // Check header
    let lines = buffer_lines(&buffer);
    assert!(lines[0].starts_with("NHL Standings - League"));

    // Check we have all 32 teams (header + 32 rows)
    assert!(height >= 33);

    // Check table headers
    assert!(lines[2].contains("Team"));
    assert!(lines[2].contains("GP"));
    assert!(lines[2].contains("W"));
    assert!(lines[2].contains("L"));
    assert!(lines[2].contains("OT"));
    assert!(lines[2].contains("PTS"));
}

#[test]
fn test_tab_navigation_through_document() {
    let standings = create_test_standings();
    let doc = Arc::new(StandingsDocument::new(
        Arc::new(Some(standings)),
        GroupBy::League,
        Config::default(),
    ));

    let mut view = DocumentView::new(doc, 20);

    // Tab through all team links
    for i in 0..32 {
        assert!(view.focus_next());

        // Check that viewport scrolls to keep focused item visible
        if let Some(focused_y) = view.focus_manager.get_focused_position() {
            let visible = view.viewport.visible_range();
            assert!(focused_y >= visible.start && focused_y < visible.end,
                "Focus at y={} not visible in range {:?}", focused_y, visible);
        }
    }

    // Tab again wraps to first
    assert!(view.focus_next());
    assert_eq!(view.viewport.offset(), 0); // Should scroll back to top
}
```

## 7. Migration Plan for Standings League View

### Step 1: Implement Core Document System
1. Create all document modules as specified
2. Write comprehensive unit tests for each module
3. Ensure 100% test coverage

### Step 2: Create StandingsDocument
1. Implement StandingsDocument for League view
2. Modify TableWidget to support `natural_height()` mode
3. Add `get_link_cells()` method to TableWidget
4. Test document rendering at various heights

### Step 3: Integrate with Existing System
1. Add DocumentAction variants to Action enum
2. Add DocumentState to AppState
3. Implement document reducer
4. Add key bindings for document navigation

### Step 4: Modify StandingsTab Component
1. Add a flag to switch between old and new rendering
2. When in League view, use DocumentView instead of direct rendering
3. Route navigation keys to DocumentAction when in document mode

### Step 5: Testing and Refinement
1. Test navigation with real data
2. Test viewport scrolling with various terminal sizes
3. Test focus management with Tab/Shift-Tab
4. Performance test with large documents

### Step 6: Gradual Migration
1. Enable document mode for League view only initially
2. Gather feedback and fix issues
3. Migrate Conference view
4. Migrate Division view
5. Create team detail documents
6. Create player detail documents

## 8. Key Implementation Details

### 8.1 TableWidget Natural Height

The TableWidget needs a new mode where it renders ALL rows:

```rust
impl TableWidget {
    pub fn with_natural_height(mut self) -> Self {
        self.natural_height_mode = true;
        self
    }

    pub fn natural_height(&self) -> u16 {
        if self.natural_height_mode {
            // Header (3) + all rows
            3 + self.data.len() as u16
        } else {
            // Use specified height
            self.height.unwrap_or(10)
        }
    }

    pub fn get_link_cells(&self) -> Vec<(usize, usize, LinkTarget)> {
        let mut cells = Vec::new();

        for (row_idx, row) in self.data.iter().enumerate() {
            for (col_idx, col_def) in self.columns.iter().enumerate() {
                let cell = (col_def.extract)(row);
                match cell {
                    CellValue::TeamLink { team_abbrev, .. } => {
                        cells.push((row_idx, col_idx, LinkTarget::Document(
                            DocumentLink {
                                doc_type: DocumentType::Team,
                                params: LinkParams::Team { abbrev: team_abbrev },
                            }
                        )));
                    },
                    CellValue::PlayerLink { player_id, .. } => {
                        cells.push((row_idx, col_idx, LinkTarget::Document(
                            DocumentLink {
                                doc_type: DocumentType::Player,
                                params: LinkParams::Player { id: player_id },
                            }
                        )));
                    },
                    _ => {}
                }
            }
        }

        cells
    }

    pub fn get_cell_rect(&self, row: usize, col: usize) -> Rect {
        // Calculate the x position based on column widths
        let mut x = 0;
        for i in 0..col {
            x += self.columns[i].width;
        }

        // y is header (3 lines) + row index
        let y = 3 + row as u16;

        Rect::new(x, y, self.columns[col].width, 1)
    }
}
```

### 8.2 Efficient Viewport Rendering

To avoid re-rendering the entire document on every frame:

1. Cache the full rendered buffer when document changes
2. Only re-render if width changes or document is modified
3. Copy visible portion from cached buffer to output buffer
4. Apply focus highlighting on top

### 8.3 Performance Considerations

1. **Lazy Rendering**: Only render elements that are visible or near-visible
2. **Buffer Pooling**: Reuse buffers to reduce allocations
3. **Incremental Updates**: Track dirty regions and only re-render changed parts
4. **Virtual Scrolling**: For very large documents (>1000 lines), use virtual scrolling

### 8.4 Testing with assert_buffer

Always use full buffer comparison:

```rust
// GOOD - checks entire output
assert_buffer(&buffer, &[
    "Line 1 of expected output                       ",
    "Line 2 of expected output                       ",
    "Line 3 of expected output                       ",
]);

// BAD - only checks substring
assert!(buffer_text.contains("some text"));
```

For viewport tests, create expected output for the visible portion:

```rust
#[test]
fn test_viewport_clipping() {
    let doc = create_100_line_document();
    let mut view = DocumentView::new(doc, 5); // 5 lines visible

    view.scroll_down(20); // Scroll to line 20

    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 5));
    view.render(Rect::new(0, 0, 80, 5), &mut buffer, &config);

    assert_buffer(&buffer, &[
        "Line 20                                                                         ",
        "Line 21                                                                         ",
        "Line 22                                                                         ",
        "Line 23                                                                         ",
        "Line 24                                                                         ",
    ]);
}
```

## 9. Success Criteria

The implementation will be considered successful when:

1. **100% test coverage** for all new document modules
2. **All tests use assert_buffer** for rendering verification
3. **Standings League view** renders all 32 teams without scrolling issues
4. **Tab navigation** cycles through all team links correctly
5. **Viewport scrolling** follows focused elements automatically
6. **Performance** remains smooth with documents up to 1000 lines
7. **Memory usage** is reasonable (no leaks, efficient buffer reuse)
8. **Integration** works seamlessly with existing Component/Element system

## 10. Future Enhancements

After the initial implementation:

1. **Document Caching**: Cache rendered documents for faster navigation
2. **Incremental Rendering**: Only re-render changed portions
3. **Search in Document**: Ctrl+F to search and highlight text
4. **Bookmarks**: Save positions in documents for quick navigation
5. **Document History**: Back/forward navigation through visited documents
6. **Link Preview**: Show preview of link target on hover
7. **Markdown Support**: Parse and render markdown documents
8. **Theming**: Support different color schemes for documents

This plan provides a complete roadmap for implementing the document system with all the architectural details, testing strategies, and migration steps clearly defined.