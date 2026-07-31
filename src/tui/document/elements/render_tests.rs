use super::*;
use crate::config::DisplayConfig;
use crate::tui::components::TableWidget;
use crate::tui::document::elements::DocTabDef;
use crate::tui::document::link::LinkTarget;
use crate::tui::widgets::{ScoreBox, ScoreBoxStatus};
use crate::tui::{Alignment, CellValue, ColumnDef};
use ratatui::style::{Color, Modifier};

/// Build a single-column text table with the given row labels.
/// An empty slice produces a table with `row_count() == 0`.
fn text_table(rows: &[&str]) -> TableWidget {
    let columns: Vec<ColumnDef<&str>> =
        vec![ColumnDef::new("Name", 8, Alignment::Left, |row: &&str| {
            CellValue::Text(row.to_string())
        })];
    TableWidget::from_data(&columns, rows.to_vec())
}

fn final_score_box(away: &str, home: &str) -> ScoreBox {
    ScoreBox::new(
        away,
        home,
        Some(1),
        Some(2),
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: false,
        },
    )
}

// ---- get_preferred_width ----

#[test]
fn preferred_width_score_box_returns_its_width() {
    let elem = DocumentElement::score_box_element(
        1,
        final_score_box("A", "B"),
        false,
        LinkTarget::Anchor("x".to_string()),
    );
    assert_eq!(get_preferred_width(&elem), Some(25));
}

#[test]
fn preferred_width_team_boxscore_returns_fixed_constant() {
    let elem = DocumentElement::team_boxscore(
        "away",
        "Team",
        text_table(&[]),
        text_table(&[]),
        text_table(&[]),
    );
    assert_eq!(get_preferred_width(&elem), Some(TEAM_BOXSCORE_WIDTH));
}

#[test]
fn preferred_width_text_element_returns_none() {
    let elem = DocumentElement::text("hello");
    assert_eq!(get_preferred_width(&elem), None);
}

// ---- render_row ----

#[test]
fn row_empty_children_does_not_touch_buffer() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &[],
        2,
        RowAlignment::Left,
        Rect::new(0, 0, 10, 3),
        &mut buf,
        &ctx,
    );

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
}

#[test]
fn row_zero_width_area_does_not_touch_buffer() {
    let children = vec![DocumentElement::text("x")];
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &children,
        2,
        RowAlignment::Left,
        Rect::new(0, 0, 0, 3),
        &mut buf,
        &ctx,
    );

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
}

#[test]
fn row_left_alignment_places_children_with_minimum_gap() {
    let children = vec![
        DocumentElement::score_box_element(
            1,
            final_score_box("A", "B"),
            false,
            LinkTarget::Anchor("g1".to_string()),
        ),
        DocumentElement::score_box_element(
            2,
            final_score_box("C", "D"),
            false,
            LinkTarget::Anchor("g2".to_string()),
        ),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 6));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &children,
        3,
        RowAlignment::Left,
        Rect::new(0, 0, 60, 6),
        &mut buf,
        &ctx,
    );

    // Second box (width 25) should start at 25 + gap(3) = 28
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "╔"); // first box top border
    assert_eq!(buf.cell((28, 1)).unwrap().symbol(), "╔"); // second box top border
}

#[test]
fn row_spread_alignment_with_single_child_has_no_gap_math() {
    // A single preferred-width child means num_gaps == 0, exercising that branch.
    let children = vec![DocumentElement::score_box_element(
        1,
        final_score_box("A", "B"),
        false,
        LinkTarget::Anchor("g1".to_string()),
    )];
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 6));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &children,
        2,
        RowAlignment::Spread,
        Rect::new(0, 0, 40, 6),
        &mut buf,
        &ctx,
    );

    // Single child starts flush at area.x with Spread alignment
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "╔");
}

