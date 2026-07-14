//! Conference standings document - two tables side-by-side in a Row element

use std::borrow::Cow;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};

use super::{
    build_sections_group, group_standings_by, order_conferences, two_column_row, StandingsSection,
};

/// Conference standings document - two tables side-by-side in a Row element
pub struct ConferenceStandingsDocument {
    standings: Arc<Vec<Standing>>,
    config: Arc<Config>,
}

impl ConferenceStandingsDocument {
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

    /// Group standings by conference and return (Eastern, Western) sorted by points
    fn group_by_conference(&self) -> (Vec<Standing>, Vec<Standing>) {
        // Standing::conference_name is optional in the API; unset values fall
        // into an "Unknown" bucket rather than being silently dropped.
        let mut grouped = group_standings_by(self.standings.as_ref(), |s| {
            s.conference_name
                .clone()
                .unwrap_or_else(|| "Unknown".to_string())
        });

        let eastern = grouped.remove("Eastern").unwrap_or_default();
        let western = grouped.remove("Western").unwrap_or_default();

        (eastern, western)
    }
}

impl Document for ConferenceStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let (eastern, western) = self.group_by_conference();

        let ((left_header, left_teams), (right_header, right_teams)) = order_conferences(
            ("Eastern", eastern),
            ("Western", western),
            self.config.display_standings_western_first,
        );

        let column = |header, table_name: &str, teams| {
            build_sections_group(
                vec![StandingsSection {
                    title: header,
                    table_name: table_name.to_string(),
                    teams,
                }],
                focus,
            )
        };

        DocumentBuilder::new()
            .element(two_column_row(
                column(left_header, "conference_left", left_teams),
                column(right_header, "conference_right", right_teams),
            ))
            .build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("Conference Standings")
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Borrowed("conference_standings")
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

    fn section_title(elem: &DocumentElement) -> &str {
        match elem {
            DocumentElement::Indented { element, .. } => match element.as_ref() {
                DocumentElement::SectionTitle { content, .. } => content,
                other => panic!("Expected SectionTitle inside Indented, got {other:?}"),
            },
            other => panic!("Expected Indented element, got {other:?}"),
        }
    }

    #[test]
    fn test_group_by_conference_sorts_each_conference_by_points_desc() {
        let doc = ConferenceStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let (eastern, western) = doc.group_by_conference();

        assert_eq!(eastern.len(), 16); // Atlantic + Metropolitan
        assert_eq!(western.len(), 16); // Central + Pacific

        // League-wide top scorer for the fixture is Avalanche (Western, 33 pts).
        assert_eq!(western[0].team_common_name.default, "Avalanche");
        // Eastern's top scorer is Devils (Metropolitan, 31 pts) -- higher than
        // Atlantic's own leader (Panthers, 30 pts) since conference sorting spans
        // both divisions.
        assert_eq!(eastern[0].team_common_name.default, "Devils");
        assert_eq!(eastern[1].team_common_name.default, "Panthers");
    }

    #[test]
    fn test_group_by_conference_missing_conference_is_empty() {
        let standings = vec![create_division_team(
            "Panthers", "FLA", "Atlantic", "Eastern", 14, 3, 2, 30,
        )];
        let doc =
            ConferenceStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));
        let (eastern, western) = doc.group_by_conference();

        assert_eq!(eastern.len(), 1);
        assert!(western.is_empty());
    }

    #[test]
    fn test_group_by_conference_defaults_missing_conference_name_to_unknown() {
        // Standing::conference_name is optional in the API; unset values fall back
        // to the "Unknown" bucket rather than panicking or being silently dropped.
        let mut standing =
            create_division_team("Panthers", "FLA", "Atlantic", "Eastern", 14, 3, 2, 30);
        standing.conference_name = None;
        let doc =
            ConferenceStandingsDocument::new(Arc::new(vec![standing]), Arc::new(Config::default()));
        let (eastern, western) = doc.group_by_conference();

        assert!(eastern.is_empty());
        assert!(western.is_empty());
    }

    #[test]
    fn test_build_respects_western_first_config_team_order() {
        let standings = create_test_standings();

        let eastern_first = Arc::new(Config {
            display_standings_western_first: false,
            ..Default::default()
        });
        let doc = ConferenceStandingsDocument::new(standings.clone().into(), eastern_first);
        let elements = doc.build(&FocusContext::default());
        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert_eq!(section_title(&group_children(&children[0])[0]), "Eastern");
        assert_eq!(
            table_team_names(&group_children(&children[0])[1])[0],
            "Devils" // Eastern conference leader by points
        );
        assert_eq!(section_title(&group_children(&children[1])[0]), "Western");
        assert_eq!(
            table_team_names(&group_children(&children[1])[1])[0],
            "Avalanche" // Western conference leader by points
        );

        let western_first = Arc::new(Config {
            display_standings_western_first: true,
            ..Default::default()
        });
        let doc = ConferenceStandingsDocument::new(standings.into(), western_first);
        let elements = doc.build(&FocusContext::default());
        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert_eq!(section_title(&group_children(&children[0])[0]), "Western");
        assert_eq!(section_title(&group_children(&children[1])[0]), "Eastern");
    }

    #[test]
    fn test_document_metadata() {
        let doc = ConferenceStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.title(), "Conference Standings");
        assert_eq!(doc.id(), "conference_standings");
    }

    #[test]
    fn test_focusables_returns_one_per_team_across_both_columns() {
        let doc = ConferenceStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.focusables(&FocusContext::default()).len(), 32);
    }

    #[test]
    fn test_render_full_small_fixture() {
        let standings = vec![
            create_division_team("Atl One", "AO", "Atlantic", "Eastern", 20, 5, 1, 41),
            create_division_team("Met One", "MO", "Metropolitan", "Eastern", 18, 7, 1, 37),
        ];
        let doc =
            ConferenceStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));

        let display_config = crate::config::DisplayConfig::default();
        let ctx = crate::config::RenderContext::focused(&display_config);
        let (buf, height) = doc.render_full(130, &ctx, &FocusContext::default());

        // The fixture has no Western teams, so the Western column is skipped
        // entirely (see test_build_skips_empty_conference_column below) and the
        // single remaining Eastern column fills the whole row width.
        assert_eq!(height, 6);
        crate::tui::testing::assert_buffer(
            &buf,
            &[
                "  Eastern",
                "",
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Atl One                       26    20    5    1     41",
                "  Met One                       26    18    7    1     37",
            ],
        );
    }

    #[test]
    fn test_build_skips_empty_conference_column() {
        // When a conference has zero teams (e.g. standings not yet loaded for it),
        // the Row must contain only the populated conference's Group -- not a
        // title-over-empty-table for the missing one.
        let standings = vec![create_division_team(
            "Panthers", "FLA", "Atlantic", "Eastern", 14, 3, 2, 30,
        )];
        let doc =
            ConferenceStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));
        let elements = doc.build(&FocusContext::default());

        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(section_title(&group_children(&children[0])[0]), "Eastern");
    }

    #[test]
    fn test_build_skips_both_columns_when_no_teams() {
        let doc =
            ConferenceStandingsDocument::new(Arc::new(Vec::new()), Arc::new(Config::default()));
        let elements = doc.build(&FocusContext::default());

        let DocumentElement::Row { children, .. } = &elements[0] else {
            panic!("Expected Row element");
        };
        assert!(children.is_empty());
    }
}
