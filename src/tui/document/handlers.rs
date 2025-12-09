//! Handler implementations for stacked documents
//!
//! This module contains the concrete implementations of `StackedDocumentHandler`
//! for each stacked document type (Boxscore, TeamDetail, PlayerDetail).

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document_nav::DocumentNavState;
use crate::tui::state::DataState;
use crate::tui::types::StackedDocument;

use super::{Document, StackedDocumentHandler};

/// Handler for Boxscore documents
pub(super) struct BoxscoreDocumentHandler {
    pub(super) game_id: i64,
}

impl StackedDocumentHandler for BoxscoreDocumentHandler {
    fn activate(&self, nav: &DocumentNavState, data: &DataState) -> Effect {
        if let Some(idx) = nav.focus_index {
            if let Some(player_id) = self.get_player_id_at_index(idx, data) {
                return Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                    player_id,
                }));
            }
        }
        Effect::None
    }

    fn populate_focusable_metadata(
        &self,
        nav: &mut DocumentNavState,
        data: &DataState,
        width: u16,
    ) {
        use crate::tui::components::boxscore_document::{BoxscoreDocumentContent, TeamView};
        use crate::tui::document::FocusContext;

        if let Some(boxscore) = data.boxscores.get(&self.game_id) {
            let doc = BoxscoreDocumentContent::new(self.game_id, boxscore.clone(), TeamView::Away);
            // Build with width so layout (side-by-side vs stacked) is correct
            let focus = FocusContext::default().with_width(width);
            let elements = doc.build(&focus);

            // Extract metadata from built elements
            let mut focusable = Vec::new();
            let mut y_offset = 0u16;
            for elem in &elements {
                elem.collect_focusable(&mut focusable, y_offset);
                y_offset += elem.height();
            }

            nav.focusable_positions = focusable.iter().map(|f| f.y).collect();
            nav.focusable_heights = focusable.iter().map(|f| f.height).collect();
            nav.focusable_ids = focusable.iter().map(|f| f.id.clone()).collect();
            nav.link_targets = focusable.iter().map(|f| f.link_target.clone()).collect();
            nav.focusable_row_positions = focusable.iter().map(|f| f.row_position).collect();
        }
    }
}

impl BoxscoreDocumentHandler {
    /// Get the player ID at the given focus index
    pub(super) fn get_player_id_at_index(&self, index: usize, data: &DataState) -> Option<i64> {
        let boxscore = data.boxscores.get(&self.game_id)?;
        let away_stats = &boxscore.player_by_game_stats.away_team;
        let home_stats = &boxscore.player_by_game_stats.home_team;

        // Calculate section boundaries
        let away_forwards_count = away_stats.forwards.len();
        let away_defense_count = away_stats.defense.len();
        let away_goalies_count = away_stats.goalies.len();
        let away_total = away_forwards_count + away_defense_count + away_goalies_count;

        let home_forwards_count = home_stats.forwards.len();
        let home_defense_count = home_stats.defense.len();

        if index < away_forwards_count {
            away_stats.forwards.get(index).map(|p| p.player_id)
        } else if index < away_forwards_count + away_defense_count {
            let defense_idx = index - away_forwards_count;
            away_stats.defense.get(defense_idx).map(|p| p.player_id)
        } else if index < away_total {
            let goalie_idx = index - away_forwards_count - away_defense_count;
            away_stats.goalies.get(goalie_idx).map(|p| p.player_id)
        } else if index < away_total + home_forwards_count {
            let forward_idx = index - away_total;
            home_stats.forwards.get(forward_idx).map(|p| p.player_id)
        } else if index < away_total + home_forwards_count + home_defense_count {
            let defense_idx = index - away_total - home_forwards_count;
            home_stats.defense.get(defense_idx).map(|p| p.player_id)
        } else {
            let goalie_idx = index - away_total - home_forwards_count - home_defense_count;
            home_stats.goalies.get(goalie_idx).map(|p| p.player_id)
        }
    }
}

