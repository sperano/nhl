use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::tui::component::Component;

/// Stores component states by path for lifecycle management
///
/// The ComponentStateStore maintains component state instances across renders,
/// enabling the React-like component lifecycle. Each component instance is
/// identified by a unique path (e.g., "app/scores_tab") and stores its state
/// with type safety via TypeId.
pub struct ComponentStateStore {
    states: HashMap<String, (TypeId, Box<dyn Any + Send + Sync>)>,
}

impl ComponentStateStore {
    /// Create a new empty component state store
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Get or initialize component state
    ///
    /// If state for this component path doesn't exist, calls `C::init(props)`
    /// to create it. Returns an immutable reference to the state.
    ///
    /// # Panics
    ///
    /// Panics if the stored state type doesn't match the requested type.
    pub fn get_or_init<C: Component>(&mut self, path: &str, props: &C::Props) -> &C::State {
        let type_id = TypeId::of::<C::State>();

        self.states
            .entry(path.to_string())
            .or_insert_with(|| (type_id, Box::new(C::init(props))))
            .1
            .downcast_ref::<C::State>()
            .expect("State type mismatch")
    }

    /// Get immutable state
    ///
    /// Returns None if no state exists for this path, or Some(&S) if found.
    ///
    /// # Panics
    ///
    /// Panics if the stored state type doesn't match the requested type.
    pub fn get<S: 'static + Send + Sync>(&self, path: &str) -> Option<&S> {
        self.states
            .get(path)
            .map(|(_, state)| state.downcast_ref().expect("State type mismatch"))
    }

    /// Get mutable state for update
    ///
    /// Returns None if no state exists for this path, or Some(&mut S) if found.
    ///
    /// # Panics
    ///
    /// Panics if the stored state type doesn't match the requested type.
    pub fn get_mut<S: 'static + Send + Sync>(&mut self, path: &str) -> Option<&mut S> {
        self.states
            .get_mut(path)
            .map(|(_, state)| state.downcast_mut().expect("State type mismatch"))
    }

    /// Get mutable state as Any for dynamic dispatch
    ///
    /// Used when the concrete type is not known at compile time.
    pub fn get_mut_any(&mut self, path: &str) -> Option<&mut (dyn Any + Send + Sync)> {
        self.states.get_mut(path).map(|(_, state)| &mut **state)
    }

    /// Insert state for a component path
    ///
    /// Used for testing or direct state initialization.
    pub fn insert<S: 'static + Send + Sync>(&mut self, path: String, state: S) {
        let type_id = TypeId::of::<S>();
        self.states.insert(path, (type_id, Box::new(state)));
    }

    /// Remove state for a component path
    ///
    /// Used for cleanup when a component is unmounted.
    pub fn remove(&mut self, path: &str) -> bool {
        self.states.remove(path).is_some()
    }

    /// Clear all component states
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Get the number of stored component states
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

impl Default for ComponentStateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::component::{Component, Element};

    // Test component with simple state
    #[derive(Clone)]
    struct TestProps {
        value: i32,
    }

    #[derive(Clone, Default)]
    struct TestState {
        counter: i32,
    }

    struct TestComponent;

    impl Component for TestComponent {
        type Props = TestProps;
        type State = TestState;
        type Message = ();

        fn init(props: &Self::Props) -> Self::State {
            TestState {
                counter: props.value * 2,
            }
        }

        fn view(&self, _props: &Self::Props, _state: &Self::State) -> Element {
            Element::None
        }
    }

    #[test]
    fn test_component_store_init_creates_default_state() {
        let mut store = ComponentStateStore::new();
        let props = TestProps { value: 5 };

        let state = store.get_or_init::<TestComponent>("test", &props);
        assert_eq!(state.counter, 10);
    }

    #[test]
    fn test_component_store_get_returns_same_instance() {
        let mut store = ComponentStateStore::new();
        let props = TestProps { value: 5 };

        let state1 = store.get_or_init::<TestComponent>("test", &props);
        let counter1 = state1.counter;

        let state2 = store.get_or_init::<TestComponent>("test", &props);
        let counter2 = state2.counter;

        assert_eq!(counter1, counter2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_component_store_get_mut_allows_modification() {
        let mut store = ComponentStateStore::new();
        let props = TestProps { value: 5 };

        // Initialize
        store.get_or_init::<TestComponent>("test", &props);

        // Modify
        {
            let state = store.get_mut::<TestState>("test").unwrap();
            state.counter = 42;
        }

        // Verify
        let state = store.get_or_init::<TestComponent>("test", &props);
        assert_eq!(state.counter, 42);
    }

    #[test]
    fn test_component_store_multiple_paths() {
        let mut store = ComponentStateStore::new();
        let props = TestProps { value: 5 };

        store.get_or_init::<TestComponent>("path1", &props);
        store.get_or_init::<TestComponent>("path2", &props);

        assert_eq!(store.len(), 2);

        // Modify one path
        store.get_mut::<TestState>("path1").unwrap().counter = 100;

        // Verify independence
        let state1 = store.get_or_init::<TestComponent>("path1", &props);
        assert_eq!(state1.counter, 100);

        let state2 = store.get_or_init::<TestComponent>("path2", &props);
        assert_eq!(state2.counter, 10);
    }

    #[test]
    fn test_component_store_remove() {
        let mut store = ComponentStateStore::new();
        let props = TestProps { value: 5 };

        store.get_or_init::<TestComponent>("test", &props);
        assert_eq!(store.len(), 1);

        let removed = store.remove("test");
        assert!(removed);
        assert_eq!(store.len(), 0);

        let removed_again = store.remove("test");
        assert!(!removed_again);
    }

    #[test]
    fn test_component_store_clear() {
        let mut store = ComponentStateStore::new();
        let props = TestProps { value: 5 };

        store.get_or_init::<TestComponent>("path1", &props);
        store.get_or_init::<TestComponent>("path2", &props);
        assert_eq!(store.len(), 2);

        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_component_store_get_mut_none_for_missing_path() {
        let mut store = ComponentStateStore::new();

        let result = store.get_mut::<TestState>("nonexistent");
        assert!(result.is_none());
    }
}
