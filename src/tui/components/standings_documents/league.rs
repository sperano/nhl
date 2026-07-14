//! League standings document - single table with all teams sorted by points

use std::borrow::Cow;
use std::sync::Arc;

use nhl_api::Standing;

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext};
use crate::tui::helpers::StandingsSorting;

use super::{standings_columns, TableWidget};

/// League standings document - single table with all teams sorted by points
pub struct LeagueStandingsDocument {
    standings: Arc<Vec<Standing>>,
}

impl LeagueStandingsDocument {
    /// Unlike the Conference/Division/Wildcard documents, League standings show
    /// one flat table, so there is no east/west split to read out of `Config`.
    /// The parameter is kept (and simply dropped) so that `StandingsDocumentWidget`'s
    /// four constructors -- one per `GroupBy` view -- stay call-site symmetric.
    pub fn new(standings: Arc<Vec<Standing>>, _config: impl Into<Arc<Config>>) -> Self {
        Self { standings }
    }
}

impl Document for LeagueStandingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let focused_row = focus.focused_table_row("league_standings");

        let mut standings = self.standings.as_ref().clone();
        standings.sort_by_points_desc();

        let table =
            TableWidget::from_data(standings_columns(), standings).with_focused_row(focused_row);

        DocumentBuilder::new()
            .table("league_standings", table)
            .build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("League Standings")
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Borrowed("league_standings")
    }

    fn cache_token(&self) -> Option<Vec<usize>> {
        // This document is recreated every frame around the persistent
        // standings Arc; build() output depends only on its contents
        // (config is ignored -- see `new`).
        Some(vec![Arc::as_ptr(&self.standings) as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::{create_division_team, create_test_standings};

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

    #[test]
    fn test_build_produces_single_table_with_all_teams() {
        let doc = LeagueStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let elements = doc.build(&FocusContext::default());

        assert_eq!(elements.len(), 1);
        assert_eq!(table_team_names(&elements[0]).len(), 32);
    }

    #[test]
    fn test_build_sorts_by_points_desc() {
        // The struct/module doc comments describe this as "single table with all
        // teams sorted by points". `build()` now sorts via the same
        // `sort_by_points_desc()` mechanism used by Wildcard, Division, and
        // Conference, so the rendered table order matches the doc comment
        // regardless of input order.
        let standings = vec![
            create_division_team("Low Points", "LP", "Atlantic", "Eastern", 1, 10, 0, 5),
            create_division_team("High Points", "HP", "Atlantic", "Eastern", 10, 1, 0, 25),
            create_division_team("Mid Points", "MP", "Atlantic", "Eastern", 5, 5, 0, 15),
        ];
        let doc = LeagueStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));
        let elements = doc.build(&FocusContext::default());

        assert_eq!(
            table_team_names(&elements[0]),
            vec!["High Points", "Mid Points", "Low Points"]
        );
    }

    #[test]
    fn test_new_ignores_config_parameter() {
        // Unlike the other three GroupBy views, League standings have no
        // east/west split, so `config` is accepted only for call-site symmetry
        // with StandingsDocumentWidget's four constructors and has no effect.
        let standings = create_test_standings();
        let western_first = LeagueStandingsDocument::new(
            Arc::new(standings.clone()),
            Arc::new(Config {
                display_standings_western_first: true,
                ..Default::default()
            }),
        );
        let eastern_first = LeagueStandingsDocument::new(
            Arc::new(standings),
            Arc::new(Config {
                display_standings_western_first: false,
                ..Default::default()
            }),
        );

        assert_eq!(
            table_team_names(&western_first.build(&FocusContext::default())[0]),
            table_team_names(&eastern_first.build(&FocusContext::default())[0])
        );
    }

    #[test]
    fn test_build_threads_focused_row_from_context() {
        let doc = LeagueStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        let focus = FocusContext::with_table_cell("league_standings", 5, 0);
        let elements = doc.build(&focus);

        // The focus context's row applies to the "league_standings" table; the
        // table's own team names are unaffected by focus (focus only changes
        // rendered styling), so we just confirm build() didn't panic and still
        // produced the full 32-row table using that focus context.
        match &elements[0] {
            DocumentElement::Table { widget, .. } => assert_eq!(widget.row_count(), 32),
            other => panic!("Expected Table element, got {other:?}"),
        }
    }

    #[test]
    fn test_focusables_returns_one_per_team() {
        let doc = LeagueStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.focusables(&FocusContext::default()).len(), 32);
    }

    #[test]
    fn test_document_metadata() {
        let doc = LeagueStandingsDocument::new(
            Arc::new(create_test_standings()),
            Arc::new(Config::default()),
        );
        assert_eq!(doc.title(), "League Standings");
        assert_eq!(doc.id(), "league_standings");
    }

    #[test]
    fn test_render_full_small_fixture() {
        let standings = vec![
            create_division_team("Team A", "AA", "Atlantic", "Eastern", 10, 5, 1, 21),
            create_division_team("Team B", "BB", "Central", "Western", 8, 7, 1, 17),
        ];
        let doc = LeagueStandingsDocument::new(Arc::new(standings), Arc::new(Config::default()));

        let display_config = crate::config::DisplayConfig::default();
        let ctx = crate::config::RenderContext::focused(&display_config);
        let (buf, height) = doc.render_full(60, &ctx, &FocusContext::default());

        assert_eq!(height, 4); // header + separator + 2 rows
        crate::tui::testing::assert_buffer(
            &buf,
            &[
                "  Team                          GP     W    L   OT    PTS",
                "  ───────────────────────────────────────────────────────",
                "  Team A                        16    10    5    1     21",
                "  Team B                        16     8    7    1     17",
            ],
        );
    }
}
