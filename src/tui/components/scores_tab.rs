use ratatui::{buffer::Buffer, layout::Rect};
use std::collections::HashMap;
use std::sync::Arc;

use nhl_api::{DailySchedule, GameDate, GameMatchup};

use crate::commands::scores_format::PeriodScores;
use crate::component_message_impl;
use crate::config::RenderContext;
use crate::tui::action::Action;
use crate::tui::component::{Component, Effect, Element, ElementWidget};
use crate::tui::document::{DocumentView, FocusableId};
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};
use crate::tui::tab_component::{
    activate_focused_link, enter_item_focus, exit_item_focus, handle_common_message,
    CommonTabMessage, TabMessage, TabState, BASE_CHROME_LINES, SUBTAB_CHROME_LINES,
};

use super::score_boxes_document::ScoreBoxesDocument;
use super::{TabItem, TabbedPanel, TabbedPanelProps};
//
/// Number of dates shown at once in the scores tab's date selector, e.g.
/// [-2, -1, today, +1, +2] centered on `selected_date_index`.
const DATE_WINDOW_SIZE: usize = 5;
//
/// Component state for ScoresTab - managed by the component itself
#[derive(Clone, Debug)]
pub struct ScoresTabState {
    // Date window state
    pub selected_date_index: usize,
    pub game_date: GameDate,

    // Document navigation (replaces browse_mode and selected_game_index)
    pub doc_nav: DocumentNavState,
}

impl Default for ScoresTabState {
    fn default() -> Self {
        Self {
            selected_date_index: DATE_WINDOW_SIZE / 2, // Middle of the date window
            game_date: GameDate::today(),
            doc_nav: DocumentNavState::default(),
        }
    }
}

impl TabState for ScoresTabState {
    fn doc_nav(&self) -> &DocumentNavState {
        &self.doc_nav
    }

    fn doc_nav_mut(&mut self) -> &mut DocumentNavState {
        &mut self.doc_nav
    }

    /// Scores has a nested date-selector subtab bar above its document viewport.
    fn chrome_lines() -> u16 {
        BASE_CHROME_LINES + SUBTAB_CHROME_LINES
    }
}

/// Messages handled by ScoresTab component
#[derive(Clone, Debug)]
pub enum ScoresTabMsg {
    /// Navigate up request (ESC in browse mode, returns to tab bar otherwise)
    /// Returns Effect::Handled if consumed, Effect::None if should bubble up
    NavigateUp,

    // Date navigation
    NavigateLeft,
    NavigateRight,

    // Browse mode (game selection)
    EnterBoxSelection,
    ExitBoxSelection,

    // Document navigation (delegated)
    DocNav(DocumentNavMsg),

    // Viewport management
    UpdateViewportHeight(u16),

    // Game activation
    ActivateGame,
}

impl TabMessage for ScoresTabMsg {
    fn as_common(&self) -> Option<CommonTabMessage<'_>> {
        match self {
            Self::DocNav(msg) => Some(CommonTabMessage::DocNav(msg)),
            Self::UpdateViewportHeight(h) => Some(CommonTabMessage::UpdateViewportHeight(*h)),
            Self::NavigateUp => Some(CommonTabMessage::NavigateUp),
            _ => None,
        }
    }

    fn from_doc_nav(msg: DocumentNavMsg) -> Self {
        Self::DocNav(msg)
    }
}

// Use macro to eliminate ComponentMessageTrait boilerplate
component_message_impl!(ScoresTabMsg, ScoresTab, ScoresTabState);

/// Props for ScoresTab component (data from parent)
#[derive(Clone)]
pub struct ScoresTabProps {
    // API data
    pub schedule: Arc<Option<DailySchedule>>,
    pub game_info: Arc<HashMap<i64, GameMatchup>>,
    pub period_scores: Arc<HashMap<i64, PeriodScores>>,

    // Navigation state
    pub focused: bool,

    // Animation frame for loading indicator
    pub animation_frame: u8,
}
//
/// ScoresTab component - renders scores with date selector
#[derive(Default)]
pub struct ScoresTab;
//
impl Component for ScoresTab {
    type Props = ScoresTabProps;
    type State = ScoresTabState;
    type Message = ScoresTabMsg;

