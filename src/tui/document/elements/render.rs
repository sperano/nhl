//! Rendering functions for document elements
//!
//! This module contains all the render_* helper functions used by DocumentElement::render()

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::config::RenderContext;
use crate::tui::component::ElementWidget;
use crate::tui::components::TableWidget;
use crate::tui::widgets::StandaloneWidget;

use super::{DocumentElement, RowAlignment};

/// Fixed width for team boxscore
/// Calculation: SELECTOR(2) + sum(column_widths) + gaps(2 * (num_columns - 1)) + borders(2)
/// Skater: 2 + 69 + 32 + 2 = 105 (17 columns)
/// Goalie: 2 + 63 + 24 + 2 = 91 (13 columns)
/// Using skater width as it's the wider table
pub const TEAM_BOXSCORE_WIDTH: u16 = 105;

/// Gap between two team boxscores when displayed side by side (centered)
pub const TEAM_BOXSCORE_GAP: u16 = 4;

/// Minimum width needed to display two team boxscores side by side
pub const TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH: u16 = TEAM_BOXSCORE_WIDTH * 2 + TEAM_BOXSCORE_GAP;

/// Rows above each non-empty section's table: the `╞══╡ Title ╞═...` header
/// line plus one blank line.
pub(super) const TEAM_BOXSCORE_SECTION_HEADER_ROWS: u16 = 2;

/// Rows below each non-empty section's table: one blank line (followed by the
/// next section's header, or by the bottom border after the last section).
pub(super) const TEAM_BOXSCORE_SECTION_TRAILING_BLANK: u16 = 1;

/// The single `╘═...═╛` row closing a team boxscore.
pub(super) const TEAM_BOXSCORE_BOTTOM_BORDER_HEIGHT: u16 = 1;

/// Height of one team boxscore section exactly as `render_team_boxscore` lays
/// it out; empty sections are skipped entirely and contribute nothing.
pub(super) fn team_boxscore_section_height(table: &TableWidget) -> u16 {
    if table.row_count() == 0 {
        return 0;
    }
    TEAM_BOXSCORE_SECTION_HEADER_ROWS
        + table.preferred_height().unwrap_or(0)
        + TEAM_BOXSCORE_SECTION_TRAILING_BLANK
}

/// Total height of a team boxscore.
///
/// Single source of the boxscore's vertical layout math, shared by
/// `DocumentElement::height()`, the focusable-position math in
/// `DocumentElement::team_boxscore`, and `render_team_boxscore` (which
/// debug_asserts each section it draws against `team_boxscore_section_height`).
pub(super) fn team_boxscore_height(
    forwards_table: &TableWidget,
    defense_table: &TableWidget,
    goalies_table: &TableWidget,
) -> u16 {
    team_boxscore_section_height(forwards_table)
        + team_boxscore_section_height(defense_table)
        + team_boxscore_section_height(goalies_table)
        + TEAM_BOXSCORE_BOTTOM_BORDER_HEIGHT
}

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

