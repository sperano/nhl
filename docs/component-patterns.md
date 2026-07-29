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
}

// 3. Define Messages
#[derive(Debug, Clone)]
pub enum MyComponentMsg {
    SelectNext,
    SelectPrev,
}

// 4. Implement Component
#[derive(Default)]
pub struct MyComponent;

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
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        vertical([Constraint::Min(0)], vec![/* ... */])
    }
}

// 5. Implement ComponentMessageTrait via the macro (see src/tui/tab_component.rs)
// This expands to a downcast + Component::update call - see the macro
// definition for the exact generated code.
component_message_impl!(MyComponentMsg, MyComponent, MyComponentState);
```

There is no separate "register with Runtime" step. A component's state is
created lazily the first time it's asked for. Add a path constant to
`src/tui/constants.rs` (e.g. `pub const MY_COMPONENT_PATH: &str =
"app/my_component";`), then wire it into `App::build_with_component_states`
(`src/tui/components/app.rs`):

```rust
let my_state = component_states.get_or_init::<MyComponent>(MY_COMPONENT_PATH, &props);
MyComponent.view(&props, my_state)
```

`get_or_init` calls `MyComponent::init(&props)` the first time it's called
for that path and returns the same stored `&State` on every subsequent
call. Key handling that needs to dispatch into this component's `update()`
produces `Action::ComponentMessage { path: MY_COMPONENT_PATH.into(), message:
Box::new(MyComponentMsg::SelectNext) }`; `reduce()` looks the path up and
calls `message.apply(state)`.

## Pattern 2: Tab Component with a Document

Every tab component (`ScoresTab`, `StandingsTab`, `SettingsTab`) follows
this shape. It embeds `DocumentNavState` for scroll/focus, implements
`TabState`/`TabMessage` (`src/tui/tab_component.rs`) to pick up the three
messages every tab shares (`DocNav`, `UpdateViewportHeight`, `NavigateUp`),
and constructs a `DocumentView` at render time inside an `ElementWidget`.

```rust
// 1. State embeds DocumentNavState plus whatever else the tab needs
#[derive(Clone, Debug, Default)]
pub struct MyTabState {
    pub doc_nav: DocumentNavState,
}

impl TabState for MyTabState {
    fn doc_nav(&self) -> &DocumentNavState { &self.doc_nav }
    fn doc_nav_mut(&mut self) -> &mut DocumentNavState { &mut self.doc_nav }
}

// 2. Messages: tab-specific variants + the three common ones, wrapped
#[derive(Clone, Debug)]
pub enum MyTabMsg {
    NavigateUp,
    DocNav(DocumentNavMsg),
    UpdateViewportHeight(u16),
    ActivateItem, // tab-specific
}

impl TabMessage for MyTabMsg {
    fn as_common(&self) -> Option<CommonTabMessage<'_>> {
        match self {
            Self::DocNav(msg) => Some(CommonTabMessage::DocNav(msg)),
            Self::UpdateViewportHeight(h) => Some(CommonTabMessage::UpdateViewportHeight(*h)),
            Self::NavigateUp => Some(CommonTabMessage::NavigateUp),
            _ => None,
        }
    }

    fn from_doc_nav(msg: DocumentNavMsg) -> Self {
        Self::DocNav(msg)
    }
}

component_message_impl!(MyTabMsg, MyTab, MyTabState);

