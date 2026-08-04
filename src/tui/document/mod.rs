//! Document system for unbounded content with viewport-based scrolling
//!
//! This module provides a document abstraction that allows content of any height
//! to be rendered with viewport-based scrolling. Key features:
//!
//! - **Unbounded content**: Documents render at their natural height
//! - **Viewport scrolling**: Only the visible portion is displayed
//! - **Tab/Shift-Tab navigation**: Focus cycles through focusable elements
//! - **Autoscrolling**: Viewport automatically follows focused element
//! - **Focus highlighting**: Currently focused element is highlighted

pub mod builder;
pub mod elements;
mod factory;
pub mod focus;
mod handlers;
pub mod link;
mod view;
pub mod viewport;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::borrow::Cow;
use std::collections::HashMap;
// Only used by tests.rs (via `use super::*`); the production code in this
// module never constructs an `Arc` directly.
#[cfg(test)]
use std::sync::Arc;

use crate::config::RenderContext;
use crate::formatting::BoxChars;

pub use builder::DocumentBuilder;
pub use elements::{
    DocTabDef, DocumentElement, RowAlignment, TAB_BAR_HEIGHT, TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH,
};
pub use factory::build_stacked_document;
pub use focus::{FocusableElement, FocusableId, RowPosition};
pub use handlers::handle_stacked_document_key;
pub use link::LinkTarget;
pub use view::{render_document_widget, DocumentRenderCache, DocumentView, DocumentWidgetParams};
pub use viewport::Viewport;

/// Focus context passed when building a document
#[derive(Clone, Debug, PartialEq)]
pub struct FocusContext {
    /// The currently focused element (if any)
    pub focused_id: Option<FocusableId>,
    /// Available width for layout decisions (if known)
    pub available_width: Option<u16>,
    /// Whether to use unicode characters for rendering
    pub use_unicode: bool,
    /// Box drawing characters for rendering
    pub box_chars: BoxChars,
    /// Active tab selections for Tabs elements (tabs_id -> active_index)
    pub tab_selections: HashMap<String, usize>,
}

impl Default for FocusContext {
    fn default() -> Self {
        Self {
            focused_id: None,
            available_width: None,
            use_unicode: true,
            box_chars: BoxChars::unicode(),
            tab_selections: HashMap::new(),
        }
    }
}

impl FocusContext {
    /// Create a new focus context from a FocusableId
    pub fn from_id(id: &FocusableId) -> Self {
        Self::default().with_id(id.clone())
    }

    /// Create a new focus context with a focused table cell
    pub fn with_table_cell(table_name: impl Into<String>, row: usize, col: usize) -> Self {
        Self::default().with_id(FocusableId::table_cell(table_name, row, col))
    }

    /// Set the focused element ID
    pub fn with_id(mut self, id: FocusableId) -> Self {
        self.focused_id = Some(id);
        self
    }

    /// Set the available width for layout decisions
    pub fn with_width(mut self, width: u16) -> Self {
        self.available_width = Some(width);
        self
    }

    /// Set whether to use unicode characters
    pub fn with_unicode(mut self, use_unicode: bool) -> Self {
        self.use_unicode = use_unicode;
        self
    }

    /// Set the box drawing characters
    pub fn with_box_chars(mut self, box_chars: BoxChars) -> Self {
        self.box_chars = box_chars;
        self
    }

    /// Get the focused table row (if focus is on a table cell)
    pub fn focused_table_row(&self, table_name: &str) -> Option<usize> {
        match &self.focused_id {
            Some(FocusableId::TableCell {
                table_name: name,
                row,
                ..
            }) if name == table_name => Some(*row),
            _ => None,
        }
    }

    /// Check if a link with the given ID is focused
    pub fn is_link_focused(&self, id: &str) -> bool {
        matches!(&self.focused_id, Some(FocusableId::Link(link_id)) if link_id == id)
    }

