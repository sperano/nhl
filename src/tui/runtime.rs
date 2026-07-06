use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, trace};

use super::action::Action;
use super::component::{Effect, Element};
use super::component_store::ComponentStateStore;
#[cfg(feature = "development")]
use super::constants::DEMO_TAB_PATH;
use super::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use super::effects::DataEffects;
use super::reducer::reduce;
use super::state::AppState;
use super::tab_component::TabState;

/// Component runtime - manages component lifecycle and action processing
///
/// The Runtime is responsible for:
/// - Managing the application state
/// - Managing component state instances (React-like lifecycle)
/// - Dispatching actions through the reducer
/// - Executing side effects asynchronously
/// - Building the virtual component tree
pub struct Runtime {
    /// Current application state
    state: AppState,

    /// Component state storage for lifecycle management
    component_states: ComponentStateStore,

    /// Channel for dispatching actions
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    /// Channel for queuing effects
    effect_tx: mpsc::UnboundedSender<Effect>,

    /// Data effects handler
    data_effects: Arc<DataEffects>,
}

impl Runtime {
    /// Create a new runtime with initial state and data effects handler
    pub fn new(initial_state: AppState, data_effects: Arc<DataEffects>) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (effect_tx, mut effect_rx) = mpsc::unbounded_channel();

        // Spawn effect executor task
        let action_tx_clone = action_tx.clone();
        tokio::spawn(async move {
            Self::run_effect_executor(&mut effect_rx, action_tx_clone).await;
        });

