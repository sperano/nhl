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
use std::borrow::Cow;
use std::sync::Arc;

use crate::config::RenderContext;
use crate::tui::widgets::{LoadingAnimation, StandaloneWidget};

pub use builder::DocumentBuilder;
pub use elements::{
    DocTabDef, DocumentElement, RowAlignment, TAB_BAR_HEIGHT, TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH,
};
pub use factory::build_stacked_document;
pub use focus::{FocusableElement, FocusableId, RowPosition};
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

/// Build a document's element tree and render it to a full-height offscreen
/// buffer, with focus highlighting (e.g. `Link` vs `focused_link`, a table's
/// `focused_row`) baked in by `build()` when `focused_id` is set.
///
/// Always builds exactly once: the caller supplies the focused element's
/// `FocusableId` directly (resolved by the navigation layer, whose
/// `DocumentNavState.focusables` defines the focus index space), so no
/// preliminary unfocused build is needed to translate an index into an ID.
fn build_full_document(
    document: &dyn Document,
    focused_id: Option<&FocusableId>,
    content_width: u16,
    ctx: &RenderContext,
) -> (Buffer, u16) {
    let mut focus = FocusContext::default()
        .with_width(content_width)
        .with_unicode(ctx.use_unicode())
        .with_box_chars(ctx.config.box_chars)
        .with_tab_selections(ctx.tab_selections.clone());
    focus.focused_id = focused_id.cloned();
    render_elements_to_buffer(document.build(&focus), content_width, ctx)
}

/// The parts of [`crate::config::DisplayConfig`] that influence rendered
/// output, in `PartialEq` form for cache validation. `theme_name` stands in
/// for the derived `theme` (which is looked up from a static map, so equal
/// names imply equal themes).
#[derive(PartialEq)]
struct ConfigFingerprint {
    use_unicode: bool,
    theme_name: Option<String>,
    error_fg: ratatui::style::Color,
    box_chars: crate::formatting::BoxChars,
}

impl ConfigFingerprint {
    fn of(config: &crate::config::DisplayConfig) -> Self {
        Self {
            use_unicode: config.use_unicode,
            theme_name: config.theme_name.clone(),
            error_fg: config.error_fg,
            box_chars: config.box_chars,
        }
    }
}

/// One cached document render: the full-height buffer plus every input it
/// was produced from. Holding `document` doubles as the ABA guard for
/// `cache_token` pointer comparisons -- as long as the entry exists, the
/// data `Arc`s behind the token stay alive, so a new document's equal token
/// can only mean the same live allocations.
struct CacheEntry {
    document: Arc<dyn Document>,
    token: Option<Vec<usize>>,
    focused_id: Option<FocusableId>,
    content_width: u16,
    focused: bool,
    tab_selections: std::collections::HashMap<String, usize>,
    config: ConfigFingerprint,
    full_buffer: Buffer,
    height: u16,
}

impl CacheEntry {
    fn matches(
        &self,
        document: &Arc<dyn Document>,
        focused_id: Option<&FocusableId>,
        content_width: u16,
        ctx: &RenderContext,
    ) -> bool {
        // Compare data pointers (not fat pointers) to sidestep vtable
        // identity issues; a false negative would only mean a rebuild.
        let same_document = std::ptr::eq(
            Arc::as_ptr(&self.document) as *const (),
            Arc::as_ptr(document) as *const (),
        ) || (self.token.is_some() && self.token == document.cache_token());

        same_document
            && self.focused_id.as_ref() == focused_id
            && self.content_width == content_width
            && self.focused == ctx.focused
            && self.tab_selections == ctx.tab_selections
            && self.config == ConfigFingerprint::of(ctx.config)
    }
}

/// How many documents' renders are kept. The TUI shows one document per
/// widget at a time and ids form a small set (one per standings view, one
/// per stacked-document kind), so a handful of entries covers everything on
/// screen; the cap just bounds memory if ids turn out to be data-dependent.
const RENDER_CACHE_CAPACITY: usize = 8;

