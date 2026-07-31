//! Core behavior of `DocumentElement`: computing layout height, collecting
//! focusable regions for keyboard navigation, and dispatching to the
//! `render` module's `render_*` functions.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

use crate::big_digits::BIG_DIGIT_HEIGHT;
use crate::config::RenderContext;
use crate::tui::component::ElementWidget;
use crate::tui::document::focus::{FocusableElement, FocusableId, RowPosition};
use crate::tui::widgets::StandaloneWidget;

use super::render::{
    self, render_group, render_heading, render_link, render_row, render_section_title,
    render_separator, render_team_boxscore, render_text,
};
use super::{DocumentElement, TAB_BAR_HEIGHT};

impl DocumentElement {
    /// Calculate the height this element needs
    pub fn height(&self) -> u16 {
        match self {
            Self::Text { content, .. } => {
                // Count lines in text (minimum 1)
                content.lines().count().max(1) as u16
            }
            Self::Heading { level, .. } => render::heading_height(*level),
            Self::SectionTitle { underline, .. } => render::section_title_height(*underline),
            Self::Link { .. } => 1,
            Self::Separator => 1,
            Self::Spacer { height } => *height,
            Self::Group { children, .. } => children.iter().map(|c| c.height()).sum(),
            Self::Custom { height, .. } => *height,
            Self::Table { widget, .. } => widget.preferred_height().unwrap_or(0),
            Self::Row { children, .. } => {
                // Height is the maximum height of all children (side by side)
                children.iter().map(|c| c.height()).max().unwrap_or(0)
            }
            Self::ScoreBoxElement { score_box, .. } => {
                // ScoreBox has fixed height of 6
                score_box.preferred_height().unwrap_or(6)
            }
            Self::Indented { element, .. } => {
                // Same height as inner element
                element.height()
            }
            Self::TeamBoxscore {
                forwards_table,
                defense_table,
                goalies_table,
                ..
            } => render::team_boxscore_height(forwards_table, defense_table, goalies_table),
            Self::BigScoreElement { big_score } => {
                big_score.preferred_height().unwrap_or(BIG_DIGIT_HEIGHT + 1)
            }
            Self::Tabs {
                tabs, active_index, ..
            } => render::tabs_height(tabs, *active_index),
        }
    }
}

impl DocumentElement {
    /// Collect focusable elements from this element
    ///
    /// # Arguments
    /// - `out`: Vector to append focusable elements to
    /// - `y_offset`: Current y offset in the document
    pub fn collect_focusable(&self, out: &mut Vec<FocusableElement>, y_offset: u16) {
        match self {
            Self::Link {
                display,
                target,
                id,
                ..
            } => {
                out.push(FocusableElement {
                    id: FocusableId::link(id),
                    y: y_offset,
                    height: 1,
                    // Display width, not char count: render_link draws the label
                    // with real glyph widths, so the rect must match what's on
                    // screen for wide glyphs (CJK, emoji).
                    rect: Rect::new(0, y_offset, display.width() as u16, 1),
                    link_target: Some(target.clone()),
                    row_position: None,
                });
            }
            Self::Group { children, .. } => {
                let mut child_offset = y_offset;
                for child in children {
                    child.collect_focusable(out, child_offset);
                    child_offset += child.height();
                }
            }
            Self::Custom { focusable, .. } => {
                // Add focusable elements with adjusted y positions
                for elem in focusable {
                    let mut adjusted = elem.clone();
                    adjusted.y += y_offset;
                    adjusted.rect.y += y_offset;
                    out.push(adjusted);
                }
            }
            Self::Table { focusable, .. } => {
                for elem in focusable {
                    let mut adjusted = elem.clone();
                    adjusted.y += y_offset;
                    adjusted.rect.y += y_offset;
                    out.push(adjusted);
                }
            }
            Self::Row { children, .. } => {
                // Collect left to right - all elements from first child, then second, etc.
                // Set row_position so left/right navigation can jump between children
                for (child_idx, child) in children.iter().enumerate() {
                    let start_idx = out.len();
                    child.collect_focusable(out, y_offset);
                    // Tag each element with its row position
                    for (idx_within_child, elem) in out[start_idx..].iter_mut().enumerate() {
                        elem.row_position = Some(RowPosition {
                            row_y: y_offset,
                            child_idx,
                            idx_within_child,
                        });
                    }
                }
            }
            Self::ScoreBoxElement {
                game_id,
                score_box,
                link_target,
                ..
            } => {
                // ScoreBox is a single focusable element with typed GameLink ID
                let height = score_box.preferred_height().unwrap_or(6);
                let width = score_box.preferred_width().unwrap_or(25);
                out.push(FocusableElement {
                    id: FocusableId::game_link(*game_id),
                    y: y_offset,
                    height,
                    rect: Rect::new(0, y_offset, width, height),
                    link_target: Some(link_target.clone()),
                    row_position: None,
                });
            }
            Self::Indented { element, .. } => {
                // Delegate to inner element (margin doesn't affect focusable collection)
                element.collect_focusable(out, y_offset);
            }
            Self::TeamBoxscore { focusable, .. } => {
                // Add focusable elements with adjusted y positions
                for elem in focusable {
                    let mut adjusted = elem.clone();
                    adjusted.y += y_offset;
                    adjusted.rect.y += y_offset;
                    out.push(adjusted);
                }
            }
            Self::Tabs {
                tabs, active_index, ..
            } => {
                // Only collect focusable elements from the active tab
                if let Some(tab) = tabs.get(*active_index) {
                    // Content starts after tab bar
                    let content_y = y_offset + TAB_BAR_HEIGHT;
                    let mut content_offset = content_y;
                    for child in &tab.content {
                        child.collect_focusable(out, content_offset);
                        content_offset += child.height();
                    }
                }
            }
            _ => {}
        }
    }
}