    /// Set tab selections from a map
    pub fn with_tab_selections(mut self, selections: HashMap<String, usize>) -> Self {
        self.tab_selections = selections;
        self
    }
}

/// A document represents unbounded content that can be scrolled through a viewport
pub trait Document: Send + Sync {
    /// Build the document's element tree
    ///
    /// # Arguments
    /// - `focus`: Focus context indicating which element should be focused
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement>;

    /// Get the document's title for navigation/history
    ///
    /// `Cow<'static, str>` so documents with a fixed title (the common case)
    /// can return a `&'static str` literal with no allocation, while
    /// documents whose title is computed from their data (e.g. a team or
    /// player name) can still return an owned `String`.
    fn title(&self) -> Cow<'static, str>;

    /// Get the document's unique ID
    fn id(&self) -> Cow<'static, str>;

    /// Cheap identity of the data this document was built from, used by
    /// [`DocumentRenderCache`] to recognize "same content" across documents
    /// that are recreated every frame around persistent `Arc`s (e.g. the
    /// standings documents). Return the addresses of every `Arc` whose
    /// contents influence `build()`'s output.
    ///
    /// The default `None` opts out: such a document only gets cache hits
    /// when the *same* `Arc<dyn Document>` is rendered again (pointer
    /// equality), which is the natural fit for documents built once and
    /// stored in state (team/player/boxscore).
    ///
    /// Tokens are raw pointers, so they are only meaningful while the
    /// allocations they point to stay alive; the cache guarantees that by
    /// holding the previously rendered document (and therefore its data
    /// `Arc`s) inside the entry being compared against.
    fn cache_token(&self) -> Option<Vec<usize>> {
        None
    }

    /// Calculate the total height needed to render all elements
    fn calculate_height(&self) -> u16 {
        self.build(&FocusContext::default())
            .iter()
            .map(|elem| elem.height())
            .sum()
    }

    /// Build the document once and collect its focusable elements in one pass.
    ///
    /// This is the single source of "what's focusable in this document,
    /// with what metadata" -- every field (position, height, ID, row
    /// membership, link target) is collected together from one `build()`
    /// call, so a sync site can no longer update some fields and forget
    /// others. Prefer this over `build()` + manual focusable collection
    /// whenever you need focus metadata but not the full render/viewport
    /// machinery of `DocumentView`.
    fn focusables(&self, ctx: &FocusContext) -> Vec<FocusableElement> {
        let elements = self.build(ctx);
        let mut focusable = Vec::new();
        let mut y_offset = 0u16;
        for elem in &elements {
            elem.collect_focusable(&mut focusable, y_offset);
            y_offset += elem.height();
        }
        focusable
    }

    /// Render the document to a buffer at full height
    /// Returns the buffer and the actual height used
    fn render_full(&self, width: u16, ctx: &RenderContext, focus: &FocusContext) -> (Buffer, u16) {
        render_elements_to_buffer(self.build(focus), width, ctx)
    }
}

/// Render an already-built element tree to a full-height offscreen buffer.
///
/// Shared by [`Document::render_full`] (a standalone entry point some
/// documents' own tests call directly, e.g. the standings documents) and
/// [`DocumentView::render`] (which builds the elements itself via
/// [`build_full_document`]). Factoring this out keeps both callers'
/// element trees rendered identically without either one calling
/// `Document::build` on the other's behalf.
fn render_elements_to_buffer(
    elements: Vec<DocumentElement>,
    width: u16,
    ctx: &RenderContext,
) -> (Buffer, u16) {
    let height = elements.iter().map(|e| e.height()).sum();

    let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));

    // Fill entire buffer with background color
    buffer.set_style(buffer.area, ctx.base_style());

    let mut y_offset = 0;

    for element in elements {
        let element_height = element.height();
        let area = Rect::new(0, y_offset, width, element_height);
        element.render(area, &mut buffer, ctx);
        y_offset += element_height;
    }

    (buffer, height)
}

#[cfg(test)]
mod tests;
