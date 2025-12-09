//! Conference standings document - two tables side-by-side in a Row element

use std::collections::BTreeMap;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};
use crate::tui::helpers::StandingsSorting;

use super::{standings_columns, TableWidget};

/// Conference standings document - two tables side-by-side in a Row element
pub struct ConferenceStandingsDocument {
    standings: Arc<Vec<Standing>>,
    config: Config,
}

impl ConferenceStandingsDocument {
    pub fn new(standings: Arc<Vec<Standing>>, config: Config) -> Self {
        Self { standings, config }
    }

    /// Group standings by conference and return (Eastern, Western) sorted by points
    fn group_by_conference(&self) -> (Vec<Standing>, Vec<Standing>) {
        let mut grouped: BTreeMap<String, Vec<Standing>> = BTreeMap::new();
        for standing in self.standings.as_ref() {
            let conference = standing
                .conference_name
                .clone()
                .unwrap_or_else(|| "Unknown".to_string());
            grouped
                .entry(conference)
                .or_default()
                .push(standing.clone());
        }

        // Sort teams within each conference by points
        for teams in grouped.values_mut() {
            teams.sort_by_points_desc();
        }

        // Extract Eastern and Western (BTreeMap gives us alphabetically sorted keys)
        let eastern = grouped.get("Eastern").cloned().unwrap_or_default();
        let western = grouped.get("Western").cloned().unwrap_or_default();

        (eastern, western)
    }
}

impl Document for ConferenceStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        const LEFT_TABLE: &str = "conference_left";
        const RIGHT_TABLE: &str = "conference_right";

        let (eastern, western) = self.group_by_conference();

        // Determine column order based on western_first config
        let (left_teams, right_teams, left_header, right_header) =
            if self.config.display_standings_western_first {
                (western, eastern, "Western", "Eastern")
            } else {
                (eastern, western, "Eastern", "Western")
            };

        // Create left table
        let left_table = TableWidget::from_data(standings_columns(), left_teams)
            .with_focused_row(focus.focused_table_row(LEFT_TABLE));

        // Create right table
        let right_table = TableWidget::from_data(standings_columns(), right_teams)
            .with_focused_row(focus.focused_table_row(RIGHT_TABLE));

        // Use Row element to place tables side-by-side with section titles
        // Section titles are indented by 2 to align with table content (after selector space)
        const MARGIN: u16 = 2;
        DocumentBuilder::new()
            .row(vec![
                DocumentElement::group(vec![
                    DocumentElement::indented(
                        DocumentElement::section_title(left_header, false),
                        MARGIN,
                    ),
                    DocumentElement::table(LEFT_TABLE, left_table),
                ]),
                DocumentElement::group(vec![
                    DocumentElement::indented(
                        DocumentElement::section_title(right_header, false),
                        MARGIN,
                    ),
                    DocumentElement::table(RIGHT_TABLE, right_table),
                ]),
            ])
            .build()
    }

    fn title(&self) -> String {
        "Conference Standings".to_string()
    }

    fn id(&self) -> String {
        "conference_standings".to_string()
    }
}
