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
