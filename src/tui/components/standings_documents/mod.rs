//! Document implementations for standings views
//!
//! This module contains Document implementations for the different standings
//! views (League, Conference, Division, Wildcard).

mod conference;
mod division;
mod league;
mod wildcard;

use std::collections::BTreeMap;
use std::sync::Arc;

use nhl_api::Standing;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::config::{Config, RenderContext};
use crate::tui::component::ElementWidget;
use crate::tui::document::{Document, DocumentElement, DocumentView, FocusContext, FocusableId};
use crate::tui::helpers::StandingsSorting;

pub use conference::ConferenceStandingsDocument;
pub use division::DivisionStandingsDocument;
pub use league::LeagueStandingsDocument;
pub use wildcard::WildcardStandingsDocument;

use super::{standings_columns, TableWidget};

/// Left indent applied to section titles so they align with table content
/// (after the selector space).
const SECTION_TITLE_MARGIN: u16 = 2;

/// Horizontal gap between the two side-by-side standings columns.
const COLUMN_GAP: u16 = 4;

/// Group standings with `key`, sorting each group's teams by points descending.
fn group_standings_by(
    standings: &[Standing],
    key: impl Fn(&Standing) -> String,
) -> BTreeMap<String, Vec<Standing>> {
    let mut grouped: BTreeMap<String, Vec<Standing>> = BTreeMap::new();
    for standing in standings {
        grouped
            .entry(key(standing))
            .or_default()
            .push(standing.clone());
    }
    for teams in grouped.values_mut() {
        teams.sort_by_points_desc();
    }
    grouped
}

/// Standings for the four NHL divisions, each sorted by points descending,
/// as (Atlantic, Metropolitan, Central, Pacific). Divisions absent from the
/// input (e.g. standings not yet loaded) come back empty.
fn division_standings(
    standings: &[Standing],
) -> (Vec<Standing>, Vec<Standing>, Vec<Standing>, Vec<Standing>) {
    let mut grouped = group_standings_by(standings, |s| s.division_name.clone());
    (
        grouped.remove("Atlantic").unwrap_or_default(),
        grouped.remove("Metropolitan").unwrap_or_default(),
        grouped.remove("Central").unwrap_or_default(),
        grouped.remove("Pacific").unwrap_or_default(),
    )
}

/// Order the (eastern, western) pair per the western-first display setting.
fn order_conferences<T>(eastern: T, western: T, western_first: bool) -> (T, T) {
    if western_first {
        (western, eastern)
    } else {
        (eastern, western)
    }
}

/// A titled, focus-aware standings table section within a column.
struct StandingsSection<'a> {
    title: &'a str,
    table_name: String,
    teams: Vec<Standing>,
}

/// Build a vertical column of standings sections, skipping empty ones (no
/// title over an empty table) and separating the rest with single-line
/// spacers -- never a trailing spacer.
fn build_sections_group(sections: Vec<StandingsSection>, focus: &FocusContext) -> DocumentElement {
    let mut children = Vec::new();
    for section in sections.into_iter().filter(|s| !s.teams.is_empty()) {
        if !children.is_empty() {
            children.push(DocumentElement::spacer(1));
        }
        children.push(DocumentElement::indented(
            DocumentElement::section_title(section.title, false),
            SECTION_TITLE_MARGIN,
        ));
        let table = TableWidget::from_data(standings_columns(), section.teams)
            .with_focused_row(focus.focused_table_row(&section.table_name));
        children.push(DocumentElement::table(section.table_name, table));
    }
    DocumentElement::group(children)
}

/// Place two standings columns side-by-side (centered, standard gap). A
/// column whose group came out empty (e.g. standings not yet loaded for that
/// conference) is omitted so the remaining column fills the row.
fn two_column_row(left: DocumentElement, right: DocumentElement) -> DocumentElement {
    let columns = [left, right]
        .into_iter()
        .filter(
            |col| !matches!(col, DocumentElement::Group { children, .. } if children.is_empty()),
        )
        .collect();
    DocumentElement::row_center_with_gap(columns, COLUMN_GAP)
}

/// Widget that renders a standings document with DocumentView
///
/// This widget wraps DocumentView and applies focus/scroll state from AppState.
/// It can render League, Conference, or Division standings based on the document type.
pub struct StandingsDocumentWidget {
    doc: Arc<dyn Document>,
    focused_id: Option<FocusableId>,
    scroll_offset: u16,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl StandingsDocumentWidget {
    /// Create widget for League standings
    pub fn league(
        standings: Arc<Vec<Standing>>,
        config: impl Into<Arc<Config>>,
        focused_id: Option<FocusableId>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(LeagueStandingsDocument::new(standings, config)),
            focused_id,
            scroll_offset,
            focused,
        }
    }

    /// Create widget for Conference standings
    pub fn conference(
        standings: Arc<Vec<Standing>>,
        config: impl Into<Arc<Config>>,
        focused_id: Option<FocusableId>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(ConferenceStandingsDocument::new(standings, config)),
            focused_id,
            scroll_offset,
            focused,
        }
    }

    /// Create widget for Division standings
    pub fn division(
        standings: Arc<Vec<Standing>>,
        config: impl Into<Arc<Config>>,
        focused_id: Option<FocusableId>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(DivisionStandingsDocument::new(standings, config)),
            focused_id,
            scroll_offset,
            focused,
        }
    }

    /// Create widget for Wildcard standings
    pub fn wildcard(
        standings: Arc<Vec<Standing>>,
        config: impl Into<Arc<Config>>,
        focused_id: Option<FocusableId>,
        scroll_offset: u16,
        focused: bool,
    ) -> Self {
        Self {
            doc: Arc::new(WildcardStandingsDocument::new(standings, config)),
            focused_id,
            scroll_offset,
            focused,
        }
    }
}

impl ElementWidget for StandingsDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create DocumentView with viewport height
        let mut view = DocumentView::new(self.doc.clone(), area.height);

        // Apply focus state from AppState
        if let Some(id) = self.focused_id.clone() {
            view.focus_id(id);
        }

        // Apply scroll offset from AppState
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        let child_ctx = ctx.child(self.focused);

        // Render the document
        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(StandingsDocumentWidget {
            doc: self.doc.clone(),
            focused_id: self.focused_id.clone(),
            scroll_offset: self.scroll_offset,
            focused: self.focused,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

#[cfg(test)]
mod tests;
