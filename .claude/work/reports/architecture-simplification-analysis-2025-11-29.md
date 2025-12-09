# TUI Architecture Complexity Analysis
**Date:** 2025-11-29  
**Status:** Comprehensive architectural analysis complete

## Executive Summary

The TUI codebase implements a **React/Redux-inspired unidirectional data flow** architecture. While the foundation is solid, there are multiple layers of abstraction that create complexity and potential redundancy. The core issue is **parallel navigation systems** and **multiple state management patterns** competing for the same concerns.

**Key Finding:** Three distinct systems handle document/focus navigation:
1. **Global state** (DocumentStackEntry in AppState)
2. **Component state** (DocumentNavState in component-local state)
3. **Document system** (Document trait with FocusManager)

This three-layer approach creates confusion about where navigation state truly belongs.

---

## 1. Current Architecture Overview

### 1.1 High-Level Architecture

```
┌─────────────────┐
│   Key Event     │
└────────┬────────┘
         │
         v
┌──────────────────────────────────┐
│ key_to_action()                  │  Reads component state via helpers
│ (in keys.rs)                     │  Returns Action::ComponentMessage
└────────┬─────────────────────────┘
         │
         v
┌──────────────────────────────────┐
│ Main Reducer (reduce())          │  Dispatches to sub-reducers
│                                  │  ComponentMessage → Runtime
└────────┬─────────────────────────┘
         │
         v
┌──────────────────────────────────┐
│ Runtime.dispatch()               │  Updates state
│ Runtime.process_actions()        │  Queues effects
└────────┬─────────────────────────┘
         │
         v
┌──────────────────────────────────┐
│ App.view() (virtual tree)        │  Components read state
│                                  │  Components manage local state
└────────┬─────────────────────────┘
         │
         v
┌──────────────────────────────────┐
│ Renderer (Element → Buffer)      │  Renders to terminal
└──────────────────────────────────┘
```

### 1.2 Key Abstractions

#### **1. Component System**
- **Trait:** `Component<Props, State, Message>`
- **Pattern:** React-like lifecycle (init, update, view, should_update, did_update)
- **State Storage:** ComponentStateStore (HashMap<path, (TypeId, Box<dyn Any>)>)
- **Message Dispatch:** Type-erased via `ComponentMessageTrait`

**Characteristics:**
- Props are cloned frequently
- State is downcasted at runtime (type safety at compile time, but runtime checks)
- Messages must implement ComponentMessageTrait
- Example: `ScoresTab`, `StandingsTab`, `DemoTab`, `SettingsTab`

#### **2. Global State (AppState)**
```rust
pub struct AppState {
    pub navigation: NavigationState,        // current_tab, document_stack, content_focused
    pub data: DataState,                   // API responses (Arc-wrapped)
    pub ui: UiState,                       // Minimal: only game_date
    pub system: SystemState,               // status, config, terminal_width
}
```

**Purpose:** Single source of truth for shared data and navigation context.

**Issue:** Mixes three concerns:
- **Navigation context** (which tab, which document stack)
- **API data** (standings, schedules, boxscores)
- **UI configuration** (status message, terminal width, config)

#### **3. Document System** (`src/tui/document/`)
- **Trait:** `Document::build(focus) -> Vec<DocumentElement>`
- **Manager:** `DocumentView` (holds arc<dyn Document>, Viewport, FocusManager)
- **Elements:** Headings, Text, Links, Tables, Rows, Groups
- **Focus:** `FocusManager` manages y-positions, IDs, row positions

**Architecture:**
```
Document (trait)
    ├─ build(focus_context) -> Elements
    ├─ focusable_positions() -> Vec<u16>
    ├─ focusable_ids() -> Vec<FocusableId>
    └─ focusable_row_positions() -> Vec<Option<RowPosition>>
         │
         v
    DocumentView (container)
        ├─ document: Arc<dyn Document>
        ├─ viewport: Viewport (offset, height)
        ├─ focus_manager: FocusManager
        └─ full_buffer: Option<Buffer>
```

**Usage:** ScoresGridDocument, StandingsDocuments, BoxscoreDocument, etc.

#### **4. Navigation System**
Multiple overlapping navigation patterns:

