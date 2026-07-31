use super::*;
use crate::tui::testing::assert_buffer;

#[test]
fn test_data_settings_navigation_skips_inert_rows() {
    // Data category renders "Refresh Interval" (inert), then "Western
    // Teams First" (focusable), then "Time Format" (inert). The first
    // *focusable* row's ID is what gets focused, so the selector marker
    // should land directly on "Western Teams First" and never on the two
    // inert, display-only rows.
    let first_focusable = get_focusable_ids_for_category(SettingsCategory::Data)
        .into_iter()
        .next();
    let widget = SettingsTabWidget {
        category: SettingsCategory::Data,
        config: Arc::new(Config::default()),
        focused_id: first_focusable,
        scroll_offset: 0,
        viewport_height: 8,
        focused: true,
    };

    let area = Rect::new(0, 0, 60, 8);
    let mut buf = Buffer::empty(area);
    let display_config = Config::default().display;
    let ctx = RenderContext::focused(&display_config);
    widget.render(area, &mut buf, &ctx);

    assert_buffer(
        &buf,
        &[
            "",
            " Refresh Interval:      60 seconds",
            "",
            " ▶ Western Teams First:   false",
            "",
            " Time Format:           %H:%M:%S",
            "",
            "",
        ],
    );
}

#[test]
fn test_settings_tab_init() {
    let props = SettingsTabProps {
        config: Arc::new(Config::default()),
        focused: false,
    };
    let state = SettingsTab::init(&props);

    assert_eq!(state.selected_category, SettingsCategory::Logging);
    assert_eq!(state.doc_nav.focus_index, None);
    assert_eq!(state.doc_nav.scroll_offset, 0);
}

#[test]
fn test_settings_tab_renders() {
    let settings_tab = SettingsTab;
    let props = SettingsTabProps {
        config: Arc::new(Config::default()),
        focused: false,
    };
    let state = SettingsTabState::default();

    let element = settings_tab.view(&props, &state);

    // Should create a container element (from TabbedPanel's vertical layout)
    match element {
        Element::Container { children, .. } => {
            // Container created successfully with children
            assert_eq!(children.len(), 2); // Tab bar + content
        }
        _ => panic!("Expected container element"),
    }
}

#[test]
fn test_category_to_key() {
    let settings_tab = SettingsTab;

    assert_eq!(
        settings_tab.category_to_key(SettingsCategory::Logging),
        "logging"
    );
    assert_eq!(
        settings_tab.category_to_key(SettingsCategory::Display),
        "display"
    );
    assert_eq!(settings_tab.category_to_key(SettingsCategory::Data), "data");
}

#[test]
fn test_doc_nav_message_handling() {
    use crate::tui::document_nav::DocumentNavMsg;

    let mut component = SettingsTab;
    let mut state = SettingsTabState::default();

    // Set up some focusable elements
    state.doc_nav.focusables = vec![
        FocusableElement::at(0, 1, FocusableId::link("a")),
        FocusableElement::at(2, 1, FocusableId::link("b")),
        FocusableElement::at(4, 1, FocusableId::link("c")),
    ];
    state.doc_nav.focus_index = Some(0);

    let effect = component.update(
        SettingsTabMsg::DocNav(DocumentNavMsg::FocusNext),
        &mut state,
    );

    assert_eq!(state.doc_nav.focus_index, Some(1));
    assert!(matches!(effect, Effect::None));
}

#[test]
fn test_update_viewport_height() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState::default();

    let effect = component.update(SettingsTabMsg::UpdateViewportHeight(50), &mut state);

    assert_eq!(state.doc_nav.viewport_height, 50);
    assert!(matches!(effect, Effect::None));
}

// --- SettingsTabMsg::NavigateCategoryLeft/Right (category cycling) ------
//
// Ported from reducer.rs's test_settings_navigate_category_left/right_from_*
// and reducers/settings.rs's navigate_category tests, now that category
// selection and its doc_nav rebuild both live in this component's `update()`
// instead of being split across global state and a global reducer.

#[test]
fn navigate_category_left_from_logging_wraps_to_data() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState {
        selected_category: SettingsCategory::Logging,
        ..Default::default()
    };

    let effect = component.update(
        SettingsTabMsg::NavigateCategoryLeft(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Data);
    assert!(matches!(effect, Effect::None));
}

#[test]
fn navigate_category_left_from_display_goes_to_logging() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState {
        selected_category: SettingsCategory::Display,
        ..Default::default()
    };

    let effect = component.update(
        SettingsTabMsg::NavigateCategoryLeft(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Logging);
    assert!(matches!(effect, Effect::None));
}

#[test]
fn navigate_category_left_from_data_goes_to_display() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState {
        selected_category: SettingsCategory::Data,
        ..Default::default()
    };

    let effect = component.update(
        SettingsTabMsg::NavigateCategoryLeft(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Display);
    assert!(matches!(effect, Effect::None));
}

#[test]
fn navigate_category_right_from_logging_goes_to_display() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState {
        selected_category: SettingsCategory::Logging,
        ..Default::default()
    };

    let effect = component.update(
        SettingsTabMsg::NavigateCategoryRight(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Display);
    assert!(matches!(effect, Effect::None));
}

#[test]
fn navigate_category_right_from_display_goes_to_data() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState {
        selected_category: SettingsCategory::Display,
        ..Default::default()
    };

    let effect = component.update(
        SettingsTabMsg::NavigateCategoryRight(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Data);
    assert!(matches!(effect, Effect::None));
}