/// Cross-frame cache of full-height document renders, keyed by document id.
///
/// Owned by the TUI run loop and threaded to [`DocumentView::render`]
/// through [`RenderContext::child`]. A hit skips `Document::build` and the
/// offscreen render entirely -- scrolling and idle redraws become a pure
/// viewport copy. Any changed input (data identity, focused element, width,
/// focus flag, tab selections, theme/unicode config) misses and re-renders
/// exactly once.
#[derive(Default)]
pub struct DocumentRenderCache {
    /// Insertion-ordered; linear scans are fine at this size.
    entries: Vec<(String, CacheEntry)>,
}

impl DocumentRenderCache {
    /// Return a valid cached entry for the document, rendering and storing a
    /// fresh one first if anything relevant changed since the last frame.
    fn entry_for(
        &mut self,
        document: &Arc<dyn Document>,
        focused_id: Option<&FocusableId>,
        content_width: u16,
        ctx: &RenderContext,
    ) -> &CacheEntry {
        let id = document.id();
        let pos = self.entries.iter().position(|(k, _)| *k == id.as_ref());

        if let Some(i) = pos {
            if self.entries[i]
                .1
                .matches(document, focused_id, content_width, ctx)
            {
                return &self.entries[i].1;
            }
        }

        let (full_buffer, height) =
            build_full_document(document.as_ref(), focused_id, content_width, ctx);
        let entry = CacheEntry {
            document: Arc::clone(document),
            token: document.cache_token(),
            focused_id: focused_id.cloned(),
            content_width,
            focused: ctx.focused,
            tab_selections: ctx.tab_selections.clone(),
            config: ConfigFingerprint::of(ctx.config),
            full_buffer,
            height,
        };

        match pos {
            Some(i) => {
                self.entries[i].1 = entry;
                &self.entries[i].1
            }
            None => {
                if self.entries.len() >= RENDER_CACHE_CAPACITY {
                    self.entries.remove(0);
                }
                self.entries.push((id.into_owned(), entry));
                &self.entries.last().expect("just pushed").1
            }
        }
    }
}

/// Render shim that pairs a document with a viewport offset and focused
/// element.
///
/// `DocumentView` holds no navigation logic of its own -- all focus/scroll
/// semantics (next/prev, wrap, paging, autoscroll) live in `document_nav.rs`
/// and its `DocumentNavState`. Each frame, the render path constructs a
/// fresh `DocumentView`, injects the current focused element's ID and the
/// scroll offset computed by `DocumentNavState`, and renders. This keeps
/// exactly one navigation engine: `DocumentNavState` decides "where",
/// `DocumentView` just draws "what's visible from here".
///
/// Focused ID and scroll offset are only *requested* before `render` --
/// neither can be resolved into a `Document::build` call until `render`
/// knows the real content width, so `DocumentView` stores them as pending
/// state rather than building early. `render` then obtains the full-height
/// buffer from the [`DocumentRenderCache`] when one is attached to the
/// context (reused across frames while inputs are unchanged), or builds it
/// directly via [`build_full_document`] otherwise.
pub struct DocumentView {
    document: Arc<dyn Document>,
    /// Height of the visible viewport, supplied at construction. Combined
    /// with `pending_scroll_offset` and the document's real height (known
    /// only once `render` builds the element tree) to size the `Viewport`.
    viewport_height: u16,
    /// Focused element requested via `focus_id`, baked into the element
    /// tree once `render` builds it.
    pending_focused_id: Option<FocusableId>,
    /// Scroll offset requested via `set_scroll_offset`, clamped once
    /// `render` knows the document's real height.
    pending_scroll_offset: u16,
}

impl DocumentView {
    /// Create a new document view
    ///
    /// # Arguments
    /// - `document`: The document to display
    /// - `viewport_height`: Height of the visible viewport
    pub fn new(document: Arc<dyn Document>, viewport_height: u16) -> Self {
        // No build here: at construction time neither the real content
        // width (known only once `render` sees its `area`) nor the focused
        // element (set afterwards via `focus_id`) are known yet, so a
        // build performed now would just be discarded. `render` does the
        // one build that matters, using the pending state recorded below.
        Self {
            document,
            viewport_height,
            pending_focused_id: None,
            pending_scroll_offset: 0,
        }
    }

    /// Focus a specific element by ID (resolved by the navigation layer
    /// from `DocumentNavState.focusables`, which is built from the same
    /// document).
    pub fn focus_id(&mut self, id: FocusableId) {
        self.pending_focused_id = Some(id);
    }

