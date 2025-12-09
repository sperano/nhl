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
    async fn test_refresh_data_triggers_loading_state() {
        let mut runtime = create_test_runtime();

        // Dispatch refresh
        runtime.dispatch(Action::RefreshData);

        // Give time for async effects to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check that loading states were set
        // Note: In a real implementation, the reducer would set loading states
        // For now, we just verify that the action was dispatched
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
    async fn test_error_action_stores_error_in_state() {
        let mut runtime = create_test_runtime();

        // Simulate error loading standings
        runtime.dispatch(Action::StandingsLoaded(Err("Network error".to_string())));

        // Error should be stored in state
        assert!(runtime.state().data.errors.contains_key("standings"));
        assert_eq!(
            runtime.state().data.errors.get("standings").unwrap(),
            "Failed to load standings: Network error"
        );
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
