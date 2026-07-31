use super::*;
use crate::config::DisplayConfig;
use crate::tui::renderer::Renderer;
use crate::tui::testing::{assert_buffer, create_test_standings};
use ratatui::{buffer::Buffer, layout::Rect};
const RENDER_WIDTH: u16 = 120;
const RENDER_HEIGHT: u16 = 40;

/// Regression test: component state is created lazily on first render, which
/// can be AFTER StandingsLoaded already ran its metadata rebuild against a
/// store with no standings state. init() must therefore populate focusable
/// metadata itself when data is available, or EnterBrowseMode (Down in
/// view-selection mode) finds no focusables and silently does nothing.
#[test]
fn test_init_populates_focusable_metadata_when_standings_present() {
    let props = StandingsTabProps {
        standings: Arc::new(Some(create_test_standings())),
        document_stack: Vec::new(),
        focused: true,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let mut state = StandingsTab::init(&props);

    assert!(
        !state.doc_nav.focusables.is_empty(),
        "init with standings data must produce focusable metadata"
    );
    assert!(
        state
            .doc_nav
            .focusables
            .iter()
            .any(|f| f.link_target.is_some()),
        "at least one focusable team row must carry a link target"
    );
    // The actual user-visible symptom: entering browse mode must focus a team.
    state.focus_first_item();
    assert_eq!(state.doc_nav.focus_index, Some(0));
}

#[test]
fn test_init_with_no_standings_leaves_metadata_empty() {
    let props = StandingsTabProps {
        standings: Arc::new(None),
        document_stack: Vec::new(),
        focused: true,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let state = StandingsTab::init(&props);
    assert!(state.doc_nav.focusables.is_empty());
}

#[test]
fn test_standings_tab_renders_with_no_standings() {
    let standings_tab = StandingsTab;
    let props = StandingsTabProps {
        standings: Arc::new(None),
        document_stack: Vec::new(),
        focused: false,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let element = standings_tab.view(&props, &StandingsTabState::default());

    match element {
        Element::Container { children, .. } => {
            assert_eq!(children.len(), 2);
        }
        _ => panic!("Expected container element"),
    }
}

#[test]
fn test_standings_tab_renders_league_view() {
    let standings_tab = StandingsTab;
    let standings = create_test_standings();

    let props = StandingsTabProps {
        standings: Arc::new(Some(standings)),
        document_stack: Vec::new(),
        focused: false,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    // This should not panic - verifies TableWidget can be created
    let element = standings_tab.view(&props, &StandingsTabState::default());

    match element {
        Element::Container { children, .. } => {
            assert_eq!(children.len(), 2); // Tab bar + content
        }
        _ => panic!("Expected container element"),
    }
}

// === Rendering Tests ===

/// Helper to render element to buffer
fn render_element_to_buffer(
    element: &Element,
    width: u16,
    height: u16,
    config: &DisplayConfig,
) -> Buffer {
    use crate::config::RenderContext;
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    let mut renderer = Renderer::new();
    let ctx = RenderContext::focused(config);
    renderer.render(element.clone(), buf.area, &mut buf, &ctx);
    buf
}

#[test]
fn test_league_view_full_render() {
    let standings_tab = StandingsTab;
    let standings = create_test_standings();

    let props = StandingsTabProps {
        standings: Arc::new(Some(standings)),
        document_stack: Vec::new(),
        focused: false,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let state = StandingsTabState {
        view: GroupBy::League,
        ..Default::default()
    };

    let element = standings_tab.view(&props, &state);
    let config = DisplayConfig::default();
    let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

    // Note: Tab bar has leading space, document content has left/right margins
    assert_buffer(&buf, &[
        " Wildcard │ Division │ Conference │ League",
        "──────────┴──────────┴────────────┴─────────────────────────────────────────────────────────────────────────────────────",
        "   Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────",
        "   Avalanche                     19    16    2    1     33",
        "   Devils                        18    15    2    1     31",
        "   Golden Knights                19    15    3    1     31",
        "   Panthers                      19    14    3    2     30",
        "   Hurricanes                    19    14    3    2     30",
        "   Stars                         20    14    4    2     30",
        "   Oilers                        20    14    4    2     30",
        "   Bruins                        18    13    4    1     27",
        "   Jets                          19    13    5    1     27",
        "   Maple Leafs                   19    12    5    2     26",
        "   Rangers                       18    12    5    1     25",
        "   Kings                         19    12    6    1     25",
        "   Penguins                      19    11    6    2     24",
        "   Wild                          19    11    6    2     24",
        "   Kraken                        19    11    6    2     24",
        "   Lightning                     18    11    6    1     23",
        "   Canadiens                     18    10    5    3     23",
        "   Predators                     19    10    7    2     22",
        "   Canucks                       19    10    7    2     22",
        "   Capitals                      18    10    7    1     21",
        "   Senators                      18     9    7    2     20",
        "   Islanders                     18     9    7    2     20",
        "   Flames                        19     9    8    2     20",
        "   Blues                         19     8    8    3     19",
        "   Red Wings                     18     8    8    2     18",
        "   Flyers                        18     8    9    1     17",
        "   Ducks                         19     7   10    2     16",
        "   Blackhawks                    18     7   10    1     15",
        "   Sabres                        18     6   10    2     14",
        "   Blue Jackets                  18     5   11    2     12",
        "   Sharks                        18     5   12    1     11",
        "   Coyotes                       18     4   13    1      9",
        " ",
        " ",
        " ",
        " ",
    ]);
}

#[test]
fn test_division_view_full_render() {
    let standings_tab = StandingsTab;
    let standings = create_test_standings();

    let props = StandingsTabProps {
        standings: Arc::new(Some(standings)),
        document_stack: Vec::new(),
        focused: false,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let state = StandingsTabState {
        view: GroupBy::Division,
        ..Default::default()
    };

    let element = standings_tab.view(&props, &state);
    let config = DisplayConfig::default();
    let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

    // Division view now uses document system with two Groups in a Row (centered with gap 4)
    // Layout: Atlantic + Metropolitan on left, Central + Pacific on right
    // (when western_first = false, which is the default)
    // Note: Tab bar has leading space, document content has left/right margins
    assert_buffer(&buf, &[
        " Wildcard │ Division │ Conference │ League",
        "──────────┴──────────┴────────────┴─────────────────────────────────────────────────────────────────────────────────────",
        "   Atlantic                                                     Central",
        " ",
        "   Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
        "   Panthers                      19    14    3    2     30      Avalanche                     19    16    2    1     33",
        "   Bruins                        18    13    4    1     27      Stars                         20    14    4    2     30",
        "   Maple Leafs                   19    12    5    2     26      Jets                          19    13    5    1     27",
        "   Lightning                     18    11    6    1     23      Wild                          19    11    6    2     24",
        "   Canadiens                     18    10    5    3     23      Predators                     19    10    7    2     22",
        "   Senators                      18     9    7    2     20      Blues                         19     8    8    3     19",
        "   Red Wings                     18     8    8    2     18      Blackhawks                    18     7   10    1     15",
        "   Sabres                        18     6   10    2     14      Coyotes                       18     4   13    1      9",
        " ",
        "   Metropolitan                                                 Pacific",
        " ",
        "   Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
        "   Devils                        18    15    2    1     31      Golden Knights                19    15    3    1     31",
        "   Hurricanes                    19    14    3    2     30      Oilers                        20    14    4    2     30",
        "   Rangers                       18    12    5    1     25      Kings                         19    12    6    1     25",
        "   Penguins                      19    11    6    2     24      Kraken                        19    11    6    2     24",
        "   Capitals                      18    10    7    1     21      Canucks                       19    10    7    2     22",
        "   Islanders                     18     9    7    2     20      Flames                        19     9    8    2     20",
        "   Flyers                        18     8    9    1     17      Ducks                         19     7   10    2     16",
        "   Blue Jackets                  18     5   11    2     12      Sharks                        18     5   12    1     11",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
    ]);
}

#[test]
fn test_conference_view_full_render() {
    let standings_tab = StandingsTab;
    let standings = create_test_standings();

    let props = StandingsTabProps {
        standings: Arc::new(Some(standings)),
        document_stack: Vec::new(),
        focused: false,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let state = StandingsTabState {
        view: GroupBy::Conference,
        ..Default::default()
    };

    let element = standings_tab.view(&props, &state);
    let config = DisplayConfig::default();
    let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

    // Conference view now uses document system with teams sorted by points (centered with gap 4)
    // Note: Tab bar has leading space, document content has left/right margins (2 chars total)
    assert_buffer(&buf, &[
        " Wildcard │ Division │ Conference │ League",
        "──────────┴──────────┴────────────┴─────────────────────────────────────────────────────────────────────────────────────",
        "   Eastern                                                      Western",
        " ",
        "   Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
        "   Devils                        18    15    2    1     31      Avalanche                     19    16    2    1     33",
        "   Panthers                      19    14    3    2     30      Golden Knights                19    15    3    1     31",
        "   Hurricanes                    19    14    3    2     30      Stars                         20    14    4    2     30",
        "   Bruins                        18    13    4    1     27      Oilers                        20    14    4    2     30",
        "   Maple Leafs                   19    12    5    2     26      Jets                          19    13    5    1     27",
        "   Rangers                       18    12    5    1     25      Kings                         19    12    6    1     25",
        "   Penguins                      19    11    6    2     24      Wild                          19    11    6    2     24",
        "   Lightning                     18    11    6    1     23      Kraken                        19    11    6    2     24",
        "   Canadiens                     18    10    5    3     23      Predators                     19    10    7    2     22",
        "   Capitals                      18    10    7    1     21      Canucks                       19    10    7    2     22",
        "   Senators                      18     9    7    2     20      Flames                        19     9    8    2     20",
        "   Islanders                     18     9    7    2     20      Blues                         19     8    8    3     19",
        "   Red Wings                     18     8    8    2     18      Ducks                         19     7   10    2     16",
        "   Flyers                        18     8    9    1     17      Blackhawks                    18     7   10    1     15",
        "   Sabres                        18     6   10    2     14      Sharks                        18     5   12    1     11",
        "   Blue Jackets                  18     5   11    2     12      Coyotes                       18     4   13    1      9",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
    ]);
}

#[test]
fn test_wildcard_view_full_render() {
    let standings_tab = StandingsTab;
    let standings = create_test_standings();
    let props = StandingsTabProps {
        standings: Arc::new(Some(standings)),
        document_stack: Vec::new(),
        focused: false,
        config: Arc::new(Config::default()),
        animation_frame: 0,
    };

    let element = standings_tab.view(&props, &StandingsTabState::default());
    let config = DisplayConfig::default();
    let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

    // Note: Tab bar has leading space, document content has left/right margins (centered with gap 4)
    assert_buffer(&buf, &[
        " Wildcard │ Division │ Conference │ League",
        "──────────┴──────────┴────────────┴─────────────────────────────────────────────────────────────────────────────────────",
        "   Atlantic                                                     Central",
        " ",
        "   Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
        "   Panthers                      19    14    3    2     30      Avalanche                     19    16    2    1     33",
        "   Bruins                        18    13    4    1     27      Stars                         20    14    4    2     30",
        "   Maple Leafs                   19    12    5    2     26      Jets                          19    13    5    1     27",
        " ",
        "   Metropolitan                                                 Pacific",
        " ",
        "   Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
        "   Devils                        18    15    2    1     31      Golden Knights                19    15    3    1     31",
        "   Hurricanes                    19    14    3    2     30      Oilers                        20    14    4    2     30",
        "   Rangers                       18    12    5    1     25      Kings                         19    12    6    1     25",
        " ",
        "   Wildcard                                                     Wildcard",
        " ",
        "   Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
        "   ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
        "   Penguins                      19    11    6    2     24      Wild                          19    11    6    2     24",
        "   Lightning                     18    11    6    1     23      Kraken                        19    11    6    2     24",
        "   Canadiens                     18    10    5    3     23      Predators                     19    10    7    2     22",
        "   Capitals                      18    10    7    1     21      Canucks                       19    10    7    2     22",
        "   Senators                      18     9    7    2     20      Flames                        19     9    8    2     20",
        "   Islanders                     18     9    7    2     20      Blues                         19     8    8    3     19",
        "   Red Wings                     18     8    8    2     18      Ducks                         19     7   10    2     16",
        "   Flyers                        18     8    9    1     17      Blackhawks                    18     7   10    1     15",
        "   Sabres                        18     6   10    2     14      Sharks                        18     5   12    1     11",
        "   Blue Jackets                  18     5   11    2     12      Coyotes                       18     4   13    1      9",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
        " ",
    ]);
}

/// Regression test: focusable_positions must be rebuilt when switching views
///
/// Bug: When switching between standings views (League, Division, Conference, Wildcard),
/// the focusable_positions were not being updated. This caused autoscroll to use
/// stale position data from the previous view, resulting in incorrect scrolling behavior.
///
/// For example, switching from Conference view (positions [5-20, 5-20] for two columns)
/// to League view (positions [2-33] for single column) would use the wrong positions.
#[test]
fn test_cycle_view_triggers_rebuild_focusable_metadata() {
    use crate::tui::action::Action;
    use crate::tui::component::{Component, Effect};

    let mut standings_tab = StandingsTab;
    let mut state = StandingsTabState {
        view: GroupBy::Wildcard,
        ..Default::default()
    };

    // Cycle view left should return Effect::Action to rebuild metadata
    let effect = standings_tab.update(StandingsTabMsg::CycleViewLeft, &mut state);

    // View should change
    assert_eq!(state.view, GroupBy::League);

    // Focus/scroll should be reset
    assert_eq!(state.doc_nav.focus_index, None);
    assert_eq!(state.doc_nav.scroll_offset, 0);

    // Effect should trigger RebuildStandingsFocusable
    match effect {
        Effect::Action(Action::RebuildStandingsFocusable) => {
            // Good - this is the fix for the regression
        }
        _ => panic!(
            "Expected RebuildStandingsFocusable action, got {:?}",
            effect
        ),
    }
}

#[test]
fn test_cycle_view_right_triggers_rebuild_focusable_metadata() {
    use crate::tui::action::Action;
    use crate::tui::component::{Component, Effect};

    let mut standings_tab = StandingsTab;
    let mut state = StandingsTabState {
        view: GroupBy::League,
        ..Default::default()
    };

    // Cycle view right should return Effect::Action to rebuild metadata
    let effect = standings_tab.update(StandingsTabMsg::CycleViewRight, &mut state);

    // View should change
    assert_eq!(state.view, GroupBy::Wildcard);

    // Focus/scroll should be reset
    assert_eq!(state.doc_nav.focus_index, None);
    assert_eq!(state.doc_nav.scroll_offset, 0);

    // Effect should trigger RebuildStandingsFocusable
    match effect {
        Effect::Action(Action::RebuildStandingsFocusable) => {
            // Good - this is the fix for the regression
        }
        _ => panic!(
            "Expected RebuildStandingsFocusable action, got {:?}",
            effect
        ),
    }
}

#[test]
fn test_activate_team_pushes_team_detail_document() {
    use crate::tui::action::Action;
    use crate::tui::component::{Component, Effect};
    use crate::tui::document::{FocusableElement, FocusableId, LinkTarget};
    use crate::tui::types::StackedDocument;

    let mut standings_tab = StandingsTab;
    let mut state = StandingsTabState {
        view: GroupBy::League,
        ..Default::default()
    };

    // Set link targets for teams (what table cells now use)
    state.doc_nav.focusables = vec![
        FocusableElement::at(0, 1, FocusableId::team_link("TOR")).with_link_target(
            LinkTarget::Push(StackedDocument::TeamDetail {
                abbrev: "TOR".to_string(),
                season: None,
            }),
        ),
        FocusableElement::at(1, 1, FocusableId::team_link("BOS")).with_link_target(
            LinkTarget::Push(StackedDocument::TeamDetail {
                abbrev: "BOS".to_string(),
                season: None,
            }),
        ),
        FocusableElement::at(2, 1, FocusableId::team_link("MTL")).with_link_target(
            LinkTarget::Push(StackedDocument::TeamDetail {
                abbrev: "MTL".to_string(),
                season: None,
            }),
        ),
    ];

    // Set focus to second team (BOS)
    state.doc_nav.focus_index = Some(1);

    // ActivateTeam should push TeamDetail document
    let effect = standings_tab.update(StandingsTabMsg::ActivateTeam, &mut state);

    match effect {
        Effect::Action(Action::PushDocument(StackedDocument::TeamDetail {
            abbrev, ..
        })) => {
            assert_eq!(abbrev, "BOS");
        }
        _ => panic!("Expected PushDocument(TeamDetail) action, got {:?}", effect),
    }
}

#[test]
fn test_activate_team_without_focus_does_nothing() {
    use crate::tui::component::{Component, Effect};
    use crate::tui::document::{FocusableElement, FocusableId, LinkTarget};
    use crate::tui::types::StackedDocument;

    let mut standings_tab = StandingsTab;
    let mut state = StandingsTabState {
        view: GroupBy::League,
        ..Default::default()
    };

    // Set link targets for teams
    state.doc_nav.focusables = vec![FocusableElement::at(0, 1, FocusableId::team_link("TOR"))
        .with_link_target(LinkTarget::Push(StackedDocument::TeamDetail {
            abbrev: "TOR".to_string(),
            season: None,
        }))];

    // No focus set
    state.doc_nav.focus_index = None;

    let effect = standings_tab.update(StandingsTabMsg::ActivateTeam, &mut state);

    assert!(matches!(effect, Effect::None));
}
