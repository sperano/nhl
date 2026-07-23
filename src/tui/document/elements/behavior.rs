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
    /// Render this element to a buffer
    pub fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
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
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::document::link::LinkTarget;
    use crate::tui::types::StackedDocument;
    use ratatui::style::{Color, Style};

    // ---- height() ----

    #[test]
    fn test_text_element_height() {
        let elem = DocumentElement::text("Hello");
        assert_eq!(elem.height(), 1);

        let elem = DocumentElement::text("Line 1\nLine 2\nLine 3");
        assert_eq!(elem.height(), 3);

        let elem = DocumentElement::text("");
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_heading_element_height() {
        let elem = DocumentElement::heading(1, "Title");
        assert_eq!(elem.height(), 2); // Heading + underline

        let elem = DocumentElement::heading(2, "Subtitle");
        assert_eq!(elem.height(), 1);

        let elem = DocumentElement::heading(3, "Section");
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_link_element_height() {
        let elem =
            DocumentElement::link("link1", "Click me", LinkTarget::Anchor("test".to_string()));
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_separator_element_height() {
        let elem = DocumentElement::separator();
        assert_eq!(elem.height(), 1);
    }

    #[test]
    fn test_spacer_element_height() {
        let elem = DocumentElement::spacer(5);
        assert_eq!(elem.height(), 5);
    }

    #[test]
    fn test_group_element_height() {
        let elem = DocumentElement::group(vec![
            DocumentElement::text("Line 1"),
            DocumentElement::spacer(2),
            DocumentElement::text("Line 2"),
        ]);
        assert_eq!(elem.height(), 4); // 1 + 2 + 1
    }

    // ---- collect_focusable() ----

    #[test]
    fn test_collect_focusable_link() {
        let elem = DocumentElement::link(
            "my_link",
            "Click here",
            LinkTarget::Push(StackedDocument::TeamDetail {
                abbrev: "BOS".to_string(),
            }),
        );

        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 10);

        assert_eq!(focusable.len(), 1);
        assert_eq!(focusable[0].id, FocusableId::link("my_link"));
        assert_eq!(focusable[0].y, 10);
        assert_eq!(focusable[0].rect.width, 10); // "Click here" = 10 display columns
    }

    #[test]
    fn test_collect_focusable_link_rect_uses_display_width() {
        // "日本" is 2 chars but 4 display columns; a char-count rect would be
        // half as wide as the rendered label.
        let elem = DocumentElement::link("wide", "日本", LinkTarget::Anchor("a".to_string()));
        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 0);
        assert_eq!(focusable[0].rect.width, 4);

        // "Génie" is 6 bytes but 5 display columns.
        let elem = DocumentElement::link("acc", "Génie", LinkTarget::Anchor("b".to_string()));
        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 0);
        assert_eq!(focusable[0].rect.width, 5);
    }

    #[test]
    fn test_collect_focusable_group() {
        let elem = DocumentElement::group(vec![
            DocumentElement::text("Not focusable"),
            DocumentElement::link("link1", "First", LinkTarget::Anchor("a".to_string())),
            DocumentElement::spacer(2),
            DocumentElement::link("link2", "Second", LinkTarget::Anchor("b".to_string())),
        ]);

        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 0);

        assert_eq!(focusable.len(), 2);
        assert_eq!(focusable[0].id, FocusableId::link("link1"));
        assert_eq!(focusable[0].y, 1); // After "Not focusable"
        assert_eq!(focusable[1].id, FocusableId::link("link2"));
        assert_eq!(focusable[1].y, 4); // 1 (text) + 1 (link) + 2 (spacer)
    }

    #[test]
    fn test_collect_focusable_nested_groups() {
        let elem = DocumentElement::group(vec![
            DocumentElement::group(vec![DocumentElement::link(
                "inner1",
                "Inner",
                LinkTarget::Anchor("x".to_string()),
            )]),
            DocumentElement::link("outer1", "Outer", LinkTarget::Anchor("y".to_string())),
        ]);

        let mut focusable = Vec::new();
        elem.collect_focusable(&mut focusable, 5);

        assert_eq!(focusable.len(), 2);
        assert_eq!(focusable[0].id, FocusableId::link("inner1"));
        assert_eq!(focusable[0].y, 5);
        assert_eq!(focusable[1].id, FocusableId::link("outer1"));
        assert_eq!(focusable[1].y, 6);
    }

    // ---- render() ----

    #[test]
    fn test_render_text() {
        let elem = DocumentElement::text("Hello");
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &ctx);

        // Check that "Hello" was rendered
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "H");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "e");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "l");
        assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "l");
        assert_eq!(buf.cell((4, 0)).unwrap().symbol(), "o");
    }

    #[test]
    fn test_render_heading_level_1() {
        let elem = DocumentElement::heading(1, "Title");
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &ctx);

        // Check heading text
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "T");
        // Check underline
        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "═");
    }

    #[test]
    fn test_render_link() {
        let elem =
            DocumentElement::link("test_link", "Click", LinkTarget::Anchor("test".to_string()));
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &ctx);

        // Unfocused links have "  " prefix for alignment
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "C");
        assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "l");
        // Style uses fg2 from theme (or default if no theme)
        // No specific color assertion since we use theme.fg2
    }

    #[test]
    fn test_render_focused_link() {
        let elem = DocumentElement::focused_link(
            "test_link",
            "Click",
            LinkTarget::Anchor("test".to_string()),
        );
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 20, 5), &mut buf, &ctx);

        // Focused links have "▶ " prefix
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "▶");
        // Check that "Click" starts at position 2
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "C");
        // Focused links use BOLD + REVERSED modifiers
        let style = buf.cell((2, 0)).unwrap().style();
        assert!(style.add_modifier.contains(ratatui::style::Modifier::BOLD));
        assert!(style
            .add_modifier
            .contains(ratatui::style::Modifier::REVERSED));
    }

    #[test]
    fn test_render_separator() {
        let elem = DocumentElement::separator();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 10, 1), &mut buf, &ctx);

        // All cells should be horizontal line
        for x in 0..10 {
            let symbol = buf.cell((x, 0)).unwrap().symbol();
            assert!(symbol == "─" || symbol == "-"); // Unicode or ASCII
        }
    }

    #[test]
    fn test_render_spacer() {
        let elem = DocumentElement::spacer(3);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        // Fill buffer with 'X' first
        for y in 0..5 {
            for x in 0..10 {
                buf.cell_mut((x, y)).unwrap().set_char('X');
            }
        }
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 10, 3), &mut buf, &ctx);

        // Spacer doesn't change buffer - cells should still be 'X'
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "X");
    }

    #[test]
    fn test_styled_text() {
        let style = Style::default().fg(Color::Red);
        let elem = DocumentElement::styled_text("Red text", style);

        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        elem.render(Rect::new(0, 0, 20, 1), &mut buf, &ctx);

        assert_eq!(buf.cell((0, 0)).unwrap().style().fg, Some(Color::Red));
    }
}
