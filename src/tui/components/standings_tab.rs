use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};
use std::sync::Arc;

use nhl_api::Standing;

use crate::commands::standings::GroupBy;
use crate::config::Config;
use crate::config::RenderContext;
use crate::tui::{
    component::{Component, Element, ElementWidget},
    state::DocumentStackEntry,
    widgets::{LoadingAnimation, StandaloneWidget},
};

use super::{TabItem, TabbedPanel, TabbedPanelProps};

use crate::component_message_impl;
use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};
use crate::tui::tab_component::{
    activate_focused_link, enter_item_focus, exit_item_focus, handle_common_message,
    CommonTabMessage, TabMessage, TabState, BASE_CHROME_LINES, SUBTAB_CHROME_LINES,
};

/// Component state for StandingsTab - managed by the component itself
#[derive(Clone, Debug)]
pub struct StandingsTabState {
    pub view: GroupBy,
    // Document navigation state (embedded, has_item_focus derived from focus_index)
    // Contains the focusable elements (id, position, height, link target, etc.)
    pub doc_nav: DocumentNavState,
}

impl Default for StandingsTabState {
    fn default() -> Self {
        Self {
            view: GroupBy::Wildcard,
            doc_nav: DocumentNavState::default(),
        }
    }
}

impl TabState for StandingsTabState {
    fn doc_nav(&self) -> &DocumentNavState {
        &self.doc_nav
    }

    fn doc_nav_mut(&mut self) -> &mut DocumentNavState {
        &mut self.doc_nav
    }

    /// Standings has a nested group-by subtab bar above its document viewport.
    fn chrome_lines() -> u16 {
        BASE_CHROME_LINES + SUBTAB_CHROME_LINES
    }
}

/// Messages handled by StandingsTab component
#[derive(Clone, Debug)]
pub enum StandingsTabMsg {
    /// Navigate up request (ESC in browse mode, returns to tab bar otherwise)
    NavigateUp,

    CycleViewLeft,
    CycleViewRight,
    EnterBrowseMode,
    ExitBrowseMode,

    // Document navigation (delegated to DocumentNavMsg)
    DocNav(DocumentNavMsg),

    // Update viewport height
    UpdateViewportHeight(u16),

    // Activate the currently focused team (push TeamDetail document)
    ActivateTeam,
}

impl TabMessage for StandingsTabMsg {
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
component_message_impl!(StandingsTabMsg, StandingsTab, StandingsTabState);

/// Props for StandingsTab component
#[derive(Clone)]
pub struct StandingsTabProps {
    // API data
    pub standings: Arc<Option<Vec<Standing>>>,
    // Navigation state
    pub document_stack: Vec<DocumentStackEntry>,
    pub focused: bool,
    // Config (Arc'd by the caller so cloning props each render is a pointer bump,
    // not a deep copy of the underlying Config)
    pub config: Arc<Config>,
    // Animation frame for loading indicator
    pub animation_frame: u8,
}

/// StandingsTab component - renders standings with view selector
#[derive(Default)]
pub struct StandingsTab;

impl Component for StandingsTab {
    type Props = StandingsTabProps;
    type State = StandingsTabState;
    type Message = StandingsTabMsg;

    fn init(props: &Self::Props) -> Self::State {
        use crate::tui::components::WildcardStandingsDocument;
        use crate::tui::document::FocusContext;

        let mut state = StandingsTabState::default();
        // Component state is created lazily on first render, which can happen AFTER
        // StandingsLoaded already ran rebuild_standings_focusable_metadata against a
        // store that had no standings state yet. Populate metadata for the default
        // view from the data in props, or Down in view-selection mode finds no
        // focusables and browse mode can never be entered.
        // state.view is GroupBy::Wildcard here (StandingsTabState::default()), so the
        // wildcard document is the right one to build.
        if let Some(standings) = props.standings.as_ref().as_ref() {
            let doc =
                WildcardStandingsDocument::new(Arc::new(standings.clone()), props.config.clone());
            state
                .doc_nav
                .sync_focusables(&doc, &FocusContext::default());
        }
        state
    }

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        // Handle common tab messages (DocNav, UpdateViewportHeight, NavigateUp)
        if let Some(effect) = handle_common_message(msg.as_common(), state) {
            return effect;
        }

