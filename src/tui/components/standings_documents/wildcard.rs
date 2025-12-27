//! Wildcard standings document - two columns showing playoff picture

use std::collections::BTreeMap;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};
use crate::tui::helpers::StandingsSorting;

use super::{standings_columns, TableWidget};

/// Wildcard standings document - two columns showing playoff picture
///
/// Layout (with western_first=true):
/// ```text
/// +-------------------+-------------------+
/// |   Central (top 3) |   Atlantic (top 3)|
/// |   Pacific (top 3) |   Metropolitan    |
/// |                   |   (top 3)         |
/// |   Wildcard        |   Wildcard        |
/// |   (remaining)     |   (remaining)     |
/// +-------------------+-------------------+
/// ```
///
/// Each conference column shows:
/// 1. Division 1 top 3 teams (guaranteed playoff spots)
/// 2. Division 2 top 3 teams (guaranteed playoff spots)
/// 3. Wildcard section with remaining teams sorted by points
pub struct WildcardStandingsDocument {
    standings: Arc<Vec<Standing>>,
    config: Config,
}

impl WildcardStandingsDocument {
    pub fn new(standings: Arc<Vec<Standing>>, config: Config) -> Self {
        Self { standings, config }
    }

    /// Group standings by division and return sorted teams for each division
    fn group_by_division(&self) -> (Vec<Standing>, Vec<Standing>, Vec<Standing>, Vec<Standing>) {
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

        let atlantic = grouped.get("Atlantic").cloned().unwrap_or_default();
        let metropolitan = grouped.get("Metropolitan").cloned().unwrap_or_default();
        let central = grouped.get("Central").cloned().unwrap_or_default();
        let pacific = grouped.get("Pacific").cloned().unwrap_or_default();

        (atlantic, metropolitan, central, pacific)
    }

    /// Build a wildcard conference column (div1 top 3 + div2 top 3 + wildcards)
    fn build_wildcard_group(
        div1_name: &str,
        div1_teams: &[Standing],
        div2_name: &str,
        div2_teams: &[Standing],
        table_prefix: &str,
        focus: &FocusContext,
    ) -> DocumentElement {
        const MARGIN: u16 = 2;
        let mut children = Vec::new();

        // Division 1 - top 3 teams
        let div1_top3: Vec<_> = div1_teams.iter().take(3).cloned().collect();
        if !div1_top3.is_empty() {
            let table_name = format!("{}_{}", table_prefix, div1_name.to_lowercase());
            children.push(DocumentElement::indented(
                DocumentElement::section_title(div1_name, false),
                MARGIN,
            ));
            let table = TableWidget::from_data(standings_columns(), div1_top3)
                .with_focused_row(focus.focused_table_row(&table_name));
            children.push(DocumentElement::table(table_name, table));
            children.push(DocumentElement::spacer(1));
        }

        // Division 2 - top 3 teams
        let div2_top3: Vec<_> = div2_teams.iter().take(3).cloned().collect();
        if !div2_top3.is_empty() {
            let table_name = format!("{}_{}", table_prefix, div2_name.to_lowercase());
            children.push(DocumentElement::indented(
                DocumentElement::section_title(div2_name, false),
                MARGIN,
            ));
            let table = TableWidget::from_data(standings_columns(), div2_top3)
                .with_focused_row(focus.focused_table_row(&table_name));
            children.push(DocumentElement::table(table_name, table));
            children.push(DocumentElement::spacer(1));
        }

        // Wildcard section - remaining teams from both divisions, sorted by points
        let div1_remaining: Vec<_> = div1_teams.iter().skip(3).cloned().collect();
        let div2_remaining: Vec<_> = div2_teams.iter().skip(3).cloned().collect();

        let mut wildcard_teams: Vec<_> = div1_remaining.into_iter().chain(div2_remaining).collect();
        wildcard_teams.sort_by_points_desc();

        if !wildcard_teams.is_empty() {
            let table_name = format!("{}_wildcard", table_prefix);
            children.push(DocumentElement::indented(
                DocumentElement::section_title("Wildcard", false),
                MARGIN,
            ));
            let table = TableWidget::from_data(standings_columns(), wildcard_teams)
                .with_focused_row(focus.focused_table_row(&table_name));
            children.push(DocumentElement::table(table_name, table));
        }

        DocumentElement::group(children)
    }
}

impl Document for WildcardStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let (atlantic, metropolitan, central, pacific) = self.group_by_division();

        // Determine column order based on western_first config
        let (left_group, right_group) = if self.config.display_standings_western_first {
            // Western left, Eastern right
            let western = Self::build_wildcard_group(
                "Central",
                &central,
                "Pacific",
                &pacific,
                "wildcard_left",
                focus,
            );
            let eastern = Self::build_wildcard_group(
                "Atlantic",
                &atlantic,
                "Metropolitan",
                &metropolitan,
                "wildcard_right",
                focus,
            );
            (western, eastern)
        } else {
            // Eastern left, Western right
            let eastern = Self::build_wildcard_group(
                "Atlantic",
                &atlantic,
                "Metropolitan",
                &metropolitan,
                "wildcard_left",
                focus,
            );
            let western = Self::build_wildcard_group(
                "Central",
                &central,
                "Pacific",
                &pacific,
                "wildcard_right",
                focus,
            );
            (eastern, western)
        };

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
        "Wildcard Standings".to_string()
    }

    fn id(&self) -> String {
        "wildcard_standings".to_string()
    }
}
