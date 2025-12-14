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
pub mod focus;
mod handlers;
pub mod link;
pub mod viewport;
pub mod widget;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::sync::Arc;

use crate::config::RenderContext;
use crate::tui::component::Effect;
use crate::tui::document_nav::{handle_message, DocumentNavState};
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::DataState;
use crate::tui::types::StackedDocument;

pub use builder::DocumentBuilder;
pub use elements::{DocumentElement, RowAlignment, TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH};
pub use focus::{FocusManager, FocusableElement, FocusableId, RowPosition};
pub use link::{DocumentLink, DocumentType, LinkParams, LinkTarget};
pub use viewport::Viewport;
pub use widget::DocumentElementWidget;

/// Focus context passed when building a document
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusContext {
    /// The currently focused element (if any)
    pub focused_id: Option<FocusableId>,
    /// Available width for layout decisions (if known)
    pub available_width: Option<u16>,
    /// Whether to use unicode characters for rendering
    pub use_unicode: bool,
}

impl FocusContext {
    /// Create a new focus context from a FocusableId
    pub fn from_id(id: &FocusableId) -> Self {
        Self {
            focused_id: Some(id.clone()),
            available_width: None,
            use_unicode: true,
        }
    }

    /// Create a new focus context with a focused link ID
    pub fn with_link(id: impl Into<String>) -> Self {
        Self {
            focused_id: Some(FocusableId::link(id)),
            available_width: None,
            use_unicode: true,
        }
    }

