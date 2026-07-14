//! Wildcard standings document - two columns showing playoff picture

use std::borrow::Cow;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};
use crate::tui::helpers::StandingsSorting;

use super::{
    build_sections_group, division_standings, order_conferences, two_column_row, StandingsSection,
};

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
    config: Arc<Config>,
}

impl WildcardStandingsDocument {
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

    /// Group standings by division and return sorted teams for each division
    fn group_by_division(&self) -> (Vec<Standing>, Vec<Standing>, Vec<Standing>, Vec<Standing>) {
        division_standings(self.standings.as_ref())
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
        let top3 = |teams: &[Standing]| teams.iter().take(3).cloned().collect::<Vec<_>>();

        // Wildcard section: remaining teams from both divisions, re-sorted by
        // points since they compete for the same slots.
        let mut wildcard_teams: Vec<_> = div1_teams
            .iter()
            .skip(3)
            .chain(div2_teams.iter().skip(3))
            .cloned()
            .collect();
        wildcard_teams.sort_by_points_desc();

        build_sections_group(
            vec![
                StandingsSection {
                    title: div1_name,
                    table_name: format!("{}_{}", table_prefix, div1_name.to_lowercase()),
                    teams: top3(div1_teams),
                },
                StandingsSection {
                    title: div2_name,
                    table_name: format!("{}_{}", table_prefix, div2_name.to_lowercase()),
                    teams: top3(div2_teams),
                },
                StandingsSection {
                    title: "Wildcard",
                    table_name: format!("{}_wildcard", table_prefix),
                    teams: wildcard_teams,
                },
            ],
            focus,
        )
    }
}

