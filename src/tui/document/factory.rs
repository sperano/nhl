//! Single construction path for stacked-document content
//!
//! Before this module existed, each stacked document type's content struct
//! (`BoxscoreDocumentContent`, `TeamDetailDocumentContent`,
//! `PlayerDetailDocumentContent`) was built in two unrelated places: once by
//! the render path (inside each document widget's `render()`), and once by
//! the input path (`document::handlers`, to extract focus metadata). Two
//! construction sites per type meant two places that had to agree on which
//! view/width/lookup to use -- a sync-by-discipline seam rather than a
//! structural guarantee.
//!
//! `build_stacked_document` is now the only place these structs are
//! constructed in production. Both the render path
//! (`components::app::App::render_stacked_document`) and the input path
//! (`document::handle_stacked_document_key`) call it and get the same
//! document.

use std::sync::Arc;

use crate::tui::components::boxscore_document::{BoxscoreDocumentContent, TeamView};
use crate::tui::components::player_detail_document::PlayerDetailDocumentContent;
use crate::tui::components::team_detail_document::TeamDetailDocumentContent;
use crate::tui::state::DataState;
use crate::tui::types::StackedDocument;

use super::Document;

/// Build the content document for a stacked document from loaded app data.
///
/// Returns `None` while the variant's backing data hasn't arrived yet. The
/// render path shows a loading spinner in that case; the input path leaves
/// navigation metadata untouched rather than clearing it (matching what the
/// key-event handling has always done while data is loading).
pub fn build_stacked_document(
    doc: &StackedDocument,
    data: &DataState,
) -> Option<Arc<dyn Document>> {
    match doc {
        StackedDocument::Boxscore { game_id, .. } => {
            let boxscore = data.boxscores.get(game_id)?.clone();
            Some(Arc::new(BoxscoreDocumentContent::new(
                *game_id,
                boxscore,
                TeamView::Away,
            )))
        }
        StackedDocument::TeamDetail { abbrev, season } => {
            // None = "latest" still resolving; the render path shows the
            // loading spinner until the first fetch rewrites it.
            let season_id = (*season)?;
            let club_stats = data
                .team_roster_stats
                .get(&(abbrev.clone(), season_id))?
                .clone();
            let is_current_season = data
                .team_seasons
                .get(abbrev)
                .and_then(|ids| ids.last())
                .is_none_or(|latest| *latest == season_id);
            let standing = data.standings.as_ref().as_ref().and_then(|standings| {
                standings
                    .iter()
                    .find(|s| s.team_abbrev.default == *abbrev)
                    .cloned()
            });
            Some(Arc::new(TeamDetailDocumentContent::new(
                abbrev.clone(),
                standing,
                Some(club_stats),
                is_current_season,
            )))
        }
        StackedDocument::PlayerDetail { player_id, .. } => {
            let player_data = data.player_data.get(player_id)?.clone();
            Some(Arc::new(PlayerDetailDocumentContent::new(
                Some(player_data),
                *player_id,
            )))
        }
    }
}

#[cfg(test)]
#[path = "factory_tests.rs"]
mod tests;
