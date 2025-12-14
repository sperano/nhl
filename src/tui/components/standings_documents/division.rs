//! Division standings document - two columns with two divisions each

use std::collections::BTreeMap;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};
use crate::tui::helpers::StandingsSorting;

use super::{standings_columns, TableWidget};

/// Division standings document - two columns with two divisions each
///
/// Layout (with western_first=true):
/// ```text
/// +-------------------+-------------------+
/// |   Central         |   Atlantic        |
/// |   (8 teams)       |   (8 teams)       |
/// |                   |                   |
/// |   Pacific         |   Metropolitan    |
/// |   (8 teams)       |   (8 teams)       |
/// +-------------------+-------------------+
/// ```
///
/// Navigation order: Central -> Pacific -> Atlantic -> Metropolitan (down through
/// left column first, then down through right column).
pub struct DivisionStandingsDocument {
    standings: Arc<Vec<Standing>>,
    config: Config,
}

impl DivisionStandingsDocument {
    pub fn new(standings: Arc<Vec<Standing>>, config: Config) -> Self {
        Self { standings, config }
    }

    /// Group standings by division and return maps for each conference
    #[allow(clippy::type_complexity)]
    fn group_by_division(
        &self,
    ) -> (
        Vec<(&'static str, Vec<Standing>)>,
        Vec<(&'static str, Vec<Standing>)>,
    ) {
        let mut grouped: BTreeMap<String, Vec<Standing>> = BTreeMap::new();
        for standing in self.standings.as_ref() {
            grouped
                .entry(standing.division_name.clone())
                .or_default()
                .push(standing.clone());
        }

        // Sort teams within each division by points
        for teams in grouped.values_mut() {
            teams.sort_by_points_desc();
        }

        // Eastern divisions (alphabetically: Atlantic, Metropolitan)
        let atlantic = grouped.get("Atlantic").cloned().unwrap_or_default();
        let metropolitan = grouped.get("Metropolitan").cloned().unwrap_or_default();

        // Western divisions (alphabetically: Central, Pacific)
        let central = grouped.get("Central").cloned().unwrap_or_default();
        let pacific = grouped.get("Pacific").cloned().unwrap_or_default();

        // Return as ordered tuples for each conference
        let eastern = vec![("Atlantic", atlantic), ("Metropolitan", metropolitan)];
        let western = vec![("Central", central), ("Pacific", pacific)];

        (eastern, western)
    }

    /// Build a vertical group of division tables
    fn build_division_group(
        divisions: &[(&str, Vec<Standing>)],
        table_prefix: &str,
        focus: &FocusContext,
    ) -> DocumentElement {
        const MARGIN: u16 = 2;
        let mut children = Vec::new();

        for (idx, (div_name, teams)) in divisions.iter().enumerate() {
            let table_name = format!("{}_{}", table_prefix, div_name.to_lowercase());

            // Add section title for division name (indented to align with table content)
            children.push(DocumentElement::indented(
                DocumentElement::section_title(*div_name, false),
                MARGIN,
            ));

            let table = TableWidget::from_data(standings_columns(), teams.clone())
                .with_focused_row(focus.focused_table_row(&table_name));

            children.push(DocumentElement::table(table_name, table));

            // Add spacer between divisions (not after the last one)
            if idx < divisions.len() - 1 {
                children.push(DocumentElement::spacer(1));
            }
        }

        DocumentElement::group(children)
    }
}

impl Document for DivisionStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let (eastern, western) = self.group_by_division();

        // Determine column order based on western_first config
        let (left_divs, right_divs, left_prefix, right_prefix) =
            if self.config.display_standings_western_first {
                (western, eastern, "division_left", "division_right")
            } else {
                (eastern, western, "division_left", "division_right")
            };

        // Build left column (Group with 2 division tables)
        let left_group = Self::build_division_group(&left_divs, left_prefix, focus);

        // Build right column (Group with 2 division tables)
        let right_group = Self::build_division_group(&right_divs, right_prefix, focus);

        // Use Row element to place columns side-by-side with center alignment
        const GAP: u16 = 4;
        DocumentBuilder::new()
            .element(DocumentElement::row_center_with_gap(
                vec![left_group, right_group],
                GAP,
            ))
            .build()
    }

    fn title(&self) -> String {
        "Division Standings".to_string()
    }

    fn id(&self) -> String {
        "division_standings".to_string()
    }
}