    /// Create a new focus context with a focused table cell
    pub fn with_table_cell(table_name: impl Into<String>, row: usize, col: usize) -> Self {
        Self {
            focused_id: Some(FocusableId::table_cell(table_name, row, col)),
            available_width: None,
            use_unicode: true,
        }
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

    /// Check if the given ID is focused
    pub fn is_focused(&self, id: &FocusableId) -> bool {
        self.focused_id.as_ref() == Some(id)
    }

    /// Check if a link with the given ID is focused
    pub fn is_link_focused(&self, id: &str) -> bool {
        matches!(&self.focused_id, Some(FocusableId::Link(link_id)) if link_id == id)
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

    /// Get y-positions of all focusable elements in this document
    ///
    /// This is useful for storing positions in state so reducers can
    /// perform accurate autoscrolling without access to the document itself.
    fn focusable_positions(&self) -> Vec<u16> {
        let elements = self.build(&FocusContext::default());
        FocusManager::from_elements(&elements).y_positions()
    }

    /// Get heights of all focusable elements in this document
    ///
    /// This is useful for autoscrolling - tall elements (like GameBox)
    /// need the viewport to scroll enough to show the entire element,
    /// not just the top.
    fn focusable_heights(&self) -> Vec<u16> {
        let elements = self.build(&FocusContext::default());
        FocusManager::from_elements(&elements).heights()
    }

    /// Get row positions for all focusable elements in this document
    ///
    /// Returns RowPosition for elements in Rows, None for others.
    fn focusable_row_positions(&self) -> Vec<Option<RowPosition>> {
        let elements = self.build(&FocusContext::default());
        FocusManager::from_elements(&elements).row_positions()
    }

    /// Get IDs of all focusable elements in this document
    ///
    /// Returns IDs in document order (top to bottom, left to right for rows).
    fn focusable_ids(&self) -> Vec<FocusableId> {
        let elements = self.build(&FocusContext::default());
        FocusManager::from_elements(&elements).ids()
    }

    /// Get link targets of all focusable elements in this document
    ///
    /// Returns link targets in document order. None for elements without links.
    fn focusable_link_targets(&self) -> Vec<Option<LinkTarget>> {
        let elements = self.build(&FocusContext::default());
        FocusManager::from_elements(&elements).link_targets()
    }

    /// Render the document to a buffer at full height
    /// Returns the buffer and the actual height used
    fn render_full(
        &self,
        width: u16,
        ctx: &RenderContext,
        focus: &FocusContext,
    ) -> (Buffer, u16) {
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

/// Handler trait for stacked documents to handle their own key events
///
/// Stacked documents (Boxscore, TeamDetail, PlayerDetail) implement this trait
/// to encapsulate their navigation and activation logic. This keeps key handling
/// close to the document that understands its structure.
///
/// Implementors provide `activate()` and `populate_focusable_metadata()`.
/// The default `handle_key()` populates metadata on-demand before navigation,
/// eliminating the need to sync cached metadata when data loads.
pub trait StackedDocumentHandler: Send + Sync {
    /// Activate the focused element (Enter key)
    ///
    /// Called when the user presses Enter on a focused element.
    /// Returns Effect::Action to push a new document, or Effect::None if nothing to activate.
    fn activate(&self, nav: &DocumentNavState, data: &DataState) -> Effect;

    /// Populate focusable metadata from the document into nav state
    ///
    /// Called before navigation to ensure metadata is current.
    /// Builds the document from data and extracts focusable positions/heights.
    /// Width is used to determine layout (e.g., side-by-side vs stacked).
    fn populate_focusable_metadata(&self, nav: &mut DocumentNavState, data: &DataState, width: u16);

    /// Handle a key event for this document
    ///
    /// Default implementation populates focusable metadata on-demand, then handles
    /// navigation via `key_to_nav_msg` and delegates Enter to `activate()`.
    fn handle_key(
        &self,
        key: KeyEvent,
        nav: &mut DocumentNavState,
        data: &DataState,
        width: u16,
    ) -> Effect {
        // Populate focusable metadata on-demand before navigation
        self.populate_focusable_metadata(nav, data, width);

        // Try navigation first (Tab, arrows, Page keys, etc.)
        if let Some(nav_msg) = key_to_nav_msg(key) {
            return handle_message(nav, &nav_msg);
        }
        // Handle Enter for activation
        if key.code == KeyCode::Enter {
            return self.activate(nav, data);
        }
        Effect::None
    }
}

/// Get the appropriate handler for a stacked document type
///
/// Returns a handler that implements key handling for the specific document type.
/// Each document type (Boxscore, TeamDetail, PlayerDetail) has its own handler
/// that understands how to navigate and activate elements within it.
pub fn get_stacked_document_handler(doc: &StackedDocument) -> Box<dyn StackedDocumentHandler> {
    use handlers::{
        BoxscoreDocumentHandler, PlayerDetailDocumentHandler, TeamDetailDocumentHandler,
    };

    match doc {
        StackedDocument::Boxscore { game_id, .. } => {
            Box::new(BoxscoreDocumentHandler { game_id: *game_id })
        }
        StackedDocument::TeamDetail { abbrev } => Box::new(TeamDetailDocumentHandler {
            abbrev: abbrev.clone(),
        }),
        StackedDocument::PlayerDetail { player_id, .. } => Box::new(PlayerDetailDocumentHandler {
            player_id: *player_id,
        }),
    }
}

/// Container that holds a document and manages viewport/scrolling/focus
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
        let doc_height = document.calculate_height();
        let viewport = Viewport::new(0, viewport_height, doc_height);

        // Build focus manager from document elements (no focus initially)
        let elements = document.build(&FocusContext::default());
        let focus_manager = FocusManager::from_elements(&elements);

        Self {
            document,
            viewport,
            focus_manager,
            full_buffer: None,
            cached_height: doc_height,
        }
    }

    /// Update viewport height (e.g., on terminal resize)
    pub fn set_viewport_height(&mut self, height: u16) {
        self.viewport.set_height(height);
    }

    /// Get the current viewport offset
    pub fn viewport_offset(&self) -> u16 {
        self.viewport.offset()
    }

    /// Get the viewport
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Get the focus manager
    pub fn focus_manager(&self) -> &FocusManager {
        &self.focus_manager
    }

    // === Manual Scroll Operations ===

    /// Scroll the viewport up by a number of lines
    pub fn scroll_up(&mut self, lines: u16) {
        self.viewport.scroll_up(lines);
    }

    /// Scroll the viewport down by a number of lines
    pub fn scroll_down(&mut self, lines: u16) {
        self.viewport.scroll_down(lines);
    }

    /// Scroll to the top of the document
    pub fn scroll_to_top(&mut self) {
        self.viewport.scroll_to_top();
    }

    /// Scroll to the bottom of the document
    pub fn scroll_to_bottom(&mut self) {
        self.viewport.scroll_to_bottom();
    }

    /// Page up (scroll by viewport height - overlap)
    pub fn page_up(&mut self) {
        const PAGE_OVERLAP_LINES: u16 = 2;
        let page_size = self.viewport.height().saturating_sub(PAGE_OVERLAP_LINES);
        self.viewport.scroll_up(page_size);
    }

    /// Page down (scroll by viewport height - overlap)
    pub fn page_down(&mut self) {
        const PAGE_OVERLAP_LINES: u16 = 2;
        let page_size = self.viewport.height().saturating_sub(PAGE_OVERLAP_LINES);
        self.viewport.scroll_down(page_size);
    }

    // === Focus Navigation with Autoscrolling ===

    /// Navigate focus forward (Tab) with autoscrolling
    ///
    /// When focus moves to a new element, the viewport automatically scrolls
    /// to keep the focused element visible. If focus wraps from last to first
    /// element, the viewport scrolls to the top.
    pub fn focus_next(&mut self) -> bool {
        let prev_focus = self.focus_manager.current_index();

        if self.focus_manager.focus_next() {
            // Check if we wrapped around (from last to first)
            let wrapped = self.focus_manager.did_wrap_forward(prev_focus);

            if wrapped {
                // Wrapped to first element, scroll to top
                self.viewport.scroll_to_top();
            } else {
                // Normal navigation, ensure new focus is visible with padding
                self.autoscroll_to_focused();
            }
            true
        } else {
            false
        }
    }

    /// Navigate focus backward (Shift-Tab) with autoscrolling
    ///
    /// When focus moves to a new element, the viewport automatically scrolls
    /// to keep the focused element visible. If focus wraps from first to last
    /// element, the viewport scrolls to show the last element.
    pub fn focus_prev(&mut self) -> bool {
        let prev_focus = self.focus_manager.current_index();

        if self.focus_manager.focus_prev() {
            // Check if we wrapped around (from first to last)
            let wrapped = self.focus_manager.did_wrap_backward(prev_focus);

            if wrapped {
                // Wrapped to last element, scroll to show it
                if let Some(rect) = self.focus_manager.get_focused_rect() {
                    let element_bottom = rect.y + rect.height;
                    let desired_offset = element_bottom.saturating_sub(self.viewport.height());
                    self.viewport.set_offset(desired_offset);
                }
            } else {
                // Normal navigation, ensure new focus is visible with padding
                self.autoscroll_to_focused();
            }
            true
        } else {
            false
        }
    }

    /// Autoscroll to make the focused element visible with smart padding
    fn autoscroll_to_focused(&mut self) {
        if let Some(rect) = self.focus_manager.get_focused_rect() {
            let padding = self.viewport.smart_padding();
            self.viewport
                .ensure_visible_with_padding(rect.y, rect.height, padding);
        }
    }

    /// Focus a specific element by ID with autoscrolling
    pub fn focus_element_by_id(&mut self, id: &FocusableId) -> bool {
        if self.focus_manager.focus_by_id(id) {
            self.autoscroll_to_focused();
            true
        } else {
            false
        }
    }

    /// Clear focus (no element focused)
    pub fn clear_focus(&mut self) {
        self.focus_manager.clear_focus();
    }

    /// Focus a specific element by index
    pub fn focus_by_index(&mut self, index: usize) {
        self.focus_manager.focus_by_index(index);
    }

    /// Set the scroll offset directly
    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.viewport.set_offset(offset);
    }

    // === Link Activation ===

    /// Activate the currently focused element (Enter)
    ///
    /// Returns a LinkTarget if a link was activated.
    pub fn activate_focused(&self) -> Option<LinkTarget> {
        self.focus_manager.activate_current()
    }

    /// Get the currently focused element's link target (if any)
    pub fn get_focused_link(&self) -> Option<&LinkTarget> {
        self.focus_manager.get_current_link()
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
            .with_unicode(ctx.use_unicode());

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
                    let dst_idx = (dst_y * buf.area.width + (area.x + HORIZONTAL_MARGIN + x)) as usize;

                    if src_idx < full_buffer.content.len() && dst_idx < buf.content.len() {
                        buf.content[dst_idx] = full_buffer.content[src_idx].clone();
                    }
                }
            }