        Self {
            state: initial_state,
            component_states: ComponentStateStore::new(),
            action_tx,
            action_rx,
            effect_tx,
            data_effects,
        }
    }

    /// Get a reference to the current state
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get a reference to the component state store
    pub fn component_states(&self) -> &ComponentStateStore {
        &self.component_states
    }

    /// Dispatch an action to be processed by the reducer
    ///
    /// Uses mem::take to avoid cloning AppState. Reducers now return fetch effects
    /// directly instead of runtime comparing old/new state.
    pub fn dispatch(&mut self, action: Action) {
        trace!("ACTION: Dispatching {:?}", action);

        // Handle RefreshData and RefreshSchedule actions specially - generate data fetch effects
        let effect = if matches!(action, Action::RefreshData) {
            debug!("ACTION: RefreshData - generating fetch effects");

            // Take ownership temporarily, run reducer, put back
            let state = std::mem::take(&mut self.state);
            let (new_state, _reducer_effect) =
                reduce(state, action.clone(), &mut self.component_states);
            self.state = new_state;

            // Then generate data fetch effects
            self.data_effects.handle_refresh(&self.state)
        } else if let Action::RefreshSchedule(date) = &action {
            debug!(
                "ACTION: RefreshSchedule({:?}) - generating fetch effects",
                date
            );
            let date = date.clone();

            // Take ownership temporarily, run reducer (handles the state transition), put back
            let state = std::mem::take(&mut self.state);
            let (new_state, _reducer_effect) =
                reduce(state, action.clone(), &mut self.component_states);
            self.state = new_state;

            // Then generate the schedule fetch effect for the specific date
            self.data_effects.handle_refresh_schedule(date)
        } else {
            // Take ownership temporarily using mem::take pattern (no clone!)
            let state = std::mem::take(&mut self.state);
            let (new_state, reducer_effect) = reduce(state, action, &mut self.component_states);
            self.state = new_state;

            // Reducer now returns fetch effects directly, no need to compare old/new state
            reducer_effect
        };

        // Execute the effect (handles both legacy Effect::Async and new fetch variants)
        self.execute_effect(effect);
    }

    /// Execute an effect, handling both legacy async effects and new fetch variants
    fn execute_effect(&self, effect: Effect) {
        match effect {
            Effect::None | Effect::Handled => {
                // Nothing to do
            }
            Effect::FetchBoxscore(game_id) => {
                debug!("EFFECT: Executing boxscore fetch for game_id={}", game_id);
                let fetch_effect = self.data_effects.fetch_boxscore(game_id);
                let _ = self.effect_tx.send(fetch_effect);
            }
            Effect::FetchTeamRosterStats(abbrev) => {
                debug!(
                    "EFFECT: Executing team roster stats fetch for team={}",
                    abbrev
                );
                let fetch_effect = self.data_effects.fetch_team_roster_stats(abbrev);
                let _ = self.effect_tx.send(fetch_effect);
            }
            Effect::FetchPlayerStats(player_id) => {
                debug!(
                    "EFFECT: Executing player stats fetch for player_id={}",
                    player_id
                );
                let fetch_effect = self.data_effects.fetch_player_stats(player_id);
                let _ = self.effect_tx.send(fetch_effect);
            }
            Effect::FetchGameDetails(game_id) => {
                debug!(
                    "EFFECT: Executing game details fetch for game_id={}",
                    game_id
                );
                let fetch_effect = self.data_effects.fetch_game_details(game_id);
                let _ = self.effect_tx.send(fetch_effect);
            }
            Effect::Batch(effects) => {
                // Execute each effect in the batch
                for e in effects {
                    self.execute_effect(e);
                }
            }
            // Legacy effects - queue for async executor
            Effect::Action(_) | Effect::Async(_) => {
                trace!("ACTION: Queueing effect for async execution");
                let _ = self.effect_tx.send(effect);
            }
        }
    }

    /// Process all pending actions in the queue
    ///
    /// Returns the number of actions processed
    pub fn process_actions(&mut self) -> usize {
        let mut count = 0;
        while let Ok(action) = self.action_rx.try_recv() {
            self.dispatch(action);
            count += 1;
        }
        count
    }

    /// Build the virtual element tree from current state
    ///
    /// This will be used by the Renderer to produce the actual terminal output.
    /// It builds the component tree by calling the root App component's view() method
    /// with the current state as props.
    ///
    /// Note: Currently needs &mut self to manage component states, but the build itself
    /// is logically a read operation. In the future, we might use RefCell or similar
    /// for interior mutability if needed.
    pub fn build(&mut self) -> Element {
        use crate::tui::components::App;

        let app = App;
        // App needs access to component_states to get child component states,
        // so we call a special method instead of the normal view()
        app.build_with_component_states(&self.state, &mut self.component_states)
    }

    /// Get a sender for dispatching actions from external sources
    pub fn action_sender(&self) -> mpsc::UnboundedSender<Action> {
        self.action_tx.clone()
    }

    /// Update viewport heights for all document-based components
    ///
    /// Called from the main render loop with the current terminal area height.
    /// Each tab declares its own chrome height via `TabState::chrome_lines()`
    /// (tab bar + status bar, plus a nested subtab bar for tabs that have
    /// one), so the runtime doesn't need to know per-tab chrome shapes.
    pub fn update_viewport_heights(&mut self, terminal_height: u16) {
        use crate::tui::components::scores_tab::ScoresTabState;
        use crate::tui::components::settings_tab::SettingsTabState;
        use crate::tui::components::standings_tab::StandingsTabState;
        #[cfg(feature = "development")]
        use crate::tui::document_nav::DocumentNavState;

        Self::sync_viewport_height::<StandingsTabState>(
            &mut self.component_states,
            STANDINGS_TAB_PATH,
            terminal_height,
        );
        Self::sync_viewport_height::<ScoresTabState>(
            &mut self.component_states,
            SCORES_TAB_PATH,
            terminal_height,
        );
        Self::sync_viewport_height::<SettingsTabState>(
            &mut self.component_states,
            SETTINGS_TAB_PATH,
            terminal_height,
        );

        // DemoTab uses DocumentNavState directly as its state type (no
        // dedicated tab-state struct to hang a `chrome_lines()` override
        // off), so it falls back to `TabState`'s default (base chrome only).
        #[cfg(feature = "development")]
        Self::sync_viewport_height::<DocumentNavState>(
            &mut self.component_states,
            DEMO_TAB_PATH,
            terminal_height,
        );
    }

    /// Resize one tab's viewport to fit the terminal height, subtracting
    /// that tab's own declared chrome height.
    fn sync_viewport_height<S>(
        component_states: &mut ComponentStateStore,
        path: &str,
        terminal_height: u16,
    ) where
        S: TabState + 'static + Send + Sync,
    {
        let target_height = terminal_height.saturating_sub(S::chrome_lines());
        if let Some(state) = component_states.get_mut::<S>(path) {
            let doc_nav = state.doc_nav_mut();
            if doc_nav.viewport_height != target_height {
                doc_nav.viewport_height = target_height;
            }
        }
    }

    /// Execute effects asynchronously
    ///
    /// This runs in a separate tokio task and processes effects as they come in.
    /// Effects can dispatch new actions which feed back into the runtime.
    ///
    /// Note: FetchBoxscore, FetchTeamRosterStats, FetchPlayerStats, FetchGameDetails
    /// are handled synchronously by execute_effect() and should never reach here.
    /// They are converted to Effect::Async before being sent to this channel.
    async fn run_effect_executor(
        effect_rx: &mut mpsc::UnboundedReceiver<Effect>,
        action_tx: mpsc::UnboundedSender<Action>,
    ) {
        while let Some(effect) = effect_rx.recv().await {
            Self::process_effect_async(effect, &action_tx);
        }
    }

    /// Process a single effect in the async executor
    fn process_effect_async(effect: Effect, action_tx: &mpsc::UnboundedSender<Action>) {
        match effect {
            Effect::None | Effect::Handled => {
                // Nothing to do
            }
            Effect::Action(action) => {
                // Dispatch action immediately
                let _ = action_tx.send(action);
            }
            Effect::Batch(effects) => {
                // Process each effect in the batch
                for e in effects {
                    Self::process_effect_async(e, action_tx);
                }
            }
            Effect::Async(future) => {
                // Spawn async task to execute the future
                let action_tx = action_tx.clone();
                tokio::spawn(async move {
                    let action = future.await;
                    let _ = action_tx.send(action);
                });
            }
            // Fetch effects should never reach here - they're handled by execute_effect()
            // before being queued. Log a warning if they somehow slip through.
            Effect::FetchBoxscore(_)
            | Effect::FetchTeamRosterStats(_)
            | Effect::FetchPlayerStats(_)
            | Effect::FetchGameDetails(_) => {
                tracing::warn!(
                    "Fetch effect reached async executor - this should be handled by execute_effect()"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::keys::key_to_action;
    use crate::tui::testing::create_client;
    use crate::tui::types::Tab;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_test_data_effects() -> Arc<DataEffects> {
        let client = create_client();
        Arc::new(DataEffects::new(client))
    }

    fn create_test_runtime() -> Runtime {
        let client = create_client();
        let data_effects = Arc::new(DataEffects::new(client));
        Runtime::new(AppState::default(), data_effects)
    }

    #[tokio::test]
    async fn test_runtime_initial_state() {
        let state = AppState::default();
        let data_effects = create_test_data_effects();
        let runtime = Runtime::new(state.clone(), data_effects);

        assert_eq!(runtime.state().navigation.current_tab, Tab::Scores);
    }

    #[tokio::test]
    async fn test_dispatch_action() {
        let state = AppState::default();
        let data_effects = create_test_data_effects();
        let mut runtime = Runtime::new(state, data_effects);

        // Dispatch navigation action
        runtime.dispatch(Action::NavigateTab(Tab::Standings));

        assert_eq!(runtime.state().navigation.current_tab, Tab::Standings);
    }

    #[tokio::test]
    async fn test_refresh_schedule_updates_state_via_reducer() {
        use crate::commands::scores_format::PeriodScores;
        use nhl_api::GameDate;

        let mut state = AppState::default();
        state.data.period_scores = Arc::new(std::collections::HashMap::from([(
            1,
            PeriodScores {
                away_periods: vec![1, 0],
                home_periods: vec![0, 1],
                has_ot: false,
                has_so: false,
            },
        )]));
        let data_effects = create_test_data_effects();
        let mut runtime = Runtime::new(state, data_effects);

        let new_date = GameDate::today().add_days(1);
        runtime.dispatch(Action::RefreshSchedule(new_date.clone()));

        // The state transition (game_date switch + stale-data clear) must go through the
        // reducer, not be hand-mutated in Runtime::dispatch - this is what a plain state
        // inspection after dispatch verifies regardless of which layer did the mutation.
        assert_eq!(runtime.state().ui.scores.game_date, new_date);
        assert!(runtime.state().data.schedule.is_none());
        assert!(runtime.state().data.period_scores.is_empty());
    }

    #[tokio::test]
    async fn test_action_queue() {
        let state = AppState::default();
        let data_effects = create_test_data_effects();
        let mut runtime = Runtime::new(state, data_effects);

        // Send actions through the action channel
        let tx = runtime.action_sender();
        tx.send(Action::NavigateTab(Tab::Standings)).unwrap();

        // Process the queued actions
        let count = runtime.process_actions();

        assert_eq!(count, 1);
        assert_eq!(runtime.state().navigation.current_tab, Tab::Standings);
    }

    #[tokio::test]
    async fn test_effect_execution() {
        use std::sync::{Arc, Mutex};

        let state = AppState::default();
        let data_effects = create_test_data_effects();
        let mut runtime = Runtime::new(state, data_effects);

        // Create a flag to track if async effect executed
        let executed = Arc::new(Mutex::new(false));
        let executed_clone = executed.clone();

        // Create an async effect
        let effect = Effect::Async(Box::pin(async move {
            *executed_clone.lock().unwrap() = true;
            Action::NavigateTab(Tab::Settings)
        }));

        // Queue the effect
        runtime.effect_tx.send(effect).unwrap();

        // Give the async task time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Process any actions that resulted from the effect
        runtime.process_actions();

        // Verify the effect executed and dispatched the action
        assert!(*executed.lock().unwrap());
        assert_eq!(runtime.state().navigation.current_tab, Tab::Settings);
    }

    #[tokio::test]
    async fn test_build_returns_component_tree() {
        let state = AppState::default();
        let data_effects = create_test_data_effects();
        let mut runtime = Runtime::new(state, data_effects);

        // build() should return the App component tree
        let element = runtime.build();

        // Should be a container with 2 children (TabbedPanel, StatusBar)
        match element {
            Element::Container { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected container element from App component"),
        }
    }

    #[tokio::test]
    async fn test_refresh_data_triggers_data_effects() {
        let state = AppState::default();
        let data_effects = create_test_data_effects();
        let mut runtime = Runtime::new(state, data_effects);

        // Dispatch RefreshData action
        runtime.dispatch(Action::RefreshData);

        // Poll for actions with a timeout (network calls can be slow)
        let mut total_count = 0;
        let max_wait = tokio::time::Duration::from_secs(5);
        let poll_interval = tokio::time::Duration::from_millis(50);
        let start = tokio::time::Instant::now();

        while start.elapsed() < max_wait {
            tokio::time::sleep(poll_interval).await;
            let count = runtime.process_actions();
            total_count += count;

            // If we got at least one action, the test passed
            if total_count >= 1 {
                break;
            }
        }

        // Should have received at least StandingsLoaded and ScheduleLoaded actions
        // Note: actual count depends on network and what data is returned
        assert!(
            total_count >= 1,
            "Expected at least 1 action from data refresh after {} seconds",
            start.elapsed().as_secs_f32()
        );
    }

    /// Regression test for the "fake auto-refresh" bug: `Tick` used to only advance
    /// the animation frame and never re-triggered `RefreshData`, so live scores froze
    /// even though the status bar implied a refresh was imminent. This drives `Tick`
    /// through the real Runtime/reducer/effect pipeline with a mocked data provider
    /// (no real network, no real wall-clock sleeps) and asserts a refresh actually
    /// lands as new state - not just that a counter incremented.
    #[tokio::test]
    #[cfg(feature = "development")]
    async fn test_tick_triggers_real_refresh_once_interval_elapses() {
        use crate::dev::mock_client::MockClient;
        use std::time::{Duration, SystemTime};

        const REFRESH_INTERVAL_SECS: u32 = 30;

        let mut state = AppState::default();
        state.system.config.refresh_interval = REFRESH_INTERVAL_SECS;
        // Simulate that the refresh interval has already elapsed by backdating
        // last_refresh - no real sleep needed to observe the elapsed-time check.
        state.system.last_refresh =
            Some(SystemTime::now() - Duration::from_secs(u64::from(REFRESH_INTERVAL_SECS) + 1));

        let data_effects = Arc::new(DataEffects::new(Arc::new(MockClient::new())));
        let mut runtime = Runtime::new(state, data_effects);

        assert!(
            runtime.state().data.standings.is_none(),
            "Precondition: standings should not be loaded yet"
        );

        // Tick should detect the elapsed interval and queue Effect::Action(RefreshData),
        // which the background effect executor forwards back onto the action queue.
        runtime.dispatch(Action::Tick);

        // Drain the action queue, yielding so the effect executor and mock fetches
        // (which resolve instantly - no real I/O) get scheduled. Bounded iteration
        // count avoids hanging forever if this regresses; no wall-clock sleeps used.
        let mut total_actions = 0;
        for _ in 0..500 {
            total_actions += runtime.process_actions();
            if runtime.state().data.standings.is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }

        assert!(
            total_actions > 0,
            "Expected Tick to trigger at least one follow-up action via RefreshData"
        );
        assert!(
            runtime.state().data.standings.is_some(),
            "Expected the Tick-triggered auto-refresh to load standings from the mock provider"
        );
    }

    #[tokio::test]
    async fn test_tab_navigation_keys() {
        let runtime = create_test_runtime();
        let state = runtime.state();
        let component_states = runtime.component_states();

        // Test number keys - should work on any tab regardless of focus
        let key1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty());
        let action1 = key_to_action(key1, state, component_states);
        assert!(matches!(action1, Some(Action::NavigateTab(_))));

        // With tab bar focused (default), arrows should navigate tabs
        let key_right = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
        let action_right = key_to_action(key_right, state, component_states);
        assert!(matches!(action_right, Some(Action::NavigateTabRight)));

        let key_left = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
        let action_left = key_to_action(key_left, state, component_states);
        assert!(matches!(action_left, Some(Action::NavigateTabLeft)));
    }

    #[tokio::test]
    async fn test_quit_key() {
        let runtime = create_test_runtime();
        let state = runtime.state();
        let component_states = runtime.component_states();

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        let action = key_to_action(key, state, component_states);

        assert!(matches!(action, Some(Action::Quit)));
    }

    #[tokio::test]
    async fn test_focus_level_keys() {
        let mut runtime = create_test_runtime();
        let state = runtime.state();
        let component_states = runtime.component_states();

        // Start with tab bar focused - Down should enter content focus
        let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        let action_down = key_to_action(key_down, state, component_states);
        assert!(matches!(action_down, Some(Action::EnterContentFocus)));

        // After entering content focus, arrows should be context-sensitive
        runtime.dispatch(Action::EnterContentFocus);
        let state = runtime.state();
        let component_states = runtime.component_states();
        assert!(state.navigation.focus_in_content);

        // Now arrows should navigate dates on Scores tab (dispatches ComponentMessage)
        let key_right = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
        let action_right = key_to_action(key_right, state, component_states);
        assert!(matches!(
            action_right,
            Some(Action::ComponentMessage { .. })
        ));

        // Up should return to tab bar
        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let action_up = key_to_action(key_up, state, component_states);
        assert!(matches!(action_up, Some(Action::ExitContentFocus)));
    }

    #[tokio::test]
    async fn test_action_dispatching_navigation() {
        let mut runtime = create_test_runtime();

        // Dispatch a NavigateTabRight action
        runtime.dispatch(Action::NavigateTabRight);

        // State should have changed
        let state = runtime.state();
        assert_eq!(state.navigation.current_tab, crate::tui::Tab::Standings);
    }

    #[tokio::test]
    #[cfg(feature = "development")]
    async fn test_tab_cycling_with_demo() {
        let mut runtime = create_test_runtime();

        // Start on Scores, go right to Standings
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Standings
        );

        // Go right to Settings
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Settings
        );

        // Go right to Demo
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Demo
        );

        // Go right to wrap around to Scores
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Scores
        );
    }

    #[tokio::test]
    #[cfg(not(feature = "development"))]
    async fn test_tab_cycling() {
        let mut runtime = create_test_runtime();

        // Start on Scores, go right to Standings
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Standings
        );

        // Go right to Settings
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Settings
        );

        // Go right to wrap around to Scores
        runtime.dispatch(Action::NavigateTabRight);
        assert_eq!(
            runtime.state().navigation.current_tab,
            crate::tui::Tab::Scores
        );
    }

    /// Locks in the effective viewport height per tab after the F5 refactor
    /// that moved chrome-height ownership from `update_viewport_heights`
    /// onto `TabState::chrome_lines()`. Standings/Scores/Settings each have
    /// a nested subtab bar (6 total chrome lines); Demo has none (4 chrome
    /// lines, the `TabState` default). These numbers must not change as a
    /// side effect of that refactor.
    #[tokio::test]
    async fn test_update_viewport_heights_matches_pre_refactor_values() {
        use crate::tui::components::scores_tab::ScoresTabState;
        use crate::tui::components::settings_tab::SettingsTabState;
        use crate::tui::components::standings_tab::StandingsTabState;

        let mut runtime = create_test_runtime();
        runtime
            .component_states
            .insert(STANDINGS_TAB_PATH.to_string(), StandingsTabState::default());
        runtime
            .component_states
            .insert(SCORES_TAB_PATH.to_string(), ScoresTabState::default());
        runtime
            .component_states
            .insert(SETTINGS_TAB_PATH.to_string(), SettingsTabState::default());
        #[cfg(feature = "development")]
        {
            use crate::tui::document_nav::DocumentNavState;
            runtime
                .component_states
                .insert(DEMO_TAB_PATH.to_string(), DocumentNavState::default());
        }

        let terminal_height = 40;
        runtime.update_viewport_heights(terminal_height);

        assert_eq!(
            runtime
                .component_states
                .get_mut::<StandingsTabState>(STANDINGS_TAB_PATH)
                .unwrap()
                .doc_nav
                .viewport_height,
            34,
            "Standings: 40 - (4 base + 2 subtab) chrome lines"
        );
        assert_eq!(
            runtime
                .component_states
                .get_mut::<ScoresTabState>(SCORES_TAB_PATH)
                .unwrap()
                .doc_nav
                .viewport_height,
            34,
            "Scores: 40 - (4 base + 2 subtab) chrome lines"
        );
        assert_eq!(
            runtime
                .component_states
                .get_mut::<SettingsTabState>(SETTINGS_TAB_PATH)
                .unwrap()
                .doc_nav
                .viewport_height,
            34,
            "Settings: 40 - (4 base + 2 subtab) chrome lines"
        );
        #[cfg(feature = "development")]
        {
            use crate::tui::document_nav::DocumentNavState;
            assert_eq!(
                runtime
                    .component_states
                    .get_mut::<DocumentNavState>(DEMO_TAB_PATH)
                    .unwrap()
                    .viewport_height,
                36,
                "Demo: 40 - 4 base chrome lines (no subtab bar)"
            );
        }
    }
}
