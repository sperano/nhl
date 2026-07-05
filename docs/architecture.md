# TUI Architecture

## Overview

The TUI uses a React/Redux-inspired architecture with unidirectional data flow:

```
┌─────────────┐     ┌────────────────┐     ┌───────────┐
│  Key Event  │────>│ key_to_action()│────>│  Action   │
└─────────────┘     └────────────────┘     └─────┬─────┘
                                                  │
                                                  v
┌─────────────┐     ┌──────────┐          ┌────────────┐
│   Renderer  │<────│ Element  │<─────────│  reduce()  │
│  ::render() │     │  (tree)  │          │ (+ Effect) │
└─────────────┘     └────┬─────┘          └─────┬──────┘
                          ^                       │
                          │                       v
                    ┌─────────────┐        ┌─────────────┐
                    │ Runtime::   │        │  Runtime::  │
                    │ build()     │        │execute_effect│
                    └─────────────┘        └─────────────┘
```

`Runtime::dispatch(action)` runs `reduce()` to get a new `AppState` plus an
`Effect`, then executes that effect. Effects either resolve immediately
(`Effect::FetchBoxscore` and friends, handled synchronously by
`Runtime::execute_effect`) or asynchronously (`Effect::Async`, run on a
background Tokio task and fed back in as a new `Action`). The main loop in
`src/tui/mod.rs` only calls `Runtime::build()` + `Renderer::render()` when a
`dirty` flag is set — see "Main Loop" below.

## Module Structure

```
src/tui/
├── mod.rs                # Entry point: run(), dirty-flag render loop
├── component.rs          # Component trait, Element tree, Effect enum, ElementWidget
├── component_store.rs    # ComponentStateStore - type-erased per-path component state
├── constants.rs          # Component path constants (SCORES_TAB_PATH, etc.)
├── action.rs             # Action enum, ComponentMessageTrait
├── state.rs              # AppState - single source of truth for global state
├── reducer.rs            # reduce(): ComponentMessage dispatch + sub-reducer chain
├── reducers/             # Sub-reducers (modular)
│   ├── navigation.rs        # Tab navigation, focus-in-content toggling
│   ├── document_stack.rs    # Push/pop stacked documents, StackedDocumentKey routing
│   ├── data_loading.rs      # *Loaded actions (API responses) + RefreshData
│   ├── settings.rs          # SettingsAction handling + config persistence
│   └── standings.rs         # rebuild_standings_focusable_metadata()
├── document_nav.rs       # DocumentNavState + DocumentNavMsg (generic scroll/focus)
├── document/             # Document trait, DocumentView, FocusManager, Viewport
├── tab_component.rs      # TabState/TabMessage traits, component_message_impl! macro
├── nav_handler.rs        # key_to_nav_msg(): KeyEvent -> DocumentNavMsg
├── focus_helpers.rs      # Shared focus-index helper functions
├── settings_helpers.rs   # Settings modal option helpers
├── runtime.rs            # Runtime - owns AppState + ComponentStateStore
├── renderer.rs           # Stateless Renderer - Element tree -> ratatui Buffer
├── effects.rs            # DataEffects - async NHL API fetches (cached)
├── keys.rs               # key_to_action(): KeyEvent -> Action
├── table.rs              # Generic table rendering primitives (Alignment, ColumnDef, CellValue)
├── types.rs              # Tab, SettingsCategory, StackedDocument
├── testing.rs            # setup_test_render!, assert_buffer, create_client, create_test_standings
├── components/           # React-like components (ScoresTab, StandingsTab, App, ...)
└── widgets/              # Low-level StandaloneWidget implementations (ScoreBox, BigScore, ...)
```

## State Ownership Model

**CRITICAL PRINCIPLE**: Components own their UI state. Global state only holds shared data.

### Component State (owned by components, stored in `ComponentStateStore`)
- UI state: selected indices, scroll positions, focus state (`DocumentNavState`)
- Navigation state: which view is active (e.g. `StandingsTabState.view: GroupBy`)
- Modal state (e.g. `SettingsTabState.modal: Option<ModalState>`)
- Component-specific flags and modes

### Global State (in `AppState`)
- **Data**: API responses (standings, schedules, games, boxscores) — `AppState::data`
- **Navigation**: Current tab, document stack (shared across components) — `AppState::navigation`
- **System**: Configuration, status messages, last refresh time — `AppState::system`
- **UI**: A thin slice needed by the effects system (`game_date` for schedule
  refreshes, selected settings category) — `AppState::ui`