impl Document for WildcardStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let (atlantic, metropolitan, central, pacific) = self.group_by_division();

        // Each conference column is (div1_name, div1_teams, div2_name, div2_teams)
        let ((d1_left, t1_left, d2_left, t2_left), (d1_right, t1_right, d2_right, t2_right)) =
            order_conferences(
                ("Atlantic", &atlantic, "Metropolitan", &metropolitan),
                ("Central", &central, "Pacific", &pacific),
                self.config.display_standings_western_first,
            );

        DocumentBuilder::new()
            .element(two_column_row(
                Self::build_wildcard_group(
                    d1_left,
                    t1_left,
                    d2_left,
                    t2_left,
                    "wildcard_left",
                    focus,
                ),
                Self::build_wildcard_group(
                    d1_right,
                    t1_right,
                    d2_right,
                    t2_right,
                    "wildcard_right",
                    focus,
                ),
            ))
            .build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("Wildcard Standings")
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Borrowed("wildcard_standings")
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
    /// shape `build_wildcard_group` produces for each section header.
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
    fn test_group_by_division_sorts_each_division_by_points_desc() {
        let doc = WildcardStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let (atlantic, metropolitan, central, pacific) = doc.group_by_division();

        assert_eq!(atlantic.len(), 8);
        assert_eq!(metropolitan.len(), 8);
        assert_eq!(central.len(), 8);
        assert_eq!(pacific.len(), 8);

        assert_eq!(atlantic[0].team_common_name.default, "Panthers"); // 30 pts, division leader
        assert_eq!(atlantic.last().unwrap().team_common_name.default, "Sabres"); // 14 pts, lowest
        assert_eq!(central[0].team_common_name.default, "Avalanche"); // 33 pts, best in fixture
    }

    #[test]
    fn test_group_by_division_missing_divisions_are_empty() {
        let standings = vec![create_division_team(
            "Panthers", "FLA", "Atlantic", "Eastern", 14, 3, 2, 30,
        )];
        let doc = WildcardStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));
        let (atlantic, metropolitan, central, pacific) = doc.group_by_division();

        assert_eq!(atlantic.len(), 1);
        assert!(metropolitan.is_empty());
        assert!(central.is_empty());
        assert!(pacific.is_empty());
    }

    #[test]
    fn test_build_wildcard_group_splits_top3_and_combines_remainder_by_points() {
        let doc = WildcardStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let (atlantic, metropolitan, _, _) = doc.group_by_division();

        let group = WildcardStandingsDocument::build_wildcard_group(
            "Atlantic",
            &atlantic,
            "Metropolitan",
            &metropolitan,
            "test",
            &FocusContext::default(),
        );
        let children = group_children(&group);

        // [title, table, spacer] x 2 divisions + [title, table] wildcard = 8 elements
        assert_eq!(children.len(), 8);

        assert_eq!(section_title(&children[0]), "Atlantic");
        assert_eq!(
            table_team_names(&children[1]),
            vec!["Panthers", "Bruins", "Maple Leafs"]
        );
        assert!(matches!(children[2], DocumentElement::Spacer { .. }));

        assert_eq!(section_title(&children[3]), "Metropolitan");
        assert_eq!(
            table_team_names(&children[4]),
            vec!["Devils", "Hurricanes", "Rangers"]
        );
        assert!(matches!(children[5], DocumentElement::Spacer { .. }));

        assert_eq!(section_title(&children[6]), "Wildcard");
        // Remaining 5 Atlantic + 5 Metropolitan teams, re-sorted by points desc.
        // Senators and Islanders tie at 20 pts; the stable sort keeps Senators
        // first because it appears earlier in the pre-sort (Atlantic-then-
        // Metropolitan) chain.
        assert_eq!(
            table_team_names(&children[7]),
            vec![
                "Penguins",
                "Lightning",
                "Canadiens",
                "Capitals",
                "Senators",
                "Islanders",
                "Red Wings",
                "Flyers",
                "Sabres",
                "Blue Jackets",
            ]
        );
    }

    #[test]
    fn test_build_wildcard_group_omits_wildcard_section_when_no_teams_remain() {
        // Exactly 3 teams per division: fully consumed by the top-3 cutoff, so no
        // wildcard remainder exists and the "Wildcard" section must not render.
        let div1 = vec![
            create_division_team("A1", "A1", "Atlantic", "Eastern", 10, 5, 1, 21),
            create_division_team("A2", "A2", "Atlantic", "Eastern", 9, 6, 1, 19),
            create_division_team("A3", "A3", "Atlantic", "Eastern", 8, 7, 1, 17),
        ];
        let div2 = vec![
            create_division_team("M1", "M1", "Metropolitan", "Eastern", 11, 4, 1, 23),
            create_division_team("M2", "M2", "Metropolitan", "Eastern", 10, 5, 1, 21),
        ];
        let group = WildcardStandingsDocument::build_wildcard_group(
            "Atlantic",
            &div1,
            "Metropolitan",
            &div2,
            "test",
            &FocusContext::default(),
        );
        let children = group_children(&group);

        // [title, table, spacer, title, table] -- no wildcard section, and no
        // trailing spacer after the last division (spacers only separate).
        assert_eq!(children.len(), 5);
        assert_eq!(section_title(&children[0]), "Atlantic");
        assert_eq!(table_team_names(&children[1]), vec!["A1", "A2", "A3"]);
        assert_eq!(section_title(&children[3]), "Metropolitan");
        assert_eq!(table_team_names(&children[4]), vec!["M1", "M2"]);
    }

    #[test]
    fn test_build_wildcard_group_omits_empty_division_section() {
        // An empty division (e.g. standings not yet loaded for it) must not render
        // a title/table -- only the populated division's top-3 and the wildcard
        // remainder (drawn entirely from that division) should appear.
        let div2 = vec![
            create_division_team("M1", "M1", "Metropolitan", "Eastern", 15, 2, 1, 31),
            create_division_team("M2", "M2", "Metropolitan", "Eastern", 12, 5, 1, 25),
            create_division_team("M3", "M3", "Metropolitan", "Eastern", 10, 7, 1, 21),
            create_division_team("M4", "M4", "Metropolitan", "Eastern", 5, 12, 1, 11),
        ];
        let group = WildcardStandingsDocument::build_wildcard_group(
            "Atlantic",
            &[],
            "Metropolitan",
            &div2,
            "test",
            &FocusContext::default(),
        );
        let children = group_children(&group);

        // [title, table, spacer] for Metropolitan + [title, table] wildcard = 5
        assert_eq!(children.len(), 5);
        assert_eq!(section_title(&children[0]), "Metropolitan");
        assert_eq!(table_team_names(&children[1]), vec!["M1", "M2", "M3"]);
        assert_eq!(section_title(&children[3]), "Wildcard");
        assert_eq!(table_team_names(&children[4]), vec!["M4"]);
    }

    #[test]
    fn test_build_wildcard_group_all_empty_produces_empty_group() {
        let group = WildcardStandingsDocument::build_wildcard_group(
            "Atlantic",
            &[],
            "Metropolitan",
            &[],
            "test",
            &FocusContext::default(),
        );
        assert!(group_children(&group).is_empty());
    }

    #[test]
    fn test_build_western_first_puts_central_pacific_on_the_left() {
        let standings = create_test_standings();

        let western_first = Arc::new(Config {
            display_standings_western_first: true,
            ..Default::default()
        });
        let doc = WildcardStandingsDocument::new(Arc::new(standings.clone()), western_first);
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
        let doc = WildcardStandingsDocument::new(Arc::new(standings), eastern_first);
        let elements = doc.build(&FocusContext::default());
        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert_eq!(section_title(&group_children(&children[0])[0]), "Atlantic");
        assert_eq!(section_title(&group_children(&children[1])[0]), "Central");
    }

    #[test]
    fn test_focusables_returns_one_per_team_across_both_columns() {
        let doc = WildcardStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        // 32 teams total, each rendered as a focusable TeamLink cell regardless
        // of which section (division top-3 or wildcard) it lands in.
        assert_eq!(doc.focusables(&FocusContext::default()).len(), 32);
    }

    #[test]
    fn test_document_metadata() {
        let doc = WildcardStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.title(), "Wildcard Standings");
        assert_eq!(doc.id(), "wildcard_standings");
    }

    #[test]
    fn test_render_full_small_fixture() {
        // A compact 4-teams-per-division fixture (one populated conference) keeps
        // the rendered buffer small enough to assert against exactly, while still
        // exercising the top-3 / wildcard split (cutoff after 3 of 4 teams).
        let standings = vec![
            create_division_team("Atl One", "AO", "Atlantic", "Eastern", 20, 5, 1, 41),
            create_division_team("Atl Two", "AT", "Atlantic", "Eastern", 15, 10, 1, 31),
            create_division_team("Atl Three", "AH", "Atlantic", "Eastern", 10, 15, 1, 21),
            create_division_team("Atl Four", "AF", "Atlantic", "Eastern", 5, 20, 1, 11),
            create_division_team("Met One", "MO", "Metropolitan", "Eastern", 18, 7, 1, 37),
            create_division_team("Met Two", "MT", "Metropolitan", "Eastern", 13, 12, 1, 27),
            create_division_team("Met Three", "MH", "Metropolitan", "Eastern", 8, 17, 1, 17),
            create_division_team("Met Four", "MF", "Metropolitan", "Eastern", 3, 22, 1, 7),
        ];
        let doc = WildcardStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));

        let display_config = crate::config::DisplayConfig::default();
        let ctx = crate::config::RenderContext::focused(&display_config);
        let (buf, height) = doc.render_full(60, &ctx, &FocusContext::default());

        assert_eq!(height, 22);
        crate::tui::testing::assert_buffer(
            &buf,
            &[
                "  Atlantic",
                "",
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Atl One                       26    20    5    1     41",
                "  Atl Two                       26    15   10    1     31",
                "  Atl Three                     26    10   15    1     21",
                "",
                "  Metropolitan",
                "",
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Met One                       26    18    7    1     37",
                "  Met Two                       26    13   12    1     27",
                "  Met Three                     26     8   17    1     17",
                "",
                "  Wildcard",
                "",
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Atl Four                      26     5   20    1     11",
                "  Met Four                      26     3   22    1      7",
            ],
        );
    }
}