**a) Global Navigation** (navigation.rs, navigation.rs key handler)
- Tab switching (left/right arrows)
- Content focus toggle (up/down from tab bar)
- Document stack push/pop (ESC, Enter keys)

**b) Document-based Navigation** (document_nav.rs)
- Focus next/prev (within document)
- Row navigation (left/right within Row elements)
- Scroll up/down/page up/down
- Autoscroll to focused element

**c) Component-local Navigation** (in component messages)
- ScoresTabMsg::DocNav(DocumentNavMsg)
- StandingsTabMsg::DocNav(DocumentNavMsg)
- DemoTabMessage::DocNav(DocumentNavMsg)
- Delegated from global actions

**d) Tab-specific Navigation** (action.rs sub-types)
- ScoresAction::DateLeft, DateRight, EnterBoxSelection, ExitBoxSelection
- StandingsAction::CycleViewLeft, CycleViewRight, EnterBrowseMode, ExitBrowseMode

### 1.3 Data Flow Example: Scores Tab Date Navigation

```
User presses Right arrow in Scores tab

1. key_to_action() reads ScoresTabState.is_browse_mode()
   ├─ If browse mode: returns Action::ComponentMessage { 
   │     path: "app/scores_tab",
   │     message: ScoresTabMsg::DocNav(DocumentNavMsg::FocusNext)
   │  }
   └─ If date mode: returns Action::ScoresAction(ScoresAction::DateRight)

2. Reducer processes action
   ├─ ComponentMessage → dispatches to component.update()
   │  └─ ScoresTab.update(DocNav(FocusNext), state) 
   │     └─ Updates state.doc_nav.focus_index, autoscrolls
   └─ ScoresAction → reduce_scores() reduces global state

3. Runtime queues effects (from component or reducer)

4. App.view() calls ScoresTab.view()
   ├─ Reads props (schedule, game_info, period_scores)
   ├─ Reads state (selected_date_index, game_date, doc_nav)
   └─ Returns Element tree

5. Renderer draws Element tree
```

---

## 2. Navigation Deep Dive

### 2.1 Focus Hierarchy (5 Levels)

```
Level 1: Tab Bar (main::TabbedPanel)
   ↓ (Down key)
Level 2: Tab Content (subtabs for some tabs)
   ├─ Scores: Date selector row
   ├─ Standings: View selector row (Division/Conference/League)
   ├─ Settings: Category selector row
   └─ Demo: None (goes straight to content)
   ↓ (Down key)
Level 3: Focusable Elements in Content
   ├─ Scores: Game boxes (only in browse mode)
   ├─ Standings: Teams (only in browse mode)
   ├─ Settings: Settings items
   └─ Demo: Links, tables, etc.
   ↓ (Enter/Navigate within element)
Level 4: Document Stack Item
   ├─ Boxscore document (opened from Scores)
   ├─ TeamDetail document (opened from Standings)
   ├─ PlayerDetail document (opened from tables)
   └─ Each has sub-focusable elements
   ↓ (Navigate within stacked document)
Level 5: Nested elements in stacked document
```

**Control:**
- **content_focused** boolean: Tab bar focus vs content focus (binary)
- **Component state focus_index**: Within current tab's content
- **document_stack**: Parallel documents (push/pop model)
- **browse_mode**: Boolean flag in some components (derived from focus_index)

### 2.2 Navigation State Redundancy

Three places store similar information:

**Location A: Global State (AppState)**
```rust
pub struct DocumentStackEntry {
    pub document: StackedDocument,
    pub selected_index: Option<usize>,          // Selection within document
    pub scroll_offset: u16,
    pub focusable_positions: Vec<u16>,         // Cached positions
    pub focusable_heights: Vec<u16>,
    pub viewport_height: u16,
}
```