/// Render `text` starting at `(x, y)`, clipped to `max_width` display columns, and return the
/// number of display columns actually written.
///
/// This is the single place in the module that turns a string into buffer cells, so every
/// caller gets the same unicode-width-aware behavior: double-width glyphs (CJK, emoji) advance
/// the cursor by two columns and blank the cell they span (matching how a real terminal renders
/// them), instead of the naive one-column-per-`char` assumption. A glyph that would straddle
/// `max_width` is dropped rather than half-rendered.
fn render_line(buf: &mut Buffer, x: u16, y: u16, max_width: u16, text: &str, style: Style) -> u16 {
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

/// Render a team boxscore with decorative borders
///
/// Renders section headers with embedded titles and box borders around tables:
/// ```text
/// ╒══╡ Team - Forwards ╞═══════════════════════════════╕
/// │                                                    │
/// │  (table content)                                   │
/// │                                                    │
/// ╞══╡ Team - Defense ╞════════════════════════════════╡
/// ...
/// ╘════════════════════════════════════════════════════╛
/// ```
pub(super) fn render_team_boxscore(
    team_name: &str,
    forwards_table: &TableWidget,
    defense_table: &TableWidget,
    goalies_table: &TableWidget,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();

    // Use fixed width but respect area constraints
    let width = TEAM_BOXSCORE_WIDTH.min(area.width);
    let inner_width = width.saturating_sub(2); // Subtract 2 for side borders

    let mut y = area.y;
    let mut is_first_section = true;

    // All direct row writes below are guarded against area.bottom(): ratatui's
    // set_string panics when its start position lies outside the buffer, so
    // when the area is shorter than the boxscore's full height the overflow
    // rows must be skipped (truncated), not attempted. The `y` cursor still
    // advances through skipped rows to keep the section math intact.
    let bottom = area.bottom();

    // Helper to render an empty bordered line
    let render_empty_bordered_line = |y: u16, buf: &mut Buffer| {
        if y >= bottom {
            return;
        }
        buf.set_string(area.x, y, bc.vertical, border_style);
        if width > 1 {
            buf.set_string(area.x + width - 1, y, bc.vertical, border_style);
        }
    };

    // Render sections
    let sections: Vec<(&str, &TableWidget)> = vec![
        ("Forwards", forwards_table),
        ("Defense", defense_table),
        ("Goalies", goalies_table),
    ];

    for (section_name, table) in sections {
        if table.row_count() == 0 {
            continue;
        }
        let section_start = y;

        // Section header with embedded title
        if y < bottom {
            let title = format!("{} - {}", team_name, section_name);
            render_section_header(area.x, y, width, &title, is_first_section, buf, ctx);
        }
        y += 1;
        is_first_section = false;

        // Blank line after header
        render_empty_bordered_line(y, buf);
        y += 1;

        // Table content - render with side borders
        let table_height = table.preferred_height().unwrap_or(0);
        let visible_table_height = table_height.min(bottom.saturating_sub(y));
        for row in 0..visible_table_height {
            // Left border
            buf.set_string(area.x, y + row, bc.vertical, border_style);
            // Right border
            if width > 1 {
                buf.set_string(area.x + width - 1, y + row, bc.vertical, border_style);
            }
        }

        // Render table content inside borders, clipped so a table wider than
        // the boxscore (narrow area) truncates instead of overwriting the
        // right border it was just drawn inside of, and a table taller than
        // the remaining rows truncates instead of panicking in set_string.
        let table_area = Rect::new(area.x + 1, y, inner_width, visible_table_height);
        clipped(table_area, buf, |scratch| {
            table.render(table_area, scratch, ctx)
        });
        y += table_height;

        // Blank line after table (before next section or bottom border)
        render_empty_bordered_line(y, buf);
        y += 1;

        debug_assert_eq!(
            y - section_start,
            team_boxscore_section_height(table),
            "rows drawn for a boxscore section drifted from team_boxscore_section_height"
        );
    }

    // Bottom border (skipped entirely when the area ran out of rows)
    if y < bottom {
        render_bottom_border(area.x, y, width, buf, ctx);
    }
}

/// Render section header with embedded title
///
/// First section: ╒══╡ Title ╞═══════════════════════════════════╕
/// Later sections: ╞══╡ Title ╞═══════════════════════════════════╡
fn render_section_header(
    x: u16,
    y: u16,
    width: u16,
    title: &str,
    is_first: bool,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();
    let title_style = ctx.text_style();

    // Choose corner characters based on whether this is first section
    let (left_corner, right_corner) = if is_first {
        (&bc.mixed_dh_top_left, &bc.mixed_dh_top_right)
    } else {
        (&bc.mixed_dh_left_t, &bc.mixed_dh_right_t)
    };

    // Build the header line: corner + == + ╡ + title + ╞ + === + corner
    let title_prefix = format!(
        "{}{}{}",
        left_corner,
        bc.double_horizontal.repeat(2),
        bc.mixed_dh_right_t,
    );
    let title_with_space = format!(" {} ", title);

    // The last column is reserved for the right corner so it always wins, even when the
    // title is too long for the available width: everything else is laid out left-to-right
    // within the remaining budget and clipped/truncated to fit before it, rather than being
    // written at a fixed offset and later overwritten mid-glyph by the corner.
    const RIGHT_CORNER_WIDTH: u16 = 1;
    let body_end = x + width.saturating_sub(RIGHT_CORNER_WIDTH);
    let mut cursor = x;

    cursor += render_line(
        buf,
        cursor,
        y,
        body_end.saturating_sub(cursor),
        &title_prefix,
        border_style,
    );
    cursor += render_line(
        buf,
        cursor,
        y,
        body_end.saturating_sub(cursor),
        &title_with_space,
        title_style,
    );
    cursor += render_line(
        buf,
        cursor,
        y,
        body_end.saturating_sub(cursor),
        bc.mixed_dh_left_t,
        border_style,
    );

    let trailing_width = body_end.saturating_sub(cursor);
    if trailing_width > 0 {
        let trailing = bc.double_horizontal.repeat(trailing_width as usize);
        buf.set_string(cursor, y, &trailing, border_style);
    }

    // Right corner always wins the last column.
    if width > 0 {
        buf.set_string(x + width - 1, y, right_corner, border_style);
    }
}

/// Render bottom border: ╘═══════════════════════════════════════════════╛
fn render_bottom_border(x: u16, y: u16, width: u16, buf: &mut Buffer, ctx: &RenderContext) {
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();

    // Left corner
    buf.set_string(x, y, bc.mixed_dh_bottom_left, border_style);

    // Middle ═
    let middle_width = width.saturating_sub(2) as usize;
    let middle = bc.double_horizontal.repeat(middle_width);
    buf.set_string(x + 1, y, &middle, border_style);

    // Right corner
    if width > 1 {
        buf.set_string(x + width - 1, y, bc.mixed_dh_bottom_right, border_style);
    }
}

/// Rows a tabs element occupies: the tab bar plus the active tab's stacked
/// content. Shared by `DocumentElement::height()` and `render_tabs`.
pub(super) fn tabs_height(tabs: &[super::DocTabDef], active_index: usize) -> u16 {
    let content_height: u16 = tabs
        .get(active_index)
        .map(|tab| tab.content.iter().map(|e| e.height()).sum())
        .unwrap_or(0);
    super::TAB_BAR_HEIGHT + content_height
}

/// Render a tabs element (tab bar + active tab content)
///
/// Layout:
/// ```text
///  Tab1  │ Tab2 │ Tab3
/// ───────┴──────┴───────────────
///  (active tab content)
/// ```
pub(super) fn render_tabs(
    tabs: &[super::DocTabDef],
    active_index: usize,
    area: Rect,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    use super::TAB_BAR_HEIGHT;

    if tabs.is_empty() || area.height < TAB_BAR_HEIGHT {
        return;
    }

    // Render tab bar (2 lines: labels + separator)
    render_tab_bar(tabs, active_index, area.x, area.y, area.width, buf, ctx);

    // Render active tab content
    if let Some(tab) = tabs.get(active_index) {
        let content_area = Rect::new(
            area.x,
            area.y + TAB_BAR_HEIGHT,
            area.width,
            area.height.saturating_sub(TAB_BAR_HEIGHT),
        );

        let mut y_offset = 0;
        for element in &tab.content {
            let element_height = element.height();
            if y_offset >= content_area.height {
                break;
            }
            let element_area = Rect::new(
                content_area.x,
                content_area.y + y_offset,
                content_area.width,
                element_height.min(content_area.height - y_offset),
            );
            element.render(element_area, buf, ctx);
            y_offset += element_height;
        }
    }
}

/// Render the tab bar (labels line + separator line)
fn render_tab_bar(
    tabs: &[super::DocTabDef],
    active_index: usize,
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let bc = ctx.box_chars();
    let base_style = ctx.text_style();
    let border_style = ctx.boxchar_style();

    // Calculate tab widths (each tab gets its title's display width + padding). Using display
    // width (not char count) matters once a title contains a double-width glyph: the label is
    // rendered with the real glyph width regardless, so an undercounted width here would place
    // the next tab's separator on top of the previous tab's still-visible content.
    const TAB_PADDING: u16 = 2; // space before and after title
    let tab_widths: Vec<u16> = tabs
        .iter()
        .map(|t| t.title.width() as u16 + TAB_PADDING)
        .collect();

    // Line 1: Tab labels
    let mut x_pos = x;
    for (idx, tab) in tabs.iter().enumerate() {
        let tab_width = tab_widths[idx];
        let is_active = idx == active_index;

        // Tab style - active tab is highlighted
        let tab_style = if is_active {
            ctx.emphasis_style()
        } else {
            base_style
        };

        // Render tab label with padding, clipped to the tab bar's own width
        let label = format!(" {} ", tab.title);
        let remaining = (x + width).saturating_sub(x_pos);
        render_line(buf, x_pos, y, remaining, &label, tab_style);

        // Move to next tab position
        x_pos += tab_width;

        // Add separator between tabs (not after the last one)
        if idx < tabs.len() - 1 && x_pos < x + width {
            buf.set_string(x_pos, y, bc.vertical, border_style);
            x_pos += 1;
        }
    }

    // Line 2: Separator with connectors under the tab dividers
    let sep_char = bc.horizontal.chars().next().unwrap_or('─');
    let tee_char = bc.bottom_junction.chars().next().unwrap_or('┴');

    // Fill the separator line
    for i in 0..width {
        let cell = buf.cell_mut((x + i, y + 1));
        if let Some(cell) = cell {
            cell.set_char(sep_char);
            cell.set_style(border_style);
        }
    }

    // Place tee connectors at tab boundaries
    let mut connector_x = x;
    for (idx, &tab_width) in tab_widths.iter().enumerate() {
        connector_x += tab_width;

        // Place tee at boundary (if not last tab)
        if idx < tabs.len() - 1 && connector_x < x + width {
            let cell = buf.cell_mut((connector_x, y + 1));
            if let Some(cell) = cell {
                cell.set_char(tee_char);
            }
            connector_x += 1; // Account for the vertical separator
        }
    }
}
#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