#[test]
fn row_center_alignment_centers_children_group() {
    let children = vec![
        DocumentElement::score_box_element(
            1,
            final_score_box("A", "B"),
            false,
            LinkTarget::Anchor("g1".to_string()),
        ),
        DocumentElement::score_box_element(
            2,
            final_score_box("C", "D"),
            false,
            LinkTarget::Anchor("g2".to_string()),
        ),
    ];
    // total_content_width = 25 + 2(gap) + 25 = 52; area width 62 -> left_margin = 5
    let mut buf = Buffer::empty(Rect::new(0, 0, 62, 6));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &children,
        2,
        RowAlignment::Center,
        Rect::new(0, 0, 62, 6),
        &mut buf,
        &ctx,
    );

    assert_eq!(buf.cell((5, 1)).unwrap().symbol(), "╔"); // first box shifted by margin
    assert_eq!(buf.cell((5 + 25 + 2, 1)).unwrap().symbol(), "╔"); // second box after gap
}

#[test]
fn row_flexible_children_distribute_width_equally() {
    let children = vec![
        DocumentElement::text("left"),
        DocumentElement::text("right"),
    ];
    // area width 20, gap 2 -> available 18, child_width 9 each
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &children,
        2,
        RowAlignment::Left,
        Rect::new(0, 0, 20, 1),
        &mut buf,
        &ctx,
    );

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "l");
    // second child starts at child_width(9) + gap(2) = 11
    assert_eq!(buf.cell((11, 0)).unwrap().symbol(), "r");
}

#[test]
fn row_mixed_preferred_and_flexible_children_uses_flexible_path() {
    // ScoreBoxElement has a preferred width, Text does not, so `has_preferred_widths`
    // is false and both children fall back to the equal-width distribution branch.
    let children = vec![
        DocumentElement::score_box_element(
            1,
            final_score_box("A", "B"),
            false,
            LinkTarget::Anchor("g1".to_string()),
        ),
        DocumentElement::text("hi"),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &children,
        2,
        RowAlignment::Left,
        Rect::new(0, 0, 20, 6),
        &mut buf,
        &ctx,
    );

    // ScoreBox area was allotted 9 columns (not 25), so it is not drawn at all
    // (ScoreBox::render bails out when area.width < SCORE_BOX_WIDTH).
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), " ");
    // Second child (text) starts at child_width(9) + gap(2) = 11
    assert_eq!(buf.cell((11, 0)).unwrap().symbol(), "h");
}

#[test]
fn row_clips_children_that_do_not_respect_their_own_area_width() {
    // TableWidget lays out columns at their natural width regardless of the area it's
    // given (see TableWidget::render_internal, which calls the unbounded `buf.set_string`).
    // Without per-child clipping in render_row, two such tables side-by-side in a row
    // narrower than their combined natural width would bleed into each other.
    let left = DocumentElement::table("left", text_table(&["AAAAAAAA"]));
    let right = DocumentElement::table("right", text_table(&["BBBBBBBB"]));

    // Row is 10 columns wide; with no preferred width, each child gets 5 columns from the
    // flexible equal-split branch - but the table needs SELECTOR(2) + column(8) = 10.
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(
        &[left, right],
        0,
        RowAlignment::Left,
        Rect::new(0, 0, 10, 3),
        &mut buf,
        &ctx,
    );

    // Right child owns columns 5..10 exclusively; the left table's 'A's must never
    // bleed into it (truncation of the left table's own content is fine and expected).
    let right_columns = (5..10).map(|x| buf[(x, 2)].symbol()).collect::<String>();
    assert!(
        !right_columns.contains('A'),
        "left table bled into right child's columns: {right_columns:?}"
    );
}

// ---- clipping contract (#68) ----

