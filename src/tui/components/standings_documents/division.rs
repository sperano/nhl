//! Division standings document - two columns with two divisions each

use std::borrow::Cow;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};

use super::{
    build_sections_group, division_standings, order_conferences, two_column_row, StandingsSection,
};

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
    config: Arc<Config>,
}

impl DivisionStandingsDocument {
    /// `config` accepts anything convertible to `Arc<Config>`: an owned `Config`
    /// (allocates a fresh Arc, used by the reducer's occasional focusable-metadata
    /// rebuild) or an existing `Arc<Config>` (zero-cost, used by the per-frame
    /// render path).
    pub fn new(standings: Arc<Vec<Standing>>, config: impl Into<Arc<Config>>) -> Self {
        Self {
            standings,
            config: config.into(),
        }
    }

    /// Group standings by division as ordered (name, teams) pairs per conference:
    /// Eastern (Atlantic, Metropolitan) and Western (Central, Pacific).
    #[allow(clippy::type_complexity)]
    fn group_by_division(
        &self,
    ) -> (
        Vec<(&'static str, Vec<Standing>)>,
        Vec<(&'static str, Vec<Standing>)>,
    ) {
        let (atlantic, metropolitan, central, pacific) =
            division_standings(self.standings.as_ref());

        let eastern = vec![("Atlantic", atlantic), ("Metropolitan", metropolitan)];
        let western = vec![("Central", central), ("Pacific", pacific)];

        (eastern, western)
    }

    /// Build a vertical group of division tables; empty divisions (e.g.
    /// standings not yet loaded for one) are skipped by `build_sections_group`.
    fn build_division_group(
        divisions: &[(&str, Vec<Standing>)],
        table_prefix: &str,
        focus: &FocusContext,
    ) -> DocumentElement {
        let sections = divisions
            .iter()
            .map(|(div_name, teams)| StandingsSection {
                title: div_name,
                table_name: format!("{}_{}", table_prefix, div_name.to_lowercase()),
                teams: teams.clone(),
            })
            .collect();

        build_sections_group(sections, focus)
    }
}

impl Document for DivisionStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let (eastern, western) = self.group_by_division();

        let (left_divs, right_divs) = order_conferences(
            eastern,
            western,
            self.config.display_standings_western_first,
        );

        DocumentBuilder::new()
            .element(two_column_row(
                Self::build_division_group(&left_divs, "division_left", focus),
                Self::build_division_group(&right_divs, "division_right", focus),
            ))
            .build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("Division Standings")
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Borrowed("division_standings")
    }