#[test]
fn navigate_category_right_from_data_wraps_to_logging() {
    let mut component = SettingsTab;
    let mut state = SettingsTabState {
        selected_category: SettingsCategory::Data,
        ..Default::default()
    };

    let effect = component.update(
        SettingsTabMsg::NavigateCategoryRight(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Logging);
    assert!(matches!(effect, Effect::None));
}

/// Builds a `SettingsTabState` with a `doc_nav` that is deliberately "stale"
/// (as if left over from a previously focused category), so the rebuild
/// test below can confirm navigation actually replaces it rather than
/// merely leaving it untouched.
fn stale_settings_tab_state(selected_category: SettingsCategory) -> SettingsTabState {
    SettingsTabState {
        selected_category,
        doc_nav: DocumentNavState {
            focus_index: Some(3),
            scroll_offset: 7,
            viewport_height: 20,
            focusables: vec![FocusableElement::at(99, 1, FocusableId::link("stale"))],
            ..Default::default()
        },
        modal: None,
    }
}

#[test]
fn navigate_category_rebuilds_doc_nav_focus_metadata_discarding_stale_state() {
    use crate::tui::document::{Document, FocusContext};

    let mut component = SettingsTab;
    let mut state = stale_settings_tab_state(SettingsCategory::Logging);

    component.update(
        SettingsTabMsg::NavigateCategoryRight(Config::default()),
        &mut state,
    );

    assert_eq!(state.selected_category, SettingsCategory::Display);
    // Stale focus/scroll position must be cleared, not carried over into
    // the new category.
    assert_eq!(state.doc_nav.focus_index, None);
    assert_eq!(state.doc_nav.scroll_offset, 0);
    assert_eq!(state.doc_nav.viewport_height, 0);

    let expected_doc = SettingsDocument::new(SettingsCategory::Display, Config::default());
    assert_eq!(
        state.doc_nav.focusables,
        expected_doc.focusables(&FocusContext::default())
    );
    // Sanity check: Display category actually has focusable settings, so
    // this test would fail loudly (rather than vacuously) if rebuilding broke.
    assert!(!state.doc_nav.focusables.is_empty());
}

#[test]
fn test_get_focusable_ids_logging() {
    // "log_file" is display-only (not editable via the UI), so only
    // "log_level" is focusable.
    let ids = get_focusable_ids_for_category(SettingsCategory::Logging);
    assert_eq!(ids, vec![FocusableId::Link("log_level".to_string())]);
}

#[test]
fn test_get_focusable_ids_display() {
    let ids = get_focusable_ids_for_category(SettingsCategory::Display);
    assert_eq!(
        ids,
        vec![
            FocusableId::Link("theme".to_string()),
            FocusableId::Link("use_unicode".to_string()),
        ]
    );
}

#[test]
fn test_get_focusable_ids_data() {
    // "refresh_interval" and "time_format" are display-only (not editable
    // via the UI), so only "western_teams_first" is focusable.
    let ids = get_focusable_ids_for_category(SettingsCategory::Data);
    assert_eq!(
        ids,
        vec![FocusableId::Link("western_teams_first".to_string())]
    );
}

#[test]
fn test_activate_setting_toggle_dispatches_toggle_boolean() {
    use crate::tui::action::{Action, SettingsAction};
    use crate::tui::document::LinkTarget;

    let mut settings_tab = SettingsTab;
    let props = SettingsTabProps {
        config: Arc::new(Config::default()),
        focused: true,
    };
    let mut state = SettingsTab::init(&props);
    state.selected_category = SettingsCategory::Data;
    SettingsTab::rebuild_doc_nav_for_category(&mut state, props.config.as_ref().clone());
    state.doc_nav.focus_index = Some(0);

    // Sanity check: the "western_teams_first" row is declared as a
    // ToggleSetting link by settings_document.rs.
    let link_targets: Vec<_> = state
        .doc_nav
        .focusables
        .iter()
        .map(|f| f.link_target.clone())
        .collect();
    assert_eq!(
        link_targets,
        vec![Some(LinkTarget::ToggleSetting(
            "western_teams_first".to_string()
        ))]
    );

    let effect = settings_tab.update(
        SettingsTabMsg::ActivateSetting(props.config.as_ref().clone()),
        &mut state,
    );

    match effect {
        Effect::Action(Action::SettingsAction(SettingsAction::ToggleBoolean(key))) => {
            assert_eq!(key, "western_teams_first");
        }
        _ => panic!("Expected ToggleBoolean action, got {:?}", effect),
    }
    assert!(state.modal.is_none());
}

#[test]
fn test_activate_setting_edit_opens_modal() {
    let mut settings_tab = SettingsTab;
    let props = SettingsTabProps {
        config: Arc::new(Config::default()),
        focused: true,
    };
    let mut state = SettingsTab::init(&props);
    state.doc_nav.focus_index = Some(0);

    let effect = settings_tab.update(
        SettingsTabMsg::ActivateSetting(props.config.as_ref().clone()),
        &mut state,
    );

    assert!(matches!(effect, Effect::None));
    let modal = state.modal.expect("expected modal to open for log_level");
    assert_eq!(modal.setting_key, "log_level");
}

#[test]
fn test_activate_setting_without_focus_does_nothing() {
    let mut settings_tab = SettingsTab;
    let props = SettingsTabProps {
        config: Arc::new(Config::default()),
        focused: true,
    };
    let mut state = SettingsTab::init(&props);
    // No focus set.

    let effect = settings_tab.update(
        SettingsTabMsg::ActivateSetting(props.config.as_ref().clone()),
        &mut state,
    );

    assert!(matches!(effect, Effect::None));
    assert!(state.modal.is_none());
}
