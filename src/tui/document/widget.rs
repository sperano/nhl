//! DocumentElementWidget - Bridge between Document system and Element tree
//!
//! This widget implements `ElementWidget` to allow documents to be embedded
//! in the component tree. It manages rendering a document with viewport scrolling
//! and focus highlighting.

use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::RenderContext;
use crate::tui::component::ElementWidget;

use super::{Document, DocumentView};

/// A widget that renders a document within the Element tree
///
/// This is the standard bridge between the Document system and the component system.
/// It handles:
/// - Creating a DocumentView for rendering
/// - Applying focus state from props
/// - Applying scroll offset from props
/// - Rendering with viewport clipping
///
/// # Example
///
/// ```ignore
/// let widget = DocumentElementWidget::new(
///     Arc::new(MyDocument::new()),
///     Some(2),  // focus on element 2
///     10,       // scroll offset
/// );
///
/// Element::Widget(Box::new(widget))
/// ```
#[derive(Clone)]
pub struct DocumentElementWidget {
    document: Arc<dyn Document>,
    focus_index: Option<usize>,
    scroll_offset: u16,
}

impl DocumentElementWidget {
    /// Create a new document widget
    ///
    /// # Arguments
    /// - `document`: The document to render
    /// - `focus_index`: Index of the currently focused element (None = no focus)
    /// - `scroll_offset`: Current scroll offset in lines
    pub fn new(
        document: Arc<dyn Document>,
        focus_index: Option<usize>,
        scroll_offset: u16,
    ) -> Self {
        Self {
            document,
            focus_index,
            scroll_offset,
        }
    }
}

impl ElementWidget for DocumentElementWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        let mut view = DocumentView::new(self.document.clone(), area.height);

        // Apply focus state
        if let Some(idx) = self.focus_index {
            view.focus_by_index(idx);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Render the document to the buffer
        view.render(area, buf, ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }

    fn preferred_height(&self) -> Option<u16> {
        // Documents can expand to fill available space
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::document::{DocumentBuilder, DocumentElement, FocusContext};
    use crate::tui::testing::assert_buffer;

    /// Simple test document
    struct TestDoc {
        title: String,
        lines: Vec<String>,
    }

    impl TestDoc {
        fn new(title: &str, lines: Vec<&str>) -> Self {
            Self {
                title: title.to_string(),
                lines: lines.into_iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl Document for TestDoc {
        fn build(&self, _focus: &FocusContext) -> Vec<DocumentElement> {
            DocumentBuilder::new()
                .heading(1, &self.title)
                .for_each(&self.lines, |b, line| b.text(line))
                .build()
        }

        fn title(&self) -> String {
            self.title.clone()
        }

        fn id(&self) -> String {
            "test".to_string()
        }
    }

    #[test]
    fn test_document_element_widget_render() {
        let doc = Arc::new(TestDoc::new("Hello", vec!["Line 1", "Line 2"]));
        let widget = DocumentElementWidget::new(doc, None, 0);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let area = Rect::new(0, 0, 12, 4);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &ctx);

        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" Hello", " ═════", " Line 1", " Line 2"]);
    }

    #[test]
    fn test_document_element_widget_with_scroll() {
        let doc = Arc::new(TestDoc::new("Title", vec!["A", "B", "C", "D"]));
        let widget = DocumentElementWidget::new(doc, None, 2);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let area = Rect::new(0, 0, 12, 3);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf, &ctx);

        // Scrolled past title + underline, showing lines A, B, C
        // Content has 1-char left and right margins
        assert_buffer(&buf, &[" A", " B", " C"]);
    }

    #[test]
    fn test_document_element_widget_clone_box() {
        let doc = Arc::new(TestDoc::new("Test", vec![]));
        let widget = DocumentElementWidget::new(doc, Some(5), 10);

        let cloned = widget.clone_box();

        // Verify it compiles and doesn't panic
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        let area = Rect::new(0, 0, 10, 4);
        let mut buf = Buffer::empty(area);
        cloned.render(area, &mut buf, &ctx);
    }
}
