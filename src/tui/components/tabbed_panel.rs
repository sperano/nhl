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
#[path = "tabbed_panel_tests.rs"]
mod tests;
