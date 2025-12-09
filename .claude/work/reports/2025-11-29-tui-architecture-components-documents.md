# TUI Architecture: Components, State, Props, Widgets, and Documents

**Date**: 2025-11-29

## Overview

This report documents the React/Redux-inspired TUI architecture, explaining the relationships between components, state, props, widgets, and the document system.

---

## 1. Components

Components are the main building blocks. They implement the `Component` trait with three associated types:

```rust
pub trait Component: Send {
    type Props: Clone;           // Input data (read-only)
    type State: Default + Clone; // Local mutable state
    type Message;                // Events the component handles

    fn init(props: &Self::Props) -> Self::State;
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect;
    fn view(&self, props: &Self::Props, state: &Self::State) -> Element;
    fn should_update(&self, old_props: &Self::Props, new_props: &Self::Props) -> bool;
    fn did_update(&mut self, old_props: &Self::Props, new_props: &Self::Props) -> Effect;
}
```

**Key principle**: Components own their UI state. The global `AppState` only holds shared data (API responses, current tab, etc.).

### Lifecycle Methods

- `init()`: Called once when component is first rendered. Creates initial state from props
- `update()`: Message handler that mutates component state and can return Effects
- `view()`: Pure function that takes props+state and returns an Element tree
- `should_update()`: Memoization check (default always re-renders)
- `did_update()`: Lifecycle hook when props change

### Real Example: ScoresTab

```rust
// Props - received from AppState
pub struct ScoresTabProps {
    pub schedule: Arc<Option<DailySchedule>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,
    pub period_scores: Arc<HashMap<i64, PeriodScores>>,
    pub focused: bool,
}

// State - owned by the component
pub struct ScoresTabState {
    pub selected_date_index: usize,
    pub game_date: GameDate,
    pub doc_nav: DocumentNavState,
}

// Messages - events the component handles
pub enum ScoresTabMsg {
    NavigateLeft,
    NavigateRight,
    EnterBoxSelection,
    ExitBoxSelection,
    DocNav(DocumentNavMsg),
    UpdateViewportHeight(u16),
    ActivateGame,
}
```

---

## 2. Props vs State

| Concept | Where It Lives | Examples |
|---------|---------------|----------|
| **Props** | Passed from parent/global state | API data (`Arc<Option<DailySchedule>>`), `focused: bool` |
| **State** | Owned by component | `selected_date_index`, `scroll_offset`, `doc_nav` |

### Decision Rules

**Use Props for:**
- API data that comes from global state
- Navigation state (which tab is active, is content focused?)
- Configuration that applies globally
- Anything multiple components need to read

**Use Component State for:**
- UI state: selected indices, scroll positions, focus
- Component-specific modes (browse mode, edit mode)
- View preferences (which standings view: League/Conference/Division)
- Temporary UI flags (modal open, dropdown expanded)

### Props Construction

Props are assembled at the root level and passed down:

```rust
let scores_props = ScoresTabProps {
    schedule: state.data.schedule.clone(),      // Arc::clone is cheap
    game_info: state.data.game_info.clone(),
    period_scores: state.data.period_scores.clone(),
    focused: state.navigation.current_tab == Tab::Scores,
};

let element = scores_tab.view(&scores_props, &scores_state);
```

---

## 3. Global State (AppState)

From `src/tui/state.rs`:

```rust
pub struct AppState {
    pub navigation: NavigationState,     // Current tab, document stack
    pub data: DataState,                  // API data (Arc-wrapped)
    pub ui: UiState,                      // Data effect triggers
    pub system: SystemState,              // Config, status messages
}

pub struct NavigationState {
    pub current_tab: Tab,
    pub document_stack: Vec<DocumentStackEntry>,
    pub content_focused: bool,
}

pub struct DataState {
    pub standings: Arc<Option<Vec<Standing>>>,
    pub schedule: Arc<Option<DailySchedule>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,
    pub loading: HashSet<LoadingKey>,
    pub errors: HashMap<String, String>,
}
```

### State Ownership Rules

| State Type | Global | Component-Local |
|-----------|--------|-----------------|
| API data | ✓ | ✗ |
| Navigation (tab, stack) | ✓ | ✗ |
| Configuration | ✓ | ✗ |
| Selected index (UI) | ✗ | ✓ |
| Scroll offset | ✗ | ✓ |
| Focus state | ✗ | ✓ |
| Document nav state | ✗ | ✓ |
| Browse mode flags | ✗ | ✓ |