    fn init(_props: &Self::Props) -> Self::State {
        ScoresTabState::default()
    }

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        // Handle common tab messages (DocNav, UpdateViewportHeight, NavigateUp)
        if let Some(effect) = handle_common_message(msg.as_common(), state) {
            return effect;
        }

        // Handle tab-specific messages
        match msg {
            ScoresTabMsg::NavigateLeft => Self::navigate_date(state, -1),
            ScoresTabMsg::NavigateRight => Self::navigate_date(state, 1),
            ScoresTabMsg::EnterBoxSelection => enter_item_focus(state),
            ScoresTabMsg::ExitBoxSelection => exit_item_focus(state, Effect::None),

            // Game activation: the focused score box carries its own
            // `LinkTarget::Push(Boxscore { .. })`, attached when the document
            // was built (see `ScoreBoxesDocument::build_link_target`), so
            // there's no need to re-derive the game's abbrevs/scores here.
            ScoresTabMsg::ActivateGame => activate_focused_link(state),

            // Common messages already handled above
            ScoresTabMsg::DocNav(_)
            | ScoresTabMsg::UpdateViewportHeight(_)
            | ScoresTabMsg::NavigateUp => {
                unreachable!("Common messages should be handled by handle_common_message")
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        self.render_date_tabs(props, state)
    }
}

impl ScoresTab {
    /// Move the date window by `direction` days (-1 for left, +1 for right).
    ///
    /// Shifts `selected_date_index` within the window until it hits the
    /// corresponding edge, then keeps sliding the whole window instead.
    fn navigate_date(state: &mut ScoresTabState, direction: i64) -> Effect {
        let at_edge = if direction < 0 {
            state.selected_date_index == 0
        } else {
            state.selected_date_index == DATE_WINDOW_SIZE - 1
        };

        if !at_edge {
            state.selected_date_index = (state.selected_date_index as i64 + direction) as usize;
        }
        state.game_date = state.game_date.add_days(direction);

        // Refresh schedule for new date (also updates global state and clears old data)
        Effect::Action(Action::RefreshSchedule(state.game_date.clone()))
    }

    /// Render date tabs using component state for UI, props for data
    fn render_date_tabs(&self, props: &ScoresTabProps, state: &ScoresTabState) -> Element {
        // Calculate the date window using component state
        let window_base_date = state
            .game_date
            .add_days(-(state.selected_date_index as i64));
        let dates: Vec<GameDate> = (0..DATE_WINDOW_SIZE)
            .map(|i| window_base_date.add_days(i as i64))
            .collect();
        //
        // Create TabItems for each date
        let tabs: Vec<TabItem> = dates
            .iter()
            .map(|date| {
                let key = self.date_to_key(date);
                let title = self.format_date_label(date);
                let content = self.render_game_list_from_state(props, state, date);
                //
                TabItem::new(key, title, content)
            })
            .collect();
        //
        // Active key is the current game_date from component state
        let active_key = self.date_to_key(&state.game_date);
        //
        TabbedPanel.view(
            &TabbedPanelProps {
                active_key,
                tabs,
                focused: props.focused && !state.has_item_focus(),
                content_has_focus: props.focused && state.has_item_focus(),
            },
            &(),
        )
    }