#[test]
fn element_render_clips_standalone_table_to_its_area() {
    // Natural table width is SELECTOR(2) + column(8) = 10, but the area is
    // only 5 wide. Before the central clip, only tables inside a Row were
    // protected; a standalone table bled into the rest of the buffer.
    let elem = DocumentElement::table("t", text_table(&["AAAAAAAA"]));
    let mut buf = Buffer::empty(Rect::new(0, 0, 12, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    elem.render(Rect::new(0, 0, 5, 3), &mut buf, &ctx);

    for y in 0..3 {
        for x in 5..12 {
            assert_eq!(
                buf.cell((x, y)).unwrap().symbol(),
                " ",
                "table bled outside its area at ({x},{y})"
            );
        }
    }
}

#[test]
fn element_render_clips_multi_column_table_without_panicking() {
    // The second column's start position (SELECTOR 2 + col 8 + gap 2 = 12)
    // lies beyond the 5-wide area. set_string panics on an out-of-bounds
    // *start*, so this exercises TableWidget's column start guard as well
    // as the central clip.
    let columns: Vec<ColumnDef<&str>> = vec![
        ColumnDef::new("A", 8, Alignment::Left, |row: &&str| {
            CellValue::Text(row.to_string())
        }),
        ColumnDef::new("B", 8, Alignment::Left, |row: &&str| {
            CellValue::Text(row.to_string())
        }),
    ];
    let elem = DocumentElement::table("t", TableWidget::from_data(&columns, vec!["x"]));
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    elem.render(Rect::new(0, 0, 5, 3), &mut buf, &ctx);

    for y in 0..3 {
        for x in 5..20 {
            assert_eq!(
                buf.cell((x, y)).unwrap().symbol(),
                " ",
                "table bled outside its area at ({x},{y})"
            );
        }
    }
}

#[test]
fn element_render_clips_team_boxscore_vertically() {
    let elem = DocumentElement::team_boxscore(
        "away",
        "AB",
        text_table(&["Alice"]),
        text_table(&[]),
        text_table(&[]),
    );
    let full_height = elem.height();
    let short_height = full_height - 3;
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, full_height + 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Give the element fewer rows than it needs: everything below the
    // area must stay untouched instead of bleeding into later elements.
    elem.render(Rect::new(0, 0, 40, short_height), &mut buf, &ctx);

    for y in short_height..full_height + 2 {
        assert_eq!(
            buf.cell((0, y)).unwrap().symbol(),
            " ",
            "boxscore bled below its area at row {y}"
        );
    }
}

#[test]
fn element_render_clips_custom_render_fn() {
    // A Custom element's render_fn is arbitrary code; the central clip
    // must contain it like any other element.
    let elem = DocumentElement::Custom {
        render_fn: |_area, buf, _ctx| {
            // Deliberately ignore the area and write far outside it.
            buf.set_string(8, 2, "XX", Style::default());
        },
        height: 1,
        focusable: vec![],
    };
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    elem.render(Rect::new(0, 0, 5, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((8, 2)).unwrap().symbol(), " ");
}

#[test]
fn row_preserves_prestyled_untouched_cells_in_child_areas() {
    // Regression: the old empty-scratch merge reset every cell of a row
    // child's area to a default cell, clobbering the theme background
    // fill that render_elements_to_buffer applies before elements draw.
    let children = vec![DocumentElement::text("hi"), DocumentElement::text("yo")];
    let area = Rect::new(0, 0, 20, 2);
    let mut buf = Buffer::empty(area);
    buf.set_style(area, Style::default().bg(Color::Blue));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_row(&children, 2, RowAlignment::Left, area, &mut buf, &ctx);

    // Text was drawn on row 0.
    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "h");
    // Row 1 lies inside both children's areas but is never drawn on: it
    // must keep the pre-applied background.
    assert_eq!(buf.cell((0, 1)).unwrap().style().bg, Some(Color::Blue));
    assert_eq!(buf.cell((11, 1)).unwrap().style().bg, Some(Color::Blue));
}

// ---- render_text ----

#[test]
fn text_renders_single_line() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_text("Hi", None, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "H");
    assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "i");
}