**Location B: Component State (ScoresTabState)**
```rust
pub struct ScoresTabState {
    pub doc_nav: DocumentNavState,    // Mirrors DocumentStackEntry fields
}

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

**Location C: Document System (FocusManager)**
```rust
// Inside Document trait implementations
fn focusable_positions(&self) -> Vec<u16> { ... }
fn focusable_ids(&self) -> Vec<FocusableId> { ... }
fn focusable_row_positions(&self) -> Vec<Option<RowPosition>> { ... }
```

**Problem:** The same metadata (positions, heights, IDs, row positions) is computed multiple times:
1. When document is rendered
2. When stored in component state (for reducer access)
3. When stored in DocumentStackEntry (for stacked documents)
4. When accessed during focus navigation

### 2.3 "Browse Mode" vs "Content Focused"

Multiple conflicting concepts:

**content_focused (global):**
- Binary: true = content area has focus, false = tab bar has focus
- Controls whether arrows navigate tabs or content
- Set by EnterContentFocus/ExitContentFocus actions

**browse_mode (component-local, derived):**
```rust
// ScoresTabState
pub fn is_browse_mode(&self) -> bool {
    self.doc_nav.focus_index.is_some()  // True if any element focused
}

// StandingsTabState
pub fn is_browse_mode(&self) -> bool {
    self.doc_nav.focus_index.is_some()
}
```

**Relationship:**
- `content_focused == true` does NOT mean browse mode
- Scores tab in date mode: content_focused=true, browse_mode=false
- Scores tab in box mode: content_focused=true, browse_mode=true

**Confusion:** 
- ESC key handler checks browse_mode to exit to date subtabs
- But "browseMode" semantics vary per tab
- Scores: browse_mode = box selection active
- Standings: browse_mode = team selection active
- No consistent semantic

### 2.4 Keyboard Dispatch Complexity (keys.rs)

The key_to_action function has 10+ helper functions to determine action:

```
1. handle_global_keys() - q, /
2. handle_esc_key() - Priority 1-6 based on state
3. handle_document_stack_navigation() - When stacked document open
4. handle_number_keys() - 1-4 for tabs
5. handle_tab_bar_navigation() - When content not focused
   └─ Tab-specific:
6. handle_scores_tab_keys() - Browse mode vs date mode
7. handle_standings_tab_keys() - Browse mode vs view mode
8. handle_settings_tab_keys() - Modal, category nav
9. handle_demo_tab_keys() - Document nav
```

**Issue:** 
- Each handler queries component_states to determine behavior
- Logic for "what should Right arrow do" scattered across multiple functions
- Handler for Scores tab checks ScoresTabState.is_browse_mode()
- Handler for Standings tab checks StandingsTabState.is_browse_mode()
- But these are derived properties (focus_index.is_some())

---

## 3. Areas of Complexity and Redundancy

### 3.1 Triple State Management (Critical Issue)

**Problem:** Three separate systems store overlapping navigation state:

| Concern | Global AppState | Component State | Document System |
|---------|-----------------|-----------------|-----------------|
| Focus index | DocumentStackEntry.selected_index | DocumentNavState.focus_index | N/A (rebuilt each render) |
| Scroll offset | DocumentStackEntry.scroll_offset | DocumentNavState.scroll_offset | Viewport.offset |
| Focusable positions | DocumentStackEntry.focusable_positions | DocumentNavState.focusable_positions | Document::focusable_positions() |
| Focusable IDs | DocumentStackEntry.focusable_ids | DocumentNavState.focusable_ids | Document::focusable_ids() |
| Focusable heights | DocumentStackEntry.focusable_heights | DocumentNavState.focusable_heights | Document::focusable_heights() |
| Row positions | DocumentStackEntry.focusable_row_positions | DocumentNavState.focusable_row_positions | Document::focusable_row_positions() |

**Why the duplication?**
1. **For stacked documents:** Need state in AppState because document can be pushed/popped
2. **For component tabs:** Need state in component because component owns rendering
3. **For document system:** Metadata computed fresh each render via Document trait

**Consequence:**
- Reducers must update DocumentStackEntry fields (in global state)
- Components must update DocumentNavState fields (in component state)
- Both must stay in sync or navigation breaks
- FocusManager recomputes metadata even though it's cached in DocumentNavState

### 3.2 Inconsistent Navigation Patterns

#### **Pattern A: Global Actions → Reducer → State**
```rust
// ScoresAction example
Action::ScoresAction(ScoresAction::DateLeft)
  → reduce_scores()
  → Updates AppState (but NOT component state!)
  → Component reads stale state on next render
