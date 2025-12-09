use ratatui::{buffer::Buffer, layout::Rect};
use std::future::Future;
use std::pin::Pin;

use super::action::Action;
use crate::config::DisplayConfig;

/// Core component trait - like React.Component
///
/// Components are the building blocks of the UI. Each component:
/// - Has Props (input data, like React props)
/// - Has State (internal state, like useState)
/// - Has Messages (internal events, like setState callbacks)
/// - Renders to an Element tree (virtual DOM)
pub trait Component: Send {
    /// Props type for this component
    type Props: Clone;

    /// Local state type (if any)
    type State: Default + Clone + Send + Sync + 'static;

    /// Message type for internal events
    type Message;

    /// Create initial state from props (like useState)
    fn init(_props: &Self::Props) -> Self::State {
        Self::State::default()
    }

    /// Update state based on message (like reducer)
    fn update(&mut self, _msg: Self::Message, _state: &mut Self::State) -> Effect {
        Effect::None
    }

    /// Render component given props and state (pure function)
    fn view(&self, props: &Self::Props, state: &Self::State) -> Element;

    /// Should this component re-render? (like React.shouldComponentUpdate)
    ///
    /// Override this to implement memoization. Default is to always re-render.
    /// Return false to skip re-rendering when props haven't meaningfully changed.
    fn should_update(&self, _old_props: &Self::Props, _new_props: &Self::Props) -> bool {
        true // Default: always update
    }

    /// Lifecycle: called when props change
    fn did_update(&mut self, _old_props: &Self::Props, _new_props: &Self::Props) -> Effect {
        Effect::None
    }
}

/// Element in virtual component tree
#[derive(Clone)]
pub enum Element {
    /// A component that needs to be rendered
    Component(Box<dyn ComponentWrapper>),

    /// A widget that can be directly rendered to ratatui buffer
    Widget(Box<dyn ElementWidget>),

    /// A container with layout and children
    Container {
        children: Vec<Element>,
        layout: ContainerLayout,
    },

    /// A fragment (just groups children, no layout)
    Fragment(Vec<Element>),

    /// An overlay that renders on top of base content (for modals, popups, etc.)
    Overlay {
        base: Box<Element>,
        overlay: Box<Element>,
    },

    /// Nothing to render
    None,
}

/// Layout for container elements
#[derive(Clone)]
pub enum ContainerLayout {
    Vertical(Vec<Constraint>),
    Horizontal(Vec<Constraint>),
}

/// Constraint for layout
#[derive(Clone, Copy)]
pub enum Constraint {
    Length(u16),
    Min(u16),
    Max(u16),
    Percentage(u16),
    Ratio(u32, u32),
}

/// Side effects to run after rendering
#[allow(clippy::large_enum_variant)]
pub enum Effect {
    None,
    /// Indicates the component handled the input and no further processing needed
    /// Used by NavigateUp to signal that browse mode was exited, preventing bubble up
    Handled,
    Action(Action),
    Batch(Vec<Effect>),
    Async(Pin<Box<dyn Future<Output = Action> + Send>>),
    // Data fetch effects - returned by reducers to trigger async fetches
    /// Fetch boxscore data for a game
    FetchBoxscore(i64),
    /// Fetch team roster/stats for a team
    FetchTeamRosterStats(String),
    /// Fetch player stats
    FetchPlayerStats(i64),
    /// Fetch game details (period scores, etc.)
    FetchGameDetails(i64),
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::None => write!(f, "Effect::None"),
            Effect::Handled => write!(f, "Effect::Handled"),
            Effect::Action(action) => f.debug_tuple("Effect::Action").field(action).finish(),
            Effect::Batch(effects) => f.debug_tuple("Effect::Batch").field(effects).finish(),
            Effect::Async(_) => write!(f, "Effect::Async(<future>)"),
            Effect::FetchBoxscore(id) => f.debug_tuple("Effect::FetchBoxscore").field(id).finish(),
            Effect::FetchTeamRosterStats(abbrev) => f
                .debug_tuple("Effect::FetchTeamRosterStats")
                .field(abbrev)
                .finish(),
            Effect::FetchPlayerStats(id) => {
                f.debug_tuple("Effect::FetchPlayerStats").field(id).finish()
            }
            Effect::FetchGameDetails(id) => {
                f.debug_tuple("Effect::FetchGameDetails").field(id).finish()
            }
        }
    }
}

/// Trait for widgets that can be wrapped in the Element tree
///
/// This is distinct from `widgets::StandaloneWidget` which is a simpler trait
/// for standalone widgets. ElementWidget adds `Send + Sync` bounds and `clone_box()`
/// for use with the component framework's `Element::Widget` variant.
pub trait ElementWidget: Send + Sync {
    /// Render this widget into the provided buffer
    ///
    /// # Arguments
    ///
    /// * `area` - The rectangular area to render into
    /// * `buf` - The buffer to write to
    /// * `config` - Display configuration (colors, box chars, etc.)
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig);

    /// Clone this widget into a boxed trait object
    fn clone_box(&self) -> Box<dyn ElementWidget>;

    /// Get the preferred height of this widget
    ///
    /// Returns None if the widget can adapt to any height.
    /// Returns Some(height) if the widget has a fixed or preferred height.
    fn preferred_height(&self) -> Option<u16> {
        None
    }

    /// Get the preferred width of this widget
    ///
    /// Returns None if the widget can adapt to any width.
    /// Returns Some(width) if the widget has a fixed or preferred width.
    fn preferred_width(&self) -> Option<u16> {
        None
    }
}