### `AppState` Structure

```rust
pub struct AppState {
    pub navigation: NavigationState,  // current_tab, document_stack, focus_in_content
    pub data: DataState,              // API data (Arc-wrapped), loading keys, error strings
    pub ui: UiState,                  // ScoresUiState { game_date }, SettingsUiState { selected_category }
    pub system: SystemState,          // last_refresh, config, status_message, animation_frame
}
```

`ScoresUiState.game_date` is intentionally duplicated with the component-local
`ScoresTabState.game_date`: the global copy drives the effects system
(auto-refresh needs to know what date to re-fetch), while the component copy
drives rendering. `Action::RefreshSchedule` keeps them in sync.

## Action Enum

`Action` (`src/tui/action.rs`) carries every state transition in the app:

```rust
pub enum Action {
    // Navigation
    NavigateTab(Tab), NavigateTabLeft, NavigateTabRight,
    EnterContentFocus, ExitContentFocus,
    PushDocument(StackedDocument), PopDocument,
    StackedDocumentKey(KeyEvent),

    // Data
    RefreshData,
    RefreshSchedule(GameDate),

    // Data loaded (dispatched by DataEffects futures); errors carry Arc<NHLApiError>
    // so the Result can be cheaply cloned by Action::clone()
    StandingsLoaded(Result<Vec<Standing>, Arc<NHLApiError>>),
    ScheduleLoaded(Result<DailySchedule, Arc<NHLApiError>>),
    GameDetailsLoaded(i64, Result<GameMatchup, Arc<NHLApiError>>),
    BoxscoreLoaded(i64, Result<Boxscore, Arc<NHLApiError>>),
    TeamRosterStatsLoaded(String, Result<ClubStats, Arc<NHLApiError>>),
    PlayerStatsLoaded(i64, Result<PlayerLanding, Arc<NHLApiError>>),

    FocusNext, FocusPrevious,
    SettingsAction(SettingsAction),
    SelectGame(i64),
    RebuildStandingsFocusable,

    // Type-erased dispatch to a component's own Message type
    ComponentMessage { path: String, message: Box<dyn ComponentMessageTrait> },

    Quit, Error(String),
    SetStatusMessage { message: String, is_error: bool },
    UpdateTerminalWidth(u16),
    Tick,
}
```

There is **no** `ScoresAction`/`StandingsAction` nested enum — that was
replaced by `Action::ComponentMessage`. The only remaining tab-specific
top-level action is `SettingsAction` (`NavigateCategoryLeft/Right`,
`ToggleBoolean`, `UpdateSetting`, `UpdateConfig`), because settings mutate
global `SystemState::config`, not component-local state.

`ComponentMessageTrait` (defined alongside `Action`) is what makes
`ComponentMessage` type-erased:

```rust
pub trait ComponentMessageTrait: Send + Sync + std::fmt::Debug {
    fn apply(&self, state: &mut dyn Any) -> Effect;
    fn clone_box(&self) -> Box<dyn ComponentMessageTrait>;
}
```

Every tab message enum (`ScoresTabMsg`, `StandingsTabMsg`, `SettingsTabMsg`,
...) implements this via the `component_message_impl!` macro
(`src/tui/tab_component.rs`) rather than by hand — see "Tab Component
Pattern" below.

## Reducer Pipeline (`reduce()`)

`reduce(state, action, component_states) -> (AppState, Effect)` in
`src/tui/reducer.rs` is pure (no I/O). It:

1. **Intercepts `Action::ComponentMessage` first**, before any sub-reducer.
   It looks up `component_states.get_mut_any(path)` and calls
   `message.apply(component_state)`, returning immediately. If no state is
   registered at `path`, it logs and returns `Effect::None`.
2. Otherwise chains sub-reducers, each returning `Ok((state, effect))` if it
   handled the action or `Err(state)` to pass ownership to the next one:
   `reduce_navigation` → `reduce_document_stack` → `reduce_data_loading`.
