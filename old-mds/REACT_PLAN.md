# Heavy Refactor Plan: React-like TUI Architecture

## Model Selection Guide: Sonnet vs Opus

**Use Sonnet (what you're using now) for:**
- Planning and architecture design (like this)
- Code review and analysis
- Refactoring existing code with clear patterns
- Writing tests
- Documentation
- Most implementation work where the pattern is established
- **Cost/benefit:** 95% as good as Opus for 5% of the cost

**Use Opus for:**
- Novel architectural decisions with multiple tradeoffs
- Complex trait design with lifetime/generic constraints
- Debugging really gnarly lifetime/borrow checker issues
- Initial implementation of new complex patterns (after that, Sonnet can replicate)
- When you've tried Sonnet twice and it's not getting it right
- **When to switch:** If you're on your 3rd iteration fixing the same issue

**For this refactor:** Start with Sonnet, escalate to Opus only if we hit complex trait design issues or lifetime hell.

---

## Core Philosophy

**Goal:** Transform from imperative "render what's there" to declarative "describe what should be" with:
1. **Unidirectional data flow:** Actions → State → View
2. **Component tree:** Everything is a component with props/children
3. **Pure rendering:** Components are pure functions of their props
4. **Effect system:** Side effects (data fetching) separated from rendering
5. **Immutable updates:** State changes produce new state

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                     Application                          │
│                                                          │
│  ┌────────────┐      ┌──────────────┐                  │
│  │   Actions  │─────→│  Reducer     │                  │
│  └────────────┘      │  (Pure fn)   │                  │
│         ▲            └──────┬───────┘                  │
│         │                   │                           │
│         │                   ▼                           │
│    ┌────┴─────┐      ┌──────────────┐                 │
│    │ Effects  │      │  State Tree  │                  │
│    │ (async)  │      │  (immutable) │                  │
│    └──────────┘      └──────┬───────┘                 │
│                             │                           │
│                             ▼                           │
│                      ┌──────────────┐                  │
│                      │  Component   │                  │
│                      │  Tree        │                  │
│                      │  (Virtual)   │                  │
│                      └──────┬───────┘                  │
│                             │                           │
│                             ▼                           │
│                      ┌──────────────┐                  │
│                      │  Renderer    │                  │
│                      │  (Ratatui)   │                  │
│                      └──────────────┘                  │
└─────────────────────────────────────────────────────────┘
```

---

## Phase 1: Foundation - Core Abstractions (Week 1)

### 1.1: Component Trait & Props System

**File:** `src/tui/framework/component.rs`

```rust
/// Core component trait - like React.Component
pub trait Component: Send {
    /// Props type for this component
    type Props: Clone;
    /// Local state type (if any)
    type State: Default + Clone;
    /// Message type for internal events
    type Message;

    /// Create initial state from props (like useState)
    fn init(props: &Self::Props) -> Self::State {
        Self::State::default()
    }

    /// Update state based on message (like reducer)
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect;

    /// Render component given props and state (pure function)
    fn view(&self, props: &Self::Props, state: &Self::State) -> Element;

    /// Lifecycle: called when props change
    fn did_update(&mut self, old_props: &Self::Props, new_props: &Self::Props) -> Effect {
        Effect::None
    }
}

/// Element in virtual component tree
pub enum Element {
    Component(Box<dyn AnyComponent>),
    Widget(Box<dyn RenderableWidget>),
    Container {
        children: Vec<Element>,
        layout: Layout,
    },
    Fragment(Vec<Element>),
    None,
}

/// Side effects to run after rendering
pub enum Effect {
    None,
    Action(Action),
    Batch(Vec<Effect>),
    Async(Pin<Box<dyn Future<Output = Action> + Send>>),
}
```

**Why this design:**
- `Component::view()` returns `Element`, not direct rendering → enables virtual tree
- `update()` returns `Effect` → separates pure state updates from side effects
- Generic `Props` and `State` → type safety like React with TypeScript
- `Message` type → internal component events (like `useState` setters)

### 1.2: Action System

**File:** `src/tui/framework/action.rs`

```rust
/// Global actions - like Redux actions
#[derive(Debug, Clone)]
pub enum Action {
    // Navigation actions
    NavigateTab(Tab),
    EnterSubtabMode,
    ExitSubtabMode,
    PushPanel(Panel),
    PopPanel,

    // Data actions
    SetGameDate(GameDate),
    SelectTeam(String),
    SelectPlayer(i64),
    RefreshData,

    // Data loaded (from effects)
    StandingsLoaded(Result<Vec<Standing>>),
    ScheduleLoaded(Result<DailySchedule>),
    GameDetailsLoaded(i64, Result<GameInfo>),

    // UI actions
    ScrollUp(usize),
    ScrollDown(usize),
    FocusNext,
    FocusPrevious,

    // Component-specific actions (nested)
    ScoresAction(ScoresAction),
    StandingsAction(StandingsAction),

    // System actions
    Quit,
    Error(String),
}

/// Tab-specific actions
#[derive(Debug, Clone)]
pub enum ScoresAction {
    DateLeft,
    DateRight,
    SelectGame(i64),
}

#[derive(Debug, Clone)]
pub enum StandingsAction {
    CycleView,
    SelectTeam(usize, usize), // column, row
    EnterTeamMode,
    ExitTeamMode,
}
```

**Why this design:**
- Enum-based → exhaustive matching, no missing cases
- Nested actions → namespace organization (like Redux "slice" pattern)
- Includes both user actions AND async results → single action channel
- `Clone` → can be dispatched from multiple places

### 1.3: State Tree

**File:** `src/tui/framework/state.rs`

```rust
/// Root application state - single source of truth
#[derive(Debug, Clone)]
pub struct AppState {
    // Navigation state
    pub navigation: NavigationState,

    // Application data (from API)
    pub data: DataState,

    // UI state per tab
    pub ui: UiState,

    // System state
    pub system: SystemState,
}

#[derive(Debug, Clone)]
pub struct NavigationState {
    pub current_tab: Tab,
    pub subtab_focused: bool,
    pub panel_stack: Vec<PanelState>,
}

#[derive(Debug, Clone)]
pub struct DataState {
    // API data
    pub standings: Option<Vec<Standing>>,
    pub schedule: Option<DailySchedule>,
    pub game_details: HashMap<i64, GameInfo>,
    pub team_roster: HashMap<String, Roster>,
    pub player_stats: HashMap<i64, PlayerStats>,

    // Loading states
    pub loading: HashSet<LoadingKey>,

    // Errors
    pub errors: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub scores: ScoresUiState,
    pub standings: StandingsUiState,
    pub settings: SettingsUiState,
}

#[derive(Debug, Clone)]
pub struct ScoresUiState {
    pub selected_date_index: usize,
    pub game_date: GameDate,
}

#[derive(Debug, Clone)]
pub struct StandingsUiState {
    pub view: GroupBy,
    pub team_mode: bool,
    pub selected_column: usize,
    pub selected_row: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub struct SystemState {
    pub last_refresh: Option<SystemTime>,
    pub config: Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoadingKey {
    Standings,
    Schedule(GameDate),
    GameDetails(i64),
    TeamRoster(String),
}
```

**Why this design:**
- Single root state → easy to serialize, time-travel debug, undo/redo
- `Clone` → can pass slices to components without borrow issues
- Separate `data` vs `ui` → clear boundary between "truth" and "view state"
- `loading` set → easy to show spinners for any data
- Normalized data → `HashMap` for entities, no duplication

### 1.4: Reducer

**File:** `src/tui/framework/reducer.rs`

```rust
/// Pure state reducer - like Redux reducer
pub fn reduce(state: AppState, action: Action) -> (AppState, Effect) {
    match action {
        Action::NavigateTab(tab) => {
            let mut new_state = state.clone();
            new_state.navigation.current_tab = tab;
            new_state.navigation.subtab_focused = false;
            new_state.navigation.panel_stack.clear();
            (new_state, Effect::None)
        }

        Action::SetGameDate(date) => {
            let mut new_state = state.clone();
            new_state.ui.scores.game_date = date;

            // Return effect to fetch schedule for this date
            let effect = Effect::Async(Box::pin(async move {
                Action::RefreshData
            }));

            (new_state, effect)
        }

        Action::StandingsLoaded(Ok(standings)) => {
            let mut new_state = state.clone();
            new_state.data.standings = Some(standings);
            new_state.data.loading.remove(&LoadingKey::Standings);
            new_state.data.errors.remove("standings");
            (new_state, Effect::None)
        }

        Action::StandingsLoaded(Err(e)) => {
            let mut new_state = state.clone();
            new_state.data.loading.remove(&LoadingKey::Standings);
            new_state.data.errors.insert("standings".into(), e.to_string());
            (new_state, Effect::None)
        }

        // Delegate to sub-reducers
        Action::ScoresAction(scores_action) => {
            reduce_scores(state, scores_action)
        }

        Action::StandingsAction(standings_action) => {
            reduce_standings(state, standings_action)
        }

        _ => (state, Effect::None)
    }
}

/// Sub-reducer for scores tab
fn reduce_scores(state: AppState, action: ScoresAction) -> (AppState, Effect) {
    match action {
        ScoresAction::DateLeft => {
            let mut new_state = state.clone();
            let ui = &mut new_state.ui.scores;

            if ui.selected_date_index == 0 {
                // At edge - shift window
                ui.game_date = ui.game_date.add_days(-1);
            } else {
                // Within window - move index
                ui.selected_date_index -= 1;
                let window_base = ui.game_date.add_days(-(ui.selected_date_index as i64 + 1));
                ui.game_date = window_base.add_days(ui.selected_date_index as i64);
            }

            // Effect: fetch schedule for new date
            let date = ui.game_date.clone();
            let effect = Effect::Async(Box::pin(async move {
                // This will trigger fetch_schedule effect
                Action::RefreshData
            }));

            (new_state, effect)
        }
        _ => (state, Effect::None)
    }
}
```

**Why this design:**
- Pure functions → easy to test, no side effects
- `(State, Effect)` return → separates state updates from async work
- Sub-reducers → code organization, mirrors component tree
- Immutable updates with clone → safe, can enable time-travel debugging later

---

## Phase 2: Runtime & Component System (Week 2)

### 2.1: Component Runtime

**File:** `src/tui/framework/runtime.rs`

```rust
/// Component runtime - manages component lifecycle
pub struct Runtime {
    /// Root component
    root: Box<dyn AnyComponent>,

    /// Current state
    state: AppState,

    /// Action queue
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    /// Effect executor
    effect_tx: mpsc::UnboundedSender<Effect>,

    /// Async runtime
    tokio_handle: tokio::runtime::Handle,
}

impl Runtime {
    pub fn new(root: Box<dyn AnyComponent>, initial_state: AppState) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (effect_tx, effect_rx) = mpsc::unbounded_channel();

        // Spawn effect executor
        let action_tx_clone = action_tx.clone();
        tokio::spawn(async move {
            Self::run_effect_executor(effect_rx, action_tx_clone).await;
        });

        Self {
            root,
            state: initial_state,
            action_tx,
            action_rx,
            effect_tx,
            tokio_handle: tokio::runtime::Handle::current(),
        }
    }

    /// Process one action and update state
    pub fn dispatch(&mut self, action: Action) {
        let (new_state, effect) = reduce(self.state.clone(), action);
        self.state = new_state;

        // Queue effect for execution
        if !matches!(effect, Effect::None) {
            let _ = self.effect_tx.send(effect);
        }
    }

    /// Build virtual component tree
    pub fn build(&mut self) -> Element {
        self.root.view_any(&self.state)
    }

    /// Execute effect (async)
    async fn run_effect_executor(
        mut effect_rx: mpsc::UnboundedReceiver<Effect>,
        action_tx: mpsc::UnboundedSender<Action>,
    ) {
        while let Some(effect) = effect_rx.recv().await {
            match effect {
                Effect::Action(action) => {
                    let _ = action_tx.send(action);
                }
                Effect::Batch(effects) => {
                    for e in effects {
                        // Re-queue each effect
                        let _ = effect_rx.send(e);
                    }
                }
                Effect::Async(future) => {
                    let action_tx = action_tx.clone();
                    tokio::spawn(async move {
                        let action = future.await;
                        let _ = action_tx.send(action);
                    });
                }
                Effect::None => {}
            }
        }
    }
}
```

### 2.2: Virtual Tree Renderer

**File:** `src/tui/framework/renderer.rs`

```rust
/// Renders virtual element tree to ratatui buffer
pub struct Renderer {
    /// Previous frame's tree (for diffing later)
    prev_tree: Option<Element>,
}

impl Renderer {
    pub fn render(&mut self, element: Element, area: Rect, buf: &mut Buffer) {
        self.render_element(&element, area, buf);
        self.prev_tree = Some(element);
    }

    fn render_element(&self, element: &Element, area: Rect, buf: &mut Buffer) {
        match element {
            Element::Widget(widget) => {
                widget.render(area, buf);
            }

            Element::Container { children, layout } => {
                let chunks = layout.split(area);
                for (child, chunk) in children.iter().zip(chunks.iter()) {
                    self.render_element(child, *chunk, buf);
                }
            }

            Element::Fragment(children) => {
                // Render all children in same area (for conditional rendering)
                for child in children {
                    self.render_element(child, area, buf);
                }
            }

            Element::Component(component) => {
                // Components should already be resolved to elements
                // This shouldn't happen in practice
                panic!("Unresolved component in render tree");
            }

            Element::None => {
                // Render nothing
            }
        }
    }
}
```

---

## Phase 3: Component Library (Week 3)

### 3.1: Core Components

**File:** `src/tui/components/app.rs`

```rust
/// Root App component
pub struct App {
    client: Arc<Client>,
}

impl Component for App {
    type Props = ();
    type State = ();
    type Message = ();

    fn view(&self, _props: &(), _state: &()) -> Element {
        Element::Container {
            layout: Layout::vertical([
                Constraint::Length(1), // TabBar
                Constraint::Min(0),    // Content
                Constraint::Length(1), // StatusBar
            ]),
            children: vec![
                Element::Component(Box::new(TabBar)),
                Element::Component(Box::new(TabContent)),
                Element::Component(Box::new(StatusBar)),
            ],
        }
    }
}
```

**File:** `src/tui/components/tab_content.rs`

```rust
/// Routes to appropriate tab component based on state
pub struct TabContent;

impl Component for TabContent {
    type Props = ();
    type State = ();
    type Message = ();

    fn view(&self, _props: &(), _state: &()) -> Element {
        // Access state via context (we'll add context system)
        let current_tab = use_context::<Tab>();

        match current_tab {
            Tab::Scores => Element::Component(Box::new(ScoresTab)),
            Tab::Standings => Element::Component(Box::new(StandingsTab)),
            Tab::Settings => Element::Component(Box::new(SettingsTab)),
        }
    }
}
```

### 3.2: Scores Tab Component

**File:** `src/tui/components/scores_tab.rs`

```rust
pub struct ScoresTab;

#[derive(Clone)]
pub struct ScoresTabProps {
    pub game_date: GameDate,
    pub selected_index: usize,
    pub schedule: Option<DailySchedule>,
    pub game_details: HashMap<i64, GameInfo>,
}

impl Component for ScoresTab {
    type Props = ScoresTabProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &()) -> Element {
        Element::Container {
            layout: Layout::vertical([
                Constraint::Length(1), // Date selector
                Constraint::Min(0),    // Game list
            ]),
            children: vec![
                self.render_date_selector(props),
                self.render_game_list(props),
            ],
        }
    }
}

impl ScoresTab {
    fn render_date_selector(&self, props: &ScoresTabProps) -> Element {
        let dates = calculate_date_window(&props.game_date, props.selected_index);

        Element::Component(Box::new(DateSelector {
            props: DateSelectorProps {
                dates,
                selected_index: props.selected_index,
                on_left: Action::ScoresAction(ScoresAction::DateLeft),
                on_right: Action::ScoresAction(ScoresAction::DateRight),
            }
        }))
    }

    fn render_game_list(&self, props: &ScoresTabProps) -> Element {
        match &props.schedule {
            None => Element::Widget(Box::new(LoadingSpinner)),
            Some(schedule) => {
                let game_elements = schedule.games.iter().map(|game| {
                    let details = props.game_details.get(&game.id);
                    Element::Component(Box::new(GameCard {
                        props: GameCardProps {
                            game: game.clone(),
                            details: details.cloned(),
                        }
                    }))
                }).collect();

                Element::Component(Box::new(GameGrid {
                    props: GameGridProps {
                        children: game_elements,
                    }
                }))
            }
        }
    }
}
```

**Why this design:**
- Pure function of props → easy to test
- Composition via `Element::Component` → tree of components
- `on_left`/`on_right` props → callbacks dispatch actions
- Declarative → describes what should exist, not how to make it

### 3.3: Standings Tab Component

**File:** `src/tui/components/standings_tab.rs`

```rust
pub struct StandingsTab;

#[derive(Clone)]
pub struct StandingsTabProps {
    pub view: GroupBy,
    pub team_mode: bool,
    pub selected_column: usize,
    pub selected_row: usize,
    pub standings: Option<Vec<Standing>>,
    pub panel_stack: Vec<PanelState>,
}

impl Component for StandingsTab {
    type Props = StandingsTabProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &()) -> Element {
        if !props.panel_stack.is_empty() {
            // In panel view
            return self.render_panel(props);
        }

        // Root view
        Element::Container {
            layout: Layout::vertical([
                Constraint::Length(1), // View selector
                Constraint::Min(0),    // Standings table
            ]),
            children: vec![
                self.render_view_selector(props),
                self.render_standings_table(props),
            ],
        }
    }
}

