use tracing::debug;

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document::handle_stacked_document_key;
use crate::tui::document_nav::DocumentNavState;
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
    // Season cycling for team detail is a stack-level concern (it rewrites
    // the top document's identity), so it is intercepted here rather than in
    // handle_stacked_document_key, which only sees an immutable document.
    match key.code {
        crossterm::event::KeyCode::Char('[') => return cycle_team_detail_season(state, false),
        crossterm::event::KeyCode::Char(']') => return cycle_team_detail_season(state, true),
        _ => {}
    }

    let mut new_state = state;
    let width = new_state.system.terminal_width;

    if let Some(entry) = new_state.navigation.document_stack.last_mut() {
        let effect = handle_stacked_document_key(
            &entry.document,
            key,
            &mut entry.nav,
            &new_state.data,
            width,
        );
        return (new_state, effect);
    }

    (new_state, Effect::None)
}

/// Requests a boxscore fetch for a just-pushed `Boxscore` document, unless the data is
/// already loaded or already in flight.
fn boxscore_fetch_effect(new_state: &mut AppState, game_id: i64) -> Effect {
    if !new_state.data.boxscores.contains_key(&game_id)
        && !new_state.data.loading.contains(&LoadingKey::Boxscore(game_id))
    {
        debug!(
            "DOCUMENT_STACK: Requesting boxscore fetch for game_id={}",
            game_id
        );
        new_state.data.loading.insert(LoadingKey::Boxscore(game_id));
        Effect::FetchBoxscore(game_id)
    } else {
        Effect::None
    }
}

/// Resolves the season for a just-pushed `TeamDetail` document, writes it back onto the
/// top-of-stack entry, and requests a team roster stats fetch unless the data is already
/// loaded or already in flight.
fn team_detail_fetch_effect(new_state: &mut AppState, abbrev: &str, season: Option<i32>) -> Effect {
    // Normalize "latest" locally when the seasons list is already known, so a
    // re-open hits the (abbrev, season) map instead of re-fetching through the
    // un-cached seasons endpoint.
    let season = season.or_else(|| {
        new_state
            .data
            .team_seasons
            .get(abbrev)
            .and_then(|ids| ids.last().copied())
    });
    if let Some(StackedDocument::TeamDetail { season: s, .. }) = new_state
        .navigation
        .document_stack
        .last_mut()
        .map(|entry| &mut entry.document)
    {
        *s = season;
    }

    let have_data = season.is_some_and(|s| {
        new_state
            .data
            .team_roster_stats
            .contains_key(&(abbrev.to_string(), s))
    });
    let key = LoadingKey::TeamRosterStats(abbrev.to_string(), season);
    if !have_data && !new_state.data.loading.contains(&key) {
        debug!(
            "DOCUMENT_STACK: Requesting team roster stats fetch for team={} season={:?}",
            abbrev, season
        );
        new_state.data.loading.insert(key);
        // When landing directly on a specific season (e.g. from a player's
        // past-season row) the team's season list may be unknown; fetch it
        // too so season cycling and the current-season flag work.
        let fetch_seasons = !new_state.data.team_seasons.contains_key(abbrev);
        Effect::FetchTeamRosterStats {
            abbrev: abbrev.to_string(),
            season,
            fetch_seasons,
        }
    } else {
        Effect::None
    }
}

/// Requests a player stats fetch for a just-pushed `PlayerDetail` document, unless the
/// data is already loaded or already in flight.
fn player_detail_fetch_effect(new_state: &mut AppState, player_id: i64) -> Effect {
    if !new_state.data.player_data.contains_key(&player_id)
        && !new_state
            .data
            .loading
            .contains(&LoadingKey::PlayerStats(player_id))
    {
        debug!(
            "DOCUMENT_STACK: Requesting player stats fetch for player_id={}",
            player_id
        );
        new_state
            .data
            .loading
            .insert(LoadingKey::PlayerStats(player_id));
        Effect::FetchPlayerStats(player_id)
    } else {
        Effect::None
    }
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
        StackedDocument::Boxscore { game_id, .. } => boxscore_fetch_effect(&mut new_state, *game_id),
        StackedDocument::TeamDetail { abbrev, season } => {
            team_detail_fetch_effect(&mut new_state, abbrev, *season)
        }
        StackedDocument::PlayerDetail { player_id, .. } => {
            player_detail_fetch_effect(&mut new_state, *player_id)
        }
    };

    (new_state, fetch_effect)
}