3. Falls through to a final `match` in `reduce()` itself for actions that
   don't fit the sub-reducer pattern: `SettingsAction` (forwarded to
   `reduce_settings`), `SelectGame` (looks up the game in
   `state.data.schedule` and pushes a `StackedDocument::Boxscore` via
   `reduce_document_stack`), `RefreshSchedule` (swaps `ui.scores.game_date`
   and clears stale schedule/game-info/period-score data),
   `RebuildStandingsFocusable`, `SetStatusMessage`, `UpdateTerminalWidth`,
   `Tick` (advances `animation_frame` and returns
   `Effect::Action(Action::RefreshData)` once `should_auto_refresh()` says
   the configured `refresh_interval` has elapsed), and a catch-all `_ =>
   (state, Effect::None)` for `Quit`/`Error`/anything unhandled.

`Action::RefreshData` and `Action::RefreshSchedule` get special handling one
level up, in `Runtime::dispatch` (see "Runtime" below) — after `reduce()`
returns, `Runtime` asks `DataEffects` to generate the actual fetch effects,
since the reducer itself must stay side-effect free.

## Component Messages: End-to-End Flow

```
Key Event
   │
   v
key_to_action(key, state, component_states)   // src/tui/keys.rs
   - reads component state (has_item_focus, modal open, etc.) to decide
     what the key means in context
   - returns Action::ComponentMessage { path, message } for most in-tab keys
   │
   v
reduce(state, action, component_states)       // src/tui/reducer.rs
   - matches Action::ComponentMessage at the top
   - component_states.get_mut_any(path) -> &mut dyn Any
   - message.apply(state) is called
   │
   v
<Msg as ComponentMessageTrait>::apply()       // generated by component_message_impl!
   - state.downcast_mut::<ConcreteState>()
   - <Component>::update(&mut Self::default(), msg.clone(), state)
   │
   v
Component::update() returns an Effect
```

## Effects System

```rust
pub enum Effect {
    None,
    /// Signals the input was consumed (e.g. NavigateUp exiting browse mode)
    /// so the caller does not bubble the key further.
    Handled,
    Action(Action),
    Batch(Vec<Effect>),
    Async(Pin<Box<dyn Future<Output = Action> + Send>>),
    // Data-fetch effects returned directly by reducers/components, resolved
    // synchronously by Runtime::execute_effect (see below)
    FetchBoxscore(i64),
    FetchTeamRosterStats(String),
    FetchPlayerStats(i64),
    FetchGameDetails(i64),
}
```

`Runtime::execute_effect` (called from `Runtime::dispatch` right after
`reduce()` returns) treats these differently:
- `None` / `Handled`: no-op.
- `FetchBoxscore` / `FetchTeamRosterStats` / `FetchPlayerStats` /
  `FetchGameDetails`: converted immediately into an `Effect::Async` via the
  matching `DataEffects` method and pushed onto the effect channel. They
  must never reach `process_effect_async` directly — if they do, it logs a
  warning (defensive-only; the code path always converts them first).
- `Batch(effects)`: recurses over each inner effect.
- `Action` / `Async`: pushed onto the effect channel for the background
  `run_effect_executor` task to process (dispatches the action, or awaits
  the future and dispatches its resulting action).

`DataEffects` (`src/tui/effects.rs`) wraps an `Arc<dyn NHLDataProvider>` and
exposes:
- `handle_refresh(&AppState) -> Effect` — batches `fetch_standings()` +
  `fetch_schedule(game_date)` + a `fetch_game_details()` per started game.
- `handle_refresh_schedule(GameDate) -> Effect` — used for
  `Action::RefreshSchedule`.
- `fetch_standings()`, `fetch_schedule(date)`, `fetch_game_details(id)`,
  `fetch_boxscore(id)`, `fetch_player_stats(id)` — each wraps a cached
  fetch (via `crate::cache::fetch_*_cached`) in `Effect::Async` and maps the
  result into the corresponding `*Loaded` action.
- `fetch_team_roster_stats(abbrev)` — additionally resolves the current
  season by calling `client.club_stats_season()` (uncached) before fetching
  cached club stats for the most recent season with `GameType::RegularSeason`
  data.

## Runtime

`Runtime` (`src/tui/runtime.rs`) owns `AppState`, the `ComponentStateStore`,
an `Arc<DataEffects>`, and the action/effect mpsc channels. Key methods:

- `Runtime::new(initial_state, data_effects)` — spawns the background effect
  executor task.
