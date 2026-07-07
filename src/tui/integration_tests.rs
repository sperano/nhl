//! Integration tests for the entire data flow
//!
//! These tests verify that data flows correctly through the system:
//! API → Effect → Action → Reducer → State → Component → Render

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::tui::testing::create_client;
    use crate::tui::{
        action::Action, effects::DataEffects, runtime::Runtime, state::AppState, Tab,
    };

    fn create_test_runtime() -> Runtime {
        let client = create_client();
        let data_effects = Arc::new(DataEffects::new(client));
        let state = AppState::default();
        Runtime::new(state, data_effects)
    }

    #[tokio::test]
    async fn test_data_loaded_action_updates_state() {
        let mut runtime = create_test_runtime();

        // Initial state should have no standings
        assert!(runtime.state().data.standings.is_none());

        // Simulate standings loaded
        let standings = vec![]; // Empty standings for test
        runtime.dispatch(Action::StandingsLoaded(Ok(standings.clone())));

        // State should now have standings
        assert!(runtime.state().data.standings.as_ref().is_some());
        assert_eq!(
            runtime
                .state()
                .data
                .standings
                .as_ref()
                .as_ref()
                .unwrap()
                .len(),
            0
        );
    }

    #[tokio::test]
    async fn test_error_action_surfaces_status_bar_error() {
        let mut runtime = create_test_runtime();

        // Simulate error loading standings
        runtime.dispatch(Action::StandingsLoaded(Err(std::sync::Arc::new(
            nhl_api::NHLApiError::Other("Network error".to_string()),
        ))));

        // The failure must surface as a status-bar message - the only channel
        // the UI actually renders (see AppState.data.errors, which was write-only
        // dead weight and has since been removed).
        assert!(runtime.state().system.status_is_error);
        assert_eq!(
            runtime.state().system.status_message,
            Some("Failed to load standings: Network error".to_string())
        );

        // A subsequent successful load must clear the error.
        runtime.dispatch(Action::StandingsLoaded(Ok(vec![])));

        assert!(!runtime.state().system.status_is_error);
    }

    #[tokio::test]
    async fn test_action_queue_processing() {
        let mut runtime = create_test_runtime();

        // Queue multiple actions
        let tx = runtime.action_sender();
        tx.send(Action::NavigateTab(Tab::Standings)).unwrap();
        tx.send(Action::NavigateTab(Tab::Settings)).unwrap();

        // Process all queued actions
        let count = runtime.process_actions();

        assert_eq!(count, 2);
        assert_eq!(runtime.state().navigation.current_tab, Tab::Settings);
    }
}
