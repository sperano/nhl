//! Row layout and buffer-clipping helpers used by document element rendering.
//!
//! `render_row` lays out a horizontal row of child elements; `clipped` is the
//! clipping primitive that keeps an element's rendering inside its assigned
//! area; `get_preferred_width` reports the fixed width some elements need in
//! order to lay out a row of them.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::RenderContext;
use crate::tui::widgets::StandaloneWidget;

use super::render_composites::TEAM_BOXSCORE_WIDTH;
use super::{DocumentElement, RowAlignment};

/// Render a horizontal row of elements
pub(super) fn render_row(
    children: &[DocumentElement],
    gap: u16,
    align: RowAlignment,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    if children.is_empty() || area.width == 0 {
        return;
    }

    // Check if children have preferred widths (e.g., ScoreBoxElement, TeamBoxscore)
    let has_preferred_widths = children.iter().all(|c| get_preferred_width(c).is_some());

    if has_preferred_widths {
        // Calculate total width of all children
        let total_children_width: u16 = children.iter().filter_map(get_preferred_width).sum();
        let num_gaps = children.len().saturating_sub(1) as u16;
        let total_gaps_width = gap * num_gaps;
        let total_content_width = total_children_width + total_gaps_width;

        // Calculate starting x offset and actual gap based on alignment
        let (start_x, actual_gap) = match align {
            RowAlignment::Left => (area.x, gap),
            RowAlignment::Spread => {
                // Calculate maximum gap to spread children across available width
                let remaining_space = area.width.saturating_sub(total_children_width);
                let actual_gap = remaining_space
                    .checked_div(num_gaps)
                    .map_or(0, |g| g.max(gap));
                (area.x, actual_gap)
            }
            RowAlignment::Center => {
                // Center the group of children with minimum gap between them
                let left_margin = area.width.saturating_sub(total_content_width) / 2;
                (area.x + left_margin, gap)
            }
        };

        let mut x_offset = start_x;
        for child in children {
            let child_width = get_preferred_width(child).unwrap_or(0);
            let child_area = Rect::new(x_offset, area.y, child_width, area.height);
            child.render(child_area, buf, ctx);
            x_offset += child_width + actual_gap;
        }
    } else {
        // Distribute space equally for flexible elements
        let num_children = children.len() as u16;
        let total_gap = gap * (num_children.saturating_sub(1));
        let available_width = area.width.saturating_sub(total_gap);
        let child_width = available_width / num_children;

        let mut x_offset = area.x;
        for child in children {
            let child_area = Rect::new(x_offset, area.y, child_width, area.height);
            child.render(child_area, buf, ctx);
            x_offset += child_width + gap;
        }
    }
}

/// Run `draw` against a scratch buffer covering exactly `area`, then copy the
/// region back into `buf`.
///
/// This is the document layer's clipping primitive: some `ElementWidget`
/// impls (e.g. `TableWidget`) lay out their content at its own natural size
/// and don't clip themselves to the `area` they're given, so drawing straight
/// into the shared buffer could bleed past `area` into a sibling's cells.
/// Coordinates outside `area` don't exist in the scratch buffer, so
/// out-of-bounds writes are dropped instead of landing on a neighbor.
/// Truncation at the boundary is still expected; bleeding is not.
///
/// The scratch is seeded with `buf`'s current cells so that, within `area`,
/// drawing through the scratch is indistinguishable from drawing into `buf`
/// directly: cells the drawing never touches keep their existing content and
/// style (e.g. the theme background fill) rather than being reset to default
/// cells, which a `Buffer::merge` of an empty scratch would do.
pub(super) fn clipped(area: Rect, buf: &mut Buffer, draw: impl FnOnce(&mut Buffer)) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mut scratch = Buffer::empty(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(src) = buf.cell((x, y)) {
                scratch[(x, y)] = src.clone();
            }
        }
    }
    draw(&mut scratch);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(dst) = buf.cell_mut((x, y)) {
                *dst = scratch[(x, y)].clone();
            }
        }
    }
}

/// Get preferred width for elements that have fixed dimensions
pub(super) fn get_preferred_width(element: &DocumentElement) -> Option<u16> {
    match element {
        DocumentElement::ScoreBoxElement { score_box, .. } => score_box.preferred_width(),
        DocumentElement::TeamBoxscore { .. } => Some(TEAM_BOXSCORE_WIDTH),
        _ => None,
    }
}
