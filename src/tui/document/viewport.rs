//! Viewport management for document scrolling
//!
//! Manages the visible region of a document, handling scrolling operations
//! and ensuring focused elements remain visible with appropriate padding.

use ratatui::layout::Rect;
use std::ops::Range;

/// Maximum padding as a divisor of viewport height (25% = 4)
const MAX_PADDING_DIVISOR: u16 = 4;

/// Smart padding thresholds and values
const SMALL_VIEWPORT_THRESHOLD: u16 = 10;
const MEDIUM_VIEWPORT_THRESHOLD: u16 = 20;
const LARGE_VIEWPORT_THRESHOLD: u16 = 40;

const SMALL_VIEWPORT_PADDING: u16 = 1;
const MEDIUM_VIEWPORT_PADDING: u16 = 2;
const LARGE_VIEWPORT_PADDING: u16 = 3;
const VERY_LARGE_VIEWPORT_PADDING: u16 = 5;

/// Viewport that manages scrolling through document content
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

    /// Check if a rectangle is at least partially visible
    pub fn is_rect_visible(&self, rect: &Rect) -> bool {
        let visible = self.visible_range();
        rect.y < visible.end && rect.y + rect.height > visible.start
    }

    /// Ensure a line or region is visible, scrolling if necessary
    ///
    /// Basic version that just ensures the element is visible without padding.
    pub fn ensure_visible(&mut self, y: u16, height: u16) {
        let bottom = y + height;

        // If above viewport, scroll up
        if y < self.offset {
            self.offset = y;
        }
        // If below viewport, scroll down
        else if bottom > self.offset + self.height {
            self.offset = bottom.saturating_sub(self.height);
        }
    }

    /// Ensure a line or region is visible with smart positioning
    ///
    /// Uses padding to avoid putting focused element at the very edge of the viewport.
    /// This provides better UX by keeping context visible around the focused element.
    ///
    /// # Arguments
    /// - `y`: The y-coordinate of the element's top
    /// - `height`: The height of the element
    /// - `padding`: Number of lines of padding to maintain around the element
    pub fn ensure_visible_with_padding(&mut self, y: u16, height: u16, padding: u16) {
        let element_top = y;
        let element_bottom = y + height;
        let viewport_top = self.offset;
        let viewport_bottom = self.offset + self.height;

        // Calculate ideal padding - max 25% of viewport
        let max_padding = self.height / MAX_PADDING_DIVISOR;
        let actual_padding = padding.min(max_padding);

        // If element is above viewport, scroll up to show it with padding at top
        if element_top < viewport_top {
            self.offset = element_top.saturating_sub(actual_padding);
        }
        // If element is below viewport, scroll down to show it with padding at bottom
        else if element_bottom > viewport_bottom {
            let desired_offset = element_bottom + actual_padding;
            let max_offset = self.content_height.saturating_sub(self.height);
            self.offset = desired_offset.saturating_sub(self.height).min(max_offset);
        }
        // If element is partially visible but needs adjustment
        else if element_top < viewport_top + actual_padding && element_top > viewport_top {
            // Element is near top edge - add more padding if possible
            self.offset = element_top.saturating_sub(actual_padding);
        } else if element_bottom > viewport_bottom.saturating_sub(actual_padding)
            && element_bottom < viewport_bottom
        {
            // Element is near bottom edge - add more padding if possible
            let desired_offset = element_bottom + actual_padding;
            let max_offset = self.content_height.saturating_sub(self.height);
            self.offset = desired_offset.saturating_sub(self.height).min(max_offset);
        }
    }

    /// Scroll up by a number of lines
    pub fn scroll_up(&mut self, lines: u16) {
        self.offset = self.offset.saturating_sub(lines);
    }

    /// Scroll down by a number of lines
    pub fn scroll_down(&mut self, lines: u16) {
        let max_offset = self.content_height.saturating_sub(self.height);
        self.offset = (self.offset + lines).min(max_offset);
    }

    /// Scroll to the top of the document
    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    /// Scroll to the bottom of the document
    pub fn scroll_to_bottom(&mut self) {
        self.offset = self.content_height.saturating_sub(self.height);
    }

    /// Get the current scroll offset
    pub fn offset(&self) -> u16 {
        self.offset
    }

    /// Get the viewport height
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get the content height
    pub fn content_height(&self) -> u16 {
        self.content_height
    }

    /// Set a new offset directly
    pub fn set_offset(&mut self, offset: u16) {
        let max_offset = self.content_height.saturating_sub(self.height);
        self.offset = offset.min(max_offset);
    }

    /// Set the viewport height (e.g., on terminal resize)
    pub fn set_height(&mut self, height: u16) {
        self.height = height;
        // Adjust offset if necessary to keep it valid
        let max_offset = self.content_height.saturating_sub(height);
        self.offset = self.offset.min(max_offset);
    }

    /// Set the content height (e.g., when document changes)
    pub fn set_content_height(&mut self, height: u16) {
        self.content_height = height;
        // Adjust offset if necessary to keep it valid
        let max_offset = height.saturating_sub(self.height);
        self.offset = self.offset.min(max_offset);
    }

    /// Check if the viewport is at the top
    pub fn is_at_top(&self) -> bool {
        self.offset == 0
    }

    /// Check if the viewport is at the bottom
    pub fn is_at_bottom(&self) -> bool {
        self.offset >= self.content_height.saturating_sub(self.height)
    }

    /// Calculate smart padding based on viewport height
    ///
    /// Returns appropriate padding for autoscrolling:
    /// - Small viewports (<=10): 1 line
    /// - Medium viewports (<=20): 2 lines
    /// - Large viewports (<=40): 3 lines
    /// - Very large viewports: 5 lines
    pub fn smart_padding(&self) -> u16 {
        match self.height {
            h if h <= SMALL_VIEWPORT_THRESHOLD => SMALL_VIEWPORT_PADDING,
            h if h <= MEDIUM_VIEWPORT_THRESHOLD => MEDIUM_VIEWPORT_PADDING,
            h if h <= LARGE_VIEWPORT_THRESHOLD => LARGE_VIEWPORT_PADDING,
            _ => VERY_LARGE_VIEWPORT_PADDING,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_new() {
        let viewport = Viewport::new(10, 20, 100);

        assert_eq!(viewport.offset(), 10);
        assert_eq!(viewport.height(), 20);
        assert_eq!(viewport.content_height(), 100);
    }

    #[test]
    fn test_viewport_new_clamps_offset() {
        // Offset would put us past end of content
        let viewport = Viewport::new(95, 20, 100);

        // Should be clamped to 80 (100 - 20)
        assert_eq!(viewport.offset(), 80);
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
    fn test_is_rect_visible_fully_visible() {
        let viewport = Viewport::new(20, 10, 100);
        let rect = Rect::new(0, 22, 10, 3);
        assert!(viewport.is_rect_visible(&rect));
    }

    #[test]
    fn test_is_rect_visible_partially_top() {
        let viewport = Viewport::new(20, 10, 100);
        // Rect starts at 18, viewport starts at 20 - partially visible
        let rect = Rect::new(0, 18, 10, 5);
        assert!(viewport.is_rect_visible(&rect));
    }

    #[test]
    fn test_is_rect_visible_partially_bottom() {
        let viewport = Viewport::new(20, 10, 100);
        // Rect ends at 33, viewport ends at 30 - partially visible
        let rect = Rect::new(0, 28, 10, 5);
        assert!(viewport.is_rect_visible(&rect));
    }

    #[test]
    fn test_is_rect_visible_not_visible_above() {
        let viewport = Viewport::new(20, 10, 100);
        // Rect is entirely above viewport
        let rect = Rect::new(0, 10, 10, 5);
        assert!(!viewport.is_rect_visible(&rect));
    }

    #[test]
    fn test_is_rect_visible_not_visible_below() {
        let viewport = Viewport::new(20, 10, 100);
        // Rect is entirely below viewport
        let rect = Rect::new(0, 35, 10, 5);
        assert!(!viewport.is_rect_visible(&rect));
    }

    #[test]
    fn test_ensure_visible_element_above() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible(15, 2);
        assert_eq!(viewport.offset(), 15);
    }

    #[test]
    fn test_ensure_visible_element_below() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible(35, 2);
        // bottom = 37, need offset where 37 is at viewport bottom
        // offset = 37 - 10 = 27
        assert_eq!(viewport.offset(), 27);
    }

    #[test]
    fn test_ensure_visible_element_already_visible() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible(22, 2);
        // Should not change
        assert_eq!(viewport.offset(), 20);
    }

    #[test]
    fn test_ensure_visible_with_padding_element_above() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible_with_padding(15, 2, 2);
        // Should scroll to show element with padding: 15 - 2 = 13
        assert_eq!(viewport.offset(), 13);
    }

    #[test]
    fn test_ensure_visible_with_padding_element_below() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.ensure_visible_with_padding(35, 2, 2);
        // element_bottom = 37, desired_offset = 37 + 2 = 39
        // offset = 39 - 10 = 29
        assert_eq!(viewport.offset(), 29);
    }

    #[test]
    fn test_ensure_visible_with_padding_clamps_padding() {
        let mut viewport = Viewport::new(0, 10, 100);
        // Requesting 10 lines of padding, but max is 25% of viewport = 2
        viewport.ensure_visible_with_padding(50, 2, 10);
        // element_bottom = 52, max_padding = 2, desired = 52 + 2 = 54
        // offset = 54 - 10 = 44
        assert_eq!(viewport.offset(), 44);
    }

    #[test]
    fn test_scroll_up() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.scroll_up(5);
        assert_eq!(viewport.offset(), 15);
    }

    #[test]
    fn test_scroll_up_clamps_at_zero() {
        let mut viewport = Viewport::new(5, 10, 100);
        viewport.scroll_up(10);
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn test_scroll_down() {
        let mut viewport = Viewport::new(20, 10, 100);
        viewport.scroll_down(5);
        assert_eq!(viewport.offset(), 25);
    }

    #[test]
    fn test_scroll_down_clamps_at_bottom() {
        let mut viewport = Viewport::new(85, 20, 100);
        viewport.scroll_down(20);
        assert_eq!(viewport.offset(), 80);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut viewport = Viewport::new(50, 20, 100);
        viewport.scroll_to_top();
        assert_eq!(viewport.offset(), 0);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut viewport = Viewport::new(20, 20, 100);
        viewport.scroll_to_bottom();
        assert_eq!(viewport.offset(), 80);
    }

    #[test]
    fn test_set_offset() {
        let mut viewport = Viewport::new(0, 20, 100);
        viewport.set_offset(50);
        assert_eq!(viewport.offset(), 50);
    }

    #[test]
    fn test_set_offset_clamps() {
        let mut viewport = Viewport::new(0, 20, 100);
        viewport.set_offset(200);
        assert_eq!(viewport.offset(), 80);
    }

    #[test]
    fn test_set_height() {
        let mut viewport = Viewport::new(50, 20, 100);
        viewport.set_height(30);
        assert_eq!(viewport.height(), 30);
        assert_eq!(viewport.offset(), 50);
    }

    #[test]
    fn test_set_height_adjusts_offset() {
        let mut viewport = Viewport::new(80, 20, 100);
        viewport.set_height(30);
        // Max offset is now 70, so offset should be clamped
        assert_eq!(viewport.offset(), 70);
    }

    #[test]
    fn test_set_content_height() {
        let mut viewport = Viewport::new(50, 20, 100);
        viewport.set_content_height(150);
        assert_eq!(viewport.content_height(), 150);
        assert_eq!(viewport.offset(), 50);
    }

    #[test]
    fn test_set_content_height_adjusts_offset() {
        let mut viewport = Viewport::new(80, 20, 100);
        viewport.set_content_height(50);
        // Max offset is now 30, so offset should be clamped
        assert_eq!(viewport.offset(), 30);
    }

    #[test]
    fn test_is_at_top() {
        let viewport = Viewport::new(0, 20, 100);
        assert!(viewport.is_at_top());

        let viewport = Viewport::new(10, 20, 100);
        assert!(!viewport.is_at_top());
    }

    #[test]
    fn test_is_at_bottom() {
        let viewport = Viewport::new(80, 20, 100);
        assert!(viewport.is_at_bottom());

        let viewport = Viewport::new(70, 20, 100);
        assert!(!viewport.is_at_bottom());
    }

    #[test]
    fn test_smart_padding_small_viewport() {
        let viewport = Viewport::new(0, 8, 100);
        assert_eq!(viewport.smart_padding(), 1);
    }

    #[test]
    fn test_smart_padding_medium_viewport() {
        let viewport = Viewport::new(0, 15, 100);
        assert_eq!(viewport.smart_padding(), 2);
    }

    #[test]
    fn test_smart_padding_large_viewport() {
        let viewport = Viewport::new(0, 30, 100);
        assert_eq!(viewport.smart_padding(), 3);
    }

    #[test]
    fn test_smart_padding_very_large_viewport() {
        let viewport = Viewport::new(0, 50, 100);
        assert_eq!(viewport.smart_padding(), 5);
    }

    #[test]
    fn test_viewport_with_content_smaller_than_viewport() {
        let viewport = Viewport::new(0, 50, 30);
        assert_eq!(viewport.offset(), 0);
        assert_eq!(viewport.visible_range(), 0..30);
        assert!(viewport.is_at_top());
        assert!(viewport.is_at_bottom());
    }

    #[test]
    fn test_ensure_visible_with_padding_near_top_edge() {
        let mut viewport = Viewport::new(10, 20, 100);
        // Element at y=11, which is just barely inside the viewport
        // With padding=2, we want to see 2 lines above it
        viewport.ensure_visible_with_padding(11, 1, 2);
        // Element is near top edge (within padding distance), should scroll up
        assert_eq!(viewport.offset(), 9);
    }

    #[test]
    fn test_ensure_visible_with_padding_near_bottom_edge() {
        let mut viewport = Viewport::new(10, 20, 100);
        // Element at y=28, bottom at 29, viewport bottom at 30
        // With padding=2, we want 2 lines below it
        viewport.ensure_visible_with_padding(28, 1, 2);
        // Element is near bottom edge, should scroll down
        // element_bottom = 29, desired = 29 + 2 = 31, offset = 31 - 20 = 11
        assert_eq!(viewport.offset(), 11);
    }
}