impl StandingsTab {
    fn render_view_selector(&self, props: &StandingsTabProps) -> Element {
        Element::Component(Box::new(ViewSelector {
            props: ViewSelectorProps {
                current_view: props.view,
                focused: !props.team_mode,
                on_cycle: Action::StandingsAction(StandingsAction::CycleView),
            }
        }))
    }

    fn render_standings_table(&self, props: &StandingsTabProps) -> Element {
        match &props.standings {
            None => Element::Widget(Box::new(LoadingSpinner)),
            Some(standings) => {
                let grouped = group_standings(standings, props.view);

                Element::Component(Box::new(StandingsTable {
                    props: StandingsTableProps {
                        groups: grouped,
                        selected_column: props.selected_column,
                        selected_row: props.selected_row,
                        focused: props.team_mode,
                        on_select: |col, row| {
                            Action::StandingsAction(StandingsAction::SelectTeam(col, row))
                        },
                    }
                }))
            }
        }
    }

    fn render_panel(&self, props: &StandingsTabProps) -> Element {
        let panel = props.panel_stack.last().unwrap();

        match panel {
            PanelState::TeamDetail { abbrev, .. } => {
                Element::Component(Box::new(TeamDetailPanel {
                    props: TeamDetailPanelProps {
                        team_abbrev: abbrev.clone(),
                        // ... pass necessary data from props
                    }
                }))
            }
            PanelState::PlayerDetail { player_id, .. } => {
                Element::Component(Box::new(PlayerDetailPanel {
                    props: PlayerDetailPanelProps {
                        player_id: *player_id,
                        // ... pass necessary data
                    }
                }))
            }
        }
    }
}
```

---

## Phase 4: Effects System (Week 4)

### 4.1: Effect Handlers

**File:** `src/tui/framework/effects.rs`

```rust
/// Effect handler for data fetching
pub struct DataEffects {
    client: Arc<Client>,
}