/// Resolves the season to cycle the top team-detail document to: returns
/// `(abbrev, current_season, new_season)`, or `None` if a no-op applies (top
/// document is not a team detail, its season hasn't resolved yet, the team's
/// season list is unknown, or the season is already at the requested end
/// (clamped, no wrap)).
fn resolve_season_cycle_target(state: &AppState, next: bool) -> Option<(String, i32, i32)> {
    let entry = state.navigation.document_stack.last()?;
    let StackedDocument::TeamDetail {
        abbrev,
        season: Some(current),
    } = &entry.document
    else {
        return None;
    };
    let (abbrev, current) = (abbrev.clone(), *current);

    let ids = state.data.team_seasons.get(&abbrev)?;
    let pos = ids.iter().position(|s| *s == current)?;
    let new_pos = if next {
        (pos + 1).min(ids.len() - 1)
    } else {
        pos.saturating_sub(1)
    };
    if new_pos == pos {
        return None;
    }

    Some((abbrev, current, ids[new_pos]))
}

/// Cycle the top team-detail document to the previous (`next == false`) or
/// next (`next == true`) season in the team's regular-season list.
///
/// No-ops (returning the state unchanged) when the top document is not a
/// team detail, its season hasn't resolved yet, the team's season list is
/// unknown, or the season is already at the requested end (clamped, no wrap).
fn cycle_team_detail_season(state: AppState, next: bool) -> (AppState, Effect) {
    let mut new_state = state;

    let Some((abbrev, current, new_season)) = resolve_season_cycle_target(&new_state, next) else {
        return (new_state, Effect::None);
    };

    debug!(
        "DOCUMENT_STACK: Cycling team detail season for {}: {} -> {}",
        abbrev, current, new_season
    );
    if let Some(entry) = new_state.navigation.document_stack.last_mut() {
        if let StackedDocument::TeamDetail { season, .. } = &mut entry.document {
            *season = Some(new_season);
        }
        // The roster changes entirely: reset focus and scroll, keep the
        // viewport height (it only changes on terminal resize). Focusables
        // re-sync on the next key or render.
        entry.nav = DocumentNavState {
            focus_index: Some(0),
            viewport_height: entry.nav.viewport_height,
            ..Default::default()
        };
    }

    let have_data = new_state
        .data
        .team_roster_stats
        .contains_key(&(abbrev.clone(), new_season));
    let key = LoadingKey::TeamRosterStats(abbrev.clone(), Some(new_season));
    if !have_data && !new_state.data.loading.contains(&key) {
        new_state.data.loading.insert(key);
        return (
            new_state,
            Effect::FetchTeamRosterStats {
                abbrev,
                season: Some(new_season),
                fetch_seasons: false,
            },
        );
    }

    (new_state, Effect::None)
}

fn pop_document(state: AppState) -> (AppState, Effect) {
    debug!("DOCUMENT_STACK: Popping document from stack");
    let mut new_state = state;

    if let Some(doc_entry) = new_state.navigation.document_stack.pop() {
        // Clear the loading state for the document being popped
        match &doc_entry.document {
            StackedDocument::Boxscore { game_id, .. } => {
                new_state
                    .data
                    .loading
                    .remove(&LoadingKey::Boxscore(*game_id));
            }
            StackedDocument::TeamDetail { abbrev, .. } => {
                // Remove every season's key for this team: cycling away from
                // a season with its fetch still in flight and then popping
                // must not strand a key (it would keep the animation hot).
                new_state
                    .data
                    .loading
                    .retain(|k| !matches!(k, LoadingKey::TeamRosterStats(a, _) if a == abbrev));
            }
            StackedDocument::PlayerDetail { player_id, .. } => {
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
#[path = "document_stack_tests.rs"]
mod tests;
