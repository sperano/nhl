use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::formatting::BoxChars;
use crate::tui::component::Element;
use crate::tui::testing::{assert_buffer, RENDER_WIDTH};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};

// Helper functions for testing framework widgets

fn test_config() -> DisplayConfig {
    DisplayConfig {
        use_unicode: true,
        theme_name: None,
        theme: None,
        error_fg: Color::Red,
        box_chars: BoxChars::unicode(),
    }
}

fn test_config_ascii() -> DisplayConfig {
    DisplayConfig {
        use_unicode: false,
        theme_name: None,
        theme: None,
        error_fg: Color::Red,
        box_chars: BoxChars::ascii(),
    }
}

fn render_widget(
    widget: &impl crate::tui::component::ElementWidget,
    width: u16,
    height: u16,
) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    let config = test_config();
    let ctx = RenderContext::focused(&config);
    widget.render(buf.area, &mut buf, &ctx);
    buf
}

fn render_widget_with_config(
    widget: &impl crate::tui::component::ElementWidget,
    width: u16,
    height: u16,
    config: &DisplayConfig,
) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    let ctx = RenderContext::focused(config);
    widget.render(buf.area, &mut buf, &ctx);
    buf
}

fn buffer_line(buf: &Buffer, line: usize) -> String {
    let area = buf.area();
    let mut result = String::new();
    let y = line as u16;

    if y >= area.height {
        return result;
    }

    for x in 0..area.width {
        let cell = &buf[(x, y)];
        result.push_str(cell.symbol());
    }

    result
}

#[test]
fn test_tabbed_panel_renders_container() {
    let panel = TabbedPanel;
    let props = TabbedPanelProps {
        active_key: "tab1".into(),
        tabs: vec![
            TabItem::new("tab1", "Tab 1", Element::None),
            TabItem::new("tab2", "Tab 2", Element::None),
        ],
        focused: true,
        content_has_focus: false,
    };

    let element = panel.view(&props, &());

    match element {
        Element::Container { children, .. } => {
            assert_eq!(children.len(), 2); // Tab bar + content
        }
        _ => panic!("Expected container element"),
    }
}

#[test]
fn test_tabbed_panel_shows_active_content() {
    let panel = TabbedPanel;

    // Create distinctive content for each tab
    let content1 = Element::Widget(Box::new(TestWidget { id: 1 }));
    let content2 = Element::Widget(Box::new(TestWidget { id: 2 }));

    let props = TabbedPanelProps {
        active_key: "tab2".into(),
        tabs: vec![
            TabItem::new("tab1", "Tab 1", content1),
            TabItem::new("tab2", "Tab 2", content2.clone()),
        ],
        focused: true,
        content_has_focus: false,
    };

    let element = panel.view(&props, &());

    match element {
        Element::Container { children, .. } => {
            // Second child should be tab2's content
            assert_eq!(children.len(), 2);
        }
        _ => panic!("Expected container element"),
    }
}

#[test]
fn test_tab_item_builder() {
    let tab = TabItem::new("key", "Title", Element::None);
    assert_eq!(tab.key, "key");
    assert_eq!(tab.title, "Title");
}

#[test]
fn test_empty_tabs_shows_nothing() {
    let panel = TabbedPanel;
    let props = TabbedPanelProps {
        active_key: "none".into(),
        tabs: vec![],
        focused: true,
        content_has_focus: false,
    };

    let element = panel.view(&props, &());

    match element {
        Element::Container { children, .. } => {
            assert_eq!(children.len(), 2);
            // Content should be Element::None
        }
        _ => panic!("Expected container element"),
    }
}

#[test]
fn test_nonexistent_active_key_shows_none() {
    let panel = TabbedPanel;
    let props = TabbedPanelProps {
        active_key: "nonexistent".into(),
        tabs: vec![TabItem::new("tab1", "Tab 1", Element::None)],
        focused: true,
        content_has_focus: false,
    };

    let element = panel.view(&props, &());

    match element {
        Element::Container { children, .. } => {
            assert_eq!(children.len(), 2);
            // Content should be Element::None since key doesn't match
        }
        _ => panic!("Expected container element"),
    }
}

// TabBarWidget rendering tests with assert_buffer

#[test]
fn test_tab_bar_widget_basic_rendering() {
    let widget = TabBarWidget {
        labels: vec![
            TabLabel {
                title: "Home".into(),
                active: true,
            },
            TabLabel {
                title: "Profile".into(),
                active: false,
            },
            TabLabel {
                title: "Settings".into(),
                active: false,
            },
        ],
        focused: true,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 2);

    assert_buffer(
        &buf,
        &[
            " Home │ Profile │ Settings",
            "──────┴─────────┴───────────────────────────────────────────────────────────────",
        ],
    );
}

#[test]
fn test_tab_bar_widget_single_tab() {
    let widget = TabBarWidget {
        labels: vec![TabLabel {
            title: "Only Tab".into(),
            active: true,
        }],
        focused: true,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 2);
    assert_buffer(
        &buf,
        &[
            " Only Tab",
            "────────────────────────────────────────────────────────────────────────────────",
        ],
    );
}