/// Handler for TeamDetail documents
pub(super) struct TeamDetailDocumentHandler {
    pub(super) abbrev: String,
}

impl StackedDocumentHandler for TeamDetailDocumentHandler {
    fn activate(&self, nav: &DocumentNavState, data: &DataState) -> Effect {
        use crate::tui::helpers::{ClubGoalieStatsSorting, ClubSkaterStatsSorting};

        let Some(idx) = nav.focus_index else {
            return Effect::None;
        };
        let Some(roster) = data.team_roster_stats.get(&self.abbrev) else {
            return Effect::None;
        };

        // Sort the same way as display
        let mut sorted_skaters = roster.skaters.clone();
        sorted_skaters.sort_by_points_desc();

        let mut sorted_goalies = roster.goalies.clone();
        sorted_goalies.sort_by_games_played_desc();

        let num_skaters = sorted_skaters.len();

        let player_id = if idx < num_skaters {
            sorted_skaters.get(idx).map(|p| p.player_id)
        } else {
            let goalie_idx = idx - num_skaters;
            sorted_goalies.get(goalie_idx).map(|g| g.player_id)
        };

        match player_id {
            Some(id) => Effect::Action(Action::PushDocument(StackedDocument::PlayerDetail {
                player_id: id,
            })),
            None => Effect::None,
        }
    }

    fn populate_focusable_metadata(
        &self,
        nav: &mut DocumentNavState,
        data: &DataState,
        _width: u16,
    ) {
        use crate::tui::components::team_detail_document::TeamDetailDocumentContent;

        let roster = data.team_roster_stats.get(&self.abbrev);
        let standing = data.standings.as_ref().as_ref().and_then(|standings| {
            standings
                .iter()
                .find(|s| s.team_abbrev.default == self.abbrev)
                .cloned()
        });

        let doc = TeamDetailDocumentContent::new(self.abbrev.clone(), standing, roster.cloned());
        nav.focusable_positions = doc.focusable_positions();
        nav.focusable_heights = doc.focusable_heights();
        nav.focusable_ids = doc.focusable_ids();
        nav.link_targets = doc.focusable_link_targets();
    }
}

/// Handler for PlayerDetail documents
pub(super) struct PlayerDetailDocumentHandler {
    pub(super) player_id: i64,
}

impl StackedDocumentHandler for PlayerDetailDocumentHandler {
    fn activate(&self, nav: &DocumentNavState, data: &DataState) -> Effect {
        use crate::tui::helpers::SeasonSorting;

        let Some(idx) = nav.focus_index else {
            return Effect::None;
        };
        let Some(player) = data.player_data.get(&self.player_id) else {
            return Effect::None;
        };
        let Some(seasons) = &player.season_totals else {
            return Effect::None;
        };

        // Filter and sort same as display
        let mut nhl_seasons: Vec<_> = seasons
            .iter()
            .filter(|s| s.game_type == nhl_api::GameType::RegularSeason && s.league_abbrev == "NHL")
            .collect();
        nhl_seasons.sort_by_season_desc();

        let Some(season) = nhl_seasons.get(idx) else {
            return Effect::None;
        };
        let Some(ref common_name) = season.team_common_name else {
            return Effect::None;
        };
        let Some(abbrev) = crate::team_abbrev::common_name_to_abbrev(&common_name.default) else {
            return Effect::None;
        };

        Effect::Action(Action::PushDocument(StackedDocument::TeamDetail {
            abbrev: abbrev.to_string(),
        }))
    }

    fn populate_focusable_metadata(
        &self,
        nav: &mut DocumentNavState,
        data: &DataState,
        _width: u16,
    ) {
        use crate::tui::components::player_detail_document::PlayerDetailDocumentContent;

        let player_data = data.player_data.get(&self.player_id).cloned();
        let doc = PlayerDetailDocumentContent::new(player_data, self.player_id);
        nav.focusable_positions = doc.focusable_positions();
        nav.focusable_heights = doc.focusable_heights();
        nav.focusable_ids = doc.focusable_ids();
        nav.link_targets = doc.focusable_link_targets();
    }
}
