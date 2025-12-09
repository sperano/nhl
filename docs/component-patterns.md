# Component Patterns

## Pattern 1: Simple Component (No Document)

```rust
// 1. Define Props
#[derive(Clone)]
pub struct MyComponentProps {
    pub data: Arc<Vec<MyData>>,
}

// 2. Define State
#[derive(Debug, Clone, Default)]
pub struct MyComponentState {
    pub selected_index: usize,
    pub mode: MyMode,
}

// 3. Define Messages
#[derive(Debug, Clone, PartialEq)]
pub enum MyComponentMsg {
    SelectNext,
    SelectPrev,
    ToggleMode,
}

// 4. Implement Component
impl Component for MyComponent {
    type Props = MyComponentProps;
    type State = MyComponentState;
    type Message = MyComponentMsg;

    fn init(_props: &Self::Props) -> Self::State {
        Self::State::default()
    }

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            MyComponentMsg::SelectNext => {
                state.selected_index = state.selected_index.saturating_add(1);
                Effect::None
            }
            MyComponentMsg::SelectPrev => {
                state.selected_index = state.selected_index.saturating_sub(1);
                Effect::None
            }
            MyComponentMsg::ToggleMode => {
                state.mode = !state.mode;
                Effect::None
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        vertical(vec![
            // ... elements
        ])
    }
}

// 5. Implement ComponentMessageTrait
impl ComponentMessageTrait for MyComponentMsg {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// 6. Register in Runtime
runtime.register_component("my_component", MyComponent, props)?;
```

## Pattern 2: Document-Based Component

```rust
// 1. Define State with DocumentNavState
#[derive(Debug, Clone, Default)]
pub struct MyDocTabState {
    pub doc_nav: DocumentNavState,  // ← Embed generic state
    // ... other state
}

// 2. Define Messages wrapping DocumentNavMsg
#[derive(Debug, Clone, PartialEq)]
pub enum MyDocTabMsg {
    DocNav(DocumentNavMsg),  // ← Wrap generic messages
    UpdateViewportHeight(u16),
    // ... other messages
}

// 3. Implement Component with delegation
impl Component for MyDocTab {
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            MyDocTabMsg::DocNav(nav_msg) => {
                document_nav::handle_message(&nav_msg, &mut state.doc_nav)
                    .unwrap_or(Effect::None)
            }
            MyDocTabMsg::UpdateViewportHeight(h) => {
                state.doc_nav.viewport_height = h;
                Effect::None
            }
            // ... other messages
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        let doc = MyDocument::new(props.data.clone());
        let doc_view = DocumentView {
            document: Arc::new(doc),
            viewport: Viewport {
                offset: state.doc_nav.scroll_offset,
                height: state.doc_nav.viewport_height,
            },
            focus: FocusManager {
                focus_index: state.doc_nav.focus_index,
                // ...
            },
        };
        Element::Widget(Box::new(doc_view))
    }
}

// 4. Update focusable metadata on data load
let doc = MyDocument::new(data);
runtime.update_component_state("my_doc_tab", |state: &mut MyDocTabState| {
    state.doc_nav.focusable_positions = doc.focusable_positions();
    state.doc_nav.focusable_ids = doc.focusable_ids();
    state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
})?;
```

## State Ownership Rules

### Use Component State for:
- UI state (selected indices, scroll positions, focus)
- View modes (browse mode, edit mode, filter state)
- Component-specific navigation state
- Temporary state (modal open, dropdown expanded)

### Use Global State for:
- API data (standings, schedules, games, boxscores)
- Shared navigation (current tab, document stack)
- Configuration (user settings, display config)
- System state (status messages, last refresh time)
- Data effect triggers (game_date for schedule refreshes)

**Rule of Thumb**: If multiple components need to read it, it's global. If only one component uses it, it's component state.

## Common Patterns

### Data Loading Updates Component State

```rust
// In data_loading reducer:
StandingsLoaded(Ok(standings)) => {
    state.data.standings = Some(Arc::new(standings.clone()));

    let doc = create_standings_document(&standings);
    runtime.update_component_state("standings_tab", |state: &mut StandingsTabState| {
        state.doc_nav.focusable_positions = doc.focusable_positions();
        state.doc_nav.focusable_ids = doc.focusable_ids();
        state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
    })?;

    Some((state, Effect::None))
}
```

### Reading Component State in Keys

```rust
// In keys.rs:
fn is_browse_mode_active(state: &AppState, runtime: &Runtime) -> bool {
    runtime.with_component_state("scores_tab", |state: &ScoresTabState| {
        state.browse_mode
    }).unwrap_or(false)
}
```

### Component Returns Effect with Action

```rust
impl Component for MyComponent {
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            MyComponentMsg::LoadData(id) => {
                state.loading = true;
                Effect::Action(Action::FetchMyData(id))
            }
        }
    }
}
```

### Forwarding Reducer

```rust
pub fn reduce_my_tab(
    state: AppState,
    action: MyTabAction,
    runtime: &mut Runtime,
) -> Option<(AppState, Effect)> {
    let msg = match action {
        MyTabAction::SelectNext => MyTabMsg::SelectNext,
        MyTabAction::SelectPrev => MyTabMsg::SelectPrev,
    };

    let effect = runtime
        .dispatch_component_message("my_tab", msg)
        .unwrap_or(Effect::None);

    Some((state, effect))
}
```

## Core Principles

1. **Component State is Source of Truth** - Never sync to global state
2. **Messages are the API** - Components communicate via messages
3. **Generic Over Specific** - Extract shared patterns into reusable modules
4. **Reducers Should Be Simple** - Route to components, don't hold business logic
5. **Avoid Infinite Loops** - Never dispatch actions from render loop
6. **Embedded Structs for Shared Behavior** - Rust idiom over trait inheritance

## Testing Patterns

### Unit Test Components

```rust
#[test]
fn test_component_message_handling() {
    let mut component = MyComponent;
    let mut state = MyComponentState::default();

    let effect = component.update(MyComponentMsg::SelectNext, &mut state);

    assert_eq!(state.selected_index, 1);
    assert!(matches!(effect, Effect::None));
}
```

### Test Rendering with assert_buffer

```rust
#[test]
fn test_component_rendering() {
    setup_test_render!(buf, state, config, 80, 20);

    let props = MyComponentProps { /* ... */ };
    let component_state = MyComponentState { /* ... */ };
    let component = MyComponent;

    let element = component.view(&props, &component_state);
    renderer::render(&element, buf.area(), &mut buf, &config);

    assert_buffer(&buf, vec![
        "Expected line 1",
        "Expected line 2",
    ]);
}
```

## Migration Checklist

When migrating a component to the new architecture:

- [ ] Define component state struct (no global state duplication)
- [ ] Define message enum for component
- [ ] Implement `Component` trait with `update()` handling messages
- [ ] Implement `ComponentMessageTrait` for messages
- [ ] Register component in Runtime
- [ ] Update key_to_action to dispatch ComponentMessage
- [ ] Update reducers to forward actions as messages (if needed)
- [ ] Update data loading to update component state
- [ ] Remove old global state fields
- [ ] Remove old action variants
- [ ] Write unit tests for message handling
- [ ] Write integration tests for key flow