    //
    /// Convert GameDate to string key
    fn date_to_key(&self, date: &GameDate) -> String {
        match date {
            GameDate::Date(naive_date) => naive_date.format("%Y-%m-%d").to_string(),
            GameDate::Now => "now".to_string(),
        }
    }
    //
    /// Format date for tab label (MM/DD)
    fn format_date_label(&self, date: &GameDate) -> String {
        match date {
            GameDate::Date(naive_date) => naive_date.format("%m/%d").to_string(),
            GameDate::Now => chrono::Local::now()
                .date_naive()
                .format("%m/%d")
                .to_string(),
        }
    }
    /// Render game list using the document system with ScoreBoxesDocument
    fn render_game_list_from_state(
        &self,
        props: &ScoresTabProps,
        state: &ScoresTabState,
        _date: &GameDate,
    ) -> Element {
        // Wrap in ScoreBoxesDocumentWidget which calculates boxes_per_row at render time
        // Document is only focused when in browse mode (navigating within the document)
        Element::Widget(Box::new(ScoreBoxesDocumentWidget {
            schedule: props.schedule.clone(),
            game_info: props.game_info.clone(),
            game_date: state.game_date.clone(),
            focused_id: state.doc_nav.focused_id(),
            scroll_offset: state.doc_nav.scroll_offset,
            animation_frame: props.animation_frame,
            focused: props.focused && state.has_item_focus(),
        }))
    }
}

/// Widget that renders ScoreBoxesDocument with DocumentView
///
/// This widget creates the document at render time to calculate boxes_per_row
/// based on actual viewport width.
struct ScoreBoxesDocumentWidget {
    schedule: Arc<Option<DailySchedule>>,
    game_info: Arc<HashMap<i64, GameMatchup>>,
    game_date: GameDate,
    focused_id: Option<FocusableId>,
    scroll_offset: u16,
    animation_frame: u8,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl ElementWidget for ScoreBoxesDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Calculate boxes_per_row based on actual viewport width
        let boxes_per_row = ScoreBoxesDocument::boxes_per_row_for_width(area.width);

        // Create document with correct boxes_per_row
        let doc = ScoreBoxesDocument::new(
            self.schedule.clone(),
            self.game_info.clone(),
            boxes_per_row,
            self.game_date.clone(),
            self.animation_frame,
        );

        // Create DocumentView with viewport height
        let mut view = DocumentView::new(Arc::new(doc), area.height);

        // Apply focus state
        if let Some(id) = self.focused_id.clone() {
            view.focus_id(id);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        let child_ctx = ctx.child(self.focused);

        // Render the document
        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(ScoreBoxesDocumentWidget {
            schedule: self.schedule.clone(),
            game_info: self.game_info.clone(),
            game_date: self.game_date.clone(),
            focused_id: self.focused_id.clone(),
            scroll_offset: self.scroll_offset,
            animation_frame: self.animation_frame,
            focused: self.focused,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //
    #[test]
    fn test_scores_tab_renders_with_no_schedule() {
        let scores_tab = ScoresTab;
        let props = ScoresTabProps {
            schedule: Arc::new(None),
            game_info: Arc::new(HashMap::new()),
            period_scores: Arc::new(HashMap::new()),
            focused: false,
            animation_frame: 0,
        };
        //
        let state = ScoresTabState::default();
        let element = scores_tab.view(&props, &state);
        //
        match element {
            Element::Container { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected container element"),
        }
    }

    #[test]
    fn test_activate_game_pushes_boxscore_document() {
        use crate::tui::component::Component;
        use crate::tui::document::{FocusableElement, FocusableId, LinkTarget};
        use crate::tui::types::StackedDocument;

        let mut scores_tab = ScoresTab;
        let mut state = ScoresTabState::default();

        let doc = StackedDocument::Boxscore {
            game_id: 2024020001,
            away_abbrev: "TOR".to_string(),
            home_abbrev: "MTL".to_string(),
            away_score: 3,
            home_score: 2,
            game_date: "10/04".to_string(),
        };
        state.doc_nav.focusables =
            vec![
                FocusableElement::at(0, 1, FocusableId::game_link(2024020001))
                    .with_link_target(LinkTarget::Push(doc.clone())),
            ];
        state.doc_nav.focus_index = Some(0);

        let effect = scores_tab.update(ScoresTabMsg::ActivateGame, &mut state);

        match effect {
            Effect::Action(Action::PushDocument(pushed)) => assert_eq!(pushed, doc),
            _ => panic!("Expected PushDocument action, got {:?}", effect),
        }
    }

    #[test]
    fn test_activate_game_without_focus_does_nothing() {
        use crate::tui::component::Component;

        let mut scores_tab = ScoresTab;
        let mut state = ScoresTabState::default();

        let effect = scores_tab.update(ScoresTabMsg::ActivateGame, &mut state);

        assert!(matches!(effect, Effect::None));
    }
}
