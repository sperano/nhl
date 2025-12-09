# TUI Architecture

## Overview

The TUI uses a React/Redux-inspired architecture with unidirectional data flow:

```
┌─────────────┐     ┌──────────┐     ┌─────────────┐
│  Key Event  │────>│  Action  │────>│   Reducer   │
└─────────────┘     └──────────┘     └─────────────┘
                                            │
                                            v
┌─────────────┐     ┌──────────┐     ┌─────────────┐
│   Render    │<────│  Element │<────│    State    │
└─────────────┘     │   Tree   │     │  (AppState) │
                    └──────────┘     └─────────────┘
                          ^
                          │
                    ┌─────────────┐
                    │  Component  │
                    │   .view()   │
                    └─────────────┘
```

## Module Structure

```
src/tui/
├── mod.rs              # Main entry point, TUI run loop
├── component.rs        # Core Component trait, Element types
├── action.rs           # Action enum (Redux-like actions)
├── state.rs            # AppState - single source of truth
├── reducer.rs          # Main reducer + settings reducer
├── reducers/           # Sub-reducers (modular)
│   ├── navigation.rs   # Tab navigation actions
│   ├── document_stack.rs # Document stack management
│   ├── data_loading.rs # Data loaded actions
│   ├── scores.rs       # Scores tab message forwarder
│   └── standings.rs    # Standings tab message forwarder
├── document_nav.rs     # Generic document navigation (reusable)
├── document/           # Document system for scrollable content
├── runtime.rs          # Runtime - manages component lifecycle
├── renderer.rs         # Renders Element tree to ratatui Buffer
├── effects.rs          # DataEffects - async data fetching
├── keys.rs             # Keyboard event to Action mapping
├── components/         # React-like Components
└── widgets/            # Low-level renderable widgets
```

## State Ownership Model

**CRITICAL PRINCIPLE**: Components own their UI state. Global state only holds shared data.

### Component State (owned by components via Runtime)
- UI state: selected indices, scroll positions, focus state
- Navigation state: which view is active, browse mode flags
- Document state: focus index, viewport scroll offset
- Component-specific flags and modes

### Global State (in AppState)
- **Data**: API responses (standings, schedules, games, boxscores)
- **Navigation**: Current tab, document stack (shared across components)
- **System**: Configuration, status messages, last refresh time
- **Data Effects**: What data to load (e.g., `game_date` for schedule refreshes)

### AppState Structure

```rust
pub struct AppState {
    pub navigation: NavigationState,  // current_tab, document_stack
    pub data: DataState,              // API data (Arc-wrapped), loading states, errors
    pub ui: UiState,                  // Minimal: only data effect triggers
    pub system: SystemState,          // last_refresh, config, status_message
}
```

## Action/Reducer Pattern

### Actions (`action.rs`)

```rust
pub enum Action {
    // Navigation
    NavigateTab(Tab), NavigateTabLeft, NavigateTabRight,
    EnterContentFocus, ExitContentFocus,
    PushDocument(StackedDocument), PopDocument,

    // Data
    RefreshData, SetGameDate(GameDate),
    StandingsLoaded(Result<...>), ScheduleLoaded(Result<...>), ...

    // Tab-specific (nested)
    ScoresAction(ScoresAction),
    StandingsAction(StandingsAction),
    SettingsAction(SettingsAction),

    // System
    Quit, Error(String), SetStatusMessage { ... },
}
```

### Reducers
- Main `reduce()` function delegates to sub-reducers
- Each reducer returns `Option<(AppState, Effect)>` - None means didn't handle
- Sub-reducers: `reduce_navigation`, `reduce_document_stack`, `reduce_data_loading`
- Message forwarders: `reduce_scores`, `reduce_standings`

## Message Dispatch Flow

```
┌──────────────┐
│  Key Event   │
└──────┬───────┘
       │
       v
┌──────────────────────────────────────────────────────┐
│  key_to_action(key, state)                           │
│  - Reads component state via helper functions        │
│  - Returns Action::ComponentMessage(...)             │
└──────┬───────────────────────────────────────────────┘
       │
       v
┌──────────────────────────────────────────────────────┐
│  Main Reducer                                        │
│  - Delegates to sub-reducers                         │
│  - ComponentMessage → forward to Runtime             │
└──────┬───────────────────────────────────────────────┘
       │
       v
┌──────────────────────────────────────────────────────┐
│  Runtime.dispatch_component_message()                │
│  - Looks up component by ID                          │
│  - Calls component.update(message, &mut state)       │
└──────┬───────────────────────────────────────────────┘
       │
       v
┌──────────────────────────────────────────────────────┐
│  Component.update()                                  │
│  - Modifies component state directly                 │
│  - Returns Effect (None, Action, or Async)           │
└──────────────────────────────────────────────────────┘
```

## Effects System

```rust
pub enum Effect {
    None,
    Action(Action),           // Dispatch immediately
    Batch(Vec<Effect>),       // Process multiple effects
    Async(Pin<Box<dyn Future<Output = Action> + Send>>),  // Async operation
}
```

`DataEffects` provides async data fetching:
- `fetch_standings()` -> Effect
- `fetch_schedule(date)` -> Effect
- `fetch_game_details(game_id)` -> Effect
- `fetch_boxscore(game_id)` -> Effect
- `fetch_team_roster_stats(abbrev)` -> Effect
- `fetch_player_stats(player_id)` -> Effect

## Runtime

The `Runtime` orchestrates the system:
- Holds the current `AppState`
- Dispatches actions through the reducer
- Executes side effects asynchronously via `DataEffects`
- Builds the virtual Element tree via `App.view()`
- Uses mpsc channels for action queue and effect queue

Key methods:
- `dispatch(action)` - Process action through reducer, queue effects
- `process_actions()` - Drain action queue, returns count processed
- `build()` -> Element - Build virtual tree from current state
- `action_sender()` - Get channel for external action dispatch

## Widget Architecture

### Components (`src/tui/components/`)

```rust
pub trait Component: Send {
    type Props: Clone;
    type State: Default + Clone;
    type Message;

    fn init(_props: &Self::Props) -> Self::State { ... }
    fn update(&mut self, _msg: Self::Message, _state: &mut Self::State) -> Effect { ... }
    fn view(&self, props: &Self::Props, state: &Self::State) -> Element;
}
```

### ElementWidget (`src/tui/component.rs`)

```rust
pub trait ElementWidget: Send + Sync {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig);
    fn clone_box(&self) -> Box<dyn ElementWidget>;
    fn preferred_height(&self) -> Option<u16> { None }
    fn preferred_width(&self) -> Option<u16> { None }
}
```

### StandaloneWidget (`src/tui/widgets/`)

```rust
pub trait StandaloneWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig);
    fn preferred_height(&self) -> Option<u16> { None }
    fn preferred_width(&self) -> Option<u16> { None }
}
```

## Error Handling

Network and deserialization errors are **never** output to stderr/stdout:
- All errors stored in `AppState.data.error_message`
- Displayed on status bar with red background and white text
- Format: `"ERROR: <message>"`
- Errors automatically cleared on next successful data fetch

## Key Dependencies

- `nhl_api`: NHL API client (local path dependency)
- `ratatui` (0.29.0): Terminal UI framework
- `crossterm` (0.28.1): Cross-platform terminal manipulation
- `tokio` (1.x): Async runtime
- `chrono` (0.4.42): Date/time handling
