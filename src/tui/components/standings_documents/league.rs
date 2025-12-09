//! League standings document - single table with all teams sorted by points

use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};

use super::{standings_columns, TableWidget};

/// League standings document - single table with all teams sorted by points
pub struct LeagueStandingsDocument {
    standings: Arc<Vec<Standing>>,
    #[allow(dead_code)] // Will be used for other standings views
    config: Config,
}

impl LeagueStandingsDocument {
    pub fn new(standings: Arc<Vec<Standing>>, config: Config) -> Self {
        Self { standings, config }
    }
}

impl Document for LeagueStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let focused_row = focus.focused_table_row("league_standings");

        let table = TableWidget::from_data(standings_columns(), self.standings.as_ref().clone())
            .with_focused_row(focused_row);

        DocumentBuilder::new()
            .table("league_standings", table)
            .build()
    }

    fn title(&self) -> String {
        "League Standings".to_string()
    }

    fn id(&self) -> String {
        "league_standings".to_string()
    }
}