    /// Set the scroll offset directly (computed by `document_nav.rs`)
    pub fn set_scroll_offset(&mut self, offset: u16) {
        self.pending_scroll_offset = offset;
    }

    // === Rendering ===

    /// Render the visible portion of the document.
    ///
    /// With a [`DocumentRenderCache`] attached to `ctx` (the TUI run loop
    /// always attaches one), the full-height render is reused across frames
    /// while its inputs are unchanged -- in particular, scrolling and idle
    /// redraws skip `Document::build` entirely, since the scroll offset only
    /// affects which slice of the full buffer is copied out below.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        const HORIZONTAL_MARGIN: u16 = 1;

        // Account for left and right margins
        let content_width = area.width.saturating_sub(HORIZONTAL_MARGIN * 2);
        if content_width == 0 {
            return;
        }

        match ctx.doc_cache {
            Some(cache) => {
                let mut cache = cache.borrow_mut();
                let entry = cache.entry_for(
                    &self.document,
                    self.pending_focused_id.as_ref(),
                    content_width,
                    ctx,
                );
                self.blit_visible(area, buf, ctx, &entry.full_buffer, entry.height);
            }
            None => {
                let (full_buffer, height) = build_full_document(
                    self.document.as_ref(),
                    self.pending_focused_id.as_ref(),
                    content_width,
                    ctx,
                );
                self.blit_visible(area, buf, ctx, &full_buffer, height);
            }
        }

        // Focus highlighting is handled by the elements themselves
        // (Link.focused, TableWidget.focused_row), baked in by build().
    }

    /// Copy the visible slice of the full-height buffer into `buf`, filling
    /// the one-cell horizontal margins with the background style.
    fn blit_visible(
        &self,
        area: Rect,
        buf: &mut Buffer,
        ctx: &RenderContext,
        full_buffer: &Buffer,
        height: u16,
    ) {
        const HORIZONTAL_MARGIN: u16 = 1;
        let content_width = area.width.saturating_sub(HORIZONTAL_MARGIN * 2);

        let viewport = Viewport::new(self.pending_scroll_offset, self.viewport_height, height);

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
        let visible_range = viewport.visible_range();
        let visible_start = visible_range.start;

        for y in visible_range {
            if y >= height {
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
    }
}

/// Shared state for [`render_document_widget`], the render body common to every
/// "stacked document" widget (team detail, player detail, boxscore): show a
/// loading animation until a document arrives, then hand it to a
/// [`DocumentView`] with the given focus/scroll applied.
pub struct DocumentWidgetParams<'a> {
    /// Pre-built content document, or `None` while data hasn't arrived yet.
    pub document: &'a Option<Arc<dyn Document>>,
    pub loading: bool,
    /// The focused element's ID, resolved from `DocumentNavState` (see
    /// `DocumentNavState::focused_id`).
    pub focused_id: Option<FocusableId>,
    pub scroll_offset: u16,
    pub animation_frame: u8,
    /// Whether this widget has focus (affects dim/bright rendering)
    pub focused: bool,
}