#[test]
fn text_renders_multiple_lines() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_text(
        "one\ntwo\nthree",
        None,
        Rect::new(0, 0, 10, 3),
        &mut buf,
        &ctx,
    );

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "o");
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "t");
    assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "t");
}

#[test]
fn text_stops_rendering_lines_beyond_area_height() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    // Pre-fill so we can detect untouched cells.
    for x in 0..10 {
        buf.cell_mut((x, 0)).unwrap().set_char('_');
        buf.cell_mut((x, 1)).unwrap().set_char('_');
    }
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Area height is only 2, but content has 3 lines - third line must be skipped.
    render_text("a\nb\nc", None, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "b");
}

#[test]
fn text_stops_rendering_chars_beyond_area_width() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_text("abcdef", None, Rect::new(0, 0, 3, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
    assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "c");
}

#[test]
fn text_uses_context_default_style_when_none_given() {
    // `Buffer::cell().style()` always reconstructs explicit Color::Reset fields,
    // so comparing against a hand-built Style would fail even when behavior is
    // correct. Instead, compare against a second render that passes the context's
    // style explicitly - the two should produce identical cells.
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    let mut buf_default = Buffer::empty(Rect::new(0, 0, 5, 1));
    render_text("hi", None, Rect::new(0, 0, 5, 1), &mut buf_default, &ctx);

    let mut buf_explicit = Buffer::empty(Rect::new(0, 0, 5, 1));
    render_text(
        "hi",
        Some(ctx.text_style()),
        Rect::new(0, 0, 5, 1),
        &mut buf_explicit,
        &ctx,
    );

    assert_eq!(
        buf_default.cell((0, 0)).unwrap().style(),
        buf_explicit.cell((0, 0)).unwrap().style()
    );
}

#[test]
fn text_uses_explicit_style_when_given() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    let style = Style::default().fg(Color::Green);

    render_text("hi", Some(style), Rect::new(0, 0, 5, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().style().fg, Some(Color::Green));
}

#[test]
fn text_renders_multibyte_unicode_content() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_text("héllo", None, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "h");
    assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "é");
    assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "l");
}

#[test]
fn text_wide_characters_occupy_two_columns_and_shift_following_glyphs() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // "日" is a double-width glyph; it must occupy columns 0-1 (blanking column 1),
    // pushing "Hi" to start at column 2 - not column 1, as one-column-per-char would.
    render_text("日Hi", None, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "日");
    assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
    assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "H");
    assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "i");
}

#[test]
fn text_wide_character_straddling_clip_boundary_is_dropped_not_split() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Only 1 column remains after "a"; the 2-column-wide "日" doesn't fit and must be
    // dropped entirely rather than rendering a corrupted half-glyph.
    render_text("a日", None, Rect::new(0, 0, 2, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
    assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
}

// ---- render_heading ----

#[test]
fn heading_level_1_draws_double_underline() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_heading(1, "Title", Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "T");
    for x in 0..5 {
        assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
    }
    assert_eq!(buf.cell((5, 1)).unwrap().symbol(), " ");
}

#[test]
fn heading_level_above_1_has_no_underline() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_heading(2, "Sub", Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "S");
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), " ");
}

#[test]
fn heading_level_1_with_single_row_area_skips_underline() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Should not panic even though there's no room for the underline row.
    render_heading(1, "Title", Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "T");
}

#[test]
fn heading_clips_content_wider_than_area() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_heading(2, "abcdef", Rect::new(0, 0, 3, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "a");
    assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "c");
}

#[test]
fn heading_underline_length_matches_display_width_of_wide_content() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // "日" (2 cols) + "Hi" (2 cols) = 4 display columns, even though it's only 3 chars.
    // A char-count-based underline would draw only 3 dashes and misalign with the title.
    render_heading(1, "日Hi", Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    for x in 0..4 {
        assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
    }
    assert_eq!(buf.cell((4, 1)).unwrap().symbol(), " ");
}