---

## 4. Widgets vs Components

### StandaloneWidget (Low-level rendering)

```rust
pub trait StandaloneWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig);
    fn preferred_height(&self) -> Option<u16> { None }
    fn preferred_width(&self) -> Option<u16> { None }
}
```

**Characteristics:**
- Simple rendering primitives
- No state management
- No messages
- Direct buffer manipulation
- Examples: `GameBox`, `ScoreTable`, `SettingsListWidget`

### ElementWidget (Component-tree participating)

```rust
pub trait ElementWidget: Send + Sync {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig);
    fn clone_box(&self) -> Box<dyn ElementWidget>;
    fn preferred_height(&self) -> Option<u16> { None }
    fn preferred_width(&self) -> Option<u16> { None }
}
```

**Characteristics:**
- Requires Send + Sync for thread-safe dyn dispatch
- Can be cloned into trait objects
- Participates in Element tree
- Examples: `DocumentView`, `BoxscoreDocumentWidget`

### Comparison Table

| Aspect | Component | StandaloneWidget |
|--------|-----------|-----------------|
| **Purpose** | Manages UI + state | Pure rendering |
| **State management** | Props + own State | None |
| **Lifecycle** | init, update, view, did_update | Just render |
| **Messages** | Yes, handled in update() | No |
| **Element tree** | Yes, returns Element | Can be embedded in Element |
| **Use case** | "Smart" components with behavior | "Dumb" widgets |

**Analogy**: Widgets are like HTML `<div>` with styling. Components are like React components with hooks.

---

## 5. The Document System

The document system solves: **how do you scroll through content larger than the viewport?**

### Document Trait

```rust
pub trait Document: Send + Sync {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement>;
    fn title(&self) -> String;
    fn id(&self) -> String;

    // Metadata extraction (default implementations)
    fn calculate_height(&self) -> u16;
    fn focusable_positions(&self) -> Vec<u16>;
    fn focusable_heights(&self) -> Vec<u16>;
    fn focusable_ids(&self) -> Vec<FocusableId>;
    fn focusable_row_positions(&self) -> Vec<Option<RowPosition>>;
    fn focusable_link_targets(&self) -> Vec<Option<LinkTarget>>;
}
```

### DocumentElement Types

```rust
pub enum DocumentElement {
    Text { content: String, style: Option<Style> },
    Heading { level: u8, content: String },
    Link { display: String, target: LinkTarget, id: String, focused: bool },
    Separator,
    Spacer { height: u16 },
    Group { children: Vec<DocumentElement>, style: Option<Style> },
    Table { widget: TableWidget, focusable: Vec<FocusableElement> },
    Row { children: Vec<DocumentElement>, gap: u16 },
    GameBoxElement { id: String, game_id: i64, game_box: GameBox, focused: bool },
}
```

### DocumentNavState (Reusable Navigation Logic)

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

pub enum DocumentNavMsg {
    FocusNext,        // Tab
    FocusPrev,        // Shift+Tab
    FocusLeft,        // Left arrow
    FocusRight,       // Right arrow
    ScrollUp(u16),
    ScrollDown(u16),
    PageUp,
    PageDown,
    ScrollToTop,
    ScrollToBottom,
    UpdateViewportHeight(u16),
}
```

### How Components Use DocumentNavState

Components **embed** DocumentNavState and delegate navigation:

```rust
pub struct StandingsTabState {
    pub view: GroupBy,
    pub doc_nav: DocumentNavState,  // ← Embedded
}

pub enum StandingsTabMsg {
    DocNav(DocumentNavMsg),  // ← Wrapped message
    CycleViewLeft,
}

impl Component for StandingsTab {
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            StandingsTabMsg::DocNav(nav_msg) => {
                document_nav::handle_message(&mut state.doc_nav, &nav_msg)
            }
        }
    }
}
```

### FocusContext

```rust
pub struct FocusContext {
    pub focused_id: Option<FocusableId>,
}

