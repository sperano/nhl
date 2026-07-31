use super::*;
use crate::config::{DisplayConfig, RenderContext};
use crate::tui::testing::assert_buffer;

#[test]
fn test_demo_document_builds() {
    let doc = DemoDocument::new(None);
    let elements = doc.build(&FocusContext::default());

    // Should have multiple elements
    assert!(elements.len() > 10);
}

#[test]
fn test_demo_document_height() {
    let doc = DemoDocument::new(None);
    let height = doc.calculate_height();

    // Should have significant height (all the content)
    assert!(height > 30);
}

#[test]
fn test_demo_tab_renders() {
    let props = DemoTabProps {
        focused: false,
        standings: Arc::new(None),
    };
    let state = crate::tui::document_nav::DocumentNavState::default();
    let demo_tab = DemoTab;

    let element = demo_tab.view(&props, &state);

    // Should return a widget element
    assert!(matches!(element, Element::Widget(_)));
}

#[test]
fn test_demo_document_focusable_count_no_standings() {
    let doc = DemoDocument::new(None);

    // Should have 4 focusable elements:
    // - 4 example links (BOS, TOR, NYR, MTL)
    // - The tabs content (Standings tab with no data has no focusable elements)
    assert_eq!(doc.focusables(&FocusContext::default()).len(), 4);
}

#[test]
fn test_demo_tab_widget_render() {
    let widget = DemoTabWidget {
        focused: true,
        focused_id: None,
        has_item_focus: false,
        scroll_offset: 0,
        standings: Arc::new(None),
        tab_selections: std::collections::HashMap::new(),
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 5));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);

    widget.render(buf.area, &mut buf, &ctx);

    // Should render the heading and first lines of content
    assert_buffer(
        &buf,
        &[
            " Document System Demo",
            " ════════════════════",
            "",
            " This tab demonstrates the new document system for the NHL",
            " Press Tab/Shift-Tab to navigate, Left/Right to switch tabs",
        ],
    );
}

// Focus-order navigation (Tab/Shift-Tab advancing/wrapping through
// `DemoDocument`'s focusables) used to be tested here against
// `DocumentView::focus_next/prev` (Engine A). That engine never ran in
// production -- the render path only ever calls `DocumentView::focus_id`
// with an ID resolved by `document_nav.rs` (Engine B), whose own generic
// tests (`test_focus_next_advances`, `test_focus_prev_wraps_around`, etc. in
// document_nav.rs) already cover the same advance/wrap logic.

#[test]
fn test_activate_link_team() {
    use crate::tui::component::Component;
    use crate::tui::document::{FocusableElement, FocusableId};
    use crate::tui::document_nav::DocumentNavState;

    let mut demo_tab = DemoTab;
    // Set up state with a focused team link.
    // The first 4 focusable elements are team links (BOS, TOR, NYR, MTL).
    let mut state = DocumentNavState {
        focus_index: Some(0), // BOS link
        focusables: vec![
            FocusableElement::at(0, 1, FocusableId::team_link("BOS")).with_link_target(
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "BOS".to_string(),
                    season: None,
                }),
            ),
            FocusableElement::at(1, 1, FocusableId::team_link("TOR")).with_link_target(
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "TOR".to_string(),
                    season: None,
                }),
            ),
            FocusableElement::at(2, 1, FocusableId::team_link("NYR")).with_link_target(
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "NYR".to_string(),
                    season: None,
                }),
            ),
            FocusableElement::at(3, 1, FocusableId::team_link("MTL")).with_link_target(
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "MTL".to_string(),
                    season: None,
                }),
            ),
        ],
        ..Default::default()
    };

    let effect = demo_tab.update(DemoTabMsg::ActivateLink, &mut state);

    // Should return PushDocument action for TeamDetail
    match effect {
        Effect::Action(Action::PushDocument(StackedDocument::TeamDetail {
            abbrev, ..
        })) => {
            assert_eq!(abbrev, "BOS");
        }
        _ => panic!("Expected PushDocument(TeamDetail), got {:?}", effect),
    }
}

#[test]
fn test_activate_link_player() {
    use crate::tui::component::Component;
    use crate::tui::document::{FocusableElement, FocusableId};
    use crate::tui::document_nav::DocumentNavState;

    let mut demo_tab = DemoTab;
    // Set up state with a focused player link.
    let mut state = DocumentNavState {
        focus_index: Some(0),
        focusables: vec![
            FocusableElement::at(0, 1, FocusableId::player_link(8477492)).with_link_target(
                LinkTarget::Push(StackedDocument::PlayerDetail {
                    player_id: 8477492,
                    sweater_number: None,
                    last_name: "Player 8477492".to_string(),
                }),
            ),
        ],
        ..Default::default()
    };

    let effect = demo_tab.update(DemoTabMsg::ActivateLink, &mut state);

    // Should return PushDocument action for PlayerDetail
    match effect {
        Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
            player_id,
            ..
        })) => {
            assert_eq!(player_id, 8477492);
        }
        _ => panic!("Expected PushDocument(PlayerDetail), got {:?}", effect),
    }
}

#[test]
fn test_activate_link_no_focus() {
    use crate::tui::component::Component;
    use crate::tui::document::{FocusableElement, FocusableId};
    use crate::tui::document_nav::DocumentNavState;

    let mut demo_tab = DemoTab;
    // No focus index set.
    let mut state = DocumentNavState {
        focus_index: None,
        focusables: vec![FocusableElement::at(0, 1, FocusableId::team_link("BOS"))
            .with_link_target(LinkTarget::Push(StackedDocument::TeamDetail {
                abbrev: "BOS".to_string(),
                season: None,
            }))],
        ..Default::default()
    };

    let effect = demo_tab.update(DemoTabMsg::ActivateLink, &mut state);

    // Should return None effect when no focus
    assert!(matches!(effect, Effect::None));
}
