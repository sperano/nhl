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
            Effect::FetchTeamRosterStats {
                abbrev,
                season,
                fetch_seasons,
            } => {
                debug!(
                    "EFFECT: Executing team roster stats fetch for team={} season={:?}",
                    abbrev, season
                );
                let fetch_effect =
                    self.data_effects
                        .fetch_team_roster_stats(abbrev, season, fetch_seasons);
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
            | Effect::FetchTeamRosterStats { .. }
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
#[path = "runtime_tests.rs"]
mod tests;
