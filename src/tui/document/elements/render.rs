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
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::components::TableWidget;
    use crate::tui::document::elements::DocTabDef;
    use crate::tui::document::link::LinkTarget;
    use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};
    use crate::tui::{Alignment, CellValue, ColumnDef};
    use ratatui::style::{Color, Modifier};

    /// Build a single-column text table with the given row labels.
    /// An empty slice produces a table with `row_count() == 0`.
    fn text_table(rows: &[&str]) -> TableWidget {
        let columns: Vec<ColumnDef<&str>> =
            vec![ColumnDef::new("Name", 8, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            })];
        TableWidget::from_data(&columns, rows.to_vec())
    }

    fn final_score_box(away: &str, home: &str) -> ScoreBox {
        ScoreBox::new(
            away,
            home,
            Some(1),
            Some(2),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        )
    }

    // ---- get_preferred_width ----

    #[test]
    fn preferred_width_score_box_returns_its_width() {
        let elem = DocumentElement::score_box_element(
            1,
            final_score_box("A", "B"),
            false,
            LinkTarget::Anchor("x".to_string()),
        );
        assert_eq!(get_preferred_width(&elem), Some(25));
    }

    #[test]
    fn preferred_width_team_boxscore_returns_fixed_constant() {
        let elem = DocumentElement::team_boxscore(
            "away",
            "Team",
            text_table(&[]),
            text_table(&[]),
            text_table(&[]),
        );
        assert_eq!(get_preferred_width(&elem), Some(TEAM_BOXSCORE_WIDTH));
    }

    #[test]
    fn preferred_width_text_element_returns_none() {
        let elem = DocumentElement::text("hello");
        assert_eq!(get_preferred_width(&elem), None);
    }

    // ---- render_row ----

    #[test]
    fn row_empty_children_does_not_touch_buffer() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &[],
            2,
            RowAlignment::Left,
            Rect::new(0, 0, 10, 3),
            &mut buf,
            &ctx,
        );

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    }

    #[test]
    fn row_zero_width_area_does_not_touch_buffer() {
        let children = vec![DocumentElement::text("x")];
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &children,
            2,
            RowAlignment::Left,
            Rect::new(0, 0, 0, 3),
            &mut buf,
            &ctx,
        );

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    }

    #[test]
    fn row_left_alignment_places_children_with_minimum_gap() {
        let children = vec![
            DocumentElement::score_box_element(
                1,
                final_score_box("A", "B"),
                false,
                LinkTarget::Anchor("g1".to_string()),
            ),
            DocumentElement::score_box_element(
                2,
                final_score_box("C", "D"),
                false,
                LinkTarget::Anchor("g2".to_string()),
            ),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &children,
            3,
            RowAlignment::Left,
            Rect::new(0, 0, 60, 6),
            &mut buf,
            &ctx,
        );

        // Second box (width 25) should start at 25 + gap(3) = 28
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "╔"); // first box top border
        assert_eq!(buf.cell((28, 1)).unwrap().symbol(), "╔"); // second box top border
    }

    #[test]
    fn row_spread_alignment_with_single_child_has_no_gap_math() {
        // A single preferred-width child means num_gaps == 0, exercising that branch.
        let children = vec![DocumentElement::score_box_element(
            1,
            final_score_box("A", "B"),
            false,
            LinkTarget::Anchor("g1".to_string()),
        )];
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &children,
            2,
            RowAlignment::Spread,
            Rect::new(0, 0, 40, 6),
            &mut buf,
            &ctx,
        );

        // Single child starts flush at area.x with Spread alignment
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "╔");
    }

    #[test]
    fn row_center_alignment_centers_children_group() {
        let children = vec![
            DocumentElement::score_box_element(
                1,
                final_score_box("A", "B"),
                false,
                LinkTarget::Anchor("g1".to_string()),
            ),
            DocumentElement::score_box_element(
                2,
                final_score_box("C", "D"),
                false,
                LinkTarget::Anchor("g2".to_string()),
            ),
        ];
        // total_content_width = 25 + 2(gap) + 25 = 52; area width 62 -> left_margin = 5
        let mut buf = Buffer::empty(Rect::new(0, 0, 62, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &children,
            2,
            RowAlignment::Center,
            Rect::new(0, 0, 62, 6),
            &mut buf,
            &ctx,
        );

        assert_eq!(buf.cell((5, 1)).unwrap().symbol(), "╔"); // first box shifted by margin
        assert_eq!(buf.cell((5 + 25 + 2, 1)).unwrap().symbol(), "╔"); // second box after gap
    }

    #[test]
    fn row_flexible_children_distribute_width_equally() {
        let children = vec![
            DocumentElement::text("left"),
            DocumentElement::text("right"),
        ];
        // area width 20, gap 2 -> available 18, child_width 9 each
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &children,
            2,
            RowAlignment::Left,
            Rect::new(0, 0, 20, 1),
            &mut buf,
            &ctx,
        );

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "l");
        // second child starts at child_width(9) + gap(2) = 11
        assert_eq!(buf.cell((11, 0)).unwrap().symbol(), "r");
    }

    #[test]
    fn row_mixed_preferred_and_flexible_children_uses_flexible_path() {
        // ScoreBoxElement has a preferred width, Text does not, so `has_preferred_widths`
        // is false and both children fall back to the equal-width distribution branch.
        let children = vec![
            DocumentElement::score_box_element(
                1,
                final_score_box("A", "B"),
                false,
                LinkTarget::Anchor("g1".to_string()),
            ),
            DocumentElement::text("hi"),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &children,
            2,
            RowAlignment::Left,
            Rect::new(0, 0, 20, 6),
            &mut buf,
            &ctx,
        );

        // ScoreBox area was allotted 9 columns (not 25), so it is not drawn at all
        // (ScoreBox::render bails out when area.width < SCORE_BOX_WIDTH).
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), " ");
        // Second child (text) starts at child_width(9) + gap(2) = 11
        assert_eq!(buf.cell((11, 0)).unwrap().symbol(), "h");
    }

    #[test]
    fn row_clips_children_that_do_not_respect_their_own_area_width() {
        // TableWidget lays out columns at their natural width regardless of the area it's
        // given (see TableWidget::render_internal, which calls the unbounded `buf.set_string`).
        // Without per-child clipping in render_row, two such tables side-by-side in a row
        // narrower than their combined natural width would bleed into each other.
        let left = DocumentElement::table("left", text_table(&["AAAAAAAA"]));
        let right = DocumentElement::table("right", text_table(&["BBBBBBBB"]));

        // Row is 10 columns wide; with no preferred width, each child gets 5 columns from the
        // flexible equal-split branch - but the table needs SELECTOR(2) + column(8) = 10.
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(
            &[left, right],
            0,
            RowAlignment::Left,
            Rect::new(0, 0, 10, 3),
            &mut buf,
            &ctx,
        );

        // Right child owns columns 5..10 exclusively; the left table's 'A's must never
        // bleed into it (truncation of the left table's own content is fine and expected).
        let right_columns = (5..10).map(|x| buf[(x, 2)].symbol()).collect::<String>();
        assert!(
            !right_columns.contains('A'),
            "left table bled into right child's columns: {right_columns:?}"
        );
    }

    // ---- clipping contract (#68) ----

    #[test]
    fn element_render_clips_standalone_table_to_its_area() {
        // Natural table width is SELECTOR(2) + column(8) = 10, but the area is
        // only 5 wide. Before the central clip, only tables inside a Row were
        // protected; a standalone table bled into the rest of the buffer.
        let elem = DocumentElement::table("t", text_table(&["AAAAAAAA"]));
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 5, 3), &mut buf, &ctx);

        for y in 0..3 {
            for x in 5..12 {
                assert_eq!(
                    buf.cell((x, y)).unwrap().symbol(),
                    " ",
                    "table bled outside its area at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn element_render_clips_multi_column_table_without_panicking() {
        // The second column's start position (SELECTOR 2 + col 8 + gap 2 = 12)
        // lies beyond the 5-wide area. set_string panics on an out-of-bounds
        // *start*, so this exercises TableWidget's column start guard as well
        // as the central clip.
        let columns: Vec<ColumnDef<&str>> = vec![
            ColumnDef::new("A", 8, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            }),
            ColumnDef::new("B", 8, Alignment::Left, |row: &&str| {
                CellValue::Text(row.to_string())
            }),
        ];
        let elem = DocumentElement::table("t", TableWidget::from_data(&columns, vec!["x"]));
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 5, 3), &mut buf, &ctx);

        for y in 0..3 {
            for x in 5..20 {
                assert_eq!(
                    buf.cell((x, y)).unwrap().symbol(),
                    " ",
                    "table bled outside its area at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn element_render_clips_team_boxscore_vertically() {
        let elem = DocumentElement::team_boxscore(
            "away",
            "AB",
            text_table(&["Alice"]),
            text_table(&[]),
            text_table(&[]),
        );
        let full_height = elem.height();
        let short_height = full_height - 3;
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, full_height + 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Give the element fewer rows than it needs: everything below the
        // area must stay untouched instead of bleeding into later elements.
        elem.render(Rect::new(0, 0, 40, short_height), &mut buf, &ctx);

        for y in short_height..full_height + 2 {
            assert_eq!(
                buf.cell((0, y)).unwrap().symbol(),
                " ",
                "boxscore bled below its area at row {y}"
            );
        }
    }

    #[test]
    fn element_render_clips_custom_render_fn() {
        // A Custom element's render_fn is arbitrary code; the central clip
        // must contain it like any other element.
        let elem = DocumentElement::Custom {
            render_fn: |_area, buf, _ctx| {
                // Deliberately ignore the area and write far outside it.
                buf.set_string(8, 2, "XX", Style::default());
            },
            height: 1,
            focusable: vec![],
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 5, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((8, 2)).unwrap().symbol(), " ");
    }

    #[test]
    fn row_preserves_prestyled_untouched_cells_in_child_areas() {
        // Regression: the old empty-scratch merge reset every cell of a row
        // child's area to a default cell, clobbering the theme background
        // fill that render_elements_to_buffer applies before elements draw.
        let children = vec![DocumentElement::text("hi"), DocumentElement::text("yo")];
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        buf.set_style(area, Style::default().bg(Color::Blue));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_row(&children, 2, RowAlignment::Left, area, &mut buf, &ctx);

        // Text was drawn on row 0.
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "h");
        // Row 1 lies inside both children's areas but is never drawn on: it
        // must keep the pre-applied background.
        assert_eq!(buf.cell((0, 1)).unwrap().style().bg, Some(Color::Blue));
        assert_eq!(buf.cell((11, 1)).unwrap().style().bg, Some(Color::Blue));
    }

    // ---- render_text ----

    #[test]
    fn text_renders_single_line() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_text("Hi", None, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "H");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "i");
    }

    #[test]
    fn text_renders_multiple_lines() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_text(
            "one\ntwo\nthree",
            None,
            Rect::new(0, 0, 10, 3),
            &mut buf,
            &ctx,
        );

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "o");
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "t");
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "t");
    }

    #[test]
    fn text_stops_rendering_lines_beyond_area_height() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        // Pre-fill so we can detect untouched cells.
        for x in 0..10 {
            buf.cell_mut((x, 0)).unwrap().set_char('_');
            buf.cell_mut((x, 1)).unwrap().set_char('_');
        }
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Area height is only 2, but content has 3 lines - third line must be skipped.
        render_text("a\nb\nc", None, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "b");
    }

    #[test]
    fn text_stops_rendering_chars_beyond_area_width() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_text("abcdef", None, Rect::new(0, 0, 3, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "c");
    }

    #[test]
    fn text_uses_context_default_style_when_none_given() {
        // `Buffer::cell().style()` always reconstructs explicit Color::Reset fields,
        // so comparing against a hand-built Style would fail even when behavior is
        // correct. Instead, compare against a second render that passes the context's
        // style explicitly - the two should produce identical cells.
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let mut buf_default = Buffer::empty(Rect::new(0, 0, 5, 1));
        render_text("hi", None, Rect::new(0, 0, 5, 1), &mut buf_default, &ctx);

        let mut buf_explicit = Buffer::empty(Rect::new(0, 0, 5, 1));
        render_text(
            "hi",
            Some(ctx.text_style()),
            Rect::new(0, 0, 5, 1),
            &mut buf_explicit,
            &ctx,
        );

        assert_eq!(
            buf_default.cell((0, 0)).unwrap().style(),
            buf_explicit.cell((0, 0)).unwrap().style()
        );
    }

    #[test]
    fn text_uses_explicit_style_when_given() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        let style = Style::default().fg(Color::Green);

        render_text("hi", Some(style), Rect::new(0, 0, 5, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().style().fg, Some(Color::Green));
    }

    #[test]
    fn text_renders_multibyte_unicode_content() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_text("héllo", None, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "h");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "é");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "l");
    }

    #[test]
    fn text_wide_characters_occupy_two_columns_and_shift_following_glyphs() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // "日" is a double-width glyph; it must occupy columns 0-1 (blanking column 1),
        // pushing "Hi" to start at column 2 - not column 1, as one-column-per-char would.
        render_text("日Hi", None, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "日");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "H");
        assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "i");
    }

    #[test]
    fn text_wide_character_straddling_clip_boundary_is_dropped_not_split() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Only 1 column remains after "a"; the 2-column-wide "日" doesn't fit and must be
        // dropped entirely rather than rendering a corrupted half-glyph.
        render_text("a日", None, Rect::new(0, 0, 2, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
    }

    // ---- render_heading ----

    #[test]
    fn heading_level_1_draws_double_underline() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_heading(1, "Title", Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "T");
        for x in 0..5 {
            assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
        }
        assert_eq!(buf.cell((5, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn heading_level_above_1_has_no_underline() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_heading(2, "Sub", Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "S");
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn heading_level_1_with_single_row_area_skips_underline() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Should not panic even though there's no room for the underline row.
        render_heading(1, "Title", Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "T");
    }

    #[test]
    fn heading_clips_content_wider_than_area() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_heading(2, "abcdef", Rect::new(0, 0, 3, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "c");
    }

    #[test]
    fn heading_underline_length_matches_display_width_of_wide_content() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // "日" (2 cols) + "Hi" (2 cols) = 4 display columns, even though it's only 3 chars.
        // A char-count-based underline would draw only 3 dashes and misalign with the title.
        render_heading(1, "日Hi", Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        for x in 0..4 {
            assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
        }
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), " ");
    }

    // ---- render_section_title ----

    #[test]
    fn section_title_without_underline() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_section_title("Atlantic", false, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "A");
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn section_title_with_underline_matches_title_length() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_section_title("Metro", true, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        for x in 0..5 {
            assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
        }
        assert_eq!(buf.cell((5, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn section_title_underline_skipped_when_no_second_row() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Should not panic even though underline=true and there's no second row.
        render_section_title("Metro", true, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "M");
    }

    #[test]
    fn section_title_underline_matches_display_width_of_wide_title() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // "日" (2 cols) + "Hi" (2 cols) = 4 display columns, even though it's only 3 chars.
        render_section_title("日Hi", true, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        for x in 0..4 {
            assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
        }
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), " ");
    }

    // ---- render_link ----

    #[test]
    fn link_unfocused_has_two_space_prefix() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_link("Go", false, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "G");
    }

    #[test]
    fn link_focused_uses_selector_prefix_and_themeless_modifier() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default(); // theme: None
        let ctx = RenderContext::focused(&config);

        render_link("Go", true, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "▶");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "G");
        let style = buf.cell((2, 0)).unwrap().style();
        assert!(style.add_modifier.contains(Modifier::REVERSED));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn link_focused_uses_theme_colors_when_theme_is_set() {
        use crate::config::THEME_ORANGE;

        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig {
            theme: Some(THEME_ORANGE.clone()),
            ..DisplayConfig::default()
        };
        let ctx = RenderContext::focused(&config);

        render_link("Go", true, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        let style = buf.cell((2, 0)).unwrap().style();
        assert_eq!(style.fg, Some(THEME_ORANGE.selection_text_fg));
        assert_eq!(style.bg, Some(THEME_ORANGE.selection_text_bg));
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(!style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn link_display_text_clips_to_area_width() {
        // Buffer is wider than the area passed to render_link, so this only passes if the
        // display text is clipped to the *area's* width rather than just the buffer's.
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_link(
            "VeryLongLinkText",
            false,
            Rect::new(0, 0, 6, 1),
            &mut buf,
            &ctx,
        );

        // Prefix "  " (2 cols) leaves 4 cols for the display text: "Very".
        assert_eq!(buf.cell((5, 0)).unwrap().symbol(), "y");
        assert_eq!(buf.cell((6, 0)).unwrap().symbol(), " ");
    }

    // ---- render_separator ----

    #[test]
    fn separator_fills_area_with_horizontal_char() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_separator(Rect::new(0, 0, 8, 1), &mut buf, &ctx);

        for x in 0..8 {
            assert_eq!(buf.cell((x, 0)).unwrap().symbol(), "─");
        }
    }

    #[test]
    fn separator_zero_width_area_does_nothing() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_separator(Rect::new(0, 0, 0, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    }

    // ---- render_group ----

    #[test]
    fn group_renders_children_stacked_by_height() {
        let children = vec![
            DocumentElement::text("first"),
            DocumentElement::spacer(1),
            DocumentElement::text("third"),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_group(&children, None, Rect::new(0, 0, 10, 3), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "f");
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "t");
    }

    #[test]
    fn group_stops_rendering_children_beyond_area_height() {
        let children = vec![
            DocumentElement::text("one"),
            DocumentElement::text("two"),
            DocumentElement::text("three"),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        // Pre-fill row 2 which is outside the render area, to confirm it's never touched.
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_group(&children, None, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "o");
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "t");
    }

    #[test]
    fn group_applies_style_patch_over_rendered_region() {
        let children = vec![DocumentElement::text("hi")];
        let style = Style::default().bg(Color::Blue);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_group(
            &children,
            Some(style),
            Rect::new(0, 0, 5, 1),
            &mut buf,
            &ctx,
        );

        // Background is patched on top of the child's own style (fg unaffected).
        assert_eq!(buf.cell((0, 0)).unwrap().style().bg, Some(Color::Blue));
    }

    #[test]
    fn group_with_no_children_and_style_does_not_panic() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        let style = Style::default().bg(Color::Blue);

        render_group(&[], Some(style), Rect::new(0, 0, 5, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    }

    // ---- render_team_boxscore ----

    #[test]
    fn team_boxscore_renders_all_three_sections_with_borders() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 19));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_team_boxscore(
            "AB",
            &text_table(&["Alice"]),
            &text_table(&["Bob"]),
            &text_table(&["Carl"]),
            Rect::new(0, 0, 40, 19),
            &mut buf,
            &ctx,
        );

        // First section header uses the "first section" top-left corner.
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╒");
        assert_eq!(buf.cell((39, 0)).unwrap().symbol(), "╕");
        // Later sections use the T-junction corner instead.
        assert_eq!(buf.cell((0, 6)).unwrap().symbol(), "╞");
        assert_eq!(buf.cell((0, 12)).unwrap().symbol(), "╞");
        // Bottom border on the final row.
        assert_eq!(buf.cell((0, 18)).unwrap().symbol(), "╘");
        assert_eq!(buf.cell((39, 18)).unwrap().symbol(), "╛");
    }

    #[test]
    fn team_boxscore_skips_empty_sections() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 13));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Forwards is empty; defense becomes the first rendered section.
        render_team_boxscore(
            "AB",
            &text_table(&[]),
            &text_table(&["Bob"]),
            &text_table(&["Carl"]),
            Rect::new(0, 0, 40, 13),
            &mut buf,
            &ctx,
        );

        // Defense section is now first, so it should get the top-left corner.
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╒");
    }

    #[test]
    fn team_boxscore_all_sections_empty_renders_only_bottom_border() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_team_boxscore(
            "AB",
            &text_table(&[]),
            &text_table(&[]),
            &text_table(&[]),
            Rect::new(0, 0, 40, 1),
            &mut buf,
            &ctx,
        );

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╘");
        assert_eq!(buf.cell((39, 0)).unwrap().symbol(), "╛");
    }

    #[test]
    fn team_boxscore_width_is_capped_by_area_width() {
        // Forwards section needs header(1) + blank(1) + table(3 rows) + blank(1) + bottom border(1) = 7
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 7));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_team_boxscore(
            "AB",
            &text_table(&["Alice"]),
            &text_table(&[]),
            &text_table(&[]),
            Rect::new(0, 0, 20, 7),
            &mut buf,
            &ctx,
        );

        // Right border should be at x=19 (area width - 1), not at TEAM_BOXSCORE_WIDTH - 1.
        assert_eq!(buf.cell((19, 0)).unwrap().symbol(), "╕");
    }

    #[test]
    fn team_boxscore_rendered_rows_match_element_height() {
        // Regression test for #67: height() and render_team_boxscore used to
        // derive the section chrome independently. The bottom border must land
        // exactly on the last row that height() reserves, with nothing drawn
        // past it.
        let elem = DocumentElement::team_boxscore(
            "away",
            "AB",
            text_table(&["Alice", "Anna"]),
            text_table(&["Bob"]),
            text_table(&[]),
        );
        let height = elem.height();

        // Render into a taller buffer so drift past the reserved rows is visible.
        let area = Rect::new(0, 0, 40, height + 3);
        let mut buf = Buffer::empty(area);
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        elem.render(area, &mut buf, &ctx);

        assert_eq!(buf.cell((0, height - 1)).unwrap().symbol(), "╘");
        assert_eq!(buf.cell((0, height)).unwrap().symbol(), " ");
    }

    // ---- render_section_header ----

    #[test]
    fn section_header_first_section_uses_top_corners() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_section_header(0, 0, 30, "Team - Forwards", true, &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╒");
        assert_eq!(buf.cell((29, 0)).unwrap().symbol(), "╕");
    }

    #[test]
    fn section_header_later_section_uses_t_junction_corners() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_section_header(0, 0, 30, "Team - Defense", false, &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╞");
        assert_eq!(buf.cell((29, 0)).unwrap().symbol(), "╡");
    }

    #[test]
    fn section_header_embeds_title_text() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_section_header(0, 0, 30, "Sharks - Goalies", true, &mut buf, &ctx);
        let line = (0..30).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        assert!(line.contains("Sharks - Goalies"));
    }

    #[test]
    fn section_header_narrow_width_truncates_title_and_preserves_corner() {
        // Buffer is much wider than the assigned header width, so this only passes if the
        // title is truncated to the assigned width rather than bleeding past it before the
        // corner gets a chance to (incidentally) paper over the overflow.
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_section_header(0, 0, 10, "Team - Forwards", true, &mut buf, &ctx);

        // Right corner always lands on the last column of the *assigned* width (9), not
        // wherever the untruncated title happened to end.
        assert_eq!(buf.cell((9, 0)).unwrap().symbol(), "╕");
        // Nothing should have bled past the assigned width into the rest of the buffer.
        assert_eq!(buf.cell((10, 0)).unwrap().symbol(), " ");
    }

    // ---- render_bottom_border ----

    #[test]
    fn bottom_border_draws_corners_and_double_horizontal_middle() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_bottom_border(0, 0, 10, &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╘");
        assert_eq!(buf.cell((9, 0)).unwrap().symbol(), "╛");
        for x in 1..9 {
            assert_eq!(buf.cell((x, 0)).unwrap().symbol(), "═");
        }
    }

    #[test]
    fn bottom_border_width_one_skips_right_corner() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Should not panic despite width - 2 underflowing conceptually (it saturates).
        render_bottom_border(0, 0, 1, &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╘");
    }

    // ---- render_tabs ----

    #[test]
    fn tabs_empty_list_does_not_render() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tabs(&[], 0, Rect::new(0, 0, 20, 5), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    }

    #[test]
    fn tabs_area_shorter_than_bar_height_does_not_render() {
        let tabs = vec![DocTabDef::new(
            "t1",
            "One",
            vec![DocumentElement::text("content")],
        )];
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tabs(&tabs, 0, Rect::new(0, 0, 20, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    }

    #[test]
    fn tabs_renders_bar_and_active_tab_content() {
        let tabs = vec![
            DocTabDef::new("t1", "One", vec![DocumentElement::text("first")]),
            DocTabDef::new("t2", "Two", vec![DocumentElement::text("second")]),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tabs(&tabs, 0, Rect::new(0, 0, 30, 4), &mut buf, &ctx);
        let bar = (0..30).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        assert!(bar.contains("One"));
        assert!(bar.contains("Two"));
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "f"); // "first" content on row 2
    }

    #[test]
    fn tabs_renders_second_tab_content_when_active() {
        let tabs = vec![
            DocTabDef::new("t1", "One", vec![DocumentElement::text("first")]),
            DocTabDef::new("t2", "Two", vec![DocumentElement::text("second")]),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tabs(&tabs, 1, Rect::new(0, 0, 30, 4), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "s"); // "second" content on row 2
    }

    #[test]
    fn tabs_active_index_out_of_bounds_renders_bar_but_no_content() {
        let tabs = vec![DocTabDef::new(
            "t1",
            "One",
            vec![DocumentElement::text("first")],
        )];
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // active_index 5 is out of range - should not panic, and no content renders.
        render_tabs(&tabs, 5, Rect::new(0, 0, 30, 4), &mut buf, &ctx);

        let bar = (0..30).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        assert!(bar.contains("One"));
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), " ");
    }

    #[test]
    fn tabs_content_stops_rendering_beyond_content_area_height() {
        let tabs = vec![DocTabDef::new(
            "t1",
            "One",
            vec![
                DocumentElement::text("row1"),
                DocumentElement::text("row2"),
                DocumentElement::text("row3"),
            ],
        )];
        // Bar takes 2 rows, leaving only 1 row of content area - row2/row3 are clipped.
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tabs(&tabs, 0, Rect::new(0, 0, 30, 3), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "r"); // "row1"
    }

    // ---- render_tab_bar ----

    #[test]
    fn tab_bar_active_tab_uses_emphasis_style() {
        let tabs = vec![
            DocTabDef::new("t1", "One", vec![]),
            DocTabDef::new("t2", "Two", vec![]),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tab_bar(&tabs, 0, 0, 0, 30, &mut buf, &ctx);

        // " One " occupies columns 0..5; the label text starts at column 1.
        // Active tab uses emphasis style, which is bold (unlike plain text style).
        assert!(buf
            .cell((1, 0))
            .unwrap()
            .style()
            .add_modifier
            .contains(Modifier::BOLD));
        // "Two" tab (not active) uses the base text style, which is not bold.
        let two_start = " One ".chars().count() as u16 + 1; // +1 for the separator
        assert!(!buf
            .cell((two_start + 1, 0))
            .unwrap()
            .style()
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn tab_bar_separator_line_filled_and_has_tee_connectors() {
        let tabs = vec![
            DocTabDef::new("t1", "One", vec![]),
            DocTabDef::new("t2", "Two", vec![]),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tab_bar(&tabs, 0, 0, 0, 30, &mut buf, &ctx);
        // Every cell on the separator row should be the horizontal char or a tee.
        for x in 0..30 {
            let symbol = buf.cell((x, 1)).unwrap().symbol();
            assert!(symbol == "─" || symbol == "┴");
        }
        // There should be exactly one tee connector (boundary between the two tabs).
        let tee_count = (0..30)
            .filter(|&x| buf.cell((x, 1)).unwrap().symbol() == "┴")
            .count();
        assert_eq!(tee_count, 1);
    }

    #[test]
    fn tab_bar_single_tab_has_no_separator_or_tee() {
        let tabs = vec![DocTabDef::new("t1", "Only", vec![])];
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tab_bar(&tabs, 0, 0, 0, 20, &mut buf, &ctx);

        for x in 0..20 {
            assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "─");
        }
    }

    #[test]
    fn tab_bar_wide_tab_title_advances_by_display_width_not_char_count() {
        let tabs = vec![
            DocTabDef::new("t1", "日本", vec![]), // 2 chars, but 4 display columns
            DocTabDef::new("t2", "Two", vec![]),
        ];
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 2));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        render_tab_bar(&tabs, 0, 0, 0, 30, &mut buf, &ctx);

        // Label " 日本 " occupies 6 display columns (1 + 2 + 2 + 1), so the separator must
        // land at column 6 - not column 4, which is where char-count-based math would put it
        // (right on top of "本"'s continuation cell).
        assert_eq!(buf.cell((6, 0)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((8, 0)).unwrap().symbol(), "T");
    }
}