// ---- render_section_title ----

#[test]
fn section_title_without_underline() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_section_title("Atlantic", false, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "A");
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), " ");
}

#[test]
fn section_title_with_underline_matches_title_length() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_section_title("Metro", true, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    for x in 0..5 {
        assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
    }
    assert_eq!(buf.cell((5, 1)).unwrap().symbol(), " ");
}

#[test]
fn section_title_underline_skipped_when_no_second_row() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Should not panic even though underline=true and there's no second row.
    render_section_title("Metro", true, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "M");
}

#[test]
fn section_title_underline_matches_display_width_of_wide_title() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // "日" (2 cols) + "Hi" (2 cols) = 4 display columns, even though it's only 3 chars.
    render_section_title("日Hi", true, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    for x in 0..4 {
        assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "═");
    }
    assert_eq!(buf.cell((4, 1)).unwrap().symbol(), " ");
}

// ---- render_link ----

#[test]
fn link_unfocused_has_two_space_prefix() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_link("Go", false, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
    assert_eq!(buf.cell((1, 0)).unwrap().symbol(), " ");
    assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "G");
}

#[test]
fn link_focused_uses_selector_prefix_and_themeless_modifier() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default(); // theme: None
    let ctx = RenderContext::focused(&config);

    render_link("Go", true, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "▶");
    assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "G");
    let style = buf.cell((2, 0)).unwrap().style();
    assert!(style.add_modifier.contains(Modifier::REVERSED));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn link_focused_uses_theme_colors_when_theme_is_set() {
    use crate::config::THEME_ORANGE;

    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig {
        theme: Some(THEME_ORANGE.clone()),
        ..DisplayConfig::default()
    };
    let ctx = RenderContext::focused(&config);

    render_link("Go", true, Rect::new(0, 0, 10, 1), &mut buf, &ctx);

    let style = buf.cell((2, 0)).unwrap().style();
    assert_eq!(style.fg, Some(THEME_ORANGE.selection_text_fg));
    assert_eq!(style.bg, Some(THEME_ORANGE.selection_text_bg));
    assert!(style.add_modifier.contains(Modifier::BOLD));
    assert!(!style.add_modifier.contains(Modifier::REVERSED));
}

#[test]
fn link_display_text_clips_to_area_width() {
    // Buffer is wider than the area passed to render_link, so this only passes if the
    // display text is clipped to the *area's* width rather than just the buffer's.
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_link(
        "VeryLongLinkText",
        false,
        Rect::new(0, 0, 6, 1),
        &mut buf,
        &ctx,
    );

    // Prefix "  " (2 cols) leaves 4 cols for the display text: "Very".
    assert_eq!(buf.cell((5, 0)).unwrap().symbol(), "y");
    assert_eq!(buf.cell((6, 0)).unwrap().symbol(), " ");
}

// ---- render_separator ----

#[test]
fn separator_fills_area_with_horizontal_char() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_separator(Rect::new(0, 0, 8, 1), &mut buf, &ctx);

    for x in 0..8 {
        assert_eq!(buf.cell((x, 0)).unwrap().symbol(), "─");
    }
}

