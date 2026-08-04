use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
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

impl StatusBarWidget {
    /// Left side text: the current status/error message, or empty if none.
    fn status_text(&self) -> String {
        self.status_message.clone().unwrap_or_default()
    }

    /// Right side text: how long ago data was actually refreshed (real timestamp, not a
    /// countdown to a promised refresh - see reducer::should_auto_refresh for the
    /// logic that actually triggers refreshes on a `Tick`).
    fn refresh_text(&self) -> String {
        let Some(refresh_time) = self.last_refresh else {
            return "Loading...".to_string();
        };

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
    }

    /// Separator and text styles derived from the current theme (if any).
    fn styles(&self, ctx: &RenderContext) -> (Style, Style) {
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

        (separator_style, text_style)
    }

    /// First line: horizontal separator with a connector at `bar_position`.
    fn build_separator_line(
        &self,
        area: Rect,
        bar_position: u16,
        ctx: &RenderContext,
        separator_style: Style,
    ) -> Line<'static> {
        let left_part = ctx.box_chars().horizontal.repeat(bar_position as usize);
        let right_part = ctx
            .box_chars()
            .horizontal
            .repeat((area.width.saturating_sub(bar_position + 1)) as usize);
        Line::from(vec![
            Span::styled(left_part, separator_style),
            Span::styled(ctx.box_chars().connector3, separator_style),
            Span::styled(right_part, separator_style),
        ])
    }

    /// Second line: status message on the left, refresh text on the right.
    fn build_status_line<'a>(
        &self,
        bar_position: u16,
        left_text: &'a str,
        right_text: &'a str,
        ctx: &RenderContext,
        separator_style: Style,
        text_style: Style,
    ) -> Line<'a> {
        let mut spans = Vec::new();

        if !left_text.is_empty() {
            spans.push(Span::raw(" "));
            if self.is_error {
                spans.push(Span::styled(left_text, ctx.base_style().fg(Color::Red)));
            } else {
                spans.push(Span::styled(left_text, text_style));
            }
        }

        let left_content_len = if left_text.is_empty() {
            0
        } else {
            left_text.width() + 1
        };
        let padding_len = bar_position.saturating_sub(left_content_len as u16) as usize;
        spans.push(Span::raw(" ".repeat(padding_len)));

        spans.push(Span::styled(ctx.box_chars().vertical, separator_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(right_text, text_style));
        spans.push(Span::raw(" "));

        Line::from(spans)
    }
}

impl ElementWidget for StatusBarWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        let left_text = self.status_text();
        let right_text = self.refresh_text();

        // Calculate where the vertical bar should be
        let right_text_with_margin = format!("{} ", right_text);
        let bar_position = area
            .width
            .saturating_sub(right_text_with_margin.width() as u16 + 1);

        let (separator_style, text_style) = self.styles(ctx);

        let lines = vec![
            self.build_separator_line(area, bar_position, ctx, separator_style),
            self.build_status_line(
                bar_position,
                &left_text,
                &right_text,
                ctx,
                separator_style,
                text_style,
            ),
        ];

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