        // Handle tab-specific messages
        match msg {
            StandingsTabMsg::CycleViewLeft => {
                state.view = state.view.prev();
                // Reset focus/scroll when changing views
                state.clear_item_focus();
                // Signal that focusable metadata needs to be rebuilt
                Effect::Action(Action::RebuildStandingsFocusable)
            }
            StandingsTabMsg::CycleViewRight => {
                state.view = state.view.next();
                // Reset focus/scroll when changing views
                state.clear_item_focus();
                // Signal that focusable metadata needs to be rebuilt
                Effect::Action(Action::RebuildStandingsFocusable)
            }
            StandingsTabMsg::EnterBrowseMode => enter_item_focus(state),
            StandingsTabMsg::ExitBrowseMode => exit_item_focus(state, Effect::None),

            StandingsTabMsg::ActivateTeam => activate_focused_link(state),

            // Common messages already handled above
            StandingsTabMsg::DocNav(_)
            | StandingsTabMsg::UpdateViewportHeight(_)
            | StandingsTabMsg::NavigateUp => {
                unreachable!("Common messages should be handled by handle_common_message")
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        self.render_view_tabs(props, state)
    }
}

impl StandingsTab {
    /// Render view tabs using TabbedPanel (Wildcard/Division/Conference/League)
    fn render_view_tabs(&self, props: &StandingsTabProps, state: &StandingsTabState) -> Element {
        // All inactive tabs get Element::None to avoid cloning issues
        let tabs = [
            GroupBy::Wildcard,
            GroupBy::Division,
            GroupBy::Conference,
            GroupBy::League,
        ];
        let tabs = tabs
            .iter()
            .map(|g| {
                TabItem::new(
                    g.name(),
                    g.name(),
                    if state.view == *g {
                        self.render_standings_table(props, state, g)
                    } else {
                        Element::None
                    },
                )
            })
            .collect();

        TabbedPanel.view(
            &TabbedPanelProps {
                active_key: state.view.name().to_string(),
                tabs,
                focused: props.focused && !state.has_item_focus(),
                content_has_focus: props.focused && state.has_item_focus(),
            },
            &(),
        )
    }

    fn render_standings_table(
        &self,
        props: &StandingsTabProps,
        state: &StandingsTabState,
        view: &GroupBy,
    ) -> Element {
        // If no standings data, show loading animation
        let Some(standings) = props.standings.as_ref().as_ref() else {
            return Element::Widget(Box::new(AnimatedLoadingWidget {
                animation_frame: props.animation_frame,
            }));
        };

        if standings.is_empty() {
            return Element::Widget(Box::new(LoadingWidget {
                message: "No standings available".to_string(),
            }));
        }

        match view {
            GroupBy::Conference => self.render_conference_view(props, state, standings),
            GroupBy::Division => self.render_division_view(props, state, standings),
            GroupBy::Wildcard => self.render_wildcard_view(props, state, standings),
            GroupBy::League => self.render_league_view(props, state, standings),
        }
    }

    fn render_league_view(
        &self,
        props: &StandingsTabProps,
        state: &StandingsTabState,
        standings: &[Standing],
    ) -> Element {
        use super::StandingsDocumentWidget;

        // Document is only focused when in browse mode (navigating within the document)
        Element::Widget(Box::new(StandingsDocumentWidget::league(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focused_id(),
            state.doc_nav.scroll_offset,
            props.focused && state.has_item_focus(),
        )))
    }

    fn render_conference_view(
        &self,
        props: &StandingsTabProps,
        state: &StandingsTabState,
        standings: &[Standing],
    ) -> Element {
        // Use the document system for Conference view (like League view)
        use super::StandingsDocumentWidget;

        // Document is only focused when in browse mode (navigating within the document)
        Element::Widget(Box::new(StandingsDocumentWidget::conference(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focused_id(),
            state.doc_nav.scroll_offset,
            props.focused && state.has_item_focus(),
        )))
    }

    fn render_division_view(
        &self,
        props: &StandingsTabProps,
        state: &StandingsTabState,
        standings: &[Standing],
    ) -> Element {
        // Use the document system for Division view
        use super::StandingsDocumentWidget;

        // Document is only focused when in browse mode (navigating within the document)
        Element::Widget(Box::new(StandingsDocumentWidget::division(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focused_id(),
            state.doc_nav.scroll_offset,
            props.focused && state.has_item_focus(),
        )))
    }

    fn render_wildcard_view(
        &self,
        props: &StandingsTabProps,
        state: &StandingsTabState,
        standings: &[Standing],
    ) -> Element {
        // Use the document system for Wildcard view
        use super::StandingsDocumentWidget;

        // Document is only focused when in browse mode (navigating within the document)
        Element::Widget(Box::new(StandingsDocumentWidget::wildcard(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focused_id(),
            state.doc_nav.scroll_offset,
            props.focused && state.has_item_focus(),
        )))
    }
}

/// Animated loading widget - shows the pulsing dots animation
struct AnimatedLoadingWidget {
    animation_frame: u8,
}

impl ElementWidget for AnimatedLoadingWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        LoadingAnimation::new(self.animation_frame).render(area, buf, ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(AnimatedLoadingWidget {
            animation_frame: self.animation_frame,
        })
    }
}

/// Loading widget - shows a simple loading or error message
struct LoadingWidget {
    message: String,
}

impl ElementWidget for LoadingWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, _ctx: &RenderContext) {
        let widget =
            Paragraph::new(self.message.as_str()).block(Block::default().borders(Borders::NONE));
        ratatui::widgets::Widget::render(widget, area, buf);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(LoadingWidget {
            message: self.message.clone(),
        })
    }
}

#[cfg(test)]
#[path = "standings_tab_tests.rs"]
mod tests;
