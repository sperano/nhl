//! Generic tab component patterns
//!
//! This module provides traits and helpers for implementing tab components
//! with consistent document navigation behavior. All tabs share:
//!
//! - Embedded `DocumentNavState` for focus/scroll management
//! - Common message variants: `DocNav`, `UpdateViewportHeight`, `NavigateUp`
//! - Browse mode logic (derived from `focus_index.is_some()`)
//! - Common update handling for document navigation
//!
//! # Usage
//!
//! 1. Implement `TabState` for your state struct (requires `doc_nav` accessors)
//! 2. Implement `TabMessage` for your message enum (requires conversion methods)
//! 3. Use `handle_common_message()` in your `update()` method
//! 4. Use `component_message_impl!` macro to eliminate boilerplate
//!
//! # Example
//!
//! ```ignore
//! use crate::tui::tab_component::{TabState, TabMessage, handle_common_message};
//!
//! impl TabState for MyTabState {
//!     fn doc_nav(&self) -> &DocumentNavState { &self.doc_nav }
//!     fn doc_nav_mut(&mut self) -> &mut DocumentNavState { &mut self.doc_nav }
//! }
//!
//! impl TabMessage for MyTabMsg {
//!     fn as_common(&self) -> Option<CommonTabMessage<'_>> { ... }
//!     fn from_doc_nav(msg: DocumentNavMsg) -> Self { Self::DocNav(msg) }
//! }
//!
//! // In update():
//! if let Some(effect) = handle_common_message(msg.as_common(), state) {
//!     return effect;
//! }
//! ```

use crate::tui::component::Effect;
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};

/// Implement TabState for DocumentNavState itself
///
/// This allows components that use DocumentNavState directly as their State type
/// (like DemoTab) to work with the common message handling.
impl TabState for DocumentNavState {
    fn doc_nav(&self) -> &DocumentNavState {
        self
    }

    fn doc_nav_mut(&mut self) -> &mut DocumentNavState {
        self
    }
}

/// Trait for tab component state that embeds document navigation
///
/// All tab states should implement this to enable common message handling.
pub trait TabState {
    /// Get immutable reference to document navigation state
    fn doc_nav(&self) -> &DocumentNavState;

    /// Get mutable reference to document navigation state
    fn doc_nav_mut(&mut self) -> &mut DocumentNavState;

    /// Check if browse mode is active (has focused element)
    ///
    /// Default implementation checks if focus_index is Some.
    fn is_browse_mode(&self) -> bool {
        self.doc_nav().focus_index.is_some()
    }

    /// Exit browse mode by clearing focus and scroll
    ///
    /// Default implementation clears focus_index and scroll_offset.
    fn exit_browse_mode(&mut self) {
        let nav = self.doc_nav_mut();
        nav.focus_index = None;
        nav.scroll_offset = 0;
    }

    /// Enter browse mode by focusing the first element
    ///
    /// Default implementation sets focus to first element if available.
    fn enter_browse_mode(&mut self) {
        let nav = self.doc_nav_mut();
        if !nav.focusable_positions.is_empty() && nav.focus_index.is_none() {
            nav.focus_index = Some(0);
        }
    }
}

/// Common tab message variants that all tabs share
///
/// This enum represents the messages that are handled identically across tabs.
#[derive(Debug, Clone)]
pub enum CommonTabMessage<'a> {
    /// Document navigation message (delegated to document_nav module)
    DocNav(&'a DocumentNavMsg),

    /// Update viewport height for autoscroll calculations
    UpdateViewportHeight(u16),

    /// Navigate up request (ESC in browse mode)
    ///
    /// Returns `Effect::Handled` if consumed (exited browse mode),
    /// or `Effect::None` to bubble up.
    NavigateUp,
}

/// Trait for tab message enums to enable common handling
///
/// Implement this to allow `handle_common_message()` to process shared logic.
pub trait TabMessage: Sized {
    /// Convert this message to a common message variant if applicable
    ///
    /// Returns `Some(CommonTabMessage)` for messages that should be handled
    /// by `handle_common_message()`, or `None` for tab-specific messages.
    fn as_common(&self) -> Option<CommonTabMessage<'_>>;