impl Clone for Box<dyn ElementWidget> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Type-erased component wrapper for dynamic dispatch
pub trait ComponentWrapper: Send + Sync {
    fn view_any(&self) -> Element;
    fn clone_box(&self) -> Box<dyn ComponentWrapper>;
}

impl Clone for Box<dyn ComponentWrapper> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Helper to create a container with vertical layout
pub fn vertical<const N: usize>(constraints: [Constraint; N], children: Vec<Element>) -> Element {
    Element::Container {
        children,
        layout: ContainerLayout::Vertical(constraints.to_vec()),
    }
}

/// Helper to create a container with horizontal layout
pub fn horizontal<const N: usize>(constraints: [Constraint; N], children: Vec<Element>) -> Element {
    Element::Container {
        children,
        layout: ContainerLayout::Horizontal(constraints.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test component that uses all default trait methods
    struct TestComponent;

    #[derive(Clone)]
    struct TestProps;

    #[derive(Default, Clone)]
    struct TestState {
        value: i32,
    }

    enum TestMessage {
        Increment,
    }

    impl Component for TestComponent {
        type Props = TestProps;
        type State = TestState;
        type Message = TestMessage;

        // Using default init (lines 26-27)
        // Using default update (lines 31-32)
        // Using default did_update (lines 39-40)

        fn view(&self, _props: &Self::Props, _state: &Self::State) -> Element {
            Element::None
        }
    }

    // Test widget that uses default preferred_height and preferred_width
    #[derive(Clone)]
    struct TestWidget;

    impl ElementWidget for TestWidget {
        fn render(&self, _area: Rect, _buf: &mut Buffer, _config: &DisplayConfig) {
            // Minimal implementation
        }

        fn clone_box(&self) -> Box<dyn ElementWidget> {
            Box::new(self.clone())
        }

        // Using default preferred_height (lines 115-116)
        // Using default preferred_width (lines 123-124)
    }

    // Test component wrapper for clone test
    struct TestComponentWrapper;

    impl ComponentWrapper for TestComponentWrapper {
        fn view_any(&self) -> Element {
            Element::None
        }

        fn clone_box(&self) -> Box<dyn ComponentWrapper> {
            Box::new(TestComponentWrapper)
        }
    }

    #[test]
    fn test_component_init_default() {
        let state = TestComponent::init(&TestProps);
        assert_eq!(state.value, 0); // Default value
    }

    #[test]
    fn test_component_update_default() {
        let mut component = TestComponent;
        let mut state = TestState::default();

        let effect = component.update(TestMessage::Increment, &mut state);

        // Default update returns Effect::None
        matches!(effect, Effect::None);
    }

    #[test]
    fn test_component_did_update_default() {
        let mut component = TestComponent;
        let old_props = TestProps;
        let new_props = TestProps;

        let effect = component.did_update(&old_props, &new_props);

        // Default did_update returns Effect::None
        matches!(effect, Effect::None);
    }

    #[test]
    fn test_renderable_widget_default_preferred_height() {
        let widget = TestWidget;
        assert_eq!(widget.preferred_height(), None);
    }

    #[test]
    fn test_renderable_widget_default_preferred_width() {
        let widget = TestWidget;
        assert_eq!(widget.preferred_width(), None);
    }

    #[test]
    fn test_box_element_widget_clone() {
        let widget: Box<dyn ElementWidget> = Box::new(TestWidget);
        let _cloned = widget.clone();
        // If we get here, clone worked
    }

    #[test]
    fn test_box_component_wrapper_clone() {
        let wrapper: Box<dyn ComponentWrapper> = Box::new(TestComponentWrapper);
        let _cloned = wrapper.clone();
        // If we get here, clone worked (tests lines 141-142)
    }

    #[test]
    fn test_vertical_helper() {
        let children = vec![Element::None, Element::None];
        let element = vertical(
            [Constraint::Length(10), Constraint::Min(5)],
            children.clone(),
        );

        match element {
            Element::Container {
                children: c,
                layout,
            } => {
                assert_eq!(c.len(), 2);
                matches!(layout, ContainerLayout::Vertical(_));
            }
            _ => panic!("Expected Container element"),
        }
    }

    #[test]
    fn test_horizontal_helper() {
        let children = vec![Element::None];
        let element = horizontal([Constraint::Percentage(50)], children.clone());

        match element {
            Element::Container {
                children: c,
                layout,
            } => {
                assert_eq!(c.len(), 1);
                matches!(layout, ContainerLayout::Horizontal(_));
            }
            _ => panic!("Expected Container element"),
        }
    }
}
