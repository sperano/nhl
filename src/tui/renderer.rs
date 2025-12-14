use ratatui::{
    buffer::Buffer,
    layout::{Constraint as RatatuiConstraint, Direction, Layout as RatatuiLayout, Rect},
};

use super::component::{Constraint, ContainerLayout, Element};
use crate::config::RenderContext;

/// Renders virtual element tree to ratatui buffer
///
/// The Renderer takes a virtual Element tree produced by components
/// and renders it to the terminal using ratatui.
///
/// Implements tree diffing to minimize redraws by comparing the previous
/// element tree with the current one and only re-rendering changed subtrees.
pub struct Renderer {
    /// Previously rendered element tree for diffing
    previous_tree: Option<Element>,
}

impl Renderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self {
            previous_tree: None,
        }
    }

    /// Render an element tree to the given area in the buffer
    ///
    /// This is the main entry point for rendering.
    /// Uses tree diffing to only re-render changed subtrees.
    pub fn render(
        &mut self,
        element: Element,
        area: Rect,
        buf: &mut Buffer,
        ctx: &RenderContext,
    ) {
        // Check if we should skip rendering based on tree diffing
        if let Some(ref previous) = self.previous_tree {
            if Self::trees_equal(previous, &element) {
                // Trees are identical, skip rendering entirely
                return;
            }
        }

        // Render the element (with diffing for subtrees)
        self.render_element_with_diff(&element, self.previous_tree.as_ref(), area, buf, ctx);

        // Cache the tree for next frame
        self.previous_tree = Some(element);
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

    /// Render an element with diffing against previous tree
    ///
    /// Only re-renders subtrees that have changed
    fn render_element_with_diff(
        &self,
        element: &Element,
        previous: Option<&Element>,
        area: Rect,
        buf: &mut Buffer,
        ctx: &RenderContext,
    ) {
        // If trees are equal, skip rendering
        if let Some(prev) = previous {
            if Self::trees_equal(prev, element) {
                return;
            }
        }

        match element {
            Element::Widget(widget) => {
                // Widgets always render (they're leaf nodes)
                widget.render(area, buf, ctx);
            }

            Element::Container { children, layout } => {
                // Calculate layout and render children with diffing
                let chunks = self.calculate_layout(layout, area);

                // Get previous children if available
                let previous_children = if let Some(Element::Container {
                    children: prev_children,
                    ..
                }) = previous
                {
                    Some(prev_children)
                } else {
                    None
                };

                // Render each child with diffing
                for (i, (child, chunk)) in children.iter().zip(chunks.iter()).enumerate() {
                    let prev_child = previous_children.and_then(|pc| pc.get(i));
                    self.render_element_with_diff(child, prev_child, *chunk, buf, ctx);
                }
            }

            Element::Fragment(children) => {
                // Get previous fragment children if available
                let previous_children = if let Some(Element::Fragment(prev_children)) = previous {
                    Some(prev_children)
                } else {
                    None
                };

                // Render each child with diffing
                for (i, child) in children.iter().enumerate() {
                    let prev_child = previous_children.and_then(|pc| pc.get(i));
                    self.render_element_with_diff(child, prev_child, area, buf, ctx);
                }
            }

            Element::Overlay { base, overlay } => {
                // Get previous overlay parts if available
                let (prev_base, prev_overlay) = if let Some(Element::Overlay {
                    base: pb,
                    overlay: po,
                }) = previous
                {
                    (Some(pb.as_ref()), Some(po.as_ref()))
                } else {
                    (None, None)
                };

                // Render base and overlay with diffing
                self.render_element_with_diff(base, prev_base, area, buf, ctx);
                self.render_element_with_diff(overlay, prev_overlay, area, buf, ctx);
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

                // Get previous child if available
                let prev_child = if let Some(Element::FocusContext { child: pc, .. }) = previous {
                    Some(pc.as_ref())
                } else {
                    None
                };

                self.render_element_with_diff(child, prev_child, area, buf, &child_ctx);
            }

            Element::Component(_) => {
                // Components should already be resolved to concrete elements
                panic!("Unresolved component in render tree - components should be resolved to elements before rendering");
            }

            Element::None => {
                // Render nothing
            }
        }
    }

    /// Check if two element trees are structurally equal
    ///
    /// This is a shallow comparison that checks tree structure but not widget contents.
    /// For widgets, we assume they're different (conservative approach).
    fn trees_equal(a: &Element, b: &Element) -> bool {
        match (a, b) {
            (Element::None, Element::None) => true,

            (Element::Widget(_), Element::Widget(_)) => {
                // Conservative: assume widgets are always different
                // In the future, we could add widget comparison logic
                false
            }

            (
                Element::Container {
                    children: children_a,
                    layout: layout_a,
                },
                Element::Container {
                    children: children_b,
                    layout: layout_b,
                },
            ) => {
                // Check layout compatibility
                if !Self::layouts_equal(layout_a, layout_b) {
                    return false;
                }

                // Check children count
                if children_a.len() != children_b.len() {
                    return false;
                }

                // Recursively check children
                children_a
                    .iter()
                    .zip(children_b.iter())
                    .all(|(ca, cb)| Self::trees_equal(ca, cb))
            }

            (Element::Fragment(children_a), Element::Fragment(children_b)) => {
                // Check children count
                if children_a.len() != children_b.len() {
                    return false;
                }

                // Recursively check children
                children_a
                    .iter()
                    .zip(children_b.iter())
                    .all(|(ca, cb)| Self::trees_equal(ca, cb))
            }

            (
                Element::Overlay {
                    base: base_a,
                    overlay: overlay_a,
                },
                Element::Overlay {
                    base: base_b,
                    overlay: overlay_b,
                },
            ) => Self::trees_equal(base_a, base_b) && Self::trees_equal(overlay_a, overlay_b),

            (
                Element::FocusContext {
                    focused: focused_a,
                    child: child_a,
                },
                Element::FocusContext {
                    focused: focused_b,
                    child: child_b,
                },
            ) => focused_a == focused_b && Self::trees_equal(child_a, child_b),

            (Element::Component(_), Element::Component(_)) => {
                // Components should never reach the renderer
                false
            }

            // Different element types are never equal
            _ => false,
        }
    }

    /// Check if two layouts are equal
    fn layouts_equal(a: &ContainerLayout, b: &ContainerLayout) -> bool {
        match (a, b) {
            (ContainerLayout::Vertical(ca), ContainerLayout::Vertical(cb)) => {
                Self::constraints_equal(ca, cb)
            }
            (ContainerLayout::Horizontal(ca), ContainerLayout::Horizontal(cb)) => {
                Self::constraints_equal(ca, cb)
            }
            _ => false,
        }
    }

    /// Check if two constraint lists are equal
    fn constraints_equal(a: &[Constraint], b: &[Constraint]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        a.iter()
            .zip(b.iter())
            .all(|(ca, cb)| Self::constraint_equal(*ca, *cb))
    }

    /// Check if two constraints are equal
    fn constraint_equal(a: Constraint, b: Constraint) -> bool {
        match (a, b) {
            (Constraint::Length(n1), Constraint::Length(n2)) => n1 == n2,
            (Constraint::Min(n1), Constraint::Min(n2)) => n1 == n2,
            (Constraint::Max(n1), Constraint::Max(n2)) => n1 == n2,
            (Constraint::Percentage(n1), Constraint::Percentage(n2)) => n1 == n2,
            (Constraint::Ratio(a1, b1), Constraint::Ratio(a2, b2)) => a1 == a2 && b1 == b2,
            _ => false,
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
    #[should_panic(expected = "Unresolved component")]
    fn test_unresolved_component_panics() {
        // Components should never reach the renderer
        // This test verifies that we panic if they do

        // We need a dummy component wrapper for this test
        struct DummyWrapper;

        impl super::super::component::ComponentWrapper for DummyWrapper {
            fn view_any(&self) -> Element {
                Element::None
            }

            fn clone_box(&self) -> Box<dyn super::super::component::ComponentWrapper> {
                Box::new(DummyWrapper)
            }
        }

        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let element = Element::Component(Box::new(DummyWrapper));

        // This should panic
        renderer.render(element, buffer.area, &mut buffer, &ctx);
    }

    #[test]
    fn test_tree_diffing_skips_identical_trees() {
        // Test that rendering identical trees twice only renders once
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let element = Element::None;

        // First render - should actually render
        renderer.render(element.clone(), buffer.area, &mut buffer, &ctx);

        // Verify the tree is cached
        assert!(renderer.previous_tree.is_some());

        // Second render with identical tree - should skip rendering
        // We can't directly test if rendering was skipped, but we can verify
        // that the logic runs without errors
        renderer.render(element.clone(), buffer.area, &mut buffer, &ctx);
    }

    #[test]
    fn test_tree_diffing_renders_changed_trees() {
        // Test that different trees trigger rendering
        let mut renderer = Renderer::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);

        let element1 = Element::None;
        let widget = Box::new(TestWidget {
            text: "Changed".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;
        let element2 = Element::Widget(widget);

        // First render
        renderer.render(element1, buffer.area, &mut buffer, &ctx);

        // Verify the tree is cached
        assert!(renderer.previous_tree.is_some());

        // Second render with different tree - should render
        renderer.render(element2, buffer.area, &mut buffer, &ctx);

        // Verify the buffer changed
        assert_buffer(&buffer, &["Changed", "", ""]);
    }

    #[test]
    fn test_tree_equality_none() {
        assert!(Renderer::trees_equal(&Element::None, &Element::None));
    }

    #[test]
    fn test_tree_equality_widgets_always_different() {
        let widget1 = Box::new(TestWidget {
            text: "Test1".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;
        let widget2 = Box::new(TestWidget {
            text: "Test2".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let elem1 = Element::Widget(widget1);
        let elem2 = Element::Widget(widget2);

        // Widgets are always considered different (conservative approach)
        assert!(!Renderer::trees_equal(&elem1, &elem2));
    }

    #[test]
    fn test_tree_equality_containers_same() {
        let layout = ContainerLayout::Vertical(vec![Constraint::Length(10)]);
        let children = vec![Element::None];

        let elem1 = Element::Container {
            layout: layout.clone(),
            children: children.clone(),
        };
        let elem2 = Element::Container { layout, children };

        assert!(Renderer::trees_equal(&elem1, &elem2));
    }

    #[test]
    fn test_tree_equality_containers_different_layout() {
        let layout1 = ContainerLayout::Vertical(vec![Constraint::Length(10)]);
        let layout2 = ContainerLayout::Horizontal(vec![Constraint::Length(10)]);
        let children = vec![Element::None];

        let elem1 = Element::Container {
            layout: layout1,
            children: children.clone(),
        };
        let elem2 = Element::Container {
            layout: layout2,
            children,
        };

        assert!(!Renderer::trees_equal(&elem1, &elem2));
    }

    #[test]
    fn test_tree_equality_containers_different_children_count() {
        let layout =
            ContainerLayout::Vertical(vec![Constraint::Length(10), Constraint::Length(10)]);

        let elem1 = Element::Container {
            layout: layout.clone(),
            children: vec![Element::None],
        };
        let elem2 = Element::Container {
            layout,
            children: vec![Element::None, Element::None],
        };

        assert!(!Renderer::trees_equal(&elem1, &elem2));
    }

    #[test]
    fn test_constraint_equality() {
        assert!(Renderer::constraint_equal(
            Constraint::Length(10),
            Constraint::Length(10)
        ));
        assert!(!Renderer::constraint_equal(
            Constraint::Length(10),
            Constraint::Length(20)
        ));
        assert!(!Renderer::constraint_equal(
            Constraint::Length(10),
            Constraint::Min(10)
        ));
        assert!(Renderer::constraint_equal(
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3)
        ));
        assert!(!Renderer::constraint_equal(
            Constraint::Ratio(1, 3),
            Constraint::Ratio(2, 3)
        ));
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

    #[test]
    fn test_focus_context_tree_equality_same() {
        let elem1 = Element::FocusContext {
            focused: true,
            child: Box::new(Element::None),
        };
        let elem2 = Element::FocusContext {
            focused: true,
            child: Box::new(Element::None),
        };

        assert!(Renderer::trees_equal(&elem1, &elem2));
    }

    #[test]
    fn test_focus_context_tree_equality_different_focus() {
        let elem1 = Element::FocusContext {
            focused: true,
            child: Box::new(Element::None),
        };
        let elem2 = Element::FocusContext {
            focused: false,
            child: Box::new(Element::None),
        };

        assert!(!Renderer::trees_equal(&elem1, &elem2));
    }

    #[test]
    fn test_focus_context_tree_equality_different_child() {
        let widget = Box::new(TestWidget {
            text: "Test".to_string(),
        }) as Box<dyn super::super::component::ElementWidget>;

        let elem1 = Element::FocusContext {
            focused: true,
            child: Box::new(Element::None),
        };
        let elem2 = Element::FocusContext {
            focused: true,
            child: Box::new(Element::Widget(widget)),
        };

        assert!(!Renderer::trees_equal(&elem1, &elem2));
    }
}