#[test]
fn separator_zero_width_area_does_nothing() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_separator(Rect::new(0, 0, 0, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
}

// ---- render_group ----

#[test]
fn group_renders_children_stacked_by_height() {
    let children = vec![
        DocumentElement::text("first"),
        DocumentElement::spacer(1),
        DocumentElement::text("third"),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_group(&children, None, Rect::new(0, 0, 10, 3), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "f");
    assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "t");
}

#[test]
fn group_stops_rendering_children_beyond_area_height() {
    let children = vec![
        DocumentElement::text("one"),
        DocumentElement::text("two"),
        DocumentElement::text("three"),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    // Pre-fill row 2 which is outside the render area, to confirm it's never touched.
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_group(&children, None, Rect::new(0, 0, 10, 2), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "o");
    assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "t");
}

#[test]
fn group_applies_style_patch_over_rendered_region() {
    let children = vec![DocumentElement::text("hi")];
    let style = Style::default().bg(Color::Blue);
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_group(
        &children,
        Some(style),
        Rect::new(0, 0, 5, 1),
        &mut buf,
        &ctx,
    );

    // Background is patched on top of the child's own style (fg unaffected).
    assert_eq!(buf.cell((0, 0)).unwrap().style().bg, Some(Color::Blue));
}

#[test]
fn group_with_no_children_and_style_does_not_panic() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    let style = Style::default().bg(Color::Blue);

    render_group(&[], Some(style), Rect::new(0, 0, 5, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
}

// ---- render_team_boxscore ----

#[test]
fn team_boxscore_renders_all_three_sections_with_borders() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 19));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_team_boxscore(
        "AB",
        &text_table(&["Alice"]),
        &text_table(&["Bob"]),
        &text_table(&["Carl"]),
        Rect::new(0, 0, 40, 19),
        &mut buf,
        &ctx,
    );

    // First section header uses the "first section" top-left corner.
    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╒");
    assert_eq!(buf.cell((39, 0)).unwrap().symbol(), "╕");
    // Later sections use the T-junction corner instead.
    assert_eq!(buf.cell((0, 6)).unwrap().symbol(), "╞");
    assert_eq!(buf.cell((0, 12)).unwrap().symbol(), "╞");
    // Bottom border on the final row.
    assert_eq!(buf.cell((0, 18)).unwrap().symbol(), "╘");
    assert_eq!(buf.cell((39, 18)).unwrap().symbol(), "╛");
}

#[test]
fn team_boxscore_skips_empty_sections() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 13));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Forwards is empty; defense becomes the first rendered section.
    render_team_boxscore(
        "AB",
        &text_table(&[]),
        &text_table(&["Bob"]),
        &text_table(&["Carl"]),
        Rect::new(0, 0, 40, 13),
        &mut buf,
        &ctx,
    );

    // Defense section is now first, so it should get the top-left corner.
    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╒");
}

#[test]
fn team_boxscore_all_sections_empty_renders_only_bottom_border() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_team_boxscore(
        "AB",
        &text_table(&[]),
        &text_table(&[]),
        &text_table(&[]),
        Rect::new(0, 0, 40, 1),
        &mut buf,
        &ctx,
    );

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╘");
    assert_eq!(buf.cell((39, 0)).unwrap().symbol(), "╛");
}

#[test]
fn team_boxscore_width_is_capped_by_area_width() {
    // Forwards section needs header(1) + blank(1) + table(3 rows) + blank(1) + bottom border(1) = 7
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 7));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_team_boxscore(
        "AB",
        &text_table(&["Alice"]),
        &text_table(&[]),
        &text_table(&[]),
        Rect::new(0, 0, 20, 7),
        &mut buf,
        &ctx,
    );

    // Right border should be at x=19 (area width - 1), not at TEAM_BOXSCORE_WIDTH - 1.
    assert_eq!(buf.cell((19, 0)).unwrap().symbol(), "╕");
}

#[test]
fn team_boxscore_rendered_rows_match_element_height() {
    // Regression test for #67: height() and render_team_boxscore used to
    // derive the section chrome independently. The bottom border must land
    // exactly on the last row that height() reserves, with nothing drawn
    // past it.
    let elem = DocumentElement::team_boxscore(
        "away",
        "AB",
        text_table(&["Alice", "Anna"]),
        text_table(&["Bob"]),
        text_table(&[]),
    );
    let height = elem.height();

    // Render into a taller buffer so drift past the reserved rows is visible.
    let area = Rect::new(0, 0, 40, height + 3);
    let mut buf = Buffer::empty(area);
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    elem.render(area, &mut buf, &ctx);

    assert_eq!(buf.cell((0, height - 1)).unwrap().symbol(), "╘");
    assert_eq!(buf.cell((0, height)).unwrap().symbol(), " ");
}