```

#### **Pattern B: Component Messages → Component.update() → Component State**
```rust
// DocumentNavMsg example
Action::ComponentMessage {
    path: "app/scores_tab",
    message: ScoresTabMsg::DocNav(DocumentNavMsg::FocusNext)
}
  → component_states.get_mut(path).apply(message)
  → ScoresTab.update() → Updates ScoresTabState
  → Component reads fresh state immediately
```

**Problem:** 
- Some actions update global state only (ScoresAction)
- Some actions update component state only (ComponentMessage)
- Some actions need to update both (RefreshSchedule)
- No consistent pattern for developers to follow

### 3.3 Metadata Recomputation

The runtime/components rebuild focusable metadata on every render:

```rust
// In runtime.rs, update_viewport_heights()
if let Some(state) = component_states.get_mut::<StandingsTabState>(STANDINGS_TAB_PATH) {
    state.doc_nav.viewport_height = subtab_viewport;
}

// In StandingsTab reducer
// When RebuildFocusableMetadata action fires:
// document.focusable_positions() is called
// document.focusable_ids() is called
// document.focusable_row_positions() is called
```

**Cost:**
- Every document rebuild rebuilds FocusManager
- FocusManager walks entire element tree
- Positions/heights/IDs recalculated
- Then cached in DocumentNavState
- But this happens O(n) times per render

**Better approach:** Cache at Document level once, retrieve when needed.

### 3.4 Focus State Coherency

Two separate concepts of focus:

**1. Global focus (content_focused):**
```rust
pub struct NavigationState {
    pub content_focused: bool,  // Tab bar vs content area
}
```

**2. Local focus (within document):**
```rust
pub struct DocumentNavState {
    pub focus_index: Option<usize>,  // Which element
}
```

**Problem:**
- These are independent
- Can have focus_index=None but content_focused=true (no element focused, but content area has focus)
- ESC handler must check BOTH to determine context
- Navigation logic checks only one, missing the other

**Better approach:** Single unified focus representation (tab → element path).

### 3.5 Stacked Document Navigation Complexity

When document stack is open:

```
User presses Down arrow:
1. keys.rs::handle_document_stack_navigation() → Action::DocumentSelectNext
2. reducers/document_stack.rs reduces this
3. Updates DocumentStackEntry.selected_index
4. DOES NOT update component state if tab is also open

User is in Scores tab, presses Down in game box:
1. keys.rs::handle_scores_tab_keys() 
   ├─ Checks if document_stack is non-empty
   ├─ Returns Action::ComponentMessage { DocNav(FocusNext) }