impl FocusContext {
    pub fn is_link_focused(&self, id: &str) -> bool;
    pub fn focused_table_row(&self, table_name: &str) -> Option<usize>;
}
```

When the document builds, it checks FocusContext to know which element should be highlighted.

### Rendering Pipeline

1. `Document.build(FocusContext)` → `Vec<DocumentElement>`
2. `DocumentView` renders entire document to off-screen buffer at full height
3. Only the visible portion (based on `scroll_offset` + `viewport_height`) is copied to screen
4. `FocusContext` tells the document which element should render as "focused"

### Autoscroll

The document system automatically scrolls to keep the focused element visible:

```rust
pub fn autoscroll_to_focus(state: &mut DocumentNavState) {
    let focused_y = state.focusable_positions[focus_idx];
    let focused_height = state.focusable_heights[focus_idx];
    let viewport_height = state.viewport_height;
    let scroll = state.scroll_offset;

    let element_bottom = focused_y + focused_height;
    let viewport_bottom = scroll + viewport_height;

    if element_bottom > viewport_bottom {
        let new_offset = element_bottom + PADDING - viewport_height;
        state.scroll_offset = new_offset;
    }
}
```

---

## 6. Data Flow Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                    AppState (global)                         │
│  • standings, schedule, game_info  (Arc-wrapped API data)   │
│  • current_tab, document_stack     (navigation)             │
│  • config, status_message          (system)                 │
└─────────────────────────┬────────────────────────────────────┘
                          │ Props extracted
                          ▼
        ┌─────────────────────────────────────┐
        │   component.view(&props, &state)    │
        │   Returns Element tree              │
        └─────────────────────────────────────┘
                          │
         ┌────────────────┴────────────────┐
         ▼                                 ▼
   ┌──────────────┐                 ┌──────────────┐
   │  Component   │                 │  Document    │
   │  State       │                 │  (embedded)  │
   │              │                 │              │
   │ • date_idx   │                 │ • doc_nav    │
   │ • game_date  │                 │   scroll_off │
   │ • doc_nav ───┼─────────────────┤   focus_idx  │
   └──────────────┘                 └──────────────┘
```

---

## 7. Key Files Reference

| Concept | File | Purpose |
|---------|------|---------|
| Component trait | `src/tui/component.rs` | Defines Props/State/Message pattern |
| Element enum | `src/tui/component.rs` | Virtual tree node types |
| AppState | `src/tui/state.rs` | Global state (single source of truth) |
| Runtime | `src/tui/runtime.rs` | Manages state, components, reducers |
| Reducer | `src/tui/reducer.rs` | Pure function: State + Action → State |
| Document trait | `src/tui/document/mod.rs` | Interface for scrollable content |
| DocumentView | `src/tui/document/mod.rs` | Manages viewport, focus, rendering |
| DocumentNavState | `src/tui/document_nav.rs` | Reusable navigation state |
| DocumentElement | `src/tui/document/elements.rs` | Types of elements in documents |
| StandaloneWidget | `src/tui/widgets/mod.rs` | Dumb rendering primitives |
| ElementWidget | `src/tui/component.rs` | Widgets in element tree |
| ComponentStateStore | `src/tui/component_store.rs` | Lifecycle management (TypeId-safe) |

---

## 8. Summary Table

| Concept | Purpose | Statefulness |
|---------|---------|--------------|
| **Component** | Smart container with lifecycle | Has Props + State + Messages |
| **Widget** | Dumb rendering primitive | Stateless, just `render()` |
| **Props** | Read-only data from parent | Immutable |
| **State** | Component's own mutable data | Mutable via `update()` |
| **Document** | Scrollable content definition | Builds element list |
| **DocumentNavState** | Reusable scroll/focus logic | Embedded in component state |
| **AppState** | Global source of truth | API data, navigation, config |

---

## 9. Key Insight

**Documents are content definitions** that components embed. The component owns the navigation state (`DocumentNavState`), and the document just knows how to build its content given a focus context.

This architecture achieves:
- **Unidirectional data flow** (Key Event → Action → Reducer → State → Render)
- **Clear separation of concerns** (Components vs Widgets vs Documents)
- **Reusable document navigation** across tabs
- **Type-safe component state** through generics and TypeId
- **Efficient Arc-wrapped API data** (no deep clones on every reducer call)
- **Testable pure functions** (reducers, component.view())
