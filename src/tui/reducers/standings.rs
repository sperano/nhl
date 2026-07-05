use std::sync::Arc;

use crate::commands::standings::GroupBy;
use crate::tui::components::standings_tab::StandingsTabState;
use crate::tui::components::{
    ConferenceStandingsDocument, DivisionStandingsDocument, LeagueStandingsDocument,
    WildcardStandingsDocument,
};
use crate::tui::constants::STANDINGS_TAB_PATH;
use crate::tui::document::Document;
use crate::tui::state::AppState;

/// Rebuild focusable metadata for document-based views
///
/// Called from reducer when standings data changes or view changes.
/// Updates component state with focusable positions, IDs, and link targets
/// extracted from the current standings document.
pub fn rebuild_standings_focusable_metadata(
    state: &AppState,
    component_states: &mut crate::tui::component_store::ComponentStateStore,
) {
    if let Some(standings) = state.data.standings.as_ref().as_ref() {
        // Get current view from component state
        let view = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .map(|s| s.view)
            .unwrap_or(GroupBy::Wildcard);

        // Build document for current view and extract metadata
        let (positions, ids, row_positions, link_targets) = match view {
            GroupBy::Conference => {
                let doc = ConferenceStandingsDocument::new(
                    Arc::new(standings.clone()),
                    state.system.config.clone(),
                );
                (
                    doc.focusable_positions(),
                    doc.focusable_ids(),
                    doc.focusable_row_positions(),
                    doc.focusable_link_targets(),
                )
            }
            GroupBy::Division => {
                let doc = DivisionStandingsDocument::new(
                    Arc::new(standings.clone()),
                    state.system.config.clone(),
                );
                (
                    doc.focusable_positions(),
                    doc.focusable_ids(),
                    doc.focusable_row_positions(),
                    doc.focusable_link_targets(),
                )
            }
            GroupBy::League => {
                let doc = LeagueStandingsDocument::new(
                    Arc::new(standings.clone()),
                    state.system.config.clone(),
                );
                (
                    doc.focusable_positions(),
                    doc.focusable_ids(),
                    doc.focusable_row_positions(),
                    doc.focusable_link_targets(),
                )
            }
            GroupBy::Wildcard => {
                let doc = WildcardStandingsDocument::new(
                    Arc::new(standings.clone()),
                    state.system.config.clone(),
                );
                (
                    doc.focusable_positions(),
                    doc.focusable_ids(),
                    doc.focusable_row_positions(),
                    doc.focusable_link_targets(),
                )
            }
        };

        // Update component state with new metadata
        if let Some(standings_state) =
            component_states.get_mut::<StandingsTabState>(STANDINGS_TAB_PATH)
        {
            standings_state.doc_nav.focusable_positions = positions;
            standings_state.doc_nav.focusable_ids = ids;
            standings_state.doc_nav.focusable_row_positions = row_positions;
            standings_state.doc_nav.link_targets = link_targets;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::component_store::ComponentStateStore;
    use crate::tui::document::{FocusableId, LinkTarget, RowPosition};
    use crate::tui::testing::create_test_standings;
    use nhl_api::Standing;

    /// Metadata extracted from a document, used to compare the reducer's
    /// output against an independently-built document of the same view.
    struct ExpectedMetadata {
        positions: Vec<u16>,
        ids: Vec<FocusableId>,
        row_positions: Vec<Option<RowPosition>>,
        link_targets: Vec<Option<LinkTarget>>,
    }

    /// Build the expected focusable metadata directly from a document,
    /// bypassing the reducer, so tests can assert the reducer picked the
    /// correct document type for each `GroupBy` view.
    fn expected_metadata_for_view(view: GroupBy, standings: &[Standing]) -> ExpectedMetadata {
        let standings = Arc::new(standings.to_vec());
        let config = Config::default();

        macro_rules! metadata_from {
            ($doc:expr) => {{
                let doc = $doc;
                ExpectedMetadata {
                    positions: doc.focusable_positions(),
                    ids: doc.focusable_ids(),
                    row_positions: doc.focusable_row_positions(),
                    link_targets: doc.focusable_link_targets(),
                }
            }};
        }

        match view {
            GroupBy::Conference => {
                metadata_from!(ConferenceStandingsDocument::new(standings, config))
            }
            GroupBy::Division => {
                metadata_from!(DivisionStandingsDocument::new(standings, config))
            }
            GroupBy::League => metadata_from!(LeagueStandingsDocument::new(standings, config)),
            GroupBy::Wildcard => metadata_from!(WildcardStandingsDocument::new(standings, config)),
        }
    }

    /// Build an `AppState` with standings data set to `standings`.
    fn state_with_standings(standings: Option<Vec<Standing>>) -> AppState {
        let mut state = AppState::default();
        state.data.standings = Arc::new(standings);
        state
    }

    #[test]
    fn test_rebuild_is_noop_when_no_standings_data() {
        let state = state_with_standings(None);
        let mut component_states = ComponentStateStore::new();

        // Seed the standings tab state with sentinel values to detect
        // any unwanted mutation.
        let sentinel = StandingsTabState {
            view: GroupBy::League,
            doc_nav: crate::tui::document_nav::DocumentNavState {
                focusable_positions: vec![7],
                focusable_ids: vec![FocusableId::Link("sentinel".to_string())],
                ..Default::default()
            },
        };
        component_states.insert(STANDINGS_TAB_PATH.to_string(), sentinel);

        rebuild_standings_focusable_metadata(&state, &mut component_states);

        let standings_state = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .expect("component state should still be present");
        assert_eq!(standings_state.doc_nav.focusable_positions, vec![7]);
        assert_eq!(
            standings_state.doc_nav.focusable_ids,
            vec![FocusableId::Link("sentinel".to_string())]
        );
    }

    #[test]
    fn test_rebuild_does_not_panic_when_component_state_missing() {
        let state = state_with_standings(Some(create_test_standings()));
        let mut component_states = ComponentStateStore::new();

        // No StandingsTabState has been inserted for STANDINGS_TAB_PATH.
        rebuild_standings_focusable_metadata(&state, &mut component_states);

        assert!(
            component_states
                .get::<StandingsTabState>(STANDINGS_TAB_PATH)
                .is_none(),
            "reducer must not create component state that was never initialized"
        );
    }

    #[test]
    fn test_rebuild_defaults_to_wildcard_view_when_component_state_missing() {
        // Even though there is no component state to write into, the function
        // must not panic while computing metadata for the default view
        // (Wildcard) used when no StandingsTabState is found.
        let state = state_with_standings(Some(create_test_standings()));
        let mut component_states = ComponentStateStore::new();

        rebuild_standings_focusable_metadata(&state, &mut component_states);
        // Reaching this point without panicking is the assertion; also
        // confirm the store remains untouched.
        assert!(component_states.is_empty());
    }

    #[test]
    fn test_rebuild_populates_metadata_matching_the_selected_view() {
        let standings = create_test_standings();

        for view in [
            GroupBy::Conference,
            GroupBy::Division,
            GroupBy::League,
            GroupBy::Wildcard,
        ] {
            let state = state_with_standings(Some(standings.clone()));
            let mut component_states = ComponentStateStore::new();
            component_states.insert(
                STANDINGS_TAB_PATH.to_string(),
                StandingsTabState {
                    view,
                    ..Default::default()
                },
            );

            rebuild_standings_focusable_metadata(&state, &mut component_states);

            let standings_state = component_states
                .get::<StandingsTabState>(STANDINGS_TAB_PATH)
                .unwrap_or_else(|| panic!("missing component state for view {view:?}"));
            let expected = expected_metadata_for_view(view, &standings);

            assert_eq!(
                standings_state.doc_nav.focusable_positions, expected.positions,
                "focusable_positions mismatch for view {view:?}"
            );
            assert_eq!(
                standings_state.doc_nav.focusable_ids, expected.ids,
                "focusable_ids mismatch for view {view:?}"
            );
            assert_eq!(
                standings_state.doc_nav.focusable_row_positions, expected.row_positions,
                "focusable_row_positions mismatch for view {view:?}"
            );
            assert_eq!(
                standings_state.doc_nav.link_targets, expected.link_targets,
                "focusable_link_targets mismatch for view {view:?}"
            );
            // Sanity check: a real 32-team standings list should produce
            // at least one focusable element in every view.
            assert!(
                !standings_state.doc_nav.focusable_ids.is_empty(),
                "expected at least one focusable id for view {view:?}"
            );
        }
    }

    #[test]
    fn test_rebuild_preserves_the_view_field() {
        let state = state_with_standings(Some(create_test_standings()));
        let mut component_states = ComponentStateStore::new();
        component_states.insert(
            STANDINGS_TAB_PATH.to_string(),
            StandingsTabState {
                view: GroupBy::Division,
                ..Default::default()
            },
        );

        rebuild_standings_focusable_metadata(&state, &mut component_states);

        let standings_state = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap();
        assert_eq!(standings_state.view, GroupBy::Division);
    }

    #[test]
    fn test_rebuild_with_empty_standings_produces_no_focusable_elements() {
        let state = state_with_standings(Some(Vec::new()));
        let mut component_states = ComponentStateStore::new();
        component_states.insert(
            STANDINGS_TAB_PATH.to_string(),
            StandingsTabState {
                view: GroupBy::Wildcard,
                ..Default::default()
            },
        );

        rebuild_standings_focusable_metadata(&state, &mut component_states);

        let standings_state = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap();
        assert!(standings_state.doc_nav.focusable_positions.is_empty());
        assert!(standings_state.doc_nav.focusable_ids.is_empty());
        assert!(standings_state.doc_nav.focusable_row_positions.is_empty());
        assert!(standings_state.doc_nav.link_targets.is_empty());
    }

    #[test]
    fn test_rebuild_is_idempotent_across_repeated_calls() {
        let state = state_with_standings(Some(create_test_standings()));
        let mut component_states = ComponentStateStore::new();
        component_states.insert(
            STANDINGS_TAB_PATH.to_string(),
            StandingsTabState {
                view: GroupBy::League,
                ..Default::default()
            },
        );

        rebuild_standings_focusable_metadata(&state, &mut component_states);
        let first_ids = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap()
            .doc_nav
            .focusable_ids
            .clone();

        // Calling again should produce identical metadata, not append/duplicate it.
        rebuild_standings_focusable_metadata(&state, &mut component_states);
        let second_ids = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap()
            .doc_nav
            .focusable_ids
            .clone();

        assert_eq!(first_ids, second_ids);
    }

    #[test]
    fn test_rebuild_overwrites_stale_metadata_from_previous_view() {
        // Regression-style test: switching views must replace, not merge,
        // previously computed focusable metadata (see standings_tab.rs
        // test_cycle_view_triggers_rebuild_focusable_metadata for the
        // component-level counterpart of this bug).
        let standings = create_test_standings();
        let state = state_with_standings(Some(standings.clone()));
        let mut component_states = ComponentStateStore::new();
        component_states.insert(
            STANDINGS_TAB_PATH.to_string(),
            StandingsTabState {
                view: GroupBy::Conference,
                ..Default::default()
            },
        );
        rebuild_standings_focusable_metadata(&state, &mut component_states);
        let conference_positions = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap()
            .doc_nav
            .focusable_positions
            .clone();

        // Switch view and rebuild again.
        component_states
            .get_mut::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap()
            .view = GroupBy::League;
        rebuild_standings_focusable_metadata(&state, &mut component_states);
        let league_positions = component_states
            .get::<StandingsTabState>(STANDINGS_TAB_PATH)
            .unwrap()
            .doc_nav
            .focusable_positions
            .clone();

        let expected_league = expected_metadata_for_view(GroupBy::League, &standings);
        assert_eq!(league_positions, expected_league.positions);
        // Conference view uses a two-column layout, so its focusable
        // positions should differ from the single-column League layout.
        assert_ne!(conference_positions, league_positions);
    }
}
