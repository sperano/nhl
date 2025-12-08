//! Rendering functions for document elements
//!
//! This module contains all the render_* helper functions used by DocumentElement::render()

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::config::DisplayConfig;
use crate::tui::component::ElementWidget;
use crate::tui::components::TableWidget;
use crate::tui::widgets::StandaloneWidget;

use super::DocumentElement;

/// Fixed width for team boxscore
pub const TEAM_BOXSCORE_WIDTH: u16 = 85;

/// Gap between two team boxscores when displayed side by side
pub const TEAM_BOXSCORE_GAP: u16 = 2;

/// Minimum width needed to display two team boxscores side by side
pub const TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH: u16 = TEAM_BOXSCORE_WIDTH * 2 + TEAM_BOXSCORE_GAP;

/// Render a horizontal row of elements
pub(super) fn render_row(
    children: &[DocumentElement],
    gap: u16,
    area: Rect,
    buf: &mut Buffer,
    config: &DisplayConfig,
) {
    if children.is_empty() || area.width == 0 {
        return;
    }

    // Check if children have preferred widths (e.g., ScoreBoxElement, GameBoxElement)
    // If so, use preferred width; otherwise distribute space equally
    let has_preferred_widths = children.iter().all(|c| get_preferred_width(c).is_some());

    let mut x_offset = area.x;

    if has_preferred_widths {
        // Use preferred widths for fixed-size widgets
        for child in children {
            let child_width = get_preferred_width(child).unwrap_or(0);
            let child_area = Rect::new(x_offset, area.y, child_width, area.height);
            child.render(child_area, buf, config);
            x_offset += child_width + gap;
        }
    } else {
        // Distribute space equally for flexible elements
        let num_children = children.len() as u16;
        let total_gap = gap * (num_children.saturating_sub(1));
        let available_width = area.width.saturating_sub(total_gap);
        let child_width = available_width / num_children;

        for child in children {
            let child_area = Rect::new(x_offset, area.y, child_width, area.height);
            child.render(child_area, buf, config);
            x_offset += child_width + gap;
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

/// Render a text element
pub(super) fn render_text(
    content: &str,
    style: Option<Style>,
    area: Rect,
    buf: &mut Buffer,
    config: &DisplayConfig,
) {
    let default_style = config.text_style();

    for (i, line) in content.lines().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        let y = area.y + i as u16;
        for (x, ch) in line.chars().enumerate() {
            if x as u16 >= area.width {
                break;
            }
            let cell = buf.cell_mut((area.x + x as u16, y));
            if let Some(cell) = cell {
                cell.set_char(ch);
                cell.set_style(style.unwrap_or(default_style));
            }
        }
    }
}

/// Render a heading element
pub(super) fn render_heading(level: u8, content: &str, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
    let style = config.heading_style(level);

    // Render heading text
    for (x, ch) in content.chars().enumerate() {
        if x as u16 >= area.width {
            break;
        }
        let cell = buf.cell_mut((area.x + x as u16, area.y));
        if let Some(cell) = cell {
            cell.set_char(ch);
            cell.set_style(style);
        }
    }

    // Render underline for level 1 with muted color
    if level == 1 && area.height > 1 {
        let underline_style = config.muted_style();

        for x in 0..area.width.min(content.chars().count() as u16) {
            let cell = buf.cell_mut((area.x + x, area.y + 1));
            if let Some(cell) = cell {
                cell.set_char('═');
                cell.set_style(underline_style);
            }
        }
    }
}

/// Render a section title element
pub(super) fn render_section_title(
    content: &str,
    underline: bool,
    area: Rect,
    buf: &mut Buffer,
    config: &DisplayConfig,
) {
    use ratatui::style::Modifier;

    // Bold style with theme fg1 color if available
    let style = if let Some(theme) = &config.theme {
        Style::default().fg(theme.fg1).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };

    // Render title text
    buf.set_string(area.x, area.y, content, style);

    // Render underline if enabled
    if underline && area.height > 1 {
        let underline_style = if let Some(theme) = &config.theme {
            Style::default().fg(theme.fg2)
        } else {
            Style::default()
        };

        //TODO: use Boxchar instead of hardcoded unicode character
        let underline_str: String = "═".repeat(content.chars().count());
        buf.set_string(area.x, area.y + 1, &underline_str, underline_style);
    }
}

/// Render a link element
pub(super) fn render_link(display: &str, focused: bool, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
    use crate::config::SELECTION_STYLE_MODIFIER;

    let base_style = config.text_style();

    let (prefix, link_style) = if focused {
        let prefix = format!("{} ", config.box_chars.selector);
        let style = base_style.add_modifier(SELECTION_STYLE_MODIFIER);
        (prefix, style)
    } else {
        // Use spaces to align with focused items
        ("  ".to_string(), base_style)
    };

    let prefix_len = prefix.chars().count() as u16;

    buf.set_string(area.x, area.y, &prefix, base_style);
    buf.set_string(area.x + prefix_len, area.y, display, link_style);
}

/// Render a separator element
pub(super) fn render_separator(area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
    let sep_str = &config.box_chars.horizontal;
    let sep_char = sep_str.chars().next().unwrap_or('-');
    let style = config.muted_style();

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
    config: &DisplayConfig,
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
        child.render(child_area, buf, config);
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
    config: &DisplayConfig,
) {
    let bc = &config.box_chars;
    let border_style = config.muted_style();

    // Use fixed width but respect area constraints
    let width = TEAM_BOXSCORE_WIDTH.min(area.width);
    let inner_width = width.saturating_sub(2); // Subtract 2 for side borders

    let mut y = area.y;
    let mut is_first_section = true;

    // Helper to render an empty bordered line
    let render_empty_bordered_line = |y: u16, buf: &mut Buffer| {
        buf.set_string(area.x, y, &bc.vertical, border_style);
        if width > 1 {
            buf.set_string(area.x + width - 1, y, &bc.vertical, border_style);
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

        // Section header with embedded title
        let title = format!("{} - {}", team_name, section_name);
        render_section_header(
            area.x,
            y,
            width,
            &title,
            is_first_section,
            buf,
            config,
        );
        y += 1;
        is_first_section = false;

        // Blank line after header
        render_empty_bordered_line(y, buf);
        y += 1;

        // Table content - render with side borders
        let table_height = table.preferred_height().unwrap_or(0);
        for row in 0..table_height {
            // Left border
            buf.set_string(area.x, y + row, &bc.vertical, border_style);
            // Right border
            if width > 1 {
                buf.set_string(area.x + width - 1, y + row, &bc.vertical, border_style);
            }
        }

        // Render table content inside borders
        let table_area = Rect::new(area.x + 1, y, inner_width, table_height);
        table.render(table_area, buf, config);
        y += table_height;

        // Blank line after table (before next section or bottom border)
        render_empty_bordered_line(y, buf);
        y += 1;
    }

    // Bottom border
    render_bottom_border(area.x, y, width, buf, config);
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
    config: &DisplayConfig,
) {
    let bc = &config.box_chars;
    let border_style = config.muted_style();
    let title_style = config.text_style();

    // Choose corner characters based on whether this is first section
    let (left_corner, right_corner) = if is_first {
        (&bc.mixed_dh_top_left, &bc.mixed_dh_top_right)
    } else {
        (&bc.mixed_dh_left_t, &bc.mixed_dh_right_t)
    };

    // Build the header line: corner + == + ╡ + title + ╞ + === + corner
    let title_prefix = format!("{}{}{}",
        left_corner,
        bc.double_horizontal.repeat(2),
        &bc.mixed_dh_right_t,
    );
    let title_suffix = format!("{}",
        &bc.mixed_dh_left_t,
    );

    // Calculate remaining space for trailing ═
    let prefix_len = 4; // corner + 2x═ + ╡
    let suffix_len = 2; // ╞ + corner
    let title_len = title.chars().count();
    let used = prefix_len + title_len + suffix_len;
    let remaining = (width as usize).saturating_sub(used);

    // Render prefix
    buf.set_string(x, y, &title_prefix, border_style);

    // Render title (with space padding)
    let title_with_space = format!(" {} ", title);
    buf.set_string(x + 4, y, &title_with_space, title_style);

    // Render suffix (╞ + trailing ═ + corner)
    let suffix_x = x + 4 + title_with_space.chars().count() as u16;
    buf.set_string(suffix_x, y, &title_suffix, border_style);

    // Trailing ═
    let trailing = bc.double_horizontal.repeat(remaining.saturating_sub(1));
    buf.set_string(suffix_x + 1, y, &trailing, border_style);

    // Right corner
    buf.set_string(x + width - 1, y, right_corner, border_style);
}

/// Render bottom border: ╘═══════════════════════════════════════════════╛
fn render_bottom_border(
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
    config: &DisplayConfig,
) {
    let bc = &config.box_chars;
    let border_style = config.muted_style();

    // Left corner
    buf.set_string(x, y, &bc.mixed_dh_bottom_left, border_style);

    // Middle ═
    let middle_width = width.saturating_sub(2) as usize;
    let middle = bc.double_horizontal.repeat(middle_width);
    buf.set_string(x + 1, y, &middle, border_style);

    // Right corner
    if width > 1 {
        buf.set_string(x + width - 1, y, &bc.mixed_dh_bottom_right, border_style);
    }
}
