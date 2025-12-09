/// Breadcrumb component for showing navigation path
///
/// Displays a breadcrumb trail showing the user's current location in the document stack.
/// Example: "Standings > Team: TOR > Player: Sidney Crosby"
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::config::DisplayConfig;
use crate::tui::{component::ElementWidget, state::DocumentStackEntry, StackedDocument, Tab};

/// Breadcrumb widget that renders a navigation path
#[derive(Clone)]
pub struct BreadcrumbWidget {
    pub current_tab: Tab,
    pub document_stack: Vec<DocumentStackEntry>,
}

impl BreadcrumbWidget {
    pub fn new(current_tab: Tab, document_stack: Vec<DocumentStackEntry>) -> Self {
        Self {
            current_tab,
            document_stack,
        }
    }

    /// Build breadcrumb text from tab and document stack
    fn build_breadcrumb_text(&self, config: &DisplayConfig) -> Vec<Span<'_>> {
        let mut spans = Vec::new();

        // Get styles from theme
        let (text_style, separator_style) = if let Some(theme) = &config.theme {
            (
                Style::default().fg(theme.fg2).add_modifier(Modifier::BOLD),
                Style::default().fg(theme.fg3),
            )
        } else {
            (
                Style::default().add_modifier(Modifier::BOLD),
                Style::default(),
            )
        };

        let separator = format!(" {} ", config.box_chars.breadcrumb_separator);

        // Start with the current tab name
        let tab_name = match self.current_tab {
            Tab::Scores => "Scores",
            Tab::Standings => "Standings",
            Tab::Settings => "Settings",
            #[cfg(feature = "development")]
            Tab::Demo => "Demo",
        };

        spans.push(Span::styled(tab_name.to_string(), text_style));

        // Add each document in the stack
        for doc_entry in &self.document_stack {
            spans.push(Span::styled(separator.clone(), separator_style));

            let doc_text = match &doc_entry.document {
                StackedDocument::Boxscore { game_id } => format!("Boxscore: Game {}", game_id),
                StackedDocument::TeamDetail { abbrev } => format!("Team: {}", abbrev),
                StackedDocument::PlayerDetail { player_id } => format!("Player: {}", player_id),
            };

            spans.push(Span::styled(doc_text, text_style));
        }

        spans
    }
}

/// Box drawing character for horizontal divider
const HORIZONTAL_LINE: char = '─';

impl ElementWidget for BreadcrumbWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let spans = self.build_breadcrumb_text(config);
        let line = Line::from(spans);

        // Render the breadcrumb line
        buf.set_line(area.x, area.y, &line, area.width);

        // Render the divider line on the second row
        if area.height >= 2 {
            let divider_style = if let Some(theme) = &config.theme {
                Style::default().fg(theme.fg3)
            } else {
                Style::default()
            };
            let divider: String =
                std::iter::repeat_n(HORIZONTAL_LINE, area.width as usize).collect();
            let divider_line = Line::from(Span::styled(divider, divider_style));
            buf.set_line(area.x, area.y + 1, &divider_line, area.width);
        }
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }

    fn preferred_height(&self) -> Option<u16> {
        if self.document_stack.is_empty() {
            Some(0) // No breadcrumb if no documents are open
        } else {
            Some(2) // 2 lines: breadcrumb text + divider
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::assert_buffer;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_breadcrumb_no_documents() {
        let widget = BreadcrumbWidget::new(Tab::Scores, Vec::new());
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        // With no documents, should just show the tab name and divider
        assert_buffer(
            &buf,
            &[
                "Scores",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    fn test_breadcrumb_with_team_detail() {
        let document_stack = vec![DocumentStackEntry::with_selection(
            StackedDocument::TeamDetail {
                abbrev: "TOR".to_string(),
            },
            None,
        )];

        let widget = BreadcrumbWidget::new(Tab::Standings, document_stack);
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Standings ▶ Team: TOR",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    fn test_breadcrumb_with_boxscore() {
        let document_stack = vec![DocumentStackEntry::with_selection(
            StackedDocument::Boxscore {
                game_id: 2024020001,
            },
            None,
        )];

        let widget = BreadcrumbWidget::new(Tab::Scores, document_stack);
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Scores ▶ Boxscore: Game 2024020001",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    fn test_breadcrumb_with_nested_documents() {
        let document_stack = vec![
            DocumentStackEntry::with_selection(
                StackedDocument::Boxscore {
                    game_id: 2024020001,
                },
                None,
            ),
            DocumentStackEntry::with_selection(
                StackedDocument::PlayerDetail { player_id: 8471675 },
                None,
            ),
        ];

        let widget = BreadcrumbWidget::new(Tab::Scores, document_stack);
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Scores ▶ Boxscore: Game 2024020001 ▶ Player: 8471675",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    fn test_breadcrumb_standings_tab() {
        let widget = BreadcrumbWidget::new(Tab::Standings, Vec::new());
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Standings",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    fn test_breadcrumb_settings_tab() {
        let widget = BreadcrumbWidget::new(Tab::Settings, Vec::new());
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Settings",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    #[cfg(feature = "development")]
    fn test_breadcrumb_browser_tab() {
        let widget = BreadcrumbWidget::new(Tab::Demo, Vec::new());
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Demo",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }

    #[test]
    fn test_breadcrumb_zero_height_area() {
        let widget = BreadcrumbWidget::new(Tab::Scores, Vec::new());
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 0));
        widget.render(buf.area, &mut buf, &config);

        // Should render nothing for zero-height area
    }

    #[test]
    fn test_breadcrumb_zero_width_area() {
        let widget = BreadcrumbWidget::new(Tab::Scores, Vec::new());
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 0, 1));
        widget.render(buf.area, &mut buf, &config);

        // Should render nothing for zero-width area
    }

    #[test]
    fn test_breadcrumb_clone_box() {
        let widget = BreadcrumbWidget::new(Tab::Scores, Vec::new());
        let _cloned: Box<dyn ElementWidget> = widget.clone_box();
        // If we get here, clone_box() worked
    }

    #[test]
    fn test_breadcrumb_preferred_height_with_empty_stack() {
        let widget = BreadcrumbWidget::new(Tab::Scores, Vec::new());
        assert_eq!(widget.preferred_height(), Some(0));
    }

    #[test]
    fn test_breadcrumb_preferred_height_with_documents() {
        let document_stack = vec![DocumentStackEntry::with_selection(
            StackedDocument::TeamDetail {
                abbrev: "TOR".to_string(),
            },
            None,
        )];

        let widget = BreadcrumbWidget::new(Tab::Standings, document_stack);
        assert_eq!(widget.preferred_height(), Some(2));
    }

    #[test]
    fn test_breadcrumb_with_player_detail() {
        let document_stack = vec![DocumentStackEntry::with_selection(
            StackedDocument::PlayerDetail { player_id: 8478402 },
            None,
        )];

        let widget = BreadcrumbWidget::new(Tab::Scores, document_stack);
        let config = DisplayConfig::default();

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 2));
        widget.render(buf.area, &mut buf, &config);

        assert_buffer(
            &buf,
            &[
                "Scores ▶ Player: 8478402",
                "────────────────────────────────────────────────────────────────────────────────",
            ],
        );
    }
}
