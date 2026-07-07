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
pub mod viewport;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::sync::Arc;

use crate::config::RenderContext;

pub use builder::DocumentBuilder;
pub use elements::{
    DocTabDef, DocumentElement, RowAlignment, TAB_BAR_HEIGHT, TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH,
};
pub use factory::build_stacked_document;
pub use focus::{FocusManager, FocusableElement, FocusableId, RowPosition};
pub use handlers::handle_stacked_document_key;
pub use link::LinkTarget;
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
    pub box_chars: crate::formatting::BoxChars,
    /// Active tab selections for Tabs elements (tabs_id -> active_index)
    pub tab_selections: std::collections::HashMap<String, usize>,
}

impl Default for FocusContext {
    fn default() -> Self {
        Self {
            focused_id: None,
            available_width: None,
            use_unicode: true,
            box_chars: crate::formatting::BoxChars::unicode(),
            tab_selections: std::collections::HashMap::new(),
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
    pub fn with_box_chars(mut self, box_chars: crate::formatting::BoxChars) -> Self {
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
    pub fn with_tab_selections(
        mut self,
        selections: std::collections::HashMap<String, usize>,
    ) -> Self {
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
    fn title(&self) -> String;

    /// Get the document's unique ID
    fn id(&self) -> String;

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
    /// others. Prefer this over `build()` + manual `FocusManager` wiring
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
        let elements = self.build(focus);
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
}

/// Render shim that pairs a document with a viewport offset and focus index.
///
/// `DocumentView` holds no navigation logic of its own -- all focus/scroll
/// semantics (next/prev, wrap, paging, autoscroll) live in `document_nav.rs`
/// and its `DocumentNavState`. Each frame, the render path constructs a
/// fresh `DocumentView`, injects the current focus index and scroll offset
/// computed by `DocumentNavState`, and renders. This keeps exactly one
/// navigation engine: `DocumentNavState` decides "where", `DocumentView`
/// just draws "what's visible from here".
pub struct DocumentView {
    document: Arc<dyn Document>,
    viewport: Viewport,
    focus_manager: FocusManager,
    /// Pre-rendered full document buffer
    full_buffer: Option<Buffer>,
    /// Cached document height
    cached_height: u16,
}

impl DocumentView {
    /// Create a new document view
    ///
    /// # Arguments
    /// - `document`: The document to display
    /// - `viewport_height`: Height of the visible viewport
    pub fn new(document: Arc<dyn Document>, viewport_height: u16) -> Self {
        // Build once and derive both height and focus manager from it, instead of calling
        // calculate_height() (which independently calls build() again internally) followed by
        // a second build() call here for the focus manager.
        let elements = document.build(&FocusContext::default());
        let doc_height = elements.iter().map(|e| e.height()).sum();
        let viewport = Viewport::new(0, viewport_height, doc_height);
        let focus_manager = FocusManager::from_elements(&elements);

        Self {
            document,
            viewport,
            focus_manager,
            full_buffer: None,
            cached_height: doc_height,
        }
    }

    /// Focus a specific element by index (index space matches
    /// `DocumentNavState.focusables`, which is built from the same document).
    pub fn focus_by_index(&mut self, index: usize) {
        self.focus_manager.focus_by_index(index);
    }

    /// Set the scroll offset directly (computed by `document_nav.rs`)
    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.viewport.set_offset(offset);
    }

    // === Rendering ===

    /// Render the visible portion of the document
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        const HORIZONTAL_MARGIN: u16 = 1;

        // Account for left and right margins
        let content_width = area.width.saturating_sub(HORIZONTAL_MARGIN * 2);
        if content_width == 0 {
            return;
        }

        // Build focus context from current focus state, including available width and unicode setting
        let focus = self
            .focus_manager
            .get_current_id()
            .map(|id| FocusContext::from_id(id).with_width(content_width))
            .unwrap_or_default()
            .with_width(content_width)
            .with_unicode(ctx.use_unicode())
            .with_box_chars(ctx.config.box_chars)
            .with_tab_selections(ctx.tab_selections.clone());

        let (full_buf, height) = self.document.render_full(content_width, ctx, &focus);
        self.full_buffer = Some(full_buf);
        self.cached_height = height;
        self.viewport.set_content_height(height);

        // Fill margins with background style
        let margin_style = ctx.base_style();
        for y in area.y..area.y + area.height {
            // Left margin
            buf.set_string(area.x, y, " ", margin_style);
            // Right margin
            if area.width > 1 {
                buf.set_string(area.x + area.width - 1, y, " ", margin_style);
            }
        }

        // Copy visible portion from full buffer to output buffer (with horizontal offset for margin)
        if let Some(full_buffer) = &self.full_buffer {
            let visible_range = self.viewport.visible_range();
            let visible_start = visible_range.start;

            for y in visible_range {
                if y >= self.cached_height {
                    break;
                }

                let src_y = y;
                let dst_y = area.y + (y - visible_start);

                if dst_y >= area.y + area.height {
                    break;
                }

                for x in 0..content_width {
                    let src_idx = (src_y * content_width + x) as usize;
                    let dst_idx =
                        (dst_y * buf.area.width + (area.x + HORIZONTAL_MARGIN + x)) as usize;

                    if src_idx < full_buffer.content.len() && dst_idx < buf.content.len() {
                        buf.content[dst_idx] = full_buffer.content[src_idx].clone();
                    }
                }
            }

            // Focus highlighting is now handled by the elements themselves
            // (Link.focused, TableWidget.focused_row)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::testing::assert_buffer;

    // === assert_buffer rendering tests ===
    //
    // These exercise the surviving render shim end to end: `new` builds the
    // focus manager and viewport from the document, `focus_by_index` /
    // `set_scroll_offset` apply state computed by `document_nav.rs`, and
    // `render` clips the full document buffer into the visible area. Focus
    // wrap/paging/autoscroll *decisions* are covered by document_nav.rs's
    // own tests -- there is nothing left of that logic here to test.

    /// Test document that renders predictable content
    struct RenderTestDocument {
        title: String,
        lines: Vec<String>,
    }

    impl RenderTestDocument {
        fn new(title: &str, lines: Vec<&str>) -> Self {
            Self {
                title: title.to_string(),
                lines: lines.into_iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl Document for RenderTestDocument {
        fn build(&self, _focus: &FocusContext) -> Vec<DocumentElement> {
            let mut elements = Vec::new();
            elements.push(DocumentElement::heading(1, &self.title));
            for line in &self.lines {
                elements.push(DocumentElement::text(line));
            }
            elements
        }

        fn title(&self) -> String {
            self.title.clone()
        }

        fn id(&self) -> String {
            "render_test".to_string()
        }
    }

    #[test]
    fn test_document_view_render_basic() {
        let doc = Arc::new(RenderTestDocument::new("Test", vec!["Line 1", "Line 2"]));
        let mut view = DocumentView::new(doc, 10);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let area = Rect::new(0, 0, 12, 4);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Underline only extends to title width ("Test" = 4 chars)
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Test", " ════", " Line 1", " Line 2"]);
    }

    #[test]
    fn test_document_view_render_with_viewport_offset() {
        let doc = Arc::new(RenderTestDocument::new(
            "Title",
            vec!["Line 1", "Line 2", "Line 3", "Line 4", "Line 5"],
        ));
        let mut view = DocumentView::new(doc, 3);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Scroll down to skip the heading
        view.set_scroll_offset(2);

        let area = Rect::new(0, 0, 12, 3);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Should show lines starting from offset 2 (after title + underline)
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Line 1", " Line 2", " Line 3"]);
    }

    #[test]
    fn test_document_view_render_scrolled_to_bottom() {
        let doc = Arc::new(RenderTestDocument::new(
            "Title",
            vec!["Line 1", "Line 2", "Line 3"],
        ));
        let mut view = DocumentView::new(doc, 2);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // document_nav's ScrollToBottom uses u16::MAX as a sentinel that
        // set_scroll_offset (-> Viewport::set_offset) clamps to the real max offset.
        view.set_scroll_offset(u16::MAX);

        let area = Rect::new(0, 0, 12, 2);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Total height is 5 (title + underline + 3 lines), viewport is 2
        // Scrolled to bottom shows last 2 lines
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Line 2", " Line 3"]);
    }

    /// Test document with a link for focus rendering
    struct LinkTestDocument;

    impl Document for LinkTestDocument {
        fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
            let is_focused = focus.is_link_focused("test_link");
            vec![
                DocumentElement::text("Before"),
                if is_focused {
                    DocumentElement::focused_link(
                        "test_link",
                        "Click Me",
                        LinkTarget::Anchor("test".to_string()),
                    )
                } else {
                    DocumentElement::link(
                        "test_link",
                        "Click Me",
                        LinkTarget::Anchor("test".to_string()),
                    )
                },
                DocumentElement::text("After"),
            ]
        }

        fn title(&self) -> String {
            "Link Test".to_string()
        }

        fn id(&self) -> String {
            "link_test".to_string()
        }
    }

    #[test]
    fn test_document_view_render_unfocused_link() {
        let doc = Arc::new(LinkTestDocument);
        let mut view = DocumentView::new(doc, 10);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let area = Rect::new(0, 0, 17, 3);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Unfocused link has "  " prefix for alignment
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Before", "   Click Me", " After"]);
    }

    #[test]
    fn test_document_view_render_focused_link() {
        let doc = Arc::new(LinkTestDocument);
        let mut view = DocumentView::new(doc, 10);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Focus the link
        view.focus_by_index(0);

        let area = Rect::new(0, 0, 17, 3);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Focused link has "▶ " prefix
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Before", " ▶ Click Me", " After"]);
    }
}
