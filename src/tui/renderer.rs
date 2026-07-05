use ratatui::{
    buffer::Buffer,
    layout::{Constraint as RatatuiConstraint, Direction, Layout as RatatuiLayout, Rect},
};

use super::component::{Constraint, ContainerLayout, Element};
use crate::config::RenderContext;

/// Renders virtual element tree to ratatui buffer
///
/// The Renderer takes a virtual Element tree produced by components
/// and renders it to the terminal using ratatui. The main loop only calls
/// `render` when it has a dirty flag set (see `tui::mod::run`), so redraw
/// frequency is already bounded upstream; this renderer always draws the
/// tree it's given.
pub struct Renderer {}

impl Renderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self {}
    }

    /// Render an element tree to the given area in the buffer
    ///
    /// This is the main entry point for rendering.
    pub fn render(&mut self, element: Element, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        self.render_element(&element, area, buf, ctx);
    }

    /// Calculate layout constraints and split the area
    fn calculate_layout(&self, layout: &ContainerLayout, area: Rect) -> Vec<Rect> {
        match layout {
            ContainerLayout::Vertical(constraints) => {
                let ratatui_constraints = constraints
                    .iter()
                    .map(|c| self.convert_constraint(*c))
                    .collect::<Vec<_>>();

                RatatuiLayout::default()
                    .direction(Direction::Vertical)
                    .constraints(ratatui_constraints)
                    .split(area)
                    .to_vec()
            }

            ContainerLayout::Horizontal(constraints) => {
                let ratatui_constraints = constraints
                    .iter()
                    .map(|c| self.convert_constraint(*c))
                    .collect::<Vec<_>>();

                RatatuiLayout::default()
                    .direction(Direction::Horizontal)
                    .constraints(ratatui_constraints)
                    .split(area)
                    .to_vec()
            }
        }
    }

    /// Convert our Constraint type to ratatui's Constraint
    fn convert_constraint(&self, constraint: Constraint) -> RatatuiConstraint {
        match constraint {
            Constraint::Length(n) => RatatuiConstraint::Length(n),
            Constraint::Min(n) => RatatuiConstraint::Min(n),
            Constraint::Max(n) => RatatuiConstraint::Max(n),
            Constraint::Percentage(n) => RatatuiConstraint::Percentage(n),
            Constraint::Ratio(a, b) => RatatuiConstraint::Ratio(a, b),
        }
    }

    /// Render an element tree
    fn render_element(&self, element: &Element, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        match element {
            Element::Widget(widget) => {
                widget.render(area, buf, ctx);
            }

            Element::Container { children, layout } => {
                let chunks = self.calculate_layout(layout, area);

                for (child, chunk) in children.iter().zip(chunks.iter()) {
                    self.render_element(child, *chunk, buf, ctx);
                }
            }

            Element::Fragment(children) => {
                for child in children.iter() {
                    self.render_element(child, area, buf, ctx);
                }
            }

            Element::Overlay { base, overlay } => {
                self.render_element(base, area, buf, ctx);
                self.render_element(overlay, area, buf, ctx);
            }

            Element::FocusContext { focused, child } => {
                // Create child context with specified focus state
                let child_ctx = RenderContext::new(ctx.config, *focused);

                // Fill the entire area with the appropriate background color
                // This ensures empty space is also dimmed when unfocused
                let bg_style = child_ctx.base_style();
                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        buf[(x, y)].set_style(bg_style);
                    }
                }

                self.render_element(child, area, buf, &child_ctx);
            }

            Element::None => {
                // Render nothing
            }
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;
    use crate::tui::testing::assert_buffer;
    use ratatui::{
        buffer::Buffer,
        text::Text,
        widgets::{Paragraph, Widget},
    };

    /// Test widget that renders text
    #[derive(Clone)]
    struct TestWidget {
        text: String,
    }

    impl super::super::component::ElementWidget for TestWidget {
        fn render(&self, area: Rect, buf: &mut Buffer, _ctx: &RenderContext) {
            let text = Text::from(self.text.clone());
            Paragraph::new(text).render(area, buf);
        }

        fn clone_box(&self) -> Box<dyn super::super::component::ElementWidget> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_render_none() {
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        renderer.render(Element::None, buffer.area, &mut buffer, &ctx);

        // Buffer should remain empty (all spaces)
        assert_buffer(&buffer, &["", "", ""]);
    }

    #[test]
    fn test_render_widget() {
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let widget = Box::new(TestWidget {
            text: "Hello".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;
        let element = Element::Widget(widget);

        renderer.render(element, buffer.area, &mut buffer, &ctx);

        assert_buffer(&buffer, &["Hello", "", ""]);
    }

    #[test]
    fn test_render_container_vertical() {
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let top_widget = Box::new(TestWidget {
            text: "TOP".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let bottom_widget = Box::new(TestWidget {
            text: "BOTTOM".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let element = Element::Container {
            layout: ContainerLayout::Vertical(vec![Constraint::Length(3), Constraint::Length(3)]),
            children: vec![Element::Widget(top_widget), Element::Widget(bottom_widget)],
        };

        renderer.render(element, buffer.area, &mut buffer, &ctx);

        assert_buffer(&buffer, &["TOP", "", "", "BOTTOM", "", ""]);
    }

    #[test]
    fn test_render_container_horizontal() {
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let left_widget = Box::new(TestWidget {
            text: "LEFT".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let right_widget = Box::new(TestWidget {
            text: "RIGHT".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let element = Element::Container {
            layout: ContainerLayout::Horizontal(vec![
                Constraint::Length(10),
                Constraint::Length(10),
            ]),
            children: vec![Element::Widget(left_widget), Element::Widget(right_widget)],
        };

        renderer.render(element, buffer.area, &mut buffer, &ctx);

        assert_buffer(&buffer, &["LEFT      RIGHT", "", ""]);
    }

    #[test]
    fn test_render_fragment() {
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        // Fragment renders multiple children in same area
        // The second child should overwrite the first
        let widget1 = Box::new(TestWidget {
            text: "First".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let widget2 = Box::new(TestWidget {
            text: "Second".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let element = Element::Fragment(vec![Element::Widget(widget1), Element::Widget(widget2)]);

        renderer.render(element, buffer.area, &mut buffer, &ctx);

        assert_buffer(&buffer, &["Second", "", ""]);
    }

    #[test]
    fn test_constraint_conversion() {
        let renderer = Renderer::new();

        assert_eq!(
            renderer.convert_constraint(Constraint::Length(10)),
            RatatuiConstraint::Length(10)
        );
        assert_eq!(
            renderer.convert_constraint(Constraint::Min(5)),
            RatatuiConstraint::Min(5)
        );
        assert_eq!(
            renderer.convert_constraint(Constraint::Max(20)),
            RatatuiConstraint::Max(20)
        );
        assert_eq!(
            renderer.convert_constraint(Constraint::Percentage(50)),
            RatatuiConstraint::Percentage(50)
        );
        assert_eq!(
            renderer.convert_constraint(Constraint::Ratio(1, 3)),
            RatatuiConstraint::Ratio(1, 3)
        );
    }

    #[test]
    fn test_render_called_twice_is_idempotent() {
        // Rendering the same element twice should produce the same output both times
        // (the renderer holds no cross-call state to get out of sync).
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let widget = Box::new(TestWidget {
            text: "Hello".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;
        let element = Element::Widget(widget);

        renderer.render(element.clone(), buffer.area, &mut buffer, &ctx);
        assert_buffer(&buffer, &["Hello", "", ""]);

        renderer.render(element, buffer.area, &mut buffer, &ctx);
        assert_buffer(&buffer, &["Hello", "", ""]);
    }

    #[test]
    fn test_render_reflects_latest_tree() {
        // Rendering a different element after a prior render must show the new content.
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let element1 = Element::None;
        let widget = Box::new(TestWidget {
            text: "Changed".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;
        let element2 = Element::Widget(widget);

        renderer.render(element1, buffer.area, &mut buffer, &ctx);
        renderer.render(element2, buffer.area, &mut buffer, &ctx);

        assert_buffer(&buffer, &["Changed", "", ""]);
    }

    #[test]
    fn test_render_focus_context_fills_background() {
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));

        // Use a built-in theme that has a background color (habs has a red bg)
        let mut config = DisplayConfig::default();
        config.theme_name = Some("habs".to_string());
        config.theme = crate::config::THEMES.get("habs").cloned().cloned();

        // Verify our test setup has a background color
        assert!(config.theme.as_ref().unwrap().bg.is_some());

        let ctx = RenderContext::focused(&config);

        // Create a FocusContext with focused=false wrapping Element::None
        let element = Element::FocusContext {
            focused: false,
            child: Box::new(Element::None),
        };

        renderer.render(element, buffer.area, &mut buffer, &ctx);

        // The buffer should have the dimmed background color set
        // Even though Element::None doesn't render anything, the FocusContext
        // should have filled the area with the dimmed background
        let cell = &buffer[(0, 0)];
        // bg_dark() computes the dimmed version
        let expected_bg = config.theme.as_ref().unwrap().bg_dark();
        assert_eq!(cell.bg, expected_bg.unwrap());
    }
}