            // Focus highlighting is now handled by the elements themselves
            // (Link.focused, TableWidget.focused_row)
        }
    }

    /// Invalidate the render cache (call when document content changes)
    pub fn invalidate_cache(&mut self) {
        self.full_buffer = None;
    }

    /// Rebuild the document (rebuilds element tree and focus manager)
    pub fn rebuild(&mut self) {
        let elements = self.document.build(&FocusContext::default());
        self.focus_manager = FocusManager::from_elements(&elements);
        self.cached_height = elements.iter().map(|e| e.height()).sum();
        self.viewport.set_content_height(self.cached_height);
        self.invalidate_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::testing::assert_buffer;

    /// Simple test document for testing
    struct TestDocument {
        lines: usize,
        links: usize,
    }

    impl TestDocument {
        fn new(lines: usize, links: usize) -> Self {
            Self { lines, links }
        }
    }

    impl Document for TestDocument {
        fn build(&self, _focus: &FocusContext) -> Vec<DocumentElement> {
            let mut elements = Vec::new();

            elements.push(DocumentElement::heading(1, "Test Document"));

            for i in 0..self.lines {
                elements.push(DocumentElement::text(format!("Line {}", i)));
            }

            for i in 0..self.links {
                elements.push(DocumentElement::link(
                    format!("link_{}", i),
                    format!("Link {}", i),
                    LinkTarget::Action(format!("action_{}", i)),
                ));
            }

            elements
        }

        fn title(&self) -> String {
            "Test Document".to_string()
        }

        fn id(&self) -> String {
            "test_doc".to_string()
        }
    }

    #[test]
    fn test_document_view_new() {
        let doc = Arc::new(TestDocument::new(10, 3));
        let view = DocumentView::new(doc, 20);

        assert_eq!(view.viewport_offset(), 0);
        assert_eq!(view.focus_manager().len(), 3); // 3 links
    }

    #[test]
    fn test_focus_next_basic() {
        let doc = Arc::new(TestDocument::new(5, 3));
        let mut view = DocumentView::new(doc, 20);

        // First tab should focus first link
        assert!(view.focus_next());
        assert_eq!(view.focus_manager().current_index(), Some(0));

        // Second tab should focus second link
        assert!(view.focus_next());
        assert_eq!(view.focus_manager().current_index(), Some(1));

        // Third tab should focus third link
        assert!(view.focus_next());
        assert_eq!(view.focus_manager().current_index(), Some(2));

        // Fourth tab should wrap to first
        assert!(view.focus_next());
        assert_eq!(view.focus_manager().current_index(), Some(0));
    }

    #[test]
    fn test_focus_prev_basic() {
        let doc = Arc::new(TestDocument::new(5, 3));
        let mut view = DocumentView::new(doc, 20);

        // First shift-tab should focus last link
        assert!(view.focus_prev());
        assert_eq!(view.focus_manager().current_index(), Some(2));

        // Second shift-tab should focus second link
        assert!(view.focus_prev());
        assert_eq!(view.focus_manager().current_index(), Some(1));
    }

    #[test]
    fn test_focus_next_wraps_and_scrolls_to_top() {
        let doc = Arc::new(TestDocument::new(50, 3));
        let mut view = DocumentView::new(doc, 10);

        // Navigate to last element
        view.focus_next(); // 0
        view.focus_next(); // 1
        view.focus_next(); // 2 (last)

        // Scroll down so we're not at top
        view.scroll_down(30);
        assert!(view.viewport_offset() > 0);

        // Wrap to first should scroll to top
        view.focus_next(); // wraps to 0
        assert_eq!(view.viewport_offset(), 0);
    }

    #[test]
    fn test_autoscroll_when_focusing_off_screen() {
        // Create a document with many lines and links spread throughout
        let doc = Arc::new(TestDocument::new(100, 50));
        let mut view = DocumentView::new(doc, 10);

        // Tab through many elements
        for _ in 0..30 {
            view.focus_next();
        }

        // The focused element should be visible
        if let Some(rect) = view.focus_manager().get_focused_rect() {
            let visible = view.viewport().visible_range();
            assert!(
                rect.y >= visible.start && rect.y < visible.end,
                "Focused element at y={} not visible in range {:?}",
                rect.y,
                visible
            );
        }
    }

    #[test]
    fn test_scroll_operations() {
        let doc = Arc::new(TestDocument::new(100, 0));
        let mut view = DocumentView::new(doc, 10);

        assert_eq!(view.viewport_offset(), 0);

        view.scroll_down(5);
        assert_eq!(view.viewport_offset(), 5);

        view.scroll_up(3);
        assert_eq!(view.viewport_offset(), 2);

        view.scroll_to_bottom();
        assert!(view.viewport().is_at_bottom());

        view.scroll_to_top();
        assert_eq!(view.viewport_offset(), 0);
    }

    #[test]
    fn test_page_operations() {
        let doc = Arc::new(TestDocument::new(100, 0));
        let mut view = DocumentView::new(doc, 20);

        view.page_down();
        assert_eq!(view.viewport_offset(), 18); // 20 - 2 overlap

        view.page_up();
        assert_eq!(view.viewport_offset(), 0);
    }

    #[test]
    fn test_focus_element_by_id() {
        let doc = Arc::new(TestDocument::new(5, 5));
        let mut view = DocumentView::new(doc, 10);

        assert!(view.focus_element_by_id(&FocusableId::link("link_2")));
        assert_eq!(view.focus_manager().current_index(), Some(2));

        assert!(!view.focus_element_by_id(&FocusableId::link("nonexistent")));
    }

    #[test]
    fn test_activate_focused() {
        let doc = Arc::new(TestDocument::new(5, 3));
        let mut view = DocumentView::new(doc, 10);

        // No focus yet
        assert!(view.activate_focused().is_none());

        // Focus first link
        view.focus_next();
        let target = view.activate_focused();
        assert!(matches!(target, Some(LinkTarget::Action(_))));
    }

    #[test]
    fn test_clear_focus() {
        let doc = Arc::new(TestDocument::new(5, 3));
        let mut view = DocumentView::new(doc, 10);

        view.focus_next();
        assert!(view.focus_manager().current_index().is_some());

        view.clear_focus();
        assert!(view.focus_manager().current_index().is_none());
    }

    #[test]
    fn test_set_viewport_height() {
        let doc = Arc::new(TestDocument::new(10, 0));
        let mut view = DocumentView::new(doc, 10);

        view.set_viewport_height(20);
        assert_eq!(view.viewport().height(), 20);
    }

    #[test]
    fn test_empty_document() {
        struct EmptyDoc;
        impl Document for EmptyDoc {
            fn build(&self, _focus: &FocusContext) -> Vec<DocumentElement> {
                Vec::new()
            }
            fn title(&self) -> String {
                "Empty".to_string()
            }
            fn id(&self) -> String {
                "empty".to_string()
            }
        }

        let doc = Arc::new(EmptyDoc);
        let mut view = DocumentView::new(doc, 10);

        // Focus operations should return false
        assert!(!view.focus_next());
        assert!(!view.focus_prev());
    }

    #[test]
    fn test_document_with_no_focusable_elements() {
        let doc = Arc::new(TestDocument::new(50, 0)); // No links
        let mut view = DocumentView::new(doc, 10);

        assert!(!view.focus_next());
        assert_eq!(view.focus_manager().len(), 0);
    }

    // === assert_buffer rendering tests ===

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
        view.scroll_down(2);

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

        view.scroll_to_bottom();

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
                        LinkTarget::Action("test".to_string()),
                    )
                } else {
                    DocumentElement::link(
                        "test_link",
                        "Click Me",
                        LinkTarget::Action("test".to_string()),
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
        view.focus_next();

        let area = Rect::new(0, 0, 17, 3);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Focused link has "▶ " prefix
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Before", " ▶ Click Me", " After"]);
    }
}
