//! Table-backed composite element rendering: the bordered team boxscore
//! (sections + header/bottom borders) and the tabbed-content widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use crate::config::RenderContext;
use crate::tui::component::ElementWidget;
use crate::tui::components::TableWidget;

use super::render::render_line;
use super::render_layout::clipped;

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
    // Use fixed width but respect area constraints
    let width = TEAM_BOXSCORE_WIDTH.min(area.width);

    // Rows past `bottom` are skipped (not drawn) rather than attempted:
    // ratatui's set_string panics when its start position is out of bounds.
    // `y` still advances through skipped rows to keep the section math intact.
    let bottom = area.bottom();

    let mut y = area.y;
    let mut is_first_section = true;

    let sections: [(&str, &TableWidget); 3] = [
        ("Forwards", forwards_table),
        ("Defense", defense_table),
        ("Goalies", goalies_table),
    ];

    for (section_name, table) in sections {
        if table.row_count() == 0 {
            continue;
        }
        y += render_boxscore_section(
            team_name,
            section_name,
            table,
            is_first_section,
            area.x,
            y,
            width,
            bottom,
            buf,
            ctx,
        );
        is_first_section = false;
    }

    // Bottom border (skipped entirely when the area ran out of rows)
    if y < bottom {
        render_bottom_border(area.x, y, width, buf, ctx);
    }
}

/// Render one non-empty section (header + table + blank lines) of a team
/// boxscore starting at row `y`. Returns rows drawn, always equal to
/// `team_boxscore_section_height(table)`.
#[allow(clippy::too_many_arguments)]
fn render_boxscore_section(
    team_name: &str,
    section_name: &str,
    table: &TableWidget,
    is_first: bool,
    x: u16,
    y: u16,
    width: u16,
    bottom: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) -> u16 {
    let section_start = y;
    let mut y = y;

    // Section header with embedded title
    if y < bottom {
        let title = format!("{} - {}", team_name, section_name);
        render_section_header(x, y, width, &title, is_first, buf, ctx);
    }
    y += 1;

    render_boxscore_blank_line(x, y, width, bottom, buf, ctx);
    y += 1;

    // Table content - render with side borders
    let table_height = table.preferred_height().unwrap_or(0);
    let visible_table_height = table_height.min(bottom.saturating_sub(y));
    render_boxscore_table_rows(table, x, y, width, visible_table_height, buf, ctx);
    y += table_height;

    // Blank line after table (before next section or bottom border)
    render_boxscore_blank_line(x, y, width, bottom, buf, ctx);
    y += 1;

    debug_assert_eq!(
        y - section_start,
        team_boxscore_section_height(table),
        "rows drawn for a boxscore section drifted from team_boxscore_section_height"
    );

    y - section_start
}

/// Draw one empty bordered line (just the left/right `│`), skipped entirely
/// if `y` has run past the available area.
fn render_boxscore_blank_line(
    x: u16,
    y: u16,
    width: u16,
    bottom: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    if y >= bottom {
        return;
    }
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();
    buf.set_string(x, y, bc.vertical, border_style);
    if width > 1 {
        buf.set_string(x + width - 1, y, bc.vertical, border_style);
    }
}

/// Draw a table's rows inside side borders, clipped so an oversized table
/// truncates instead of overwriting the right border or panicking in
/// `set_string`.
fn render_boxscore_table_rows(
    table: &TableWidget,
    x: u16,
    y: u16,
    width: u16,
    visible_table_height: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();
    let inner_width = width.saturating_sub(2); // Subtract 2 for side borders

    for row in 0..visible_table_height {
        buf.set_string(x, y + row, bc.vertical, border_style); // Left border
        if width > 1 {
            buf.set_string(x + width - 1, y + row, bc.vertical, border_style); // Right border
        }
    }

    let table_area = Rect::new(x + 1, y, inner_width, visible_table_height);
    clipped(table_area, buf, |scratch| {
        table.render(table_area, scratch, ctx)
    });
}

/// Render section header with embedded title
///
/// First section: ╒══╡ Title ╞═══════════════════════════════════╕
/// Later sections: ╞══╡ Title ╞═══════════════════════════════════╡
pub(super) fn render_section_header(
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

    // Choose corner characters based on whether this is first section
    let (left_corner, right_corner): (&str, &str) = if is_first {
        (bc.mixed_dh_top_left, bc.mixed_dh_top_right)
    } else {
        (bc.mixed_dh_left_t, bc.mixed_dh_right_t)
    };

    // The last column is reserved for the right corner so it always wins, even when the
    // title is too long for the available width: everything else is laid out left-to-right
    // within the remaining budget and clipped/truncated to fit before it, rather than being
    // written at a fixed offset and later overwritten mid-glyph by the corner.
    const RIGHT_CORNER_WIDTH: u16 = 1;
    let body_end = x + width.saturating_sub(RIGHT_CORNER_WIDTH);

    let cursor = render_section_header_prefix(x, y, body_end, title, left_corner, buf, ctx);

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

/// Draw the header's left cap, embedded title, and trailing `╞` divider:
/// `corner + == + ╡ + title + ╞`. Returns the cursor x after the last glyph.
fn render_section_header_prefix(
    x: u16,
    y: u16,
    body_end: u16,
    title: &str,
    left_corner: &str,
    buf: &mut Buffer,
    ctx: &RenderContext,
) -> u16 {
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();
    let title_style = ctx.text_style();

    let title_prefix = format!(
        "{}{}{}",
        left_corner,
        bc.double_horizontal.repeat(2),
        bc.mixed_dh_right_t,
    );
    let title_with_space = format!(" {} ", title);

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
    cursor
}

/// Render bottom border: ╘═══════════════════════════════════════════════╛
pub(super) fn render_bottom_border(
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
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
pub(super) fn render_tab_bar(
    tabs: &[super::DocTabDef],
    active_index: usize,
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    // Calculate tab widths (each tab gets its title's display width + padding). Using display
    // width (not char count) matters once a title contains a double-width glyph: the label is
    // rendered with the real glyph width regardless, so an undercounted width here would place
    // the next tab's separator on top of the previous tab's still-visible content.
    const TAB_PADDING: u16 = 2; // space before and after title
    let tab_widths: Vec<u16> = tabs
        .iter()
        .map(|t| t.title.width() as u16 + TAB_PADDING)
        .collect();

    render_tab_labels(tabs, &tab_widths, active_index, x, y, width, buf, ctx);
    render_tab_separator(&tab_widths, x, y, width, buf, ctx);
}

/// Line 1 of the tab bar: each tab's label, highlighted when active, with a
/// `│` divider between consecutive tabs.
#[allow(clippy::too_many_arguments)]
fn render_tab_labels(
    tabs: &[super::DocTabDef],
    tab_widths: &[u16],
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
}

/// Line 2 of the tab bar: a horizontal rule with tee connectors dropped at
/// each tab boundary, underneath the dividers drawn by [`render_tab_labels`].
fn render_tab_separator(
    tab_widths: &[u16],
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
    ctx: &RenderContext,
) {
    let bc = ctx.box_chars();
    let border_style = ctx.boxchar_style();
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
        if idx < tab_widths.len() - 1 && connector_x < x + width {
            let cell = buf.cell_mut((connector_x, y + 1));
            if let Some(cell) = cell {
                cell.set_char(tee_char);
            }
            connector_x += 1; // Account for the vertical separator
        }
    }
}
