//! Rendering logic for TableWidget
//!
//! This module contains the internal rendering implementation for tables.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::config::RenderContext;
use crate::tui::CellValue;

use super::{TableWidget, SELECTOR_WIDTH};

impl TableWidget {
    /// Get the style for a cell based on whether it receives selection styling
    ///
    /// Cells that receive selection style (StyledText, PlayerLink, TeamLink)
    /// get selection styling when the row is focused.
    /// Other cells use normal styling.
    pub(super) fn get_cell_style(
        &self,
        is_row_focused: bool,
        cell_value: &CellValue,
        ctx: &RenderContext,
    ) -> Style {
        let base = ctx.base_style();
        let is_styled = is_row_focused && cell_value.receives_selection_style();

        if is_styled {
            // Styled cell in focused row: use selection colors
            if let Some(theme) = ctx.theme() {
                base.fg(theme.selection_text_fg)
                    .bg(theme.selection_text_bg)
                    .add_modifier(crate::config::SELECTION_STYLE_MODIFIER)
            } else {
                base.add_modifier(crate::config::THEMELESS_SELECTION_STYLE_MODIFIER)
            }
        } else {
            // Not focused or not styled: use text_style (handles dimming automatically)
            ctx.text_style()
        }
    }

    /// Internal render implementation
    pub(super) fn render_internal(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Fill entire area with background color first
        buf.set_style(area, ctx.base_style());

        let mut y = area.y;

        // Render column headers
        if y < area.bottom() {
            let mut x = area.x + SELECTOR_WIDTH as u16;

            let col_header_style = ctx.text_style().add_modifier(Modifier::BOLD);

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

            let separator = ctx.box_chars().horizontal.repeat(total_width);
            let separator_line = format!("{}{}", " ".repeat(SELECTOR_WIDTH), separator);

            buf.set_string(area.x, y, &separator_line, ctx.boxchar_style());
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
                format!("{} ", ctx.box_chars().selector)
            } else {
                " ".repeat(SELECTOR_WIDTH)
            };

            // Render selector
            buf.set_string(area.x, y, &selector, ctx.boxchar_style());

            // Render cells
            let mut x = area.x + SELECTOR_WIDTH as u16;
            for (col_idx, cell_value) in row_cells.iter().enumerate() {
                let width = self.column_widths[col_idx];
                let align = self.column_aligns[col_idx];
                let cell_text = cell_value.display_text();
                let formatted = self.format_cell(cell_text, width, align);

                let style = self.get_cell_style(is_row_focused, cell_value, ctx);

                buf.set_string(x, y, &formatted, style);

                // Style the gap if current cell is styled AND next cell is also styled
                let next_cell = row_cells.get(col_idx + 1);
                let current_styled = is_row_focused && cell_value.receives_selection_style();
                let next_styled = next_cell
                    .map(|c| is_row_focused && c.receives_selection_style())
                    .unwrap_or(false);

                if current_styled && next_styled {
                    buf.set_string(x + width as u16, y, "  ", style);
                }

                x += width as u16 + 2;
            }

            y += 1;
        }
    }
}