impl DataEffects {
    pub fn handle_refresh(&self, state: &AppState) -> Effect {
        let effects = vec![
            self.fetch_standings(),
            self.fetch_schedule(state.ui.scores.game_date.clone()),
        ];

        // Add game detail fetches if needed
        if let Some(schedule) = &state.data.schedule {
            for game in &schedule.games {
                if game.game_state != GameState::Preview {
                    effects.push(self.fetch_game_details(game.id));
                }
            }
        }

        Effect::Batch(effects)
    }

    fn fetch_standings(&self) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = client.current_league_standings().await;
            Action::StandingsLoaded(result)
        }))
    }

    fn fetch_schedule(&self, date: GameDate) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = client.daily_schedule(&date).await;
            Action::ScheduleLoaded(result)
        }))
    }

    fn fetch_game_details(&self, game_id: i64) -> Effect {
        let client = self.client.clone();
        Effect::Async(Box::pin(async move {
            let result = client.landing(game_id).await;
            Action::GameDetailsLoaded(game_id, result)
        }))
    }
}
```

### 4.2: Middleware Pattern

**File:** `src/tui/framework/middleware.rs`

```rust
/// Middleware can intercept actions before/after reducer
pub trait Middleware: Send + Sync {
    fn before(&self, action: &Action, state: &AppState) -> Option<Effect>;
    fn after(&self, action: &Action, old_state: &AppState, new_state: &AppState) -> Option<Effect>;
}

