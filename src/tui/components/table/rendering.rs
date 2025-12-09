//! Rendering logic for TableWidget
//!
//! This module contains the internal rendering implementation for tables.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::config::DisplayConfig;
use crate::tui::CellValue;

use super::{TableWidget, SELECTOR_WIDTH};

impl TableWidget {
    /// Get the style for a cell based on whether it's the focused link cell
    ///
    /// Only link cells in focused rows get the selection style.
    /// Other cells use normal styling.
    pub(super) fn get_cell_style(
        &self,
        is_row_focused: bool,
        cell_value: &CellValue,
        config: &DisplayConfig,
    ) -> Style {
        let is_focused_link = is_row_focused && cell_value.is_link();

        if is_focused_link {
            // Focused link cell: use REVERSED + BOLD modifier
            if let Some(theme) = &config.theme {
                Style::default()
                    .fg(theme.fg2)
                    .add_modifier(crate::config::SELECTION_STYLE_MODIFIER)
            } else {
                Style::default().add_modifier(crate::config::SELECTION_STYLE_MODIFIER)
            }
        } else {
            // Not focused or not a link: use fg2 from theme (or default if no theme)
            if let Some(theme) = &config.theme {
                Style::default().fg(theme.fg2)
            } else {
                Style::default()
            }
        }
    }

    /// Internal render implementation
    pub(super) fn render_internal(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let mut y = area.y;

        // Render column headers
        if y < area.bottom() {
            let mut x = area.x + SELECTOR_WIDTH as u16;

            let col_header_style = if let Some(theme) = &config.theme {
                Style::default().fg(theme.fg2).add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };

            for (col_idx, header) in self.column_headers.iter().enumerate() {
                let width = self.column_widths[col_idx];
                let align = self.column_aligns[col_idx];
                let formatted = self.format_cell(header, width, align);
                buf.set_string(x, y, &formatted, col_header_style);
                x += width as u16 + 2;
            }
            y += 1;
        }

        // Render separator line under headers
        if y < area.bottom() {
            let total_width: usize = self.column_widths.iter().sum::<usize>()
                + (self.column_widths.len().saturating_sub(1) * 2);

            let separator = config.box_chars.horizontal.repeat(total_width);
            let separator_line = format!("{}{}", " ".repeat(SELECTOR_WIDTH), separator);

            let separator_style = if let Some(theme) = &config.theme {
                Style::default().fg(theme.fg3)
            } else {
                Style::default()
            };

            buf.set_string(area.x, y, &separator_line, separator_style);
            y += 1;
        }

        // Render rows
        for (row_idx, row_cells) in self.cell_data.iter().enumerate() {
            if y >= area.bottom() {
                break;
            }

            let is_row_focused = self.focused_row == Some(row_idx);

            // Render selector indicator
            let selector = if is_row_focused {
                format!("{} ", config.box_chars.selector)
            } else {
                " ".repeat(SELECTOR_WIDTH)
            };

            // Render selector
            let selector_style = if let Some(theme) = &config.theme {
                Style::default().fg(theme.fg2)
            } else {
                Style::default()
            };
            buf.set_string(area.x, y, &selector, selector_style);

            // Render cells
            let mut x = area.x + SELECTOR_WIDTH as u16;
            for (col_idx, cell_value) in row_cells.iter().enumerate() {
                let width = self.column_widths[col_idx];
                let align = self.column_aligns[col_idx];
                let cell_text = cell_value.display_text();
                let formatted = self.format_cell(cell_text, width, align);

                let style = self.get_cell_style(is_row_focused, cell_value, config);

                buf.set_string(x, y, &formatted, style);
                x += width as u16 + 2;
            }

            y += 1;
        }
    }
}
