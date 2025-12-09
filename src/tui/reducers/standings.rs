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
