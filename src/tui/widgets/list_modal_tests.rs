use super::*;
use crate::config::RenderContext;
use crate::tui::testing::{assert_buffer, RENDER_WIDTH};
use crate::tui::widgets::testing::test_config;

#[test]
fn test_list_modal_basic_render() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 24));
    let area = Rect::new(0, 0, RENDER_WIDTH, 24);

    let options = vec!["Option 1".to_string(), "Option 2".to_string()];

    let modal_area = render_list_modal(
        &options, 0, 10, // position_x
        5,  // position_y
        area, &mut buf, &ctx,
    );

    // Modal should be positioned at specified coordinates
    assert_eq!(modal_area.x, 10);
    assert_eq!(modal_area.y, 5);
}

#[test]
fn test_list_modal_no_title() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 10));
    let area = Rect::new(0, 0, 40, 10);

    let options = vec!["Option 1".to_string()];

    render_list_modal(
        &options, 0, 5, // position_x
        2, // position_y
        area, &mut buf, &ctx,
    );

    // Modal should appear at position with option immediately inside border (no title)
    assert_buffer(
        &buf,
        &[
            "",
            "",
            "     ┌────────────┐",
            "     │ ▶ Option 1 │",
            "     └────────────┘",
            "",
            "",
            "",
            "",
            "",
        ],
    );
}

#[test]
fn test_list_modal_selection_first() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
    let area = Rect::new(0, 0, 30, 8);

    let options = vec!["First".to_string(), "Second".to_string()];

    render_list_modal(
        &options, 0, // Select first
        5, // position_x
        1, // position_y
        area, &mut buf, &ctx,
    );

    // First option should have selection indicator (▸)
    assert_buffer(
        &buf,
        &[
            "",
            "     ┌──────────┐",
            "     │ ▶ First  │",
            "     │   Second │",
            "     └──────────┘",
            "",
            "",
            "",
        ],
    );
}

#[test]
fn test_list_modal_selection_second() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
    let area = Rect::new(0, 0, 30, 8);

    let options = vec!["First".to_string(), "Second".to_string()];

    render_list_modal(
        &options, 1, // Select second
        5, // position_x
        1, // position_y
        area, &mut buf, &ctx,
    );

    // Second option should have selection indicator (▸)
    assert_buffer(
        &buf,
        &[
            "",
            "     ┌──────────┐",
            "     │   First  │",
            "     │ ▶ Second │",
            "     └──────────┘",
            "",
            "",
            "",
        ],
    );
}

#[test]
fn test_list_modal_sizing() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 24));
    let area = Rect::new(0, 0, RENDER_WIDTH, 24);

    let options = vec![
        "Short".to_string(),
        "Very Long Option Name Here".to_string(),
    ];

    let modal_area = render_list_modal(
        &options, 0, 10, // position_x
        5,  // position_y
        area, &mut buf, &ctx,
    );

    // Width should accommodate the longest option
    // +6 for borders and margins
    assert!(modal_area.width >= "Very Long Option Name Here".len() as u16 + 6);

    // Height should be: 2 options + 2 (borders only, no title)
    assert_eq!(modal_area.height, 4);
}

#[test]
fn test_list_modal_empty_options() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 24));
    let area = Rect::new(0, 0, RENDER_WIDTH, 24);

    let options: Vec<String> = vec![];

    let modal_area = render_list_modal(
        &options, 0, 10, // position_x
        5,  // position_y
        area, &mut buf, &ctx,
    );

    // Should still render with minimal height (borders only, no title)
    assert_eq!(modal_area.height, 2); // borders only
}

#[test]
fn test_list_modal_positioning() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 100, 30));
    let area = Rect::new(0, 0, 100, 30);

    let options = vec!["Option".to_string()];

    let modal_area = render_list_modal(
        &options, 0, 15, // position_x
        10, // position_y
        area, &mut buf, &ctx,
    );

    // Modal should be positioned at specified coordinates
    assert_eq!(modal_area.x, 15);
    assert_eq!(modal_area.y, 10);
}

#[test]
fn test_list_modal_widget_new() {
    let options = vec!["Option 1".to_string(), "Option 2".to_string()];
    let widget = ListModalWidget::new(options.clone(), 1, 10, 5);

    assert_eq!(widget.options, options);
    assert_eq!(widget.selected_index, 1);
    assert_eq!(widget.position_x, 10);
    assert_eq!(widget.position_y, 5);
}

