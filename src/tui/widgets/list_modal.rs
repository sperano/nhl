use crate::config::RenderContext;
use crate::tui::component::ElementWidget;
/// ListModalWidget - renders a centered popup modal for list selection
///
/// Features:
/// - Centered modal positioning
/// - Clear background behind modal
/// - Border with selection color
/// - Selection indicator for current option
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Widget},
};
use unicode_width::UnicodeWidthStr;

/// Widget for rendering a list selection modal
#[derive(Clone)]
pub struct ListModalWidget {
    pub options: Vec<String>,
    pub selected_index: usize,
    pub position_x: u16,
    pub position_y: u16,
}

impl ListModalWidget {
    pub fn new(
        options: Vec<String>,
        selected_index: usize,
        position_x: u16,
        position_y: u16,
    ) -> Self {
        Self {
            options,
            selected_index,
            position_x,
            position_y,
        }
    }
}

impl ElementWidget for ListModalWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        render_list_modal(
            &self.options,
            self.selected_index,
            self.position_x,
            self.position_y,
            area,
            buf,
            ctx,
        );
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }
}

/// Computes the modal's position and size: tall enough for every option (plus borders),
/// wide enough for the longest option (plus margins), and clamped so it stays on-screen.
fn compute_modal_area(options: &[String], position_x: u16, position_y: u16, area: Rect) -> Rect {
    let modal_height = options.len() as u16 + 2; // +2 for borders
    let max_option_len = options.iter().map(|s| s.width()).max().unwrap_or(20);
    let modal_width = max_option_len as u16 + 6;

    Rect {
        x: position_x.min(area.width.saturating_sub(modal_width)),
        y: position_y.min(area.height.saturating_sub(modal_height)),
        width: modal_width.min(area.width),
        height: modal_height.min(area.height),
    }
}

/// Fills the modal area with the theme background, resetting cells first to clear any
/// underlying styling (prevents bleed-through from selected items when the theme has no
/// explicit background).
fn fill_modal_background(buf: &mut Buffer, modal_area: Rect, bg_style: Style) {
    for y in modal_area.y..modal_area.bottom() {
        for x in modal_area.x..modal_area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.reset();
                cell.set_char(' ');
                cell.set_style(bg_style);
            }
        }
    }
}

/// Renders the modal's bordered frame using the theme's box-char color.
fn render_modal_border(buf: &mut Buffer, modal_area: Rect, ctx: &RenderContext) {
    let border_style = if let Some(theme) = ctx.theme() {
        ctx.base_style().fg(theme.boxchar_fg)
    } else {
        ctx.base_style()
    };
    let border_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    border_block.render(modal_area, buf);
}

/// Renders each option inside the modal's inner area, with a selector glyph in front of
/// the currently selected one.
fn render_modal_options(
    buf: &mut Buffer,
    inner: Rect,
    options: &[String],
    selected_index: usize,
    ctx: &RenderContext,
) {
    // Text style uses theme fg with theme bg
    let text_style = if let Some(theme) = ctx.theme() {
        ctx.base_style().fg(theme.fg)
    } else {
        ctx.base_style()
    };

    // Selector style uses boxchar_fg with theme bg
    let selector_style = if let Some(theme) = ctx.theme() {
        ctx.base_style().fg(theme.boxchar_fg)
    } else {
        ctx.base_style()
    };

    for (y, (idx, option)) in (inner.y..inner.bottom()).zip(options.iter().enumerate()) {
        let is_selected = idx == selected_index;

        if is_selected {
            let selector = format!(" {} ", ctx.box_chars().selector);
            buf.set_string(inner.x, y, &selector, selector_style);
            buf.set_string(inner.x + 3, y, option, text_style);
        } else {
            buf.set_string(inner.x, y, "   ", ctx.base_style());
            buf.set_string(inner.x + 3, y, option, text_style);
        }
    }
}

/// Renders a list selection modal at the specified position
///
/// Returns the modal area that was rendered
pub fn render_list_modal(
    options: &[String],
    selected_index: usize,
    position_x: u16,
    position_y: u16,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) -> Rect {
    let modal_area = compute_modal_area(options, position_x, position_y, area);

    fill_modal_background(buf, modal_area, ctx.base_style());
    render_modal_border(buf, modal_area, ctx);

    // Calculate inner area (inside borders)
    let inner = Rect {
        x: modal_area.x + 1,
        y: modal_area.y + 1,
        width: modal_area.width.saturating_sub(2),
        height: modal_area.height.saturating_sub(2),
    };

    render_modal_options(buf, inner, options, selected_index, ctx);

    modal_area
}

#[cfg(test)]
#[path = "list_modal_tests.rs"]
mod tests;