/// Logging middleware
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn before(&self, action: &Action, _state: &AppState) -> Option<Effect> {
        tracing::debug!("Action dispatched: {:?}", action);
        None
    }

    fn after(&self, _action: &Action, old_state: &AppState, new_state: &AppState) -> Option<Effect> {
        if old_state.ui.scores.game_date != new_state.ui.scores.game_date {
            tracing::info!("Game date changed to: {}", new_state.ui.scores.game_date);
        }
        None
    }
}

/// Auto-refresh middleware
pub struct AutoRefreshMiddleware {
    data_effects: Arc<DataEffects>,
}

impl Middleware for AutoRefreshMiddleware {
    fn after(&self, _action: &Action, old_state: &AppState, new_state: &AppState) -> Option<Effect> {
        // If game date changed, fetch new schedule
        if old_state.ui.scores.game_date != new_state.ui.scores.game_date {
            return Some(self.data_effects.fetch_schedule(new_state.ui.scores.game_date.clone()));
        }

        // If team selected, fetch roster
        if old_state.navigation.panel_stack.len() < new_state.navigation.panel_stack.len() {
            if let Some(PanelState::TeamDetail { abbrev, .. }) = new_state.navigation.panel_stack.last() {
                return Some(self.data_effects.fetch_team_roster(abbrev.clone()));
            }
        }

        None
    }
}
```

---

## Phase 5: Migration Strategy (Weeks 5-7)

### 5.1: Parallel Development

**Strategy:** Build new framework alongside old code, migrate tab-by-tab

```
src/
├── tui/
│   ├── framework/         # NEW: React-like framework
│   │   ├── component.rs
│   │   ├── action.rs
│   │   ├── state.rs
│   │   ├── reducer.rs
│   │   ├── runtime.rs
│   │   ├── renderer.rs
│   │   ├── effects.rs
│   │   └── middleware.rs
│   │
│   ├── components/        # NEW: React-like components
│   │   ├── app.rs
│   │   ├── scores_tab.rs
│   │   ├── standings_tab.rs
│   │   └── ...
│   │
│   ├── legacy/            # OLD: Move existing code here
│   │   ├── app.rs
│   │   ├── scores/
│   │   ├── standings/
│   │   └── ...
│   │
│   └── mod.rs             # Hybrid: route to new vs old
```

### 5.2: Migration Order

**Week 5: Migrate Scores Tab**
1. Implement `ScoresTab` component
2. Implement `ScoresAction` and `reduce_scores()`
3. Add `fetch_schedule` effect
4. Wire up in hybrid `mod.rs`
5. **Test thoroughly** - keep old code as reference
6. Delete old scores code

**Week 6: Migrate Standings Tab (Root View)**
1. Implement `StandingsTab` component (root view only, no panels yet)
2. Implement `StandingsAction` and `reduce_standings()`
3. Add `fetch_standings` effect
4. Wire up in hybrid `mod.rs`
5. **Test thoroughly**
6. Delete old standings root view code

**Week 7: Migrate Standings Panels**
1. Implement `TeamDetailPanel` component
2. Implement `PlayerDetailPanel` component
3. Add panel navigation to `reduce_standings()`
4. Add `fetch_team_roster`, `fetch_player_stats` effects
5. **Test thoroughly**
6. Delete old panel code

**Week 8: Cleanup & Settings Tab**
1. Delete all `legacy/` code
2. Implement `SettingsTab` component (proper implementation, not placeholder)
3. Remove `SharedData` - fully replaced by `AppState`
4. Remove old event loop - fully replaced by `Runtime`
5. Final integration tests

### 5.3: Hybrid Runtime (During Migration)

**File:** `src/tui/mod.rs` (temporary hybrid version)

```rust
pub async fn run() -> Result<()> {
    // Initialize both systems
    let old_app_state = legacy::AppState::new();
    let new_app_state = framework::AppState::default();
    let runtime = framework::Runtime::new(
        Box::new(components::App::new(client.clone())),
        new_app_state,
    );

    loop {
        // Render
        terminal.draw(|f| {
            match current_tab {
                Tab::Scores => {
                    // NEW: Use React-like component
                    let element = runtime.build();
                    renderer.render(element, f.size(), f.buffer_mut());
                }
                Tab::Standings if !in_panel_view => {
                    // NEW: Use React-like component for root view
                    let element = runtime.build();
                    renderer.render(element, f.size(), f.buffer_mut());
                }
                Tab::Standings => {
                    // OLD: Use legacy rendering for panels (not migrated yet)
                    legacy::standings::render(f, &old_app_state);
                }
                Tab::Settings => {
                    // OLD: Use legacy rendering
                    legacy::settings::render(f, &old_app_state);
                }
            }
        })?;

        // Event handling
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    match current_tab {
                        Tab::Scores => {
                            // NEW: Dispatch action
                            let action = key_to_scores_action(key);
                            runtime.dispatch(action);
                        }
                        _ => {
                            // OLD: Use legacy handler
                            legacy::handle_key(key, &mut old_app_state);
                        }
                    }
                }
                _ => {}
            }
        }

        // Process actions from effects
        while let Ok(action) = runtime.action_rx.try_recv() {
            runtime.dispatch(action);
        }
    }
}
```

---

## Phase 6: Advanced Features (Week 9+)

### 6.1: Context System (like React Context)

**File:** `src/tui/framework/context.rs`

```rust
/// Context provides data down the component tree without props drilling
pub trait Context: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

