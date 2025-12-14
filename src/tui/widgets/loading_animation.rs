//! Loading animation widget for display during data fetching
//!
//! Renders a pulsing dots animation that cycles quickly to indicate loading state.

use ratatui::{buffer::Buffer, layout::Rect, style::Modifier};

use super::StandaloneWidget;
use crate::config::RenderContext;

/// Number of dots in the animation
const DOT_COUNT: usize = 3;

/// Get the animation string for the given frame
pub fn loading_animation_text(frame: u8) -> &'static str {
    match frame % 4 {
        0 => "●○○",
        1 => "○●○",
        2 => "○○●",
        _ => "○●○",
    }
}

/// A loading animation widget that displays pulsing dots
///
/// The animation cycles through 4 frames with a dot bouncing left to right.
#[derive(Debug, Clone)]
pub struct LoadingAnimation {
    /// Current animation frame (0-3, wraps automatically)
    pub frame: u8,
}

impl LoadingAnimation {
    pub fn new(frame: u8) -> Self {
        Self { frame }
    }
}

impl StandaloneWidget for LoadingAnimation {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let text = loading_animation_text(self.frame);
        let text_width = DOT_COUNT as u16;

        // Center horizontally and vertically
        let x = area.x + (area.width.saturating_sub(text_width)) / 2;
        let y = area.y + area.height / 2;

        if y < area.y + area.height {
            let style = if let Some(theme) = ctx.theme() {
                ctx.base_style().fg(theme.fg).add_modifier(Modifier::BOLD)
            } else {
                ctx.base_style().add_modifier(Modifier::BOLD)
            };

            buf.set_string(x, y, text, style);
        }
    }

    fn preferred_height(&self) -> Option<u16> {
        Some(1)
    }

    fn preferred_width(&self) -> Option<u16> {
        Some(DOT_COUNT as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use crate::tui::testing::assert_buffer;

    #[test]
    fn test_animation_frames() {
        assert_eq!(loading_animation_text(0), "●○○");
        assert_eq!(loading_animation_text(1), "○●○");
        assert_eq!(loading_animation_text(2), "○○●");
        assert_eq!(loading_animation_text(3), "○●○");
        // Wraps around
        assert_eq!(loading_animation_text(4), "●○○");
    }

    #[test]
    fn test_render_centered() {
        let widget = LoadingAnimation::new(0);
        let area = Rect::new(0, 0, 9, 3);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        widget.render(area, &mut buf, &ctx);

        // Should be centered: (9-3)/2 = 3, middle row = 1
        assert_buffer(&buf, &["         ", "   ●○○   ", "         "]);
    }

    #[test]
    fn test_render_frame_2() {
        let widget = LoadingAnimation::new(2);
        let area = Rect::new(0, 0, 7, 1);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        widget.render(area, &mut buf, &ctx);

        assert_buffer(&buf, &["  ○○●  "]);
    }

    #[test]
    fn test_preferred_dimensions() {
        let widget = LoadingAnimation::new(0);
        assert_eq!(widget.preferred_height(), Some(1));
        assert_eq!(widget.preferred_width(), Some(3));
    }
}