- `state() -> &AppState`, `component_states() -> &ComponentStateStore`
- `dispatch(action)` — runs the action through `reduce()` (via
  `std::mem::take` to avoid cloning `AppState`), then special-cases
  `RefreshData`/`RefreshSchedule` to additionally call
  `data_effects.handle_refresh()`/`handle_refresh_schedule()` for the fetch
  effect, then calls `execute_effect()`.
- `process_actions() -> usize` — drains the action queue, dispatching each;
  returns the count processed (the main loop uses this to decide whether to
  loop again immediately).
- `build() -> Element` — delegates to `App.build_with_component_states(state,
  &mut component_states)`.
- `action_sender() -> mpsc::UnboundedSender<Action>` — for effects/background
  tasks to dispatch actions back in.
- `update_viewport_heights(terminal_height)` — after `build()`, pushes the
  correct `doc_nav.viewport_height` into `ScoresTabState`, `StandingsTabState`
  and `SettingsTabState` (subtracting tab-bar/status-bar/subtab chrome).

## Component & Tab Patterns

### `Component` trait (`src/tui/component.rs`)

```rust
pub trait Component: Send {
    type Props: Clone;
    type State: Default + Clone + Send + Sync + 'static;
    type Message;

    fn init(_props: &Self::Props) -> Self::State { Self::State::default() }
    fn update(&mut self, _msg: Self::Message, _state: &mut Self::State) -> Effect { Effect::None }
    fn view(&self, props: &Self::Props, state: &Self::State) -> Element;
}
```

Components are not "registered" with the `Runtime` at startup. `App`
(`src/tui/components/app.rs`) calls
`component_states.get_or_init::<ScoresTab>(SCORES_TAB_PATH, &props)` on
each `build()`, which lazily creates the state on first access via
`Component::init`. Path constants live in `src/tui/constants.rs`
(`SCORES_TAB_PATH`, `STANDINGS_TAB_PATH`, `SETTINGS_TAB_PATH`, and
`DEMO_TAB_PATH` under the `development` feature).

`App::build_with_component_states` only builds the **active tab's** content
each frame — the other tabs get `Element::None` — since `TabbedPanel` only
renders the active `TabItem` anyway; building the others was pure waste
(including deep-cloning `Config`/standings data nobody would see).

### Tab component pattern (`src/tui/tab_component.rs`)

All tab states embed a `DocumentNavState` and implement `TabState`
(`doc_nav()`/`doc_nav_mut()`, with default `has_item_focus`/
`clear_item_focus`/`focus_first_item`). All tab messages implement
`TabMessage` (`as_common()`/`from_doc_nav()`) so `handle_common_message()`
can process the three variants shared by every tab —
`CommonTabMessage::DocNav`, `UpdateViewportHeight`, `NavigateUp` — before
the component's `update()` handles its own tab-specific messages. The
`component_message_impl!` macro then generates the `ComponentMessageTrait`
impl (downcast + `Component::update` call) for the message enum, e.g.:

```rust
component_message_impl!(ScoresTabMsg, ScoresTab, ScoresTabState);
```

### `ElementWidget` (`src/tui/component.rs`)

```rust
pub trait ElementWidget: Send + Sync {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext);
    fn clone_box(&self) -> Box<dyn ElementWidget>;
    fn preferred_height(&self) -> Option<u16> { None }
    fn preferred_width(&self) -> Option<u16> { None }
}
```

### `StandaloneWidget` (`src/tui/widgets/mod.rs`)

```rust
pub trait StandaloneWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext);
    fn preferred_height(&self) -> Option<u16> { None }
    fn preferred_width(&self) -> Option<u16> { None }
}
```

The distinction: `ElementWidget` participates in the `Element` tree (needs
`Send + Sync + clone_box`), `StandaloneWidget` is for smaller widgets
(`ScoreBox`, `BigScore`, `LoadingAnimation`) composed directly inside another
widget's `render()`.

Both traits take `ctx: &RenderContext`, not a bare `DisplayConfig`.
`RenderContext<'a>` (`src/config.rs`) bundles `config: &'a DisplayConfig`,
`focused: bool`, and the current document's tab selections, and exposes
`base_style()`/`use_unicode()` helpers.

## Document System