// 3. Component::update() handles common messages first, then its own
impl Component for MyTab {
    type Props = MyTabProps;
    type State = MyTabState;
    type Message = MyTabMsg;

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        if let Some(effect) = handle_common_message(msg.as_common(), state) {
            return effect;
        }
        match msg {
            MyTabMsg::ActivateItem => {
                // e.g. push a stacked document
                Effect::Action(Action::PushDocument(StackedDocument::TeamDetail {
                    abbrev: "BOS".to_string(),
                }))
            }
            MyTabMsg::DocNav(_) | MyTabMsg::UpdateViewportHeight(_) | MyTabMsg::NavigateUp => {
                unreachable!("handled by handle_common_message")
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        Element::Widget(Box::new(MyDocumentWidget {
            data: props.data.clone(),
            focused_id: state.doc_nav.focused_id(),
            scroll_offset: state.doc_nav.scroll_offset,
            focused: props.focused && state.has_item_focus(),
        }))
    }
}

// 4. The ElementWidget builds the Document and DocumentView at render time,
// since the viewport width/height aren't known until then.
struct MyDocumentWidget {
    data: Arc<MyData>,
    focused_id: Option<FocusableId>,
    scroll_offset: u16,
    focused: bool,
}

impl ElementWidget for MyDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        let doc = MyDocument::new(self.data.clone());
        let mut view = DocumentView::new(Arc::new(doc), area.height);
        if let Some(id) = self.focused_id.clone() {
            view.focus_id(id);
        }
        view.set_scroll_offset(self.scroll_offset);
        let child_ctx = RenderContext::new(ctx.config, self.focused);
        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(MyDocumentWidget {
            data: self.data.clone(),
            focused_id: self.focused_id.clone(),
            scroll_offset: self.scroll_offset,
            focused: self.focused,
        })
    }
}
```

When a document's shape depends on config (e.g. the standings documents,
`SettingsDocument`), its constructor takes `config: impl Into<Arc<Config>>`
so callers can pass either an owned `Config` or an already-`Arc`'d one
without an extra clone — see `src/tui/components/standings_documents/*.rs`
and `src/tui/components/settings_document.rs`. Component props that flow
into render generally carry `Arc<Config>` directly (e.g.
`StandingsTabProps::config`, `SettingsTabProps::config`), constructed once
per frame in `App::build_with_component_states` via
`Arc::new(state.system.config.clone())` so every downstream `.clone()` is a
pointer bump rather than a deep copy.

### Rebuilding focusable metadata after data changes

Document focusable metadata (`focusable_positions`, `focusable_ids`,
`focusable_row_positions`, `link_targets`) lives in the tab's
`DocumentNavState`, not in `AppState`, so it has to be rebuilt whenever the
underlying data or the active view changes. This happens directly against
the `ComponentStateStore`, from a reducer — there is no
`runtime.update_component_state()` helper:

```rust
// From reduce_data_loading::handle_schedule_loaded (src/tui/reducers/data_loading.rs)
if let Some(scores_state) = component_states.get_mut::<ScoresTabState>(SCORES_TAB_PATH) {
    let doc = ScoreBoxesDocument::new(/* ... */);
    scores_state.doc_nav.focusable_positions = doc.focusable_positions();
    scores_state.doc_nav.focusable_heights = doc.focusable_heights();
    scores_state.doc_nav.focusable_ids = doc.focusable_ids();
    scores_state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
}
```

`src/tui/reducers/standings.rs::rebuild_standings_focusable_metadata` is the
fuller example: it reads `StandingsTabState.view` to pick which of the four
standings documents to build, then writes the resulting metadata back with
`component_states.get_mut::<StandingsTabState>(...)`.

### Reading component state from `keys.rs`

Key handling often needs to know a component's local state (e.g. "is an
item currently focused inside this tab?") before deciding what a keypress
means. This reads directly from the store too:

```rust
// src/tui/keys.rs
fn has_scores_item_focus(component_states: &ComponentStateStore) -> bool {
    component_states
        .get::<ScoresTabState>(SCORES_TAB_PATH)
        .map(|s| s.has_item_focus())
        .unwrap_or(false)
}
```

### Component returns an `Effect::Action` to trigger global state changes

```rust
impl Component for ScoresTab {
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            ScoresTabMsg::NavigateLeft => {
                state.game_date = state.game_date.add_days(-1);
                // Component can't fetch data itself - it asks the reducer/Runtime
                // to do it by returning an Effect::Action.
                Effect::Action(Action::RefreshSchedule(state.game_date.clone()))
            }
            // ...
        }
    }
}
```

### There is no forwarding-reducer pattern anymore

Earlier versions of this codebase had per-tab global actions
(`ScoresAction`, `StandingsAction`) with a small "forwarding reducer" that
translated them into component messages. That machinery is gone. All
in-tab key handling now produces `Action::ComponentMessage` directly from
`key_to_action()`, and `reduce()` dispatches it centrally via
`ComponentMessageTrait::apply` (see `docs/architecture.md`). Reducers only
exist for actions that mutate global `AppState` directly: navigation,
document-stack, data-loading, settings, and the standings-focusable-rebuild
helper.

## State Ownership Rules

### Use Component State for:
- UI state (selected indices, scroll positions, focus)
- View modes (e.g. `StandingsTabState.view: GroupBy`)
- Component-specific navigation state (`DocumentNavState`)
- Temporary state (modal open — `SettingsTabState.modal: Option<ModalState>`)

### Use Global State for:
- API data (standings, schedules, games, boxscores) — `AppState::data`
- Shared navigation (current tab, document stack) — `AppState::navigation`
- Configuration (user settings, display config) — `AppState::system.config`
- System state (status messages, last refresh time) — `AppState::system`
- Data effect triggers (`game_date` for schedule refreshes) — `AppState::ui`

**Rule of Thumb**: If multiple components need to read it, it's global. If only one component uses it, it's component state.

## Core Principles

1. **Component State is Source of Truth** - Never sync it back into global state.
2. **Messages are the API** - Components communicate via messages, dispatched
   as `Action::ComponentMessage` and applied through `ComponentMessageTrait`.
3. **Shared tab behavior lives in `tab_component.rs`** - `TabState`/
   `TabMessage`/`handle_common_message`/`component_message_impl!` remove the
   boilerplate every tab would otherwise repeat for scroll/focus handling.
4. **Reducers Should Be Simple** - Only the actions that touch global
   `AppState` get a reducer; everything else is a component message.
5. **Avoid Infinite Loops** - Never dispatch actions from the render/`view()`
   path.
6. **Embedded Structs for Shared Behavior** - `DocumentNavState` is embedded
   by value, not inherited via a trait hierarchy.

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

### Test Rendering with `assert_buffer`

```rust
use crate::tui::renderer::Renderer;
use crate::tui::testing::assert_buffer;
use crate::config::RenderContext;

