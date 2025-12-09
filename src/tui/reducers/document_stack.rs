use tracing::debug;

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document::get_stacked_document_handler;
use crate::tui::state::{AppState, DocumentStackEntry, LoadingKey};
use crate::tui::types::StackedDocument;

/// Handle all document stack management actions
///
/// Returns Ok((new_state, effect)) if the action was handled,
/// or Err(state) to pass ownership back to the caller.
#[allow(clippy::result_large_err)]
pub fn reduce_document_stack(
    state: AppState,
    action: &Action,
) -> Result<(AppState, Effect), AppState> {
    match action {
        Action::PushDocument(doc) => Ok(push_document(state, doc.clone())),
        Action::PopDocument => Ok(pop_document(state)),
        Action::StackedDocumentKey(key) => Ok(stacked_document_key(state, *key)),
        _ => Err(state),
    }
}

/// Handle key events routed to stacked documents
fn stacked_document_key(state: AppState, key: crossterm::event::KeyEvent) -> (AppState, Effect) {
    let mut new_state = state;
    let width = new_state.system.terminal_width;

    if let Some(entry) = new_state.navigation.document_stack.last_mut() {
        let handler = get_stacked_document_handler(&entry.document);
        let effect = handler.handle_key(key, &mut entry.nav, &new_state.data, width);
        return (new_state, effect);
    }

    (new_state, Effect::None)
}

fn push_document(state: AppState, doc: StackedDocument) -> (AppState, Effect) {
    debug!("DOCUMENT_STACK: Pushing document onto stack: {:?}", doc);
    let mut new_state = state;
    new_state
        .navigation
        .document_stack
        .push(DocumentStackEntry::new(doc.clone()));

    // Return fetch effect directly based on document type
    // This eliminates the need for runtime to compare old/new state
    let fetch_effect = match &doc {
        StackedDocument::Boxscore { game_id } => {
            // Check if we don't already have the data and aren't already loading
            if !new_state.data.boxscores.contains_key(game_id)
                && !new_state
                    .data
                    .loading
                    .contains(&LoadingKey::Boxscore(*game_id))
            {
                debug!(
                    "DOCUMENT_STACK: Requesting boxscore fetch for game_id={}",
                    game_id
                );
                Effect::FetchBoxscore(*game_id)
            } else {
                Effect::None
            }
        }
        StackedDocument::TeamDetail { abbrev } => {
            if !new_state.data.team_roster_stats.contains_key(abbrev)
                && !new_state
                    .data
                    .loading
                    .contains(&LoadingKey::TeamRosterStats(abbrev.clone()))
            {
                debug!(
                    "DOCUMENT_STACK: Requesting team roster stats fetch for team={}",
                    abbrev
                );
                Effect::FetchTeamRosterStats(abbrev.clone())
            } else {
                Effect::None
            }
        }
        StackedDocument::PlayerDetail { player_id } => {
            if !new_state.data.player_data.contains_key(player_id)
                && !new_state
                    .data
                    .loading
                    .contains(&LoadingKey::PlayerStats(*player_id))
            {
                debug!(
                    "DOCUMENT_STACK: Requesting player stats fetch for player_id={}",
                    player_id
                );
                Effect::FetchPlayerStats(*player_id)
            } else {
                Effect::None
            }
        }
    };

    (new_state, fetch_effect)
}

fn pop_document(state: AppState) -> (AppState, Effect) {
    debug!("DOCUMENT_STACK: Popping document from stack");
    let mut new_state = state;

    if let Some(doc_entry) = new_state.navigation.document_stack.pop() {
        // Clear the loading state for the document being popped
        match &doc_entry.document {
            StackedDocument::Boxscore { game_id } => {
                new_state
                    .data
                    .loading
                    .remove(&LoadingKey::Boxscore(*game_id));
            }
            StackedDocument::TeamDetail { abbrev } => {
                new_state
                    .data
                    .loading
                    .remove(&LoadingKey::TeamRosterStats(abbrev.clone()));
            }
            StackedDocument::PlayerDetail { player_id } => {
                new_state
                    .data
                    .loading
                    .remove(&LoadingKey::PlayerStats(*player_id));
            }
        }

        debug!(
            "DOCUMENT_STACK: Popped document, {} remaining",
            new_state.navigation.document_stack.len()
        );
    }

    // If no documents left, return focus to content
    if new_state.navigation.document_stack.is_empty() {
        debug!("DOCUMENT_STACK: Document stack empty, returning focus to content");
    }

    (new_state, Effect::None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::document_nav::DocumentNavState;

    fn make_entry(document: StackedDocument, focus_index: Option<usize>) -> DocumentStackEntry {
        DocumentStackEntry {
            document,
            nav: DocumentNavState {
                focus_index,
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_push_document() {
        let state = AppState::default();
        let panel = StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
        };

        let (new_state, effect) = push_document(state, panel.clone());

        assert_eq!(new_state.navigation.document_stack.len(), 1);
        assert_eq!(
            new_state.navigation.document_stack[0].nav.focus_index,
            Some(0)
        );
        // Should return fetch effect since we don't have the data
        assert!(matches!(effect, Effect::FetchTeamRosterStats(ref abbrev) if abbrev == "BOS"));
    }

    #[test]
    fn test_push_document_boxscore_returns_fetch_effect() {
        let state = AppState::default();
        let game_id = 2024020001;
        let panel = StackedDocument::Boxscore { game_id };

        let (new_state, effect) = push_document(state, panel);

        assert_eq!(new_state.navigation.document_stack.len(), 1);
        assert!(matches!(effect, Effect::FetchBoxscore(id) if id == game_id));
    }

    #[test]
    fn test_push_document_no_fetch_if_already_loading() {
        let mut state = AppState::default();
        let game_id = 2024020001;

        // Mark as already loading
        state.data.loading.insert(LoadingKey::Boxscore(game_id));

        let panel = StackedDocument::Boxscore { game_id };
        let (_new_state, effect) = push_document(state, panel);

        // Should NOT return fetch effect since we're already loading
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_pop_document_clears_loading_state() {
        let mut state = AppState::default();
        let game_id = 2024020001;

        // Push a boxscore panel and add loading state
        state
            .navigation
            .document_stack
            .push(make_entry(StackedDocument::Boxscore { game_id }, None));
        state.data.loading.insert(LoadingKey::Boxscore(game_id));

        let (new_state, _) = pop_document(state);

        assert!(new_state.navigation.document_stack.is_empty());
        assert!(!new_state
            .data
            .loading
            .contains(&LoadingKey::Boxscore(game_id)));
    }
}