Scrollable content (standings tables, boxscores, settings) is built on the
`Document` trait (`src/tui/document/mod.rs`): `build(&FocusContext) ->
Vec<DocumentElement>`, plus default methods that derive focusable-element
metadata (`focusable_positions`, `focusable_heights`, `focusable_ids`,
`focusable_row_positions`, `focusable_link_targets`) from that same
`build()` output. `DocumentView` wraps a `document: Arc<dyn Document>` with
a `Viewport` and `FocusManager`, and is what an `ElementWidget::render`
typically constructs at render time (widths aren't known until then):

```rust
let mut view = DocumentView::new(Arc::new(doc), area.height);
if let Some(idx) = focus_index { view.focus_by_index(idx); }
view.set_scroll_offset(scroll_offset);
view.render(area, buf, ctx);
```

Documents shown inline in a tab (`ScoreBoxesDocument`, the standings
documents, `SettingsDocument`) keep their focus/scroll state in the owning
tab's `DocumentNavState`. Documents pushed onto `AppState.navigation.
document_stack` (`Boxscore`, `TeamDetail`, `PlayerDetail`) instead implement
`StackedDocumentHandler` (`activate()` + `populate_focusable_metadata()`),
dispatched via `get_stacked_document_handler()` and driven by
`Action::StackedDocumentKey` in `reduce_document_stack`. See
`docs/document-system.md` for the full design of this subsystem
(`DocumentBuilder`, `FocusManager`, link activation, etc.).

## Main Loop (`src/tui/mod.rs`)

The loop only calls `terminal.draw()` when a local `dirty` flag is set,
instead of redrawing every poll cycle:

- Any action processed (`process_actions() > 0`), a terminal resize, or a
  key dispatch sets `dirty = true`.
- `Action::Tick` is dispatched **every** iteration regardless of `dirty` —
  it drives both the loading-spinner `animation_frame` and the
  elapsed-time check that triggers periodic auto-refresh in `reduce()`.
  `Tick` itself does not set `dirty`; the loop separately marks the frame
  dirty when `needs_animation` (something is still loading) is true, or
  when `last_render_at.elapsed() >= IDLE_REDRAW_INTERVAL` (1 second) so the
  status bar's "updated Ns ago" text keeps advancing even while idle.
- Poll timeout is 50ms while `needs_animation` is true, 100ms otherwise.

## Renderer

`Renderer` (`src/tui/renderer.rs`) is stateless — it holds no fields and no
cross-call cache; there is no virtual-DOM diffing. `Renderer::render(element,
area, buf, ctx)` walks the `Element` tree once per call:

- `Element::Widget(w)` → `w.render(area, buf, ctx)`
- `Element::Container { children, layout }` → splits `area` via
  `ContainerLayout::Vertical`/`Horizontal` constraints and recurses
- `Element::Fragment(children)` → renders all children into the same `area`
  (later children overwrite earlier ones)
- `Element::Overlay { base, overlay }` → renders `base` then `overlay` into
  the same `area`
- `Element::FocusContext { focused, child }` → fills `area` with the
  focused/dimmed background style for that theme, then renders `child` with
  a `RenderContext` carrying the given `focused` flag
- `Element::None` → renders nothing

Redraw frequency is controlled entirely by the main loop's `dirty` flag, not
by the renderer.

## Error Handling

Two separate paths exist and are not unified:

- **Status-bar messages**: `Action::SetStatusMessage { message, is_error }`
  sets `SystemState::status_message`/`status_is_error`, which `StatusBar`
  renders with a red background/white text when `is_error` is true (used
  for settings-save failures, screenshot failures, etc.).
- **Data-load failures**: each `*Loaded(Err(...))` handler in
  `reduce_data_loading` inserts a message into `AppState.data.errors:
  HashMap<String, String>` and clears the relevant `LoadingKey`. Nothing
  currently reads `data.errors` back out to render it — a failed fetch
  simply leaves the previous data in place (or the loading indicator, if it
  was the first load) rather than surfacing the error string to the user.
  `Action::Error(String)` is never dispatched anywhere in the codebase
  either — it's an inert catch-all arm in `reduce()`.

Raw network/deserialization errors are never printed to stderr/stdout in
either case.

## Key Dependencies

- `nhl_api`: NHL API client (local path dependency, `../nhl-api`)
- `ratatui` (0.29.0): Terminal UI framework
- `crossterm` (0.28.1): Cross-platform terminal manipulation
- `tokio` (1.x, `full` features): Async runtime
- `chrono` (0.4.42): Date/time handling
