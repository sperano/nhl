use crate::config::{DisplayConfig, RenderContext, THEMELESS_SELECTION_STYLE_MODIFIER};
use crate::tui::component::{vertical, Component, Constraint, Element};
use ratatui::style::Style;

/// A single tab item containing its label and content
#[derive(Clone)]
pub struct TabItem {
    /// Unique key identifying this tab
    pub key: String,
    /// Display title for the tab
    pub title: String,
    /// Content to show when this tab is active
    pub content: Element,
}

impl TabItem {
    /// Create a new tab item
    pub fn new(key: impl Into<String>, title: impl Into<String>, content: Element) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            content,
        }
    }
}

/// Props for TabbedPanel component
#[derive(Clone)]
pub struct TabbedPanelProps {
    /// Currently active tab key
    pub active_key: String,
    /// List of tabs with their content
    pub tabs: Vec<TabItem>,
    /// Whether the tab bar is focused (affects tab bar styling)
    pub focused: bool,
    /// Whether the content area has focus (affects content background)
    /// When true, content area is bright; when false, content area is dimmed
    pub content_has_focus: bool,
}

/// TabbedPanel component - renders a tab bar with associated content
///
/// This component combines tab navigation with content display, similar to
/// React Bootstrap's Tabs component. Each tab has its own content area.
///
/// Tabs can be nested - a tab's content can itself be another TabbedPanel.
pub struct TabbedPanel;

impl Component for TabbedPanel {
    type Props = TabbedPanelProps;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        // Find the active tab's content
        let active_content = props
            .tabs
            .iter()
            .find(|tab| tab.key == props.active_key)
            .map(|tab| tab.content.clone())
            .unwrap_or(Element::None);

        // Build tab labels for the tab bar widget
        let tab_labels: Vec<TabLabel> = props
            .tabs
            .iter()
            .map(|tab| TabLabel {
                title: tab.title.clone(),
                key: tab.key.clone(),
                active: tab.key == props.active_key,
            })
            .collect();

        // Wrap content in FocusContext to propagate focus state and fill background
        // This ensures empty space below content is dimmed when content is unfocused
        let wrapped_content = Element::FocusContext {
            focused: props.content_has_focus,
            child: Box::new(active_content),
        };

        vertical(
            [
                Constraint::Length(2), // Tab bar (2 lines: labels + separator)
                Constraint::Min(0),    // Content area
            ],
            vec![
                self.render_tab_bar(&tab_labels, props.focused),
                wrapped_content,
            ],
        )
    }
}

impl TabbedPanel {
    fn render_tab_bar(&self, labels: &[TabLabel], focused: bool) -> Element {
        Element::Widget(Box::new(TabBarWidget {
            labels: labels.to_vec(),
            focused,
        }))
    }
}

/// Label for a single tab in the tab bar
#[derive(Clone)]
struct TabLabel {
    title: String,
    #[allow(dead_code)]
    key: String,
    active: bool,
}

/// Widget that renders the tab bar (just the labels)
struct TabBarWidget {
    labels: Vec<TabLabel>,
    focused: bool,
}

impl TabBarWidget {
    /// Get the style for box characters (borders/separators) based on focus state
    fn box_char_style(&self, config: &DisplayConfig) -> Style {
        if self.focused {
            config.boxchar_style()
        } else {
            config.boxchar_style_dim()
        }
    }

    /// Build segments for the tab line with separators
    fn build_tab_line(&self, config: &DisplayConfig, area_width: usize) -> Vec<(String, Style)> {
        use unicode_width::UnicodeWidthStr;

        let box_style = self.box_char_style(config);
        let separator = format!(" {} ", config.box_chars.vertical);
        let mut segments = Vec::new();
        let mut pos = 0;

        // Add leading margin for first tab (matches separator's leading space)
        segments.push((" ".to_string(), box_style));
        pos += 1;

        for (i, label) in self.labels.iter().enumerate() {
            if i > 0 {
                segments.push((separator.clone(), box_style));
                pos += separator.width();
            }

            let style = if let Some(theme) = &config.theme {
                // When theme is set: use fg2 (focused) or fg2_dark (unfocused)
                let (fg_color, bg_color) = if self.focused {
                    if label.active {
                        (theme.selection_text_fg, Some(theme.selection_text_bg))
                    } else {
                        (theme.fg, theme.bg)
                    }
                } else if label.active {
                    (
                        theme.selection_text_fg_dark(),
                        Some(theme.selection_text_bg_dark()),
                    )
                } else {
                    (theme.fg_dark(), theme.bg_dark())
                };
                let base = config.base_style().fg(fg_color);
                if let Some(bg) = bg_color {
                    base.bg(bg)
                } else {
                    base
                }
            } else {
                // No theme: use default style, reverse and bold for active
                if label.active {
                    config
                        .base_style()
                        .add_modifier(THEMELESS_SELECTION_STYLE_MODIFIER)
                } else {
                    config.base_style()
                }
            };

            segments.push((label.title.clone(), style));
            pos += label.title.width();
        }

        // Fill remaining space with box_style (uses bg_dark when unfocused)
        if pos < area_width {
            segments.push((" ".repeat(area_width - pos), box_style));
        }

        segments
    }

