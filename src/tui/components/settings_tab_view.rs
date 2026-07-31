//! Rendering widgets for the Settings tab.
//!
//! Split out of `settings_tab.rs` to keep that file under the project's
//! file-size guideline. These are `ElementWidget` implementations (plus a
//! test-only helper) with no behavior beyond rendering the settings document
//! and its optional modal overlay.

use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::{Config, RenderContext};
use crate::tui::component::{Element, ElementWidget};
use crate::tui::components::SettingsDocument;
use crate::tui::document::{DocumentView, FocusableId};
use crate::tui::SettingsCategory;

/// Widget for rendering the Settings tab with modal overlay
pub(crate) struct SettingsTabWithModal {
    pub(crate) base_element: Element,
    pub(crate) modal_options: Vec<String>,
    pub(crate) modal_selected_index: usize,
    pub(crate) modal_position_x: u16,
    pub(crate) modal_position_y: u16,
}

impl ElementWidget for SettingsTabWithModal {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        use crate::tui::renderer::Renderer;
        use crate::tui::widgets::ListModalWidget;

        // Render the base element first
        let mut renderer = Renderer::new();
        renderer.render(self.base_element.clone(), area, buf, ctx);

        // Render the modal on top
        let modal = ListModalWidget::new(
            self.modal_options.clone(),
            self.modal_selected_index,
            self.modal_position_x,
            self.modal_position_y,
        );
        modal.render(area, buf, ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(SettingsTabWithModal {
            base_element: self.base_element.clone(),
            modal_options: self.modal_options.clone(),
            modal_selected_index: self.modal_selected_index,
            modal_position_x: self.modal_position_x,
            modal_position_y: self.modal_position_y,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

/// Widget for rendering the Settings tab content
pub(crate) struct SettingsTabWidget {
    pub(crate) category: SettingsCategory,
    pub(crate) config: Arc<Config>,
    pub(crate) focused_id: Option<FocusableId>,
    pub(crate) scroll_offset: u16,
    pub(crate) viewport_height: u16,
    /// Whether this widget has focus (affects dim/bright rendering)
    pub(crate) focused: bool,
}

impl ElementWidget for SettingsTabWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create document for the current category
        let doc = Arc::new(SettingsDocument::new(self.category, self.config.clone()));
        let mut view = DocumentView::new(doc, area.height);

        // Apply focus state
        if let Some(id) = self.focused_id.clone() {
            view.focus_id(id);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        let child_ctx = ctx.child(self.focused);

        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(SettingsTabWidget {
            category: self.category,
            config: self.config.clone(),
            focused_id: self.focused_id.clone(),
            scroll_offset: self.scroll_offset,
            viewport_height: self.viewport_height,
            focused: self.focused,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

/// Helper to get focusable IDs for a settings category (for testing)
///
/// Reads focusable IDs from the real `SettingsDocument` rather than a hardcoded
/// list, so this can't drift from actual navigation behavior.
#[cfg(test)]
pub(crate) fn get_focusable_ids_for_category(category: SettingsCategory) -> Vec<FocusableId> {
    use crate::tui::document::{Document, FocusContext};

    SettingsDocument::new(category, Arc::new(Config::default()))
        .focusables(&FocusContext::default())
        .into_iter()
        .map(|f| f.id)
        .collect()
}