#[test]
fn test_list_modal_widget_render() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 24));
    let area = Rect::new(0, 0, RENDER_WIDTH, 24);

    let options = vec!["First".to_string(), "Second".to_string()];
    let widget = ListModalWidget::new(options, 0, 10, 5);

    widget.render(area, &mut buf, &ctx);

    // Widget should have rendered via ElementWidget trait
    // Check that something was rendered in the buffer
    let has_content = (0..buf.area.height).any(|y| {
        (0..buf.area.width).any(|x| {
            if let Some(cell) = buf.cell((x, y)) {
                !cell.symbol().trim().is_empty()
            } else {
                false
            }
        })
    });

    assert!(has_content, "Widget should have rendered content");
}

#[test]
fn test_list_modal_widget_clone_box() {
    let options = vec!["Option".to_string()];
    let widget = ListModalWidget::new(options, 0, 10, 5);

    let _cloned: Box<dyn ElementWidget> = widget.clone_box();
    // If we get here, clone_box() worked
}

#[test]
fn test_list_modal_truncation_when_too_tall() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 8)); // Small height
    let area = Rect::new(0, 0, 40, 8);

    // Many options that won't fit
    let options = vec![
        "Option 1".to_string(),
        "Option 2".to_string(),
        "Option 3".to_string(),
        "Option 4".to_string(),
        "Option 5".to_string(),
        "Option 6".to_string(),
        "Option 7".to_string(),
        "Option 8".to_string(),
        "Option 9".to_string(),
        "Option 10".to_string(),
    ];

    let modal_area = render_list_modal(
        &options, 0, 5, // position_x
        2, // position_y
        area, &mut buf, &ctx,
    );

    // Modal height should be limited by available area
    assert!(modal_area.height <= area.height);

    // Should render without panicking even when options are truncated
}

#[test]
fn test_list_modal_widget_positioning() {
    let options = vec!["Test".to_string()];
    let widget = ListModalWidget::new(options, 0, 15, 8);
    assert_eq!(widget.position_x, 15);
    assert_eq!(widget.position_y, 8);
}

#[test]
fn test_list_modal_with_emoji() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 8));
    let area = Rect::new(0, 0, 40, 8);

    // Emoji have display width of 2 but byte length > 1
    let options = vec!["Hockey 🏒".to_string(), "Goal 🥅".to_string()];

    let modal_area = render_list_modal(
        &options, 0, 5, // position_x
        2, // position_y
        area, &mut buf, &ctx,
    );

    // Modal should accommodate the display width properly
    // "Hockey 🏒" has display width of 9 (6 + 1 space + 2 for emoji)
    // With +6 for borders and margins, modal should be ~15 wide
    assert!(
        modal_area.width >= 15,
        "Modal width should accommodate emoji display width"
    );
}

#[test]
fn test_list_modal_with_cjk_characters() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 50, 8));
    let area = Rect::new(0, 0, 50, 8);

    // CJK characters have display width of 2 but byte length of 3
    let options = vec!["日本語".to_string(), "中文".to_string()];

    let modal_area = render_list_modal(
        &options, 0, 5, // position_x
        2, // position_y
        area, &mut buf, &ctx,
    );

    // "日本語" has display width of 6 (3 chars × 2 width each)
    // With +6 for borders and margins, modal should be ~12 wide
    assert!(
        modal_area.width >= 12,
        "Modal width should accommodate CJK character display width"
    );
}

#[test]
fn test_list_modal_with_mixed_unicode() {
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    let mut buf = Buffer::empty(Rect::new(0, 0, 50, 10));
    let area = Rect::new(0, 0, 50, 10);

    // Mix of ASCII, emoji, and CJK
    let options = vec![
        "Player 🏒".to_string(),
        "選手".to_string(),
        "Goalie 🥅".to_string(),
    ];

    let modal_area = render_list_modal(
        &options, 1, 5, // position_x
        2, // position_y
        area, &mut buf, &ctx,
    );

    // Should render without panic or overflow
    assert!(modal_area.width > 0);
    assert!(modal_area.height == 5); // 3 options + 2 borders
}