/// Render a document-backed widget: a loading spinner while `params.document`
/// is `None` (or `params.loading` is set), otherwise the built document via
/// `DocumentView` with focus and scroll applied.
///
/// Extracted from `TeamDetailDocumentWidget`, `PlayerDetailDocumentWidget`, and
/// `BoxscoreDocumentWidget`, whose render bodies were otherwise identical.
pub fn render_document_widget(
    params: &DocumentWidgetParams,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let child_ctx = ctx.child(params.focused);

    let Some(document) = params.document else {
        LoadingAnimation::new(params.animation_frame).render(area, buf, &child_ctx);
        return;
    };
    if params.loading {
        LoadingAnimation::new(params.animation_frame).render(area, buf, &child_ctx);
        return;
    }

    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut view = DocumentView::new(document.clone(), area.height);
    if let Some(id) = params.focused_id.clone() {
        view.focus_id(id);
    }
    view.set_scroll_offset(params.scroll_offset);
    view.render(area, buf, &child_ctx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::testing::assert_buffer;

    // === assert_buffer rendering tests ===
    //
    // These exercise the surviving render shim end to end: `focus_id` /
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

        fn title(&self) -> Cow<'static, str> {
            Cow::Owned(self.title.clone())
        }

        fn id(&self) -> Cow<'static, str> {
            Cow::Borrowed("render_test")
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

        fn title(&self) -> Cow<'static, str> {
            Cow::Borrowed("Link Test")
        }

        fn id(&self) -> Cow<'static, str> {
            Cow::Borrowed("link_test")
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
        view.focus_id(FocusableId::link("test_link"));

        let area = Rect::new(0, 0, 17, 3);
        let mut buf = Buffer::empty(area);

        view.render(area, &mut buf, &ctx);

        // Focused link has "▶ " prefix
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Before", " ▶ Click Me", " After"]);
    }

    // === render_document_widget tests ===
    //
    // Shared by TeamDetailDocumentWidget, PlayerDetailDocumentWidget, and
    // BoxscoreDocumentWidget; these exercise the seam directly instead of
    // through each widget's own render() (which now just forwards here).

    fn base_params(document: &Option<Arc<dyn Document>>) -> DocumentWidgetParams<'_> {
        DocumentWidgetParams {
            document,
            loading: false,
            focused_id: None,
            scroll_offset: 0,
            animation_frame: 0,
            focused: true,
        }
    }

    #[test]
    fn test_render_document_widget_shows_animation_when_document_is_none() {
        let document: Option<Arc<dyn Document>> = None;
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);

        render_document_widget(&base_params(&document), area, &mut buf, &ctx);

        // Loading animation renders some non-blank content; the important
        // thing is that it doesn't panic on a missing document.
        assert_eq!(*buf.area(), area);
    }

    #[test]
    fn test_render_document_widget_shows_animation_when_loading_even_with_document() {
        let document: Option<Arc<dyn Document>> =
            Some(Arc::new(RenderTestDocument::new("Test", vec!["Line 1"])));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);

        let mut params = base_params(&document);
        params.loading = true;
        render_document_widget(&params, area, &mut buf, &ctx);

        // Loading takes priority even though a document is present; the
        // document's own content ("Line 1") should not appear.
        assert_eq!(*buf.area(), area);
    }

    #[test]
    fn test_render_document_widget_renders_document_when_present() {
        let document: Option<Arc<dyn Document>> = Some(Arc::new(RenderTestDocument::new(
            "Test",
            vec!["Line 1", "Line 2"],
        )));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        let area = Rect::new(0, 0, 12, 4);
        let mut buf = Buffer::empty(area);

        render_document_widget(&base_params(&document), area, &mut buf, &ctx);

        assert_buffer(&buf, &[" Test", " ════", " Line 1", " Line 2"]);
    }

    #[test]
    fn test_render_document_widget_zero_area_does_not_panic() {
        let document: Option<Arc<dyn Document>> =
            Some(Arc::new(RenderTestDocument::new("Test", vec!["Line 1"])));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Zero-height area: should bail out before touching DocumentView.
        let area = Rect::new(0, 0, 12, 0);
        let mut buf = Buffer::empty(area);

        render_document_widget(&base_params(&document), area, &mut buf, &ctx);
    }

    // === DocumentRenderCache tests ===

    use std::cell::RefCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Document that counts build() calls, with content and identity driven
    /// by a data Arc so tests can exercise the cache_token path.
    struct CountingDocument {
        id: &'static str,
        data: Arc<Vec<String>>,
        builds: Arc<AtomicUsize>,
        /// Whether to expose a cache_token (per-frame-recreated documents do).
        tokenized: bool,
    }

    impl Document for CountingDocument {
        fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
            self.builds.fetch_add(1, Ordering::SeqCst);
            let mut elements = Vec::new();
            for (i, line) in self.data.iter().enumerate() {
                let link_id = format!("link_{i}");
                elements.push(if focus.is_link_focused(&link_id) {
                    DocumentElement::focused_link(&link_id, line, LinkTarget::Anchor(line.clone()))
                } else {
                    DocumentElement::link(&link_id, line, LinkTarget::Anchor(line.clone()))
                });
            }
            elements
        }

        fn title(&self) -> Cow<'static, str> {
            Cow::Borrowed("Counting")
        }

        fn id(&self) -> Cow<'static, str> {
            Cow::Borrowed(self.id)
        }

        fn cache_token(&self) -> Option<Vec<usize>> {
            self.tokenized
                .then(|| vec![Arc::as_ptr(&self.data) as usize])
        }
    }

    fn counting_doc(
        id: &'static str,
        data: &Arc<Vec<String>>,
        builds: &Arc<AtomicUsize>,
        tokenized: bool,
    ) -> Arc<dyn Document> {
        Arc::new(CountingDocument {
            id,
            data: Arc::clone(data),
            builds: Arc::clone(builds),
            tokenized,
        })
    }

    fn lines(strs: &[&str]) -> Arc<Vec<String>> {
        Arc::new(strs.iter().map(|s| s.to_string()).collect())
    }

    /// Render `doc` once through a cache-carrying context and return the buffer.
    fn render_with_cache(
        doc: &Arc<dyn Document>,
        cache: &RefCell<DocumentRenderCache>,
        config: &DisplayConfig,
        focused_id: Option<FocusableId>,
        scroll_offset: u16,
        width: u16,
    ) -> Buffer {
        let ctx = RenderContext::focused(config).with_doc_cache(cache);
        let area = Rect::new(0, 0, width, 5);
        let mut buf = Buffer::empty(area);
        let mut view = DocumentView::new(Arc::clone(doc), area.height);
        if let Some(id) = focused_id {
            view.focus_id(id);
        }
        view.set_scroll_offset(scroll_offset);
        view.render(area, &mut buf, &ctx);
        buf
    }

    #[test]
    fn test_cache_hit_on_identical_rerender_skips_build() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One", "Two"]);
        let doc = counting_doc("cache_hit", &data, &builds, false);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        let first = render_with_cache(&doc, &cache, &config, None, 0, 12);
        assert_eq!(builds.load(Ordering::SeqCst), 1);

        let second = render_with_cache(&doc, &cache, &config, None, 0, 12);
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "second render must not rebuild"
        );
        assert_eq!(first, second, "cached render must be identical");
    }

    #[test]
    fn test_cache_hit_when_only_scroll_offset_changes() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One", "Two", "Three", "Four", "Five", "Six", "Seven"]);
        let doc = counting_doc("cache_scroll", &data, &builds, false);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        render_with_cache(&doc, &cache, &config, None, 0, 12);
        let scrolled = render_with_cache(&doc, &cache, &config, None, 2, 12);

        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "scrolling must reuse the cached full render"
        );
        // Offset 2 shows lines starting at "Three" (unfocused links have a
        // two-space alignment prefix, plus the one-cell view margin).
        assert_buffer(
            &scrolled,
            &["   Three", "   Four", "   Five", "   Six", "   Seven"],
        );
    }

    #[test]
    fn test_cache_miss_on_width_change() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One"]);
        let doc = counting_doc("cache_width", &data, &builds, false);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        render_with_cache(&doc, &cache, &config, None, 0, 12);
        render_with_cache(&doc, &cache, &config, None, 0, 20);

        assert_eq!(
            builds.load(Ordering::SeqCst),
            2,
            "width change must rebuild"
        );
    }

    #[test]
    fn test_cache_miss_on_focused_id_change() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One", "Two"]);
        let doc = counting_doc("cache_focus", &data, &builds, false);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        render_with_cache(&doc, &cache, &config, None, 0, 12);
        assert_eq!(builds.load(Ordering::SeqCst), 1);

        // Focusing rebuilds exactly once (focus styling is baked into the
        // tree, but the focused ID is supplied directly -- no preliminary
        // unfocused build)...
        let focused = render_with_cache(
            &doc,
            &cache,
            &config,
            Some(FocusableId::link("link_1")),
            0,
            12,
        );
        assert_eq!(builds.load(Ordering::SeqCst), 2);
        assert_buffer(&focused, &["   One", " ▶ Two", "", "", ""]);

        // ...and re-rendering with the same focus is a hit again.
        render_with_cache(
            &doc,
            &cache,
            &config,
            Some(FocusableId::link("link_1")),
            0,
            12,
        );
        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_cache_token_hits_across_recreated_documents() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One"]);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        // Two distinct Arc<dyn Document> wrapping the same data Arc, as the
        // standings widgets produce every frame.
        let doc_a = counting_doc("cache_token", &data, &builds, true);
        let doc_b = counting_doc("cache_token", &data, &builds, true);
        render_with_cache(&doc_a, &cache, &config, None, 0, 12);
        render_with_cache(&doc_b, &cache, &config, None, 0, 12);
        assert_eq!(
            builds.load(Ordering::SeqCst),
            1,
            "equal tokens must hit across recreated documents"
        );

        // New data Arc -> different token -> rebuild.
        let new_data = lines(&["One"]);
        let doc_c = counting_doc("cache_token", &new_data, &builds, true);
        render_with_cache(&doc_c, &cache, &config, None, 0, 12);
        assert_eq!(builds.load(Ordering::SeqCst), 2, "new data must rebuild");
    }

    #[test]
    fn test_untokenized_recreated_documents_always_rebuild() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One"]);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        let doc_a = counting_doc("cache_untokenized", &data, &builds, false);
        let doc_b = counting_doc("cache_untokenized", &data, &builds, false);
        render_with_cache(&doc_a, &cache, &config, None, 0, 12);
        render_with_cache(&doc_b, &cache, &config, None, 0, 12);

        assert_eq!(
            builds.load(Ordering::SeqCst),
            2,
            "distinct Arcs without a token must not be treated as identical"
        );
    }

    #[test]
    fn test_cache_miss_on_config_change() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One"]);
        let doc = counting_doc("cache_config", &data, &builds, false);
        let cache = RefCell::new(DocumentRenderCache::default());

        let config = DisplayConfig::default();
        render_with_cache(&doc, &cache, &config, None, 0, 12);

        let changed = DisplayConfig {
            use_unicode: false,
            ..DisplayConfig::default()
        };
        render_with_cache(&doc, &cache, &changed, None, 0, 12);

        assert_eq!(
            builds.load(Ordering::SeqCst),
            2,
            "config change must rebuild"
        );
    }

    #[test]
    fn test_cache_miss_on_focused_flag_change() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One"]);
        let doc = counting_doc("cache_dim", &data, &builds, false);
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        render_with_cache(&doc, &cache, &config, None, 0, 12);

        // Same inputs but rendered through an unfocused (dimmed) context.
        let ctx = RenderContext::new(&config, false).with_doc_cache(&cache);
        let area = Rect::new(0, 0, 12, 5);
        let mut buf = Buffer::empty(area);
        let mut view = DocumentView::new(Arc::clone(&doc), area.height);
        view.render(area, &mut buf, &ctx);

        assert_eq!(
            builds.load(Ordering::SeqCst),
            2,
            "focused/dim change must rebuild"
        );
    }

    #[test]
    fn test_cache_evicts_oldest_at_capacity() {
        let builds = Arc::new(AtomicUsize::new(0));
        let cache = RefCell::new(DocumentRenderCache::default());
        let config = DisplayConfig::default();

        const IDS: [&str; 9] = [
            "evict_0", "evict_1", "evict_2", "evict_3", "evict_4", "evict_5", "evict_6", "evict_7",
            "evict_8",
        ];
        let data = lines(&["One"]);
        let docs: Vec<Arc<dyn Document>> = IDS
            .iter()
            .map(|id| counting_doc(id, &data, &builds, false))
            .collect();

        // Fill past capacity (8): the first entry gets evicted.
        for doc in &docs {
            render_with_cache(doc, &cache, &config, None, 0, 12);
        }
        assert_eq!(builds.load(Ordering::SeqCst), 9);

        // The most recent document is still cached...
        render_with_cache(&docs[8], &cache, &config, None, 0, 12);
        assert_eq!(builds.load(Ordering::SeqCst), 9);

        // ...but the evicted first one rebuilds.
        render_with_cache(&docs[0], &cache, &config, None, 0, 12);
        assert_eq!(builds.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_render_without_cache_rebuilds_every_frame() {
        let builds = Arc::new(AtomicUsize::new(0));
        let data = lines(&["One"]);
        let doc = counting_doc("no_cache", &data, &builds, true);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let area = Rect::new(0, 0, 12, 5);
        for _ in 0..2 {
            let mut buf = Buffer::empty(area);
            let mut view = DocumentView::new(Arc::clone(&doc), area.height);
            view.render(area, &mut buf, &ctx);
        }

        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }
}