pub struct ContextProvider {
    contexts: HashMap<TypeId, Box<dyn Context>>,
}

impl ContextProvider {
    pub fn provide<T: Context + 'static>(&mut self, value: T) {
        self.contexts.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Context + 'static>(&self) -> Option<&T> {
        self.contexts
            .get(&TypeId::of::<T>())
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }
}

// Usage in components:
pub fn use_context<T: Context + 'static>() -> &T {
    CONTEXT_PROVIDER.with(|cp| {
        cp.get::<T>().expect("Context not provided")
    })
}
```

### 6.2: Hooks System (like React Hooks)

**File:** `src/tui/framework/hooks.rs`

```rust
/// Hook for local component state
pub fn use_state<T: Clone>(initial: T) -> (T, impl Fn(T)) {
    // Implementation stores state in component-local storage
    // identified by component ID + hook call order
    todo!("Implement hook state storage")
}

/// Hook for effects that run after render
pub fn use_effect<F>(effect: F, deps: &[&dyn Any])
where
    F: FnOnce() -> Effect,
{
    // Implementation tracks dependencies and only runs effect when they change
    todo!("Implement effect dependency tracking")
}

// Usage:
impl Component for MyComponent {
    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        let (count, set_count) = use_state(0);

        use_effect(|| {
            // Run side effect
            Effect::Action(Action::Log(format!("Count changed: {}", count)))
        }, &[&count]);

