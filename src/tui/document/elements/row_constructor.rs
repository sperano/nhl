//! `DocumentElement::row*()` constructors for horizontal layouts, with
//! their alignment/gap behavior tests.

use super::{DocumentElement, RowAlignment};

impl DocumentElement {
    /// Create a horizontal row of elements (side by side)
    ///
    /// Elements are laid out horizontally. Fixed-width children are spread
    /// across available width by default (maximizing gap).
    pub fn row(children: Vec<DocumentElement>) -> Self {
        Self::Row {
            children,
            gap: 2,
            align: RowAlignment::Spread,
        }
    }

    /// Create a horizontal row with custom gap
    pub fn row_with_gap(children: Vec<DocumentElement>, gap: u16) -> Self {
        Self::Row {
            children,
            gap,
            align: RowAlignment::Spread,
        }
    }

    /// Create a horizontal row with left alignment (no gap maximization)
    pub fn row_left(children: Vec<DocumentElement>) -> Self {
        Self::Row {
            children,
            gap: 2,
            align: RowAlignment::Left,
        }
    }

    /// Create a horizontal row with left alignment and custom gap
    pub fn row_left_with_gap(children: Vec<DocumentElement>, gap: u16) -> Self {
        Self::Row {
            children,
            gap,
            align: RowAlignment::Left,
        }
    }

    /// Create a horizontal row with center alignment
    pub fn row_center(children: Vec<DocumentElement>) -> Self {
        Self::Row {
            children,
            gap: 2,
            align: RowAlignment::Center,
        }
    }

    /// Create a horizontal row with center alignment and custom gap
    pub fn row_center_with_gap(children: Vec<DocumentElement>, gap: u16) -> Self {
        Self::Row {
            children,
            gap,
            align: RowAlignment::Center,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, RenderContext};
    use crate::tui::document::link::LinkTarget;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_row_spread_alignment_maximizes_gap() {
        use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};

        // Create two score boxes (each 25 chars wide)
        let score_box1 = ScoreBox::new(
            "Team A",
            "Team B",
            Some(3),
            Some(2),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );
        let score_box2 = ScoreBox::new(
            "Team C",
            "Team D",
            Some(1),
            Some(4),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );

        // Row with Spread alignment (default)
        let row = DocumentElement::row(vec![
            DocumentElement::score_box_element(
                1,
                score_box1.clone(),
                false,
                LinkTarget::Anchor("game_1".to_string()),
            ),
            DocumentElement::score_box_element(
                2,
                score_box2.clone(),
                false,
                LinkTarget::Anchor("game_2".to_string()),
            ),
        ]);

        // With area of 60 wide: 25 + 25 = 50, leaving 10 for gap
        // Spread should put gap of 10 between them
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        row.render(Rect::new(0, 0, 60, 6), &mut buf, &ctx);

        // Second box should start at position 35 (25 + 10 gap)
        // Check the status line of the second box
        let line0 = (0..60).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        // First box status starts at 1, second should start around 35+1=36
        assert!(
            line0[36..].trim_start().starts_with("Final"),
            "Second box should start around position 36, got: '{}'",
            line0
        );
    }

    #[test]
    fn test_row_left_alignment_uses_minimum_gap() {
        use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};

        // Create two score boxes (each 25 chars wide)
        let score_box1 = ScoreBox::new(
            "Team A",
            "Team B",
            Some(3),
            Some(2),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );
        let score_box2 = ScoreBox::new(
            "Team C",
            "Team D",
            Some(1),
            Some(4),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );

        // Row with Left alignment
        let row = DocumentElement::row_left(vec![
            DocumentElement::score_box_element(
                1,
                score_box1.clone(),
                false,
                LinkTarget::Anchor("game_1".to_string()),
            ),
            DocumentElement::score_box_element(
                2,
                score_box2.clone(),
                false,
                LinkTarget::Anchor("game_2".to_string()),
            ),
        ]);

        // With area of 60 wide, Left alignment should use minimum gap (2)
        // Second box should start at 25 + 2 = 27
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        row.render(Rect::new(0, 0, 60, 6), &mut buf, &ctx);

        // Check that second box status line starts near position 27
        let line0 = (0..60).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        // Second box status should start at 27+1=28 (with space prefix)
        assert!(
            line0[28..].trim_start().starts_with("Final"),
            "Second box should start at position 28 with Left alignment, got: '{}'",
            line0
        );
    }

    #[test]
    fn test_row_spread_respects_minimum_gap() {
        use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};

        // Create two score boxes (each 25 chars wide)
        let score_box1 = ScoreBox::new(
            "Team A",
            "Team B",
            Some(3),
            Some(2),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );
        let score_box2 = ScoreBox::new(
            "Team C",
            "Team D",
            Some(1),
            Some(4),
            ScoreBoxStatus::Final {
                overtime: false,
                shootout: false,
            },
        );

        // Row with minimum gap of 5
        let row = DocumentElement::row_with_gap(
            vec![
                DocumentElement::score_box_element(
                    1,
                    score_box1.clone(),
                    false,
                    LinkTarget::Anchor("game_1".to_string()),
                ),
                DocumentElement::score_box_element(
                    2,
                    score_box2.clone(),
                    false,
                    LinkTarget::Anchor("game_2".to_string()),
                ),
            ],
            5,
        );

        // Area of 52: 25 + 25 = 50, leaving only 2 for gap
        // But minimum gap is 5, so it should use 5
        let mut buf = Buffer::empty(Rect::new(0, 0, 52, 6));
        let config = DisplayConfig::default();
        let ctx = RenderContext::focused(&config);
        row.render(Rect::new(0, 0, 52, 6), &mut buf, &ctx);

        // Second box should start at position 30 (25 + 5 minimum gap)
        let line0 = (0..52).map(|x| buf[(x, 0)].symbol()).collect::<String>();
        assert!(
            line0[31..].trim_start().starts_with("Final"),
            "Second box should start at position 31 with minimum gap of 5, got: '{}'",
            line0
        );
    }

    #[test]
    fn test_row_alignment_default_is_spread() {
        let row = DocumentElement::row(vec![DocumentElement::text("test")]);
        match row {
            DocumentElement::Row { align, .. } => {
                assert_eq!(align, RowAlignment::Spread);
            }
            _ => panic!("Expected Row variant"),
        }
    }

    #[test]
    fn test_row_left_alignment_variant() {
        let row = DocumentElement::row_left(vec![DocumentElement::text("test")]);
        match row {
            DocumentElement::Row { align, .. } => {
                assert_eq!(align, RowAlignment::Left);
            }
            _ => panic!("Expected Row variant"),
        }
    }
}