    fn cache_token(&self) -> Option<Vec<usize>> {
        // Recreated every frame around persistent Arcs; build() output
        // depends on the standings and the config (western_first).
        Some(vec![
            Arc::as_ptr(&self.standings) as usize,
            Arc::as_ptr(&self.config) as usize,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::{create_division_team, create_test_standings};

    /// Extract the text of the `SectionTitle` wrapped by `Indented`, matching the
    /// shape `build_division_group` produces for each division header.
    fn section_title(elem: &DocumentElement) -> &str {
        match elem {
            DocumentElement::Indented { element, .. } => match element.as_ref() {
                DocumentElement::SectionTitle { content, .. } => content,
                other => panic!("Expected SectionTitle inside Indented, got {other:?}"),
            },
            other => panic!("Expected Indented element, got {other:?}"),
        }
    }

    /// Extract team display names (column 0) from a Table element, in row order.
    fn table_team_names(elem: &DocumentElement) -> Vec<String> {
        match elem {
            DocumentElement::Table { widget, .. } => (0..widget.row_count())
                .map(|row| {
                    widget
                        .get_cell_value(row, 0)
                        .unwrap()
                        .display_text()
                        .to_string()
                })
                .collect(),
            other => panic!("Expected Table element, got {other:?}"),
        }
    }

    fn group_children(elem: &DocumentElement) -> &[DocumentElement] {
        match elem {
            DocumentElement::Group { children, .. } => children,
            other => panic!("Expected Group element, got {other:?}"),
        }
    }

    #[test]
    fn test_group_by_division_returns_eastern_then_western_ordered_pairs() {
        let doc = DivisionStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let (eastern, western) = doc.group_by_division();

        assert_eq!(eastern.len(), 2);
        assert_eq!(eastern[0].0, "Atlantic");
        assert_eq!(eastern[0].1.len(), 8);
        assert_eq!(eastern[1].0, "Metropolitan");
        assert_eq!(eastern[1].1.len(), 8);

        assert_eq!(western.len(), 2);
        assert_eq!(western[0].0, "Central");
        assert_eq!(western[0].1.len(), 8);
        assert_eq!(western[1].0, "Pacific");
        assert_eq!(western[1].1.len(), 8);
    }

    #[test]
    fn test_group_by_division_sorts_teams_within_division_by_points_desc() {
        let doc = DivisionStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let (eastern, _) = doc.group_by_division();
        let (_, atlantic_teams) = &eastern[0];

        assert_eq!(atlantic_teams[0].team_common_name.default, "Panthers"); // 30 pts
        assert_eq!(
            atlantic_teams.last().unwrap().team_common_name.default,
            "Sabres" // 14 pts, lowest in the division
        );
    }

    #[test]
    fn test_group_by_division_missing_divisions_are_empty() {
        let standings = vec![create_division_team(
            "Panthers", "FLA", "Atlantic", "Eastern", 14, 3, 2, 30,
        )];
        let doc = DivisionStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));
        let (eastern, western) = doc.group_by_division();

        assert_eq!(eastern[0].1.len(), 1); // Atlantic
        assert!(eastern[1].1.is_empty()); // Metropolitan
        assert!(western[0].1.is_empty()); // Central
        assert!(western[1].1.is_empty()); // Pacific
    }

    #[test]
    fn test_build_division_group_lists_all_divisions_with_spacers_between() {
        let doc = DivisionStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let (eastern, _) = doc.group_by_division();

        let group = DivisionStandingsDocument::build_division_group(
            &eastern,
            "test",
            &FocusContext::default(),
        );
        let children = group_children(&group);

        // [title, table, spacer, title, table] -- no trailing spacer after the
        // last division.
        assert_eq!(children.len(), 5);
        assert_eq!(section_title(&children[0]), "Atlantic");
        assert_eq!(table_team_names(&children[1]).len(), 8);
        assert_eq!(table_team_names(&children[1]).first().unwrap(), "Panthers");
        assert!(matches!(children[2], DocumentElement::Spacer { .. }));
        assert_eq!(section_title(&children[3]), "Metropolitan");
        assert_eq!(
            table_team_names(&children[4]).first().unwrap(),
            "Devils" // 31 pts, Metropolitan leader
        );
    }

    #[test]
    fn test_build_division_group_single_division_has_no_spacer() {
        let divisions = vec![(
            "Atlantic",
            vec![create_division_team(
                "Panthers", "FLA", "Atlantic", "Eastern", 14, 3, 2, 30,
            )],
        )];
        let group = DivisionStandingsDocument::build_division_group(
            &divisions,
            "test",
            &FocusContext::default(),
        );
        let children = group_children(&group);

        assert_eq!(children.len(), 2); // title, table -- nothing to separate
        assert_eq!(section_title(&children[0]), "Atlantic");
        assert_eq!(table_team_names(&children[1]), vec!["Panthers"]);
    }

    #[test]
    fn test_build_division_group_skips_empty_division() {
        // Matching WildcardStandingsDocument's build_wildcard_group, an empty
        // division (e.g. standings not yet loaded for it) must not render a
        // title/table pair at all.
        let divisions = vec![("Atlantic", Vec::new())];
        let group = DivisionStandingsDocument::build_division_group(
            &divisions,
            "test",
            &FocusContext::default(),
        );
        assert!(group_children(&group).is_empty());
    }

    #[test]
    fn test_build_division_group_skips_only_the_empty_division_among_several() {
        // A mix of empty and populated divisions should render just the
        // populated one, with no spacer left dangling for the skipped entry.
        let divisions = vec![
            ("Atlantic", Vec::new()),
            (
                "Metropolitan",
                vec![create_division_team(
                    "Devils",
                    "NJD",
                    "Metropolitan",
                    "Eastern",
                    15,
                    2,
                    1,
                    31,
                )],
            ),
        ];
        let group = DivisionStandingsDocument::build_division_group(
            &divisions,
            "test",
            &FocusContext::default(),
        );
        let children = group_children(&group);

        assert_eq!(children.len(), 2); // title, table -- no spacer
        assert_eq!(section_title(&children[0]), "Metropolitan");
        assert_eq!(table_team_names(&children[1]), vec!["Devils"]);
    }

    #[test]
    fn test_build_western_first_puts_central_pacific_on_the_left() {
        let standings = create_test_standings();

        let western_first = Arc::new(Config {
            display_standings_western_first: true,
            ..Default::default()
        });
        let doc = DivisionStandingsDocument::new(Arc::new(standings.clone()), western_first);
        let elements = doc.build(&FocusContext::default());
        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert_eq!(section_title(&group_children(&children[0])[0]), "Central");
        assert_eq!(section_title(&group_children(&children[1])[0]), "Atlantic");

        let eastern_first = Arc::new(Config {
            display_standings_western_first: false,
            ..Default::default()
        });
        let doc = DivisionStandingsDocument::new(Arc::new(standings), eastern_first);
        let elements = doc.build(&FocusContext::default());
        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert_eq!(section_title(&group_children(&children[0])[0]), "Atlantic");
        assert_eq!(section_title(&group_children(&children[1])[0]), "Central");
    }

    #[test]
    fn test_focusables_returns_one_per_team_across_both_columns() {
        let doc = DivisionStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.focusables(&FocusContext::default()).len(), 32);
    }

    #[test]
    fn test_document_metadata() {
        let doc = DivisionStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.title(), "Division Standings");
        assert_eq!(doc.id(), "division_standings");
    }

    #[test]
    fn test_render_full_small_fixture() {
        let standings = vec![
            create_division_team("Atl One", "AO", "Atlantic", "Eastern", 20, 5, 1, 41),
            create_division_team("Atl Two", "AT", "Atlantic", "Eastern", 15, 10, 1, 31),
            create_division_team("Met One", "MO", "Metropolitan", "Eastern", 18, 7, 1, 37),
            create_division_team("Met Two", "MT", "Metropolitan", "Eastern", 13, 12, 1, 27),
        ];
        let doc = DivisionStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));

        let display_config = crate::config::DisplayConfig::default();
        let ctx = crate::config::RenderContext::focused(&display_config);
        let (buf, height) = doc.render_full(130, &ctx, &FocusContext::default());

        assert_eq!(height, 13);
        // The fixture has no Western teams, so the Central/Pacific column is
        // skipped entirely and the Eastern column fills the whole row width.
        crate::tui::testing::assert_buffer(
            &buf,
            &[
                "  Atlantic",
                "",
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Atl One                       26    20    5    1     41",
                "  Atl Two                       26    15   10    1     31",
                "",
                "  Metropolitan",
                "",
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Met One                       26    18    7    1     37",
                "  Met Two                       26    13   12    1     27",
            ],
        );
    }
}