// ---- render_section_header ----

#[test]
fn section_header_first_section_uses_top_corners() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_section_header(0, 0, 30, "Team - Forwards", true, &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╒");
    assert_eq!(buf.cell((29, 0)).unwrap().symbol(), "╕");
}

#[test]
fn section_header_later_section_uses_t_junction_corners() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_section_header(0, 0, 30, "Team - Defense", false, &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╞");
    assert_eq!(buf.cell((29, 0)).unwrap().symbol(), "╡");
}

#[test]
fn section_header_embeds_title_text() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_section_header(0, 0, 30, "Sharks - Goalies", true, &mut buf, &ctx);
    let line = (0..30).map(|x| buf[(x, 0)].symbol()).collect::<String>();
    assert!(line.contains("Sharks - Goalies"));
}

#[test]
fn section_header_narrow_width_truncates_title_and_preserves_corner() {
    // Buffer is much wider than the assigned header width, so this only passes if the
    // title is truncated to the assigned width rather than bleeding past it before the
    // corner gets a chance to (incidentally) paper over the overflow.
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_section_header(0, 0, 10, "Team - Forwards", true, &mut buf, &ctx);

    // Right corner always lands on the last column of the *assigned* width (9), not
    // wherever the untruncated title happened to end.
    assert_eq!(buf.cell((9, 0)).unwrap().symbol(), "╕");
    // Nothing should have bled past the assigned width into the rest of the buffer.
    assert_eq!(buf.cell((10, 0)).unwrap().symbol(), " ");
}

// ---- render_bottom_border ----

#[test]
fn bottom_border_draws_corners_and_double_horizontal_middle() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_bottom_border(0, 0, 10, &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╘");
    assert_eq!(buf.cell((9, 0)).unwrap().symbol(), "╛");
    for x in 1..9 {
        assert_eq!(buf.cell((x, 0)).unwrap().symbol(), "═");
    }
}

#[test]
fn bottom_border_width_one_skips_right_corner() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // Should not panic despite width - 2 underflowing conceptually (it saturates).
    render_bottom_border(0, 0, 1, &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "╘");
}

// ---- render_tabs ----

#[test]
fn tabs_empty_list_does_not_render() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tabs(&[], 0, Rect::new(0, 0, 20, 5), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
}

#[test]
fn tabs_area_shorter_than_bar_height_does_not_render() {
    let tabs = vec![DocTabDef::new(
        "t1",
        "One",
        vec![DocumentElement::text("content")],
    )];
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tabs(&tabs, 0, Rect::new(0, 0, 20, 1), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
}

#[test]
fn tabs_renders_bar_and_active_tab_content() {
    let tabs = vec![
        DocTabDef::new("t1", "One", vec![DocumentElement::text("first")]),
        DocTabDef::new("t2", "Two", vec![DocumentElement::text("second")]),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tabs(&tabs, 0, Rect::new(0, 0, 30, 4), &mut buf, &ctx);
    let bar = (0..30).map(|x| buf[(x, 0)].symbol()).collect::<String>();
    assert!(bar.contains("One"));
    assert!(bar.contains("Two"));
    assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "f"); // "first" content on row 2
}

#[test]
fn tabs_renders_second_tab_content_when_active() {
    let tabs = vec![
        DocTabDef::new("t1", "One", vec![DocumentElement::text("first")]),
        DocTabDef::new("t2", "Two", vec![DocumentElement::text("second")]),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tabs(&tabs, 1, Rect::new(0, 0, 30, 4), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "s"); // "second" content on row 2
}

#[test]
fn tabs_active_index_out_of_bounds_renders_bar_but_no_content() {
    let tabs = vec![DocTabDef::new(
        "t1",
        "One",
        vec![DocumentElement::text("first")],
    )];
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    // active_index 5 is out of range - should not panic, and no content renders.
    render_tabs(&tabs, 5, Rect::new(0, 0, 30, 4), &mut buf, &ctx);

    let bar = (0..30).map(|x| buf[(x, 0)].symbol()).collect::<String>();
    assert!(bar.contains("One"));
    assert_eq!(buf.cell((0, 2)).unwrap().symbol(), " ");
}

#[test]
fn tabs_content_stops_rendering_beyond_content_area_height() {
    let tabs = vec![DocTabDef::new(
        "t1",
        "One",
        vec![
            DocumentElement::text("row1"),
            DocumentElement::text("row2"),
            DocumentElement::text("row3"),
        ],
    )];
    // Bar takes 2 rows, leaving only 1 row of content area - row2/row3 are clipped.
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 3));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tabs(&tabs, 0, Rect::new(0, 0, 30, 3), &mut buf, &ctx);

    assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "r"); // "row1"
}

