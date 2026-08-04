//! Text/heading/link/separator/group rendering primitives for document
//! elements.
//!
//! This module contains the simplest render_* helper functions used by
//! DocumentElement::render(); row layout and clipping live in
//! `render_layout`, and table-backed composites (team boxscore, tabs) live
//! in `render_composites`. Both sibling modules are re-exported below so
//! this module's path (and `render_tests.rs`'s `use super::*`) keep working
//! unchanged for existing callers.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::config::RenderContext;

use super::DocumentElement;

// Re-exports so `render::*` keeps resolving every name it did before the
// split, for both sibling modules (`behavior.rs`, `constructors.rs`,
// `mod.rs`) and `render_tests.rs`'s `use super::*`.
pub use super::render_composites::TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH;
pub(super) use super::render_composites::{
    render_tabs, render_team_boxscore, tabs_height, team_boxscore_height,
    TEAM_BOXSCORE_SECTION_HEADER_ROWS, TEAM_BOXSCORE_SECTION_TRAILING_BLANK,
};
pub(super) use super::render_layout::{clipped, render_row};

// Test-only re-exports: these names have no production caller outside
// `render_tests.rs` (which sees them via this module's `use super::*`), so
// they're gated to avoid an unused-import warning on non-test builds.
#[cfg(test)]
pub(super) use super::render_composites::{
    render_bottom_border, render_section_header, render_tab_bar, TEAM_BOXSCORE_WIDTH,
};
#[cfg(test)]
pub(super) use super::render_layout::get_preferred_width;
#[cfg(test)]
pub(super) use super::RowAlignment;

/// Render `text` starting at `(x, y)`, clipped to `max_width` display columns, and return the
/// number of display columns actually written.
///
/// This is the single place in the module that turns a string into buffer cells, so every
/// caller gets the same unicode-width-aware behavior: double-width glyphs (CJK, emoji) advance
/// the cursor by two columns and blank the cell they span (matching how a real terminal renders
/// them), instead of the naive one-column-per-`char` assumption. A glyph that would straddle
/// `max_width` is dropped rather than half-rendered.
pub(super) fn render_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_width: u16,
    text: &str,
    style: Style,
) -> u16 {
    let (end_x, _) = buf.set_stringn(x, y, text, max_width as usize, style);
    end_x.saturating_sub(x)
}

/// Render a text element
pub(super) fn render_text(
    content: &str,
    style: Option<Style>,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let style = style.unwrap_or_else(|| ctx.text_style());

    for (i, line) in content.lines().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        let y = area.y + i as u16;
        render_line(buf, area.x, y, area.width, line, style);
    }
}

/// Rows a heading occupies: level 1 gets a `═` underline row, deeper levels
/// are a single line. Shared by `DocumentElement::height()` and
/// `render_heading` so the underline can't be drawn outside the reserved rows.
pub(super) fn heading_height(level: u8) -> u16 {
    if level == 1 {
        2
    } else {
        1
    }
}

/// Render a heading element
pub(super) fn render_heading(
    level: u8,
    content: &str,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let style = ctx.heading_style(level);
    let rendered_width = render_line(buf, area.x, area.y, area.width, content, style);

    // Render underline for level 1 with muted color, matching the actual rendered width
    // (not the raw char count, which would be wrong for wide characters).
    if heading_height(level) > 1 && area.height > 1 {
        let underline_style = ctx.boxchar_style();
        let underline = "═".repeat(rendered_width as usize);
        buf.set_string(area.x, area.y + 1, &underline, underline_style);
    }
}

/// Blank row of spacing reserved below a section title (accounted for in the
/// layout, never drawn).
pub(super) const SECTION_TITLE_TRAILING_BLANK: u16 = 1;

/// Rows a section title occupies: the title line, an optional underline row,
/// and the trailing blank row. Shared by `DocumentElement::height()` and
/// `render_section_title`.
pub(super) fn section_title_height(underline: bool) -> u16 {
    1 + u16::from(underline) + SECTION_TITLE_TRAILING_BLANK
}

/// Render a section title element
pub(super) fn render_section_title(
    content: &str,
    underline: bool,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    // Render title text with emphasis style (handles dimming automatically), clipped to area
    let rendered_width = render_line(
        buf,
        area.x,
        area.y,
        area.width,
        content,
        ctx.emphasis_style(),
    );

    // Render underline if enabled, matching the actually-rendered (possibly clipped/wide) width
    if underline && area.height > 1 {
        //TODO: use Boxchar instead of hardcoded unicode character
        let underline_str: String = "═".repeat(rendered_width as usize);
        buf.set_string(area.x, area.y + 1, &underline_str, ctx.text_style());
    }
}

/// Render a link element
pub(super) fn render_link(
    display: &str,
    focused: bool,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    use crate::config::SELECTION_STYLE_MODIFIER;
    use crate::config::THEMELESS_SELECTION_STYLE_MODIFIER;

    let base_style = ctx.text_style();

    let (prefix, link_style) = if focused {
        let prefix = format!("{} ", ctx.box_chars().selector);
        let style = if let Some(theme) = ctx.theme() {
            base_style
                .fg(theme.selection_text_fg)
                .bg(theme.selection_text_bg)
                .add_modifier(SELECTION_STYLE_MODIFIER)
        } else {
            base_style.add_modifier(THEMELESS_SELECTION_STYLE_MODIFIER)
        };
        (prefix, style)
    } else {
        // Use spaces to align with focused items
        ("  ".to_string(), base_style)
    };

    let prefix_width = render_line(
        buf,
        area.x,
        area.y,
        area.width,
        &prefix,
        ctx.boxchar_style(),
    );
    let remaining_width = area.width.saturating_sub(prefix_width);
    render_line(
        buf,
        area.x + prefix_width,
        area.y,
        remaining_width,
        display,
        link_style,
    );
}

/// Render a separator element
pub(super) fn render_separator(area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
    let sep_str = &ctx.box_chars().horizontal;
    let sep_char = sep_str.chars().next().unwrap_or('-');
    let style = ctx.boxchar_style();

    for x in 0..area.width {
        let cell = buf.cell_mut((area.x + x, area.y));
        if let Some(cell) = cell {
            cell.set_char(sep_char);
            cell.set_style(style);
        }
    }
}

/// Render a group of elements
pub(super) fn render_group(
    children: &[DocumentElement],
    style: Option<Style>,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let mut y_offset = 0;
    for child in children {
        let child_height = child.height();
        if y_offset >= area.height {
            break;
        }
        let child_area = Rect::new(
            area.x,
            area.y + y_offset,
            area.width,
            child_height.min(area.height - y_offset),
        );
        child.render(child_area, buf, ctx);
        y_offset += child_height;
    }

    // Apply group style if any
    if let Some(s) = style {
        for y in area.y..area.y + area.height.min(y_offset) {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y));
                if let Some(cell) = cell {
                    let existing = cell.style();
                    cell.set_style(existing.patch(s));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
