//! Viewport bookkeeping for document scrolling
//!
//! Tracks the visible region of a document: current offset, viewport height,
//! and content height. Offset is always clamped so the viewport never
//! scrolls past the end of the content.
//!
//! This is pure render-time bookkeeping. Deciding *where* to scroll to
//! (autoscroll-to-focus, paging, wrap behavior) is `document_nav.rs`'s job;
//! this type just stores the resulting offset and answers "what's visible".

use std::ops::Range;

/// Viewport that tracks the visible region of a document for rendering
#[derive(Debug, Clone)]
pub struct Viewport {
    /// Current scroll offset from top
    offset: u16,
    /// Height of the viewport (visible area)
    height: u16,
    /// Total height of the content
    content_height: u16,
}

impl Viewport {
    /// Create a new viewport
    ///
    /// # Arguments
    /// - `offset`: Initial scroll offset from top
    /// - `height`: Height of the viewport (visible area)
    /// - `content_height`: Total height of the content
    pub fn new(offset: u16, height: u16, content_height: u16) -> Self {
        Self {
            offset: offset.min(content_height.saturating_sub(height)),
            height,
            content_height,
        }
    }

    /// Get the range of visible lines
    pub fn visible_range(&self) -> Range<u16> {
        self.offset
            ..self
                .offset
                .saturating_add(self.height)
                .min(self.content_height)
    }

    /// Set a new offset directly, clamped to the valid range
    pub fn set_offset(&mut self, offset: u16) {
        let max_offset = self.content_height.saturating_sub(self.height);
        self.offset = offset.min(max_offset);
    }

    /// Set the content height (e.g., when document changes), re-clamping the offset
    pub fn set_content_height(&mut self, height: u16) {
        self.content_height = height;
        let max_offset = height.saturating_sub(self.height);
        self.offset = self.offset.min(max_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_new() {
        let viewport = Viewport::new(10, 20, 100);

        assert_eq!(viewport.offset, 10);
        assert_eq!(viewport.height, 20);
        assert_eq!(viewport.content_height, 100);
    }

    #[test]
    fn test_viewport_new_clamps_offset() {
        // Offset would put us past end of content
        let viewport = Viewport::new(95, 20, 100);

        // Should be clamped to 80 (100 - 20)
        assert_eq!(viewport.offset, 80);
    }

    #[test]
    fn test_visible_range() {
        let viewport = Viewport::new(10, 20, 100);
        assert_eq!(viewport.visible_range(), 10..30);
    }

    #[test]
    fn test_visible_range_clamps_at_end() {
        let viewport = Viewport::new(90, 20, 100);
        // offset is clamped to 80
        assert_eq!(viewport.visible_range(), 80..100);
    }

    #[test]
    fn test_set_offset() {
        let mut viewport = Viewport::new(0, 20, 100);
        viewport.set_offset(50);
        assert_eq!(viewport.offset, 50);
    }

    #[test]
    fn test_set_offset_clamps() {
        let mut viewport = Viewport::new(0, 20, 100);
        viewport.set_offset(200);
        assert_eq!(viewport.offset, 80);
    }

    #[test]
    fn test_set_content_height() {
        let mut viewport = Viewport::new(50, 20, 100);
        viewport.set_content_height(150);
        assert_eq!(viewport.content_height, 150);
        assert_eq!(viewport.offset, 50);
    }

    #[test]
    fn test_set_content_height_adjusts_offset() {
        let mut viewport = Viewport::new(80, 20, 100);
        viewport.set_content_height(50);
        // Max offset is now 30, so offset should be clamped
        assert_eq!(viewport.offset, 30);
    }

    #[test]
    fn test_viewport_with_content_smaller_than_viewport() {
        let viewport = Viewport::new(0, 50, 30);
        assert_eq!(viewport.offset, 0);
        assert_eq!(viewport.visible_range(), 0..30);
    }
}