// ---- render_tab_bar ----

#[test]
fn tab_bar_active_tab_uses_emphasis_style() {
    let tabs = vec![
        DocTabDef::new("t1", "One", vec![]),
        DocTabDef::new("t2", "Two", vec![]),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tab_bar(&tabs, 0, 0, 0, 30, &mut buf, &ctx);

    // " One " occupies columns 0..5; the label text starts at column 1.
    // Active tab uses emphasis style, which is bold (unlike plain text style).
    assert!(buf
        .cell((1, 0))
        .unwrap()
        .style()
        .add_modifier
        .contains(Modifier::BOLD));
    // "Two" tab (not active) uses the base text style, which is not bold.
    let two_start = " One ".chars().count() as u16 + 1; // +1 for the separator
    assert!(!buf
        .cell((two_start + 1, 0))
        .unwrap()
        .style()
        .add_modifier
        .contains(Modifier::BOLD));
}

#[test]
fn tab_bar_separator_line_filled_and_has_tee_connectors() {
    let tabs = vec![
        DocTabDef::new("t1", "One", vec![]),
        DocTabDef::new("t2", "Two", vec![]),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tab_bar(&tabs, 0, 0, 0, 30, &mut buf, &ctx);
    // Every cell on the separator row should be the horizontal char or a tee.
    for x in 0..30 {
        let symbol = buf.cell((x, 1)).unwrap().symbol();
        assert!(symbol == "─" || symbol == "┴");
    }
    // There should be exactly one tee connector (boundary between the two tabs).
    let tee_count = (0..30)
        .filter(|&x| buf.cell((x, 1)).unwrap().symbol() == "┴")
        .count();
    assert_eq!(tee_count, 1);
}

#[test]
fn tab_bar_single_tab_has_no_separator_or_tee() {
    let tabs = vec![DocTabDef::new("t1", "Only", vec![])];
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tab_bar(&tabs, 0, 0, 0, 20, &mut buf, &ctx);

    for x in 0..20 {
        assert_eq!(buf.cell((x, 1)).unwrap().symbol(), "─");
    }
}

#[test]
fn tab_bar_wide_tab_title_advances_by_display_width_not_char_count() {
    let tabs = vec![
        DocTabDef::new("t1", "日本", vec![]), // 2 chars, but 4 display columns
        DocTabDef::new("t2", "Two", vec![]),
    ];
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    render_tab_bar(&tabs, 0, 0, 0, 30, &mut buf, &ctx);

    // Label " 日本 " occupies 6 display columns (1 + 2 + 2 + 1), so the separator must
    // land at column 6 - not column 4, which is where char-count-based math would put it
    // (right on top of "本"'s continuation cell).
    assert_eq!(buf.cell((6, 0)).unwrap().symbol(), "│");
    assert_eq!(buf.cell((8, 0)).unwrap().symbol(), "T");
}