impl DocumentElement {
    /// Render this element to a buffer.
    ///
    /// Rendering goes through [`render::clipped`], which guarantees the
    /// document layer's clipping contract: no element can draw outside the
    /// `area` it was given, regardless of whether the underlying widget
    /// clips itself (`TableWidget`, for one, lays out at its natural width).
    /// Container variants (`Group`, `Row`, `Tabs`, `Indented`) render their
    /// children through this method too, so the contract holds recursively.
    pub fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        render::clipped(area, buf, |scratch| {
            self.render_unclipped(area, scratch, ctx)
        });
    }

    fn render_unclipped(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        match self {
            Self::Text { content, style } => {
                render_text(content, *style, area, buf, ctx);
            }
            Self::Heading { level, content } => {
                render_heading(*level, content, area, buf, ctx);
            }
            Self::SectionTitle { content, underline } => {
                render_section_title(content, *underline, area, buf, ctx);
            }
            Self::Link {
                display, focused, ..
            } => {
                render_link(display, *focused, area, buf, ctx);
            }
            Self::Separator => {
                render_separator(area, buf, ctx);
            }
            Self::Spacer { .. } => {
                // Just empty space, nothing to render
            }
            Self::Group { children, style } => {
                render_group(children, *style, area, buf, ctx);
            }
            Self::Custom { render_fn, .. } => {
                render_fn(area, buf, ctx);
            }
            Self::Table { widget, .. } => {
                widget.render(area, buf, ctx);
            }
            Self::Row {
                children,
                gap,
                align,
            } => {
                render_row(children, *gap, *align, area, buf, ctx);
            }
            Self::ScoreBoxElement {
                score_box, focused, ..
            } => {
                // Clone and set selection based on focus state
                let mut box_to_render = score_box.clone();
                box_to_render.selected = *focused;
                box_to_render.render(area, buf, ctx);
            }
            Self::Indented { element, margin } => {
                // Render inner element with adjusted area (shifted right by margin)
                if area.width > *margin {
                    let indented_area =
                        Rect::new(area.x + margin, area.y, area.width - margin, area.height);
                    element.render(indented_area, buf, ctx);
                }
            }
            Self::TeamBoxscore {
                team_name,
                forwards_table,
                defense_table,
                goalies_table,
                ..
            } => {
                render_team_boxscore(
                    team_name,
                    forwards_table,
                    defense_table,
                    goalies_table,
                    area,
                    buf,
                    ctx,
                );
            }
            Self::BigScoreElement { big_score } => {
                big_score.render(area, buf, ctx);
            }
            Self::Tabs {
                tabs, active_index, ..
            } => {
                render::render_tabs(tabs, *active_index, area, buf, ctx);
            }
        }
    }
}

#[cfg(test)]
#[path = "behavior_tests.rs"]
mod tests;
