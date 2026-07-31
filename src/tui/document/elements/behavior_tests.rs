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
            season: None,
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