        // ...
    }
}
```

### 6.3: Time-Travel Debugging

**File:** `src/tui/framework/devtools.rs`

```rust
/// Records all state transitions for debugging
pub struct DevTools {
    history: Vec<(Action, AppState)>,
    current_index: usize,
}

impl DevTools {
    pub fn record(&mut self, action: Action, state: AppState) {
        self.history.truncate(self.current_index + 1);
        self.history.push((action, state));
        self.current_index = self.history.len() - 1;
    }

    pub fn undo(&mut self) -> Option<AppState> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(self.history[self.current_index].1.clone())
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<AppState> {
        if self.current_index < self.history.len() - 1 {
            self.current_index += 1;
            Some(self.history[self.current_index].1.clone())
        } else {
            None
        }
    }
}
```

---

## Implementation Checklist

### Phase 1: Foundation (Week 1)
- [ ] Create `src/tui/framework/` directory
- [ ] Implement `Component` trait with Props/State/Message
- [ ] Implement `Element` enum for virtual tree
- [ ] Implement `Action` enum with all actions
- [ ] Implement `AppState` struct with all state
- [ ] Implement `reduce()` function with sub-reducers
- [ ] Write unit tests for reducers (pure functions = easy to test)

### Phase 2: Runtime (Week 2)
- [ ] Implement `Runtime` with action queue
- [ ] Implement effect executor
- [ ] Implement `Renderer` for virtual tree → ratatui
- [ ] Wire up tokio async runtime
- [ ] Test action dispatching
- [ ] Test effect execution

### Phase 3: Components (Week 3)
- [ ] Implement `App` root component
- [ ] Implement `TabBar` component
- [ ] Implement `StatusBar` component
- [ ] Implement `ScoresTab` component
- [ ] Implement `DateSelector` component
- [ ] Implement `GameCard` component
- [ ] Implement `GameGrid` component

### Phase 4: Effects (Week 4)
- [ ] Implement `DataEffects` handler
- [ ] Implement `fetch_standings` effect
- [ ] Implement `fetch_schedule` effect
- [ ] Implement `fetch_game_details` effect
- [ ] Implement middleware system
- [ ] Implement `AutoRefreshMiddleware`
- [ ] Implement `LoggingMiddleware`

### Phase 5: Migration (Weeks 5-7)
- [ ] Create `src/tui/legacy/` and move old code
- [ ] Migrate Scores tab (Week 5)
- [ ] Migrate Standings root view (Week 6)
- [ ] Migrate Standings panels (Week 7)
- [ ] Delete legacy code (Week 8)
- [ ] Migrate Settings tab (Week 8)

### Phase 6: Polish (Week 9+)
- [ ] Implement context system
- [ ] Implement hooks system
- [ ] Add time-travel debugging
- [ ] Performance optimization (memoization, diffing)
- [ ] Documentation

---

## Testing Strategy

### Unit Tests
- **Reducers:** Test every action produces correct state
- **Components:** Test `view()` produces correct elements
- **Effects:** Test effects dispatch correct actions

### Integration Tests
- **Navigation:** Test tab switching, panel navigation
- **Data flow:** Test action → state → render → effect → action loop
- **Error handling:** Test error states display correctly

### Visual Regression Tests
- Use `assert_buffer!` for rendering tests
- Compare full buffer, not substrings

---

## Benefits of This Architecture

1. **Predictable state:** Single source of truth, pure reducers
2. **Testable:** Pure functions everywhere, easy to unit test
3. **Debuggable:** Time-travel, action logging, Redux DevTools-like experience
4. **Composable:** Components compose like React components
5. **Maintainable:** Clear separation of concerns, unidirectional flow
6. **Type-safe:** Rust's type system prevents many bugs
7. **Performant:** Virtual tree + diffing (future), async effects

## Tradeoffs

**Pros:**
- Much more maintainable long-term
- Easier to reason about state changes
- Better for complex UIs with lots of interaction
- Industry-proven pattern (React, Elm, Redux)

**Cons:**
- More boilerplate initially (actions, reducers, components)
- Steeper learning curve for contributors
- `Clone` on state could be expensive (mitigated by Arc for large data)
- More complex than simple imperative code for trivial UIs

**Verdict:** For a TUI with multiple tabs, navigation, async data, and complex interactions, the React-like architecture is worth the upfront cost.