    /// Create a message from a DocumentNavMsg
    ///
    /// Used for delegating key handling to document navigation.
    fn from_doc_nav(msg: DocumentNavMsg) -> Self;
}

/// Handle common tab messages
///
/// Call this at the start of your `update()` method to handle shared logic.
/// Returns `Some(Effect)` if the message was handled, `None` if it should
/// be processed by tab-specific logic.
///
/// # Example
///
/// ```ignore
/// fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
///     if let Some(effect) = handle_common_message(msg.as_common(), state) {
///         return effect;
///     }
///     // Handle tab-specific messages...
/// }
/// ```
pub fn handle_common_message<S: TabState>(
    msg: Option<CommonTabMessage<'_>>,
    state: &mut S,
) -> Option<Effect> {
    match msg? {
        CommonTabMessage::DocNav(nav_msg) => {
            Some(crate::tui::document_nav::handle_message(state.doc_nav_mut(), nav_msg))
        }

        CommonTabMessage::UpdateViewportHeight(height) => {
            state.doc_nav_mut().viewport_height = height;
            Some(Effect::None)
        }

        CommonTabMessage::NavigateUp => {
            if state.is_browse_mode() {
                state.exit_browse_mode();
                Some(Effect::Handled)
            } else {
                Some(Effect::None) // Let it bubble up
            }
        }
    }
}

/// Macro to implement ComponentMessageTrait for tab message enums
///
/// This eliminates the ~15 lines of boilerplate that every tab message enum requires.
///
/// # Usage
///
/// ```ignore
/// component_message_impl!(ScoresTabMsg, ScoresTab, ScoresTabState);
/// ```
///
/// Expands to:
///
/// ```ignore
/// impl ComponentMessageTrait for ScoresTabMsg {
///     fn apply(&self, state: &mut dyn Any) -> Effect {
///         if let Some(tab_state) = state.downcast_mut::<ScoresTabState>() {
///             let mut component = ScoresTab;
///             component.update(self.clone(), tab_state)
///         } else {
///             Effect::None
///         }
///     }
///
///     fn clone_box(&self) -> Box<dyn ComponentMessageTrait> {
///         Box::new(self.clone())
///     }
/// }
/// ```
#[macro_export]
macro_rules! component_message_impl {
    ($msg_type:ty, $component_type:ty, $state_type:ty) => {
        impl $crate::tui::action::ComponentMessageTrait for $msg_type {
            fn apply(&self, state: &mut dyn std::any::Any) -> $crate::tui::component::Effect {
                if let Some(tab_state) = state.downcast_mut::<$state_type>() {
                    let mut component = <$component_type>::default();
                    <$component_type as $crate::tui::component::Component>::update(
                        &mut component,
                        self.clone(),
                        tab_state,
                    )
                } else {
                    $crate::tui::component::Effect::None
                }
            }

            fn clone_box(&self) -> Box<dyn $crate::tui::action::ComponentMessageTrait> {
                Box::new(self.clone())
            }
        }
    };
}

// Re-export macro at module level
pub use component_message_impl;

#[cfg(test)]
mod tests {
    use super::*;

    // Test state implementing TabState
    #[derive(Default, Clone)]
    struct TestTabState {
        doc_nav: DocumentNavState,
        #[allow(dead_code)]
        custom_field: i32,
    }

    impl TabState for TestTabState {
        fn doc_nav(&self) -> &DocumentNavState {
            &self.doc_nav
        }

        fn doc_nav_mut(&mut self) -> &mut DocumentNavState {
            &mut self.doc_nav
        }
    }

    // Test message implementing TabMessage
    #[derive(Clone, Debug)]
    enum TestTabMsg {
        DocNav(DocumentNavMsg),
        UpdateViewportHeight(u16),
        NavigateUp,
        CustomAction,
    }

