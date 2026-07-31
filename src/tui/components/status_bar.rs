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

/// If the data hasn't refreshed in this many multiples of `refresh_interval`,
/// flag it as stale (auto-refresh is likely failing, e.g. repeated network errors).
const STALE_THRESHOLD_MULTIPLIER: u32 = 3;

/// StatusBar component - renders status bar with last-refresh indicator and error messages
///
/// Left side: status/error messages
/// Right side: time elapsed since the last data refresh (real timestamp, not a countdown)
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

        // Right side: how long ago data was actually refreshed (real timestamp, not a
        // countdown to a promised refresh - see reducer::should_auto_refresh for the
        // logic that actually triggers refreshes on a `Tick`).
        let right_text = if let Some(refresh_time) = self.last_refresh {
            match SystemTime::now().duration_since(refresh_time) {
                Ok(elapsed) => {
                    let elapsed_secs = elapsed.as_secs();
                    let stale_threshold_secs =
                        u64::from(self.refresh_interval) * u64::from(STALE_THRESHOLD_MULTIPLIER);

                    if elapsed_secs > stale_threshold_secs {
                        format!("Updated {}s ago (stale)", elapsed_secs)
                    } else {
                        format!("Updated {}s ago", elapsed_secs)
                    }
                }
                // Clock skew or a refresh timestamp from the future - can't compute elapsed time.
                Err(_) => "Updated ?s ago".to_string(),
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
            Span::styled(ctx.box_chars().connector3, separator_style),
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
        line2_spans.push(Span::styled(ctx.box_chars().vertical, separator_style));
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
#[path = "status_bar_tests.rs"]
mod tests;