#[test]
fn test_component_rendering() {
    let (_, config, area, mut buf) = setup_test_render!(80, 20);

    let props = MyComponentProps { /* ... */ };
    let component_state = MyComponentState { /* ... */ };
    let component = MyComponent;

    let element = component.view(&props, &component_state);
    let ctx = RenderContext::focused(&config);
    Renderer::new().render(element, area, &mut buf, &ctx);

    assert_buffer(&buf, &[
        "Expected line 1",
        "Expected line 2",
    ]);
}
```

`setup_test_render!` (`src/tui/testing.rs`) returns `(AppState, DisplayConfig,
Rect, Buffer)`; `Renderer::render` takes an owned `Element` (not `&Element`)
and a `&RenderContext` (not a bare `&DisplayConfig`).

## Migration Checklist

When adding a new tab-style component:

- [ ] Define component state struct embedding `DocumentNavState` (no global state duplication)
- [ ] Implement `TabState` for the state struct
- [ ] Define a message enum with tab-specific variants + `DocNav`/`UpdateViewportHeight`/`NavigateUp`
- [ ] Implement `TabMessage` for the message enum
- [ ] Use `handle_common_message()` at the top of `Component::update()`
- [ ] Call `component_message_impl!(MsgType, ComponentType, StateType)` instead of hand-writing `ComponentMessageTrait`
- [ ] Add a path constant in `src/tui/constants.rs`
- [ ] Wire the component into `App::build_with_component_states` via `component_states.get_or_init::<T>(PATH, &props)`
- [ ] Update `key_to_action` to dispatch `Action::ComponentMessage { path: PATH.into(), message: Box::new(...) }`
- [ ] If the component's document content depends on data that loads asynchronously, rebuild its focusable metadata from the relevant reducer (`reduce_data_loading`, or a dedicated helper like `rebuild_standings_focusable_metadata`)
- [ ] Add the component's state type to `Runtime::update_viewport_heights` if it has subtab chrome
- [ ] Write unit tests for message handling and an `assert_buffer` rendering test