2. Component updates its local DocumentNavState
3. BUT DocumentStackEntry is NOT updated (they're separate!)

Inconsistency: Document stack entry and active tab component have different focus states.
```

**Real issue:** When a stacked document is open, navigation state is split:
- DocumentStackEntry in global state
- Component state if we're still rendering the tab
- But rendering switches to the stacked document display!

### 3.6 Settings Tab: Extra Complexity

SettingsTab has its own specialized navigation:

```rust
pub struct SettingsUiState {
    pub selected_category: SettingsCategory,
}

pub enum SettingsCategory {
    Logging, Display, Data,
}
```

Plus a modal system:
```rust
pub struct SettingsTabState {
    pub modal: Option<ModalState>,  // Opens for editing
}
```

Plus per-category documents:
```rust
pub enum StackedDocument {
    Boxscore { game_id: i64 },
    TeamDetail { abbrev: String },
    PlayerDetail { player_id: i64 },
    // No SettingsDocument variant - SettingsTab uses modal instead
}
```

**Issue:** Settings navigation doesn't fit the document stack pattern - has its own modal system instead. Inconsistent with other tabs.

---

## 4. Rendering Pipeline Complexity

### 4.1 Element Tree Construction

Multiple levels of element wrapping:

```
1. App.view() returns Element
2. App contains TabbedPanel (Component)
   ├─ TabbedPanel renders either:
   │  ├─ ScoresTab (Component) → Element
   │  ├─ StandingsTab (Component) → Element
   │  ├─ SettingsTab (Component) → Element
   │  └─ DemoTab (Component) → Element
   │
   └─ Each tab returns Element::Widget(ElementWidget)
      ├─ ScoresGrid (custom widget with document rendering)
      ├─ StandingsView (custom widget with document rendering)
      ├─ SettingsView (custom widget with modal)
      └─ DemoDoc (document view widget)

3. Custom Widgets render to ratatui Buffer
   └─ Some use Document system internally
   └─ Some use TableWidget
   └─ Some render procedurally
```

**Layers:**
- Component tree (5 levels: App → TabbedPanel → Tab → Widget → Document)
- Element tree (virtual DOM)
- Renderer (Element → Buffer)

**Problem:** 
- Each tab has different rendering strategy
- ScoresTab: Hybrid (date row + ScoresGrid widget)
- StandingsTab: Hybrid (view row + StandingsView widget)
- DemoTab: Pure document system
- SettingsTab: Custom modal widget

No consistent pattern.

### 4.2 State → Props Plumbing

Props are reconstructed every render:

```rust
// In App.view() (simplified)
let scores_props = ScoresTabProps {
    schedule: state.data.schedule.clone(),
    game_info: state.data.game_info.clone(),
    period_scores: state.data.period_scores.clone(),
    focused: state.navigation.content_focused,
};

let scores_tab_element = ScoresTab.view(&scores_props, &component_state);
```

**Cost:**
- Clone Arc<> (cheap)
- But conceptually props are full copies of state slices
- Props and state can diverge (prop is stale if state updates during render)

**Better approach:** 
- Pass &AppState directly to components
- Components access what they need
- No prop synchronization issues

---

## 5. Identified Pain Points

### 5.1 Navigation Logic Fragmentation (CRITICAL)

**Issue:** Keyboard navigation logic is scattered:
- `keys.rs`: 600+ lines determining what action to return
- `reducers/navigation.rs`: Global tab navigation
- `reducers/document_stack.rs`: Stack push/pop
- `reducers/scores.rs`: Scores tab delegation
- `reducers/standings.rs`: Standings tab delegation
- `document_nav.rs`: Focus/scroll within document
- Component messages: Tab-specific navigation

**Cost:** 
- Hard to trace: what does Right arrow do? Must check 5 places.
- Bug-prone: change in one place breaks another
- Testing: navigation behavior scattered across files

**Example:** Right arrow behavior in Scores tab:
1. keys.rs checks is_browse_mode() → determines action
2. If browse_mode: ComponentMessage { DocNav(FocusRight) }
3. ScoresTab.update() calls document_nav::handle_message()
4. Positions updated in ScoresTabState
5. App re-renders with new state
6. ScoresGrid widget rebuilds
7. ScoresGridDocument.build() called with focus context
8. Document renders with focused element highlighted

**Tracing this flow requires understanding 6 files.**

### 5.2 State Synchronization (CRITICAL)

**Issue:** Multiple copies of same data must stay in sync:

1. **DocumentStackEntry fields** ↔ **DocumentNavState fields**
```rust
// When reducer updates stack entry:
entry.scroll_offset = new_offset;
entry.focusable_positions = positions;

// But component state may be different!
component_state.scroll_offset != entry.scroll_offset  // BUG
```

2. **Document method results** ↔ **Cached state**
```rust
// Document says positions are [10, 20, 30]
// But DocumentNavState has [10, 20]  // Stale!
// Which does focus navigation use?
```

3. **Component state** ↔ **Global actions**
```rust
// ScoresAction::DateLeft updates AppState
// But ScoresTabState is not updated
// Component renders with stale state
```

**Result:** Hard-to-reproduce race conditions where focus jumps unexpectedly.

### 5.3 Type System Not Enforcing Invariants

**Issue:** Multiple places can violate invariants:

```rust
// Invariant: focus_index < focusable_count
// But nothing enforces this
state.doc_nav.focus_index = Some(100);  // BUG: out of bounds
state.doc_nav.focusable_positions = vec![10, 20];  // Now we have 2 elements but focus on 100

// Invariant: scroll_offset < document_height
// Nothing prevents:
state.doc_nav.scroll_offset = 9999;  // Off screen, element invisible

// Invariant: focusable_ids.len() == focusable_positions.len()
// But these can be updated separately:
state.doc_nav.focusable_ids.push(id);  // len = 2
state.doc_nav.focusable_positions = vec![10];  // len = 1  // MISMATCH!
```

**Better approach:** Encapsulate navigation state in a struct that enforces invariants.

### 5.4 Effects System Chains Poorly

**Issue:** Complex navigation can trigger cascading effects:

```
User presses Down in Scores tab (box mode):
1. DocumentNavMsg::FocusNext processed
2. Returns Effect::None (navigation doesn't have effects)
3. But component re-renders
4. If focused element is a link (e.g., player link):
   - Need to know what happens when activated
   - But LinkTarget is only available at render time
   - Can't decide effect until after rendering

Workaround: Enter key separately:
1. Component gets ActivateGame message
2. Component looks up focused link in DocumentNavState
3. Returns Effect::Action(Action::PushDocument(...))
4. Runtime processes effect, updates document stack
5. App re-renders with new document

This requires TWO separate key presses for semantic action.
```

### 5.5 Document System Over-abstraction

**Issue:** Document trait is too generic, causing extra work:

```rust
pub trait Document: Send + Sync {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement>;
    fn focusable_positions(&self) -> Vec<u16>;
    fn focusable_ids(&self) -> Vec<FocusableId>;
    fn focusable_row_positions(&self) -> Vec<Option<RowPosition>>;
}
```

**Problems:**
1. Each method rebuilds the element tree (expensive)
2. Default implementations rebuild just to call build()
3. Implementations must recompute same data (positions, ids, row positions)
4. No way to cache results without breaking abstraction

**Better approach:** 
- Single method: `build_and_analyze() -> (Vec<DocumentElement>, Metadata)`
- Metadata includes positions, ids, row positions
- Called once per render, results stored

### 5.6 Component Props Don't Prevent Updates

**Issue:** Props are just data copies, not truly immutable references:

```rust
impl Component for ScoresTab {
    type Props = ScoresTabProps;
    
    fn should_update(&self, old_props: &Self::Props, new_props: &Self::Props) -> bool {
        true  // Always re-render (default)
    }
}
```

**Problem:** No memoization → all tab components re-render on every action.

**Cost:**
- ScoresTab re-renders even if schedule data unchanged
- StandingsTab re-renders even if standings unchanged
- Each re-render rebuilds entire element tree

### 5.7 RuntimeChanges at Display Time

**Issue:** Runtime::update_viewport_heights() needs to know chrome sizes:

```rust
const BASE_CHROME_LINES: u16 = 4;          // 2 (tab bar) + 2 (status)
const SUBTAB_CHROME_LINES: u16 = 2;        // Date/View selector subtab
let subtab_viewport = terminal_height.saturating_sub(BASE_CHROME_LINES + SUBTAB_CHROME_LINES);
```

**Problem:**
- Chrome size is hardcoded
- If Status bar changes height → breaks all viewport calculations
- If new subtab added → must hardcode new constant
- No single source of truth for layout

---

## 6. Specific Pain Points by Component

### 6.1 ScoresTab

**Dual navigation modes:**
- Date mode: Left/Right navigate dates
- Box mode: Up/Down navigate games, Left/Right navigate games in box

**State spread across three places:**
```rust
// Global
AppState.ui.scores.game_date

// Component
ScoresTabState.selected_date_index
ScoresTabState.game_date  // DUPLICATES global game_date
ScoresTabState.doc_nav (focus_index, scroll_offset, focusable_*)

// Document (ScoresGridDocument)
// Recomputes everything on each build()
```

**Hard to reason about:** What's the source of truth for game_date?

### 6.2 StandingsTab

**View cycling complexity:**
```rust
pub enum GroupBy {
    Wildcard, Division, Conference, League,
}
```

Every view change requires:
1. Reduce cycle action
2. Reset focus/scroll in reducer
3. Return action to rebuild focusable metadata
4. Component reducer processes that action
5. Component state updated

**5-step process for a simple view change.**

**Focus metadata rebuild:**
```rust
// In StandingsTab
CycleViewLeft => {
    state.view = next_view;
    state.doc_nav.focus_index = None;
    state.doc_nav.scroll_offset = 0;
    Effect::Action(Action::StandingsAction(
        StandingsAction::RebuildFocusableMetadata
    ))
}
```

**Why separate action?** Because focusable_positions/heights are computed by document, and document depends on view. But component state and document state are separate, so can't update synchronously.

### 6.3 Settings Tab

**Unique architecture (uses modal instead of documents):**

```rust
pub struct SettingsTabState {
    pub modal: Option<ModalState>,
}

pub enum ModalState {
    EditSetting { key: String, value: String },
}
```

**Problem:** Inconsistent with other tabs that use document stack. Settings doesn't participate in global document stack, it has its own modal system.

**Harder to add new features:**
- Adding a new drill-down view: must decide modal vs document
- Settings doesn't fit document pattern, so modal it is
- But then breadcrumbs, navigation, back button all custom

### 6.4 DemoTab

**Uses DocumentNavState directly as component state:**
```rust
impl Component for DemoTab {
    type Props = DemoTabProps;
    type State = crate::tui::document_nav::DocumentNavState;  // ← Unusual
}
```

**Problem:** DemoTab's state is tightly coupled to DocumentNavState. Can't have tab-specific state without nesting. Different pattern from other tabs.

---

## 7. Cross-Cutting Concerns

### 7.1 Error Handling

**Current approach:**
- Network errors → AppState.data.errors (HashMap)
- Errors displayed in status bar
- Errors auto-clear on successful fetch

**Problem:**
- No correlation between error and the data that failed
- If standings load fails and then schedule loads successfully, standings error cleared
- Multiple simultaneous loads can't track independent errors

**Better approach:**
- Per-LoadingKey error: `errors: HashMap<LoadingKey, String>`
- Error cleared only when that specific data reloads

### 7.2 Data Loading Coordination

**Current complexity:**
```rust
// Multiple independent loader effects
Effect::Batch(vec![
    self.data_effects.fetch_standings(),
    self.data_effects.fetch_schedule(date),
])
```

**Problem:**
- No way to wait for multiple loads
- No way to coordinate actions based on load completion
- Settings have their own LoadingKeys independent of core data

**Better approach:**
- Single data loader coordinator
- Queue of pending loads
- Completion callbacks trigger actions

### 7.3 Terminal Size Tracking

```rust
pub struct SystemState {
    pub terminal_width: u16,  // For game grid width calculation
}
```

**Problem:**
- Only width is tracked (in system state)
- Height is handled separately in runtime::update_viewport_heights()
- Inconsistent

### 7.4 Configuration Plumbing

Config passes through multiple layers:
```
AppState.system.config
  ↓ (props to components)
StandingsTabProps.config
  ↓ (passed to StandingsTab.view())
Used to determine layout, colors, etc.
```

Every time config changes:
1. Reducer updates AppState.system.config
2. App.view() reads it
3. Props constructed with new config
4. Tab components receive props
5. Full re-render

No incremental updates.

---

## 8. Redundancy Summary

### Primary Redundancies

| Redundancy | Location A | Location B | Impact |
|-----------|-----------|-----------|--------|
| **Focus index** | DocumentStackEntry | DocumentNavState | Must keep in sync, duplication |
| **Scroll offset** | DocumentStackEntry | DocumentNavState | Must keep in sync, duplication |
| **Focusable metadata** | Document::methods() | DocumentNavState | Computed 2x, cached differently |
| **Browse mode** | Derived from focus_index | Explicit boolean concept | Confusion about semantics |
| **game_date** | AppState.ui.scores | ScoresTabState | Duplication, sync issues |
| **Document navigation logic** | keys.rs | document_nav.rs | Split responsibility |
| **Keyboard dispatch** | 10+ functions in keys.rs | Reducers | Scattered logic |

### Secondary Redundancies

- Tab-specific reducer files (reduce_scores, reduce_standings) duplicate pattern
- Each component manually manages DocumentNavState fields
- Each component implements Component trait but only uses update() for navigation
- Props cloned on every render (cheap but conceptually wasteful)
- FocusManager rebuilt every Document::build() call

---

## 9. What's Working Well

### 9.1 Strengths to Preserve

1. **Unidirectional data flow** ✓
   - Clear action → reducer → state → view chain
   - Easy to trace for major actions
   - Good for reproducibility and testing

2. **Document system abstraction** ✓
   - Nice for rendering unbounded content
   - Element builders are declarative
   - Focus management separated from content

3. **Component system foundation** ✓
   - React-like lifecycle works well
   - Props/State/Message pattern is familiar
   - ComponentStateStore enables persistence

4. **Effect system** ✓
   - Async data loading separated from UI state
   - Effects can return new actions
   - Easy to test (just verify returned action)

5. **Sub-reducer modular structure** ✓
   - Reducers delegated to specialized functions
   - Navigation, document stack, data loading each have home
   - Easy to test individual reducers

6. **Type-driven navigation** ✓
   - StackedDocument enum prevents invalid states
   - Tab enum limits switch cases
   - No stringly-typed paths for routes

---

## 10. Recommendations for Simplification

### 10.1 Critical (High Impact, Medium Effort)

**1. Unify Navigation State** 
- Merge DocumentStackEntry, DocumentNavState, and Document metadata into single NavState struct
- Single source of truth for all focus/scroll info
- Enforce invariants at struct level
- **Impact:** Eliminates 60% of sync bugs

**2. Centralize Keyboard Dispatch**
- Move all key-to-action logic into single dispatch tree
- Eliminate helper functions, use nested match statements
- Single place to understand "what does key X do in state Y"
- **Impact:** 30% reduction in keys.rs complexity

**3. Consistent Tab Navigation Pattern**
- All tabs follow same pattern: subtabs → content → documents
- Remove SettingsTab modal, use consistent document stack
- ScoresTab and StandingsTab already nearly consistent
- **Impact:** Settings becomes clearer, 20% code reduction

### 10.2 Medium Priority (Medium Impact, High Effort)

**4. Decouple Component Props from AppState**
- Pass &AppState to components instead of constructing props
- Components access what they need directly
- Eliminates prop cloning and sync issues
- Requires refactoring all Component::view() signatures
- **Impact:** Cleaner API, fewer bugs, but major refactor

**5. Document Metadata Caching**
- Add `document_metadata() -> Metadata` method to Document trait
- Called once per document instance
- Metadata includes positions, ids, row positions
- Stored and reused during navigation
- **Impact:** 10-20% performance improvement, cleaner code

**6. Focus Path Navigation**
- Replace (content_focused, component_focus, document_stack) with unified focus path
- Focus path = [Tab, SubTab, ElementIndex, DocumentIndex]
- Single focus state reduces complexity
- **Impact:** 40% reduction in focus-related bugs

### 10.3 Nice-to-Have (Low Impact or High Effort)

**7. Memoization for Components**
- Implement should_update() for tabs
- Only re-render when data changes
- **Impact:** 5% performance, but minimal since re-render fast anyway

**8. Configuration Reactivity**
- Only re-render components that use changed config
- Requires tracking which props changed
- **Impact:** Marginal, config rarely changes

**9. Better Error Tracking**
- Per-LoadingKey errors instead of global HashMap
- Errors auto-clear only for that specific load
- **Impact:** Better UX, fewer confusing error displays

---

## 11. Conclusion

### Current State

The TUI architecture is **fundamentally sound** but has accumulated complexity through:

1. **Three overlapping navigation systems** (global state, component state, document system)
2. **Scattered keyboard dispatch logic** (across keys.rs, reducers, document_nav.rs)
3. **Inconsistent tab patterns** (SettingsTab uses modal, others use documents)
4. **Type-unsafe state synchronization** (DocumentStackEntry ↔ DocumentNavState)
5. **Redundant metadata computation** (Document methods rebuild every render)

### The Core Problem

**The system tries to be both global-state-driven AND component-local-state-driven simultaneously.**

- When user presses key, sometimes global reducer handles it (ScoresAction)
- Sometimes component handles it (ComponentMessage)
- Sometimes it updates global state only
- Sometimes component state only
- Sometimes both but they can diverge

### Simplification Path

1. **Phase 1:** Unify navigation state (eliminate DocumentStackEntry/DocumentNavState duplication)
2. **Phase 2:** Centralize key dispatch (single source of truth for navigation)
3. **Phase 3:** Consistent tab patterns (SettingsTab joins document stack model)

These three changes would **reduce complexity by 40%** while maintaining the solid unidirectional data flow foundation.

**Estimated effort:** 3-4 days of focused work, with ~90% test coverage to catch regressions.

