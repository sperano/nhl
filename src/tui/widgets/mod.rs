/// Widget-based rendering infrastructure for TUI
///
/// This module provides a trait-based architecture for composable, testable widgets.
/// Inspired by OO UI frameworks, widgets are small, focused components that can be
/// composed together to build complex interfaces.
///
/// ## Widget Traits
///
/// There are two widget traits in this codebase:
///
/// - **`StandaloneWidget`** (this module): For standalone widgets that render directly
///   to a buffer. These don't need Send + Sync or clone_box(). Used for small,
///   reusable rendering components like `GameBox`, `ScoreTable`.
///
/// - **`ElementWidget`** (`crate::tui::component`): For widgets that participate
///   in the Element tree. Requires Send + Sync + clone_box(). Used for component
///   widgets like `BoxscoreDocumentWidget`, `StatusBarWidget`.

#[cfg(test)]
pub mod testing;

// Small reusable widgets
pub mod list_modal;
pub use list_modal::{render_list_modal, ListModalWidget};

// Widget implementations

pub mod score_box;
pub use score_box::{ScoreBox, ScoreBoxStatus};

pub mod big_score;
pub use big_score::BigScore;

pub mod loading_animation;
pub use loading_animation::LoadingAnimation;

use crate::config::RenderContext;
use ratatui::{buffer::Buffer, layout::Rect};

/// Core trait for standalone renderable widgets
///
/// This trait is for widgets that render directly to a buffer but don't need
/// to participate in the Element tree (no Send + Sync or clone_box required).
///
/// For widgets that need to be embedded in the Element tree, use `ElementWidget`
/// from `crate::tui::component` instead.
///
/// Widgets render themselves directly to a ratatui Buffer, avoiding string-based
/// intermediate representations. This enables:
/// - Direct styling without character position calculations
/// - Composability (widgets can contain other widgets)
/// - Testability (can render to test buffers)
/// - Type safety (compiler catches layout errors)
///
/// # Object Safety
///
/// This trait is object-safe, meaning you can use trait objects to store
/// different widget types in collections.
pub trait StandaloneWidget {
    /// Render this widget into the provided buffer
    ///
    /// # Arguments
    ///
    /// * `area` - The rectangular area to render into
    /// * `buf` - The buffer to write to
    /// * `ctx` - Render context with display configuration and focus state
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestWidget;

    impl StandaloneWidget for TestWidget {
        fn render(&self, _area: Rect, _buf: &mut Buffer, _ctx: &RenderContext) {
            // Minimal implementation for testing
        }
    }

    #[test]
    fn test_default_preferred_height_returns_none() {
        let widget = TestWidget;
        assert_eq!(widget.preferred_height(), None);
    }

    #[test]
    fn test_default_preferred_width_returns_none() {
        let widget = TestWidget;
        assert_eq!(widget.preferred_width(), None);
    }
}