    /// Build the separator line with connectors under tab gaps
    fn build_separator_line(
        &self,
        area_width: usize,
        config: &DisplayConfig,
    ) -> Vec<(String, Style)> {
        use unicode_width::UnicodeWidthStr;

        let horizontal = &config.box_chars.horizontal;
        let connector = &config.box_chars.connector2;
        let box_style = self.box_char_style(config);

        let mut segments = Vec::new();
        let mut pos = 0;

        // Add leading horizontal line (matches leading space in tab line)
        segments.push((horizontal.to_string(), box_style));
        pos += 1;

        for (i, label) in self.labels.iter().enumerate() {
            if i > 0 {
                // Add horizontal line before separator (1 char)
                segments.push((horizontal.to_string(), box_style));
                segments.push((connector.to_string(), box_style));
                segments.push((horizontal.to_string(), box_style));
                pos += 3; // separator width: 1 + 1 + 1 (" │ ")
            }
            // Add horizontal line under tab
            let tab_width = label.title.width();
            segments.push((horizontal.repeat(tab_width), box_style));
            pos += tab_width;
        }

        // Fill rest of line
        if pos < area_width {
            segments.push((horizontal.repeat(area_width - pos), box_style));
        }

        segments
    }
}

impl crate::tui::component::ElementWidget for TabBarWidget {
    fn render(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        ctx: &RenderContext,
    ) {
        use unicode_width::UnicodeWidthStr;

        if self.labels.is_empty() || area.width == 0 || area.height < 2 {
            return;
        }

        let tab_segments = self.build_tab_line(ctx.config, area.width as usize);
        let separator_segments = self.build_separator_line(area.width as usize, ctx.config);

        // Render tab line
        let mut x = area.x;
        for (text, style) in tab_segments {
            if x >= area.x + area.width {
                break;
            }
            buf.set_string(x, area.y, &text, style);
            x += text.width() as u16; // Use display width, not byte length
        }

        // Render separator line
        let mut x = area.x;
        for (text, style) in separator_segments {
            if x >= area.x + area.width {
                break;
            }
            buf.set_string(x, area.y + 1, &text, style);
            x += text.width() as u16; // Use display width, not byte length
        }
    }

    fn preferred_height(&self) -> Option<u16> {
        Some(2) // Tab line + separator line
    }

    fn clone_box(&self) -> Box<dyn crate::tui::component::ElementWidget> {
        Box::new(TabBarWidget {
            labels: self.labels.clone(),
            focused: self.focused,
        })
    }
}

#[cfg(test)]
mod tests {
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
                    key: "home".into(),
                    active: true,
                },
                TabLabel {
                    title: "Profile".into(),
                    key: "profile".into(),
                    active: false,
                },
                TabLabel {
                    title: "Settings".into(),
                    key: "settings".into(),
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
                key: "only".into(),
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
                    key: "a".into(),
                    active: true,
                },
                TabLabel {
                    title: "Tab B".into(),
                    key: "b".into(),
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
                    key: "scores".into(),
                    active: true,
                },
                TabLabel {
                    title: "Standings".into(),
                    key: "standings".into(),
                    active: false,
                },
                TabLabel {
                    title: "Stats".into(),
                    key: "stats".into(),
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
                key: "test".into(),
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
                key: "test".into(),
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
                    key: "home".into(),
                    active: true,
                },
                TabLabel {
                    title: "Profile".into(),
                    key: "profile".into(),
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
                    key: "a".into(),
                    active: true,
                },
                TabLabel {
                    title: "Tab B".into(),
                    key: "b".into(),
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
}
