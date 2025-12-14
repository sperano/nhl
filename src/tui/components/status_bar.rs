use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::SystemTime;
use unicode_width::UnicodeWidthStr;

use crate::config::RenderContext;
use crate::tui::{
    component::{Component, Element, ElementWidget},
    state::SystemState,
};

/// StatusBar component - renders status bar with refresh countdown and error messages
///
/// Left side: status/error messages
/// Right side: refresh countdown
pub struct StatusBar;

impl Component for StatusBar {
    type Props = SystemState;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        Element::Widget(Box::new(StatusBarWidget {
            last_refresh: props.last_refresh,
            refresh_interval: props.config.refresh_interval,
            status_message: props.status_message.clone(),
            is_error: props.status_is_error,
        }))
    }
}

/// Renderable widget for StatusBar
struct StatusBarWidget {
    last_refresh: Option<SystemTime>,
    refresh_interval: u32,
    status_message: Option<String>,
    is_error: bool,
}

impl ElementWidget for StatusBarWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        let mut lines = Vec::new();

        // Left side: status message (if any)
        let left_text = if let Some(msg) = &self.status_message {
            msg.clone()
        } else {
            String::new()
        };

        // Right side: countdown to next refresh
        let right_text = if let Some(refresh_time) = self.last_refresh {
            if let Ok(elapsed) = SystemTime::now().duration_since(refresh_time) {
                let elapsed_secs = elapsed.as_secs();
                let remaining_secs = self.refresh_interval.saturating_sub(elapsed_secs as u32);

                if remaining_secs > 0 {
                    format!("Refresh in {}s", remaining_secs)
                } else {
                    "Refreshing...".to_string()
                }
            } else {
                "Refresh in ?s".to_string()
            }
        } else {
            "Loading...".to_string()
        };

        // Calculate where the vertical bar should be
        let right_text_with_margin = format!("{} ", right_text);
        let bar_position = area
            .width
            .saturating_sub(right_text_with_margin.width() as u16 + 1);

        // Determine styles based on theme
        let separator_style = if let Some(theme) = ctx.theme() {
            ctx.base_style().fg(theme.boxchar_fg)
        } else {
            ctx.base_style()
        };

        let text_style = if let Some(theme) = ctx.theme() {
            ctx.base_style().fg(theme.fg)
        } else {
            ctx.base_style()
        };

        // First line: horizontal separator with connector
        let left_part = ctx.box_chars().horizontal.repeat(bar_position as usize);
        let right_part = ctx
            .box_chars()
            .horizontal
            .repeat((area.width.saturating_sub(bar_position + 1)) as usize);
        let line1 = Line::from(vec![
            Span::styled(left_part, separator_style),
            Span::styled(&ctx.box_chars().connector3, separator_style),
            Span::styled(right_part, separator_style),
        ]);
        lines.push(line1);

        // Second line: status message on left, refresh on right
        let mut line2_spans = Vec::new();

        // Left side: status message
        if !left_text.is_empty() {
            line2_spans.push(Span::raw(" "));
            if self.is_error {
                line2_spans.push(Span::styled(&left_text, ctx.base_style().fg(Color::Red)));
            } else {
                line2_spans.push(Span::styled(&left_text, text_style));
            }
        }

        // Middle: padding
        let left_content_len = if left_text.is_empty() {
            0
        } else {
            left_text.width() + 1
        };
        let padding_len = bar_position.saturating_sub(left_content_len as u16) as usize;
        line2_spans.push(Span::raw(" ".repeat(padding_len)));

        // Right side: vertical bar + refresh text
        line2_spans.push(Span::styled(&ctx.box_chars().vertical, separator_style));
        line2_spans.push(Span::raw(" "));
        line2_spans.push(Span::styled(&right_text, text_style));
        line2_spans.push(Span::raw(" "));

        lines.push(Line::from(line2_spans));

        let status_bar = Paragraph::new(lines);
        ratatui::widgets::Widget::render(status_bar, area, buf);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(StatusBarWidget {
            last_refresh: self.last_refresh,
            refresh_interval: self.refresh_interval,
            status_message: self.status_message.clone(),
            is_error: self.is_error,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        Some(2) // Separator line + status line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DisplayConfig, RenderContext};
    use crate::tui::testing::{assert_buffer, RENDER_WIDTH};
    use ratatui::buffer::Buffer;

    #[test]
    fn test_status_bar_renders_loading() {
        let status_bar = StatusBar;
        let system_state = SystemState {
            last_refresh: None,
            config: Config::default(),
            status_message: None,
            status_is_error: false,
            terminal_width: 80,
            animation_frame: 0,
        };

        let element = status_bar.view(&system_state, &());

        match element {
            Element::Widget(widget) => {
                let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
                let config = DisplayConfig::default();
                let ctx = RenderContext::focused(&config);
                widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);
                assert_buffer(&buf, &[
                    "────────────────────────────────────────────────────────────────────┬───────────",
                    "                                                                    │ Loading...",
                ]);
            }
            _ => panic!("Expected widget element"),
        }
    }

    #[test]
    fn test_status_bar_renders() {
        let status_bar = StatusBar;
        let system_state = SystemState {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            config: Config::default(),
            status_message: None,
            status_is_error: false,
            terminal_width: 80,
            animation_frame: 0,
        };

        let element = status_bar.view(&system_state, &());

        match element {
            Element::Widget(widget) => {
                let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
                let config = DisplayConfig::default();
                let ctx = RenderContext::focused(&config);
                widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);
                assert_buffer(&buf, &[
                    "────────────────────────────────────────────────────────────────┬───────────────",
                    "                                                                │ Refresh in 55s",
                ]);
            }
            _ => panic!("Expected widget element"),
        }
    }

    #[test]
    fn test_status_bar_with_error_message() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("ERROR: Network timeout".to_string()),
            is_error: true,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Error message should appear on left side
        let line2 = (0..RENDER_WIDTH)
            .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
            .collect::<String>();

        assert!(
            line2.contains("ERROR: Network timeout"),
            "Error message not found in: {}",
            line2
        );
    }

    #[test]
    fn test_status_bar_refreshing_state() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(60)),
            refresh_interval: 60,
            status_message: None,
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Should show "Refreshing..." when time has elapsed
        let line2 = (0..RENDER_WIDTH)
            .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
            .collect::<String>();

        assert!(
            line2.contains("Refreshing..."),
            "Refreshing message not found in: {}",
            line2
        );
    }

    #[test]
    fn test_status_bar_future_time() {
        // Test with a future time (should handle time calculation error)
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() + std::time::Duration::from_secs(100)),
            refresh_interval: 60,
            status_message: None,
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Should show "Refresh in ?s" when duration_since fails
        let line2 = (0..RENDER_WIDTH)
            .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
            .collect::<String>();

        assert!(
            line2.contains("Refresh in ?s"),
            "Error fallback not found in: {}",
            line2
        );
    }

    #[test]
    fn test_status_bar_clone_box() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now()),
            refresh_interval: 60,
            status_message: Some("Test".to_string()),
            is_error: false,
        };

        let _cloned: Box<dyn ElementWidget> = widget.clone_box();
        // If we get here, clone_box() worked
    }

    #[test]
    fn test_status_bar_preferred_height() {
        let widget = StatusBarWidget {
            last_refresh: None,
            refresh_interval: 60,
            status_message: None,
            is_error: false,
        };

        assert_eq!(widget.preferred_height(), Some(2));
    }

    #[test]
    fn test_status_bar_with_success_message() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Configuration saved".to_string()),
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        assert_buffer(
            &buf,
            &[
                "────────────────────────────────────────────────────────────────┬───────────────",
                " Configuration saved                                            │ Refresh in 55s",
            ],
        );

        // Verify success message is NOT styled red
        if let Some(cell) = buf.cell((1, 1)) {
            assert_ne!(cell.fg, Color::Red, "Success message should not be red");
        }
    }

    #[test]
    fn test_status_bar_error_message_has_red_color() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Failed to save config".to_string()),
            is_error: true,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        assert_buffer(
            &buf,
            &[
                "────────────────────────────────────────────────────────────────┬───────────────",
                " Failed to save config                                          │ Refresh in 55s",
            ],
        );

        // Verify error message IS styled red
        if let Some(cell) = buf.cell((1, 1)) {
            assert_eq!(cell.fg, Color::Red, "Error message should be red");
        }
    }

    #[test]
    fn test_status_bar_clears_previous_status_message() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: None,
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        assert_buffer(
            &buf,
            &[
                "────────────────────────────────────────────────────────────────┬───────────────",
                "                                                                │ Refresh in 55s",
            ],
        );
    }

    #[test]
    fn test_status_bar_separators_use_fg3_when_theme_set() {
        use crate::config::THEME_ORANGE;

        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: None,
            is_error: false,
        };

        let mut config = DisplayConfig::default();
        config.theme = Some(THEME_ORANGE.clone());
        let ctx = RenderContext::focused(&config);

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Verify separator characters are styled with fg3
        // Check horizontal line character on line 1
        if let Some(cell) = buf.cell((0, 0)) {
            assert_eq!(
                cell.fg, THEME_ORANGE.boxchar_fg,
                "Horizontal separator should use theme fg3"
            );
        }

        // Check connector character (┬) on line 1
        if let Some(cell) = buf.cell((64, 0)) {
            assert_eq!(
                cell.fg, THEME_ORANGE.boxchar_fg,
                "Connector should use theme fg3"
            );
        }

        // Check vertical bar character (│) on line 2
        if let Some(cell) = buf.cell((64, 1)) {
            assert_eq!(
                cell.fg, THEME_ORANGE.boxchar_fg,
                "Vertical bar should use theme fg3"
            );
        }
    }

    #[test]
    fn test_status_bar_separators_unstyled_when_no_theme() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: None,
            is_error: false,
        };

        let config = DisplayConfig::default(); // No theme set
        let ctx = RenderContext::focused(&config);

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Verify separator characters use default color (Reset)
        // Check horizontal line character on line 1
        if let Some(cell) = buf.cell((0, 0)) {
            assert_eq!(
                cell.fg,
                Color::Reset,
                "Separator should use default color when no theme"
            );
        }

        // Check vertical bar character (│) on line 2
        if let Some(cell) = buf.cell((64, 1)) {
            assert_eq!(
                cell.fg,
                Color::Reset,
                "Vertical bar should use default color when no theme"
            );
        }
    }

    #[test]
    fn test_status_bar_text_uses_fg2_when_theme_set() {
        use crate::config::THEME_ORANGE;

        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Configuration saved".to_string()),
            is_error: false,
        };

        let mut config = DisplayConfig::default();
        config.theme = Some(THEME_ORANGE.clone());
        let ctx = RenderContext::focused(&config);

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Verify success message uses fg2
        if let Some(cell) = buf.cell((1, 1)) {
            assert_eq!(
                cell.fg, THEME_ORANGE.fg,
                "Success message should use theme fg2"
            );
        }

        // Verify refresh text uses fg2
        // Find the refresh text (right side of the vertical bar)
        if let Some(cell) = buf.cell((66, 1)) {
            assert_eq!(
                cell.fg, THEME_ORANGE.fg,
                "Refresh text should use theme fg2"
            );
        }
    }

    #[test]
    fn test_status_bar_error_text_ignores_theme() {
        use crate::config::THEME_ORANGE;

        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Failed to save config".to_string()),
            is_error: true,
        };

        let mut config = DisplayConfig::default();
        config.theme = Some(THEME_ORANGE.clone());
        let ctx = RenderContext::focused(&config);

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Verify error message still uses Color::Red, not theme fg2
        if let Some(cell) = buf.cell((1, 1)) {
            assert_eq!(
                cell.fg,
                Color::Red,
                "Error message should use Color::Red even with theme set"
            );
            assert_ne!(
                cell.fg, THEME_ORANGE.fg,
                "Error message should NOT use theme fg2"
            );
        }
    }

    #[test]
    fn test_status_bar_text_unstyled_when_no_theme() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Configuration saved".to_string()),
            is_error: false,
        };

        let config = DisplayConfig::default(); // No theme set
        let ctx = RenderContext::focused(&config);

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Verify success message uses default color when no theme
        if let Some(cell) = buf.cell((1, 1)) {
            assert_eq!(
                cell.fg,
                Color::Reset,
                "Success message should use default color when no theme"
            );
        }

        // Verify refresh text uses default color when no theme
        if let Some(cell) = buf.cell((66, 1)) {
            assert_eq!(
                cell.fg,
                Color::Reset,
                "Refresh text should use default color when no theme"
            );
        }
    }

    #[test]
    fn test_status_bar_with_emoji_in_message() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Updated 🏒".to_string()),
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Should render without panic
        // The vertical bar should still be positioned correctly
        let line1 = (0..RENDER_WIDTH)
            .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
            .collect::<String>();

        assert!(
            line1.contains("Updated"),
            "Status message should be visible"
        );
        assert!(line1.contains("│"), "Vertical separator should be present");
    }

    #[test]
    fn test_status_bar_with_cjk_in_message() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("更新完了".to_string()), // "Update complete" in Japanese
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Should render without panic or incorrect layout
        let line1 = (0..RENDER_WIDTH)
            .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
            .collect::<String>();

        assert!(
            line1.contains("│"),
            "Vertical separator should be present despite CJK characters"
        );
        assert!(
            line1.contains("Refresh in"),
            "Refresh text should still be visible"
        );
    }

    #[test]
    fn test_status_bar_with_long_unicode_message() {
        let widget = StatusBarWidget {
            last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
            refresh_interval: 60,
            status_message: Some("Loading players データを読み込み中 🏒🥅".to_string()),
            is_error: false,
        };

        let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

        // Should render without panic
        // Layout should still be functional
        assert!(buf.area.width > 0);
        assert!(buf.area.height == 2);
    }
}