    impl TabMessage for TestTabMsg {
        fn as_common(&self) -> Option<CommonTabMessage<'_>> {
            match self {
                TestTabMsg::DocNav(msg) => Some(CommonTabMessage::DocNav(msg)),
                TestTabMsg::UpdateViewportHeight(h) => Some(CommonTabMessage::UpdateViewportHeight(*h)),
                TestTabMsg::NavigateUp => Some(CommonTabMessage::NavigateUp),
                TestTabMsg::CustomAction => None,
            }
        }

        fn from_doc_nav(msg: DocumentNavMsg) -> Self {
            TestTabMsg::DocNav(msg)
        }
    }

    #[test]
    fn test_tab_state_is_browse_mode() {
        let mut state = TestTabState::default();
        assert!(!state.is_browse_mode());

        state.doc_nav.focus_index = Some(0);
        assert!(state.is_browse_mode());
    }

    #[test]
    fn test_tab_state_exit_browse_mode() {
        let mut state = TestTabState::default();
        state.doc_nav.focus_index = Some(2);
        state.doc_nav.scroll_offset = 10;

        state.exit_browse_mode();

        assert_eq!(state.doc_nav.focus_index, None);
        assert_eq!(state.doc_nav.scroll_offset, 0);
    }

    #[test]
    fn test_tab_state_enter_browse_mode() {
        let mut state = TestTabState::default();
        state.doc_nav.focusable_positions = vec![0, 5, 10];

        state.enter_browse_mode();

        assert_eq!(state.doc_nav.focus_index, Some(0));
    }

    #[test]
    fn test_tab_state_enter_browse_mode_no_focusables() {
        let mut state = TestTabState::default();
        // No focusable_positions

        state.enter_browse_mode();

        assert_eq!(state.doc_nav.focus_index, None);
    }

    #[test]
    fn test_handle_common_message_doc_nav() {
        let mut state = TestTabState::default();
        state.doc_nav.focusable_positions = vec![0, 5, 10];
        state.doc_nav.focus_index = Some(0);

        let msg = TestTabMsg::DocNav(DocumentNavMsg::FocusNext);
        let effect = handle_common_message(msg.as_common(), &mut state);

        assert!(effect.is_some());
        assert_eq!(state.doc_nav.focus_index, Some(1));
    }

    #[test]
    fn test_handle_common_message_update_viewport() {
        let mut state = TestTabState::default();

        let msg = TestTabMsg::UpdateViewportHeight(50);
        let effect = handle_common_message(msg.as_common(), &mut state);

        assert!(effect.is_some());
        assert_eq!(state.doc_nav.viewport_height, 50);
    }

    #[test]
    fn test_handle_common_message_navigate_up_in_browse_mode() {
        let mut state = TestTabState::default();
        state.doc_nav.focus_index = Some(2);
        state.doc_nav.scroll_offset = 10;

        let msg = TestTabMsg::NavigateUp;
        let effect = handle_common_message(msg.as_common(), &mut state);

        assert!(matches!(effect, Some(Effect::Handled)));
        assert_eq!(state.doc_nav.focus_index, None);
        assert_eq!(state.doc_nav.scroll_offset, 0);
    }

    #[test]
    fn test_handle_common_message_navigate_up_not_in_browse_mode() {
        let mut state = TestTabState::default();
        // Not in browse mode (focus_index is None)

        let msg = TestTabMsg::NavigateUp;
        let effect = handle_common_message(msg.as_common(), &mut state);

        // Should return None effect to bubble up
        assert!(matches!(effect, Some(Effect::None)));
    }

    #[test]
    fn test_handle_common_message_returns_none_for_custom() {
        let mut state = TestTabState::default();

        let msg = TestTabMsg::CustomAction;
        let effect = handle_common_message(msg.as_common(), &mut state);

        // Custom actions should not be handled
        assert!(effect.is_none());
    }

    #[test]
    fn test_tab_message_from_doc_nav() {
        let nav_msg = DocumentNavMsg::FocusNext;
        let msg = TestTabMsg::from_doc_nav(nav_msg.clone());

        assert!(matches!(msg, TestTabMsg::DocNav(_)));
    }
}