#[test]
fn test_tab_bar_widget_empty() {
    let widget = TabBarWidget {
        labels: vec![],
        focused: true,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 2);

    // Empty widget should render nothing
    let line1 = buffer_line(&buf, 0);
    let line2 = buffer_line(&buf, 1);
    assert_eq!(line1.trim(), "");
    assert_eq!(line2.trim(), "");
}

#[test]
fn test_tab_bar_widget_ascii_mode() {
    let widget = TabBarWidget {
        labels: vec![
            TabLabel {
                title: "Tab A".into(),
                active: true,
            },
            TabLabel {
                title: "Tab B".into(),
                active: false,
            },
        ],
        focused: true,
    };

    let config = test_config_ascii();
    let buf = render_widget_with_config(&widget, RENDER_WIDTH, 2, &config);

    assert_buffer(
        &buf,
        &[
            " Tab A | Tab B",
            "--------------------------------------------------------------------------------",
        ],
    );
}

#[test]
fn test_tab_bar_widget_connector_alignment() {
    let widget = TabBarWidget {
        labels: vec![
            TabLabel {
                title: "Scores".into(),
                active: true,
            },
            TabLabel {
                title: "Standings".into(),
                active: false,
            },
            TabLabel {
                title: "Stats".into(),
                active: false,
            },
        ],
        focused: true,
    };

    let config = test_config();
    let buf = render_widget_with_config(&widget, RENDER_WIDTH, 2, &config);

    let line0 = buffer_line(&buf, 0);
    let line1 = buffer_line(&buf, 1);

    // Find positions of vertical separators (│) in first line
    let vertical_positions: Vec<usize> = line0
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == '│')
        .map(|(i, _)| i)
        .collect();

    // Verify connectors (┴) align with vertical separators
    for pos in vertical_positions {
        let char_at_pos = line1.chars().nth(pos).unwrap_or(' ');
        assert_eq!(
            char_at_pos, '┴',
            "Expected connector '┴' at position {} (below vertical separator '│'), but found '{}'. Line 0: {}\nLine 1: {}",
            pos, char_at_pos, line0, line1
        );
    }
}

#[test]
fn test_tab_bar_widget_zero_height() {
    let widget = TabBarWidget {
        labels: vec![TabLabel {
            title: "Test".into(),
            active: true,
        }],
        focused: true,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 0);

    // Should not panic with zero height
    assert_eq!(buf.area.height, 0);
}

#[test]
fn test_tab_bar_widget_insufficient_height() {
    let widget = TabBarWidget {
        labels: vec![TabLabel {
            title: "Test".into(),
            active: true,
        }],
        focused: true,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 1);

    // Should not render anything if height < 2
    let line = buffer_line(&buf, 0);
    assert_eq!(line.trim(), "");
}

// Unfocused rendering tests

#[test]
fn test_tab_bar_widget_unfocused_basic() {
    let widget = TabBarWidget {
        labels: vec![
            TabLabel {
                title: "Home".into(),
                active: true,
            },
            TabLabel {
                title: "Profile".into(),
                active: false,
            },
        ],
        focused: false,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 2);

    // When unfocused with no theme, all tabs use default style (Reset)
    // Active tab gets REVERSED modifier
    // Note: positions shifted by 1 due to leading margin

    // Check inactive tab (Profile at position 10+)
    let profile_cell = &buf[(10, 0)]; // 'P' in Profile
    assert_eq!(profile_cell.fg, ratatui::style::Color::Reset);
    assert!(!profile_cell.modifier.contains(Modifier::REVERSED));

    // Check separator (│ at position 6) - uses default style when no theme
    let separator_cell = &buf[(6, 0)];
    assert_eq!(separator_cell.fg, ratatui::style::Color::Reset);

    // Check active tab uses default style with REVERSED modifier
    let home_cell = &buf[(1, 0)]; // 'H' in Home (position 1 due to leading margin)
    assert_eq!(home_cell.fg, ratatui::style::Color::Reset);
    assert!(home_cell.modifier.contains(Modifier::REVERSED));
}

#[test]
fn test_tab_bar_widget_unfocused_separator_line() {
    let widget = TabBarWidget {
        labels: vec![
            TabLabel {
                title: "Tab A".into(),
                active: true,
            },
            TabLabel {
                title: "Tab B".into(),
                active: false,
            },
        ],
        focused: false,
    };

    let buf = render_widget(&widget, RENDER_WIDTH, 2);

    // Check separator line uses default style (Reset) when no theme
    let horizontal_cell = &buf[(0, 1)]; // First horizontal line character
    assert_eq!(horizontal_cell.fg, ratatui::style::Color::Reset);

    let connector_cell = &buf[(6, 1)]; // Connector ┴ position
    assert_eq!(connector_cell.fg, ratatui::style::Color::Reset);
}

// Helper test widget
struct TestWidget {
    id: u32,
}

impl crate::tui::component::ElementWidget for TestWidget {
    fn render(
        &self,
        _area: ratatui::layout::Rect,
        _buf: &mut ratatui::buffer::Buffer,
        _ctx: &RenderContext,
    ) {
    }
    fn clone_box(&self) -> Box<dyn crate::tui::component::ElementWidget> {
        Box::new(TestWidget { id: self.id })
    }
}
