use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};
use std::sync::Arc;

use nhl_api::Standing;

use crate::commands::standings::GroupBy;
use crate::config::Config;
use crate::config::DisplayConfig;
use crate::tui::{
    component::{Component, Element, ElementWidget},
    state::DocumentStackEntry,
};

use super::{TabItem, TabbedPanel, TabbedPanelProps};

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};
use crate::tui::tab_component::{CommonTabMessage, TabMessage, TabState, handle_common_message};
use crate::tui::types::StackedDocument;
use crate::component_message_impl;

/// Component state for StandingsTab - managed by the component itself
#[derive(Clone, Debug)]
pub struct StandingsTabState {
    pub view: GroupBy,
    // Document navigation state (embedded, browse_mode derived from focus_index)
    // Contains focusable_ids, link_targets, positions, etc.
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
}

/// Messages handled by StandingsTab component
#[derive(Clone, Debug)]
pub enum StandingsTabMsg {
    /// Key event when this tab is focused
    Key(KeyEvent),

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
    // Config
    pub config: Config,
}

/// StandingsTab component - renders standings with view selector
#[derive(Default)]
pub struct StandingsTab;

impl Component for StandingsTab {
    type Props = StandingsTabProps;
    type State = StandingsTabState;
    type Message = StandingsTabMsg;

    fn init(_props: &Self::Props) -> Self::State {
        StandingsTabState::default()
    }

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        // Handle common tab messages (DocNav, UpdateViewportHeight, NavigateUp)
        if let Some(effect) = handle_common_message(msg.as_common(), state) {
            return effect;
        }

        // Handle tab-specific messages
        match msg {
            StandingsTabMsg::Key(key) => self.handle_key(key, state),

            StandingsTabMsg::CycleViewLeft => {
                state.view = match state.view {
                    GroupBy::Wildcard => GroupBy::League,
                    GroupBy::Division => GroupBy::Wildcard,
                    GroupBy::Conference => GroupBy::Division,
                    GroupBy::League => GroupBy::Conference,
                };
                // Reset focus/scroll when changing views
                state.exit_browse_mode();
                // Signal that focusable metadata needs to be rebuilt
                Effect::Action(crate::tui::action::Action::RebuildStandingsFocusable)
            }
            StandingsTabMsg::CycleViewRight => {
                state.view = match state.view {
                    GroupBy::Wildcard => GroupBy::Division,
                    GroupBy::Division => GroupBy::Conference,
                    GroupBy::Conference => GroupBy::League,
                    GroupBy::League => GroupBy::Wildcard,
                };
                // Reset focus/scroll when changing views
                state.exit_browse_mode();
                // Signal that focusable metadata needs to be rebuilt
                Effect::Action(crate::tui::action::Action::RebuildStandingsFocusable)
            }
            StandingsTabMsg::EnterBrowseMode => {
                state.enter_browse_mode();
                Effect::None
            }
            StandingsTabMsg::ExitBrowseMode => {
                state.exit_browse_mode();
                Effect::None
            }

            StandingsTabMsg::ActivateTeam => {
                // Get the team abbreviation from the focused element's link target
                if let Some(link_target) = state.doc_nav().focused_link_target() {
                    if let crate::tui::document::LinkTarget::Action(action) = link_target {
                        // Parse "team:TOR" format
                        if let Some(abbrev) = action.strip_prefix("team:") {
                            return Effect::Action(Action::PushDocument(
                                StackedDocument::TeamDetail {
                                    abbrev: abbrev.to_string(),
                                },
                            ));
                        }
                    }
                }
                Effect::None
            }

            // Common messages already handled above
            StandingsTabMsg::DocNav(_) | StandingsTabMsg::UpdateViewportHeight(_) | StandingsTabMsg::NavigateUp => {
                unreachable!("Common messages should be handled by handle_common_message")
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        // If in document stack view, render the stacked document instead
        if !props.document_stack.is_empty() {
            tracing::debug!(
                "RENDER: Document stack has {} items, rendering stacked document",
                props.document_stack.len()
            );
            return self.render_stacked_document(props);
        }

        self.render_view_tabs(props, state)
    }
}

impl StandingsTab {
    /// Handle key events when this tab is focused
    fn handle_key(&mut self, key: KeyEvent, state: &mut StandingsTabState) -> Effect {
        use crate::tui::nav_handler::key_to_nav_msg;

        if state.is_browse_mode() {
            // Browse mode - arrow keys navigate teams

            // Try standard navigation first (handles Tab, arrows, PageUp/Down, etc.)
            if let Some(nav_msg) = key_to_nav_msg(key) {
                return crate::tui::document_nav::handle_message(&mut state.doc_nav, &nav_msg);
            }

            // Handle Enter to activate focused element
            match key.code {
                KeyCode::Enter => self.update(StandingsTabMsg::ActivateTeam, state),
                _ => Effect::None,
            }
        } else {
            // View selection mode - arrow keys navigate views
            match key.code {
                KeyCode::Left => self.update(StandingsTabMsg::CycleViewLeft, state),
                KeyCode::Right => self.update(StandingsTabMsg::CycleViewRight, state),
                KeyCode::Down | KeyCode::Enter => {
                    self.update(StandingsTabMsg::EnterBrowseMode, state)
                }
                _ => Effect::None,
            }
        }
    }

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
                focused: props.focused && !state.is_browse_mode(),
            },
            &(),
        )
    }

    fn render_standings_table(&self, props: &StandingsTabProps, state: &StandingsTabState, view: &GroupBy) -> Element {
        // If no standings data, show loading message
        let Some(standings) = props.standings.as_ref().as_ref() else {
            return Element::Widget(Box::new(LoadingWidget {
                message: "Loading standings...".to_string(),
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

    fn render_league_view(&self, props: &StandingsTabProps, state: &StandingsTabState, standings: &[Standing]) -> Element {
        use super::StandingsDocumentWidget;

        Element::Widget(Box::new(StandingsDocumentWidget::league(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focus_index,
            state.doc_nav.scroll_offset,
        )))
    }

    fn render_conference_view(&self, props: &StandingsTabProps, state: &StandingsTabState, standings: &[Standing]) -> Element {
        // Use the document system for Conference view (like League view)
        use super::StandingsDocumentWidget;

        Element::Widget(Box::new(StandingsDocumentWidget::conference(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focus_index,
            state.doc_nav.scroll_offset,
        )))
    }

    fn render_division_view(&self, props: &StandingsTabProps, state: &StandingsTabState, standings: &[Standing]) -> Element {
        // Use the document system for Division view
        use super::StandingsDocumentWidget;

        Element::Widget(Box::new(StandingsDocumentWidget::division(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focus_index,
            state.doc_nav.scroll_offset,
        )))
    }

    fn render_wildcard_view(&self, props: &StandingsTabProps, state: &StandingsTabState, standings: &[Standing]) -> Element {
        // Use the document system for Wildcard view
        use super::StandingsDocumentWidget;

        Element::Widget(Box::new(StandingsDocumentWidget::wildcard(
            Arc::new(standings.to_vec()),
            props.config.clone(),
            state.doc_nav.focus_index,
            state.doc_nav.scroll_offset,
        )))
    }

    fn render_stacked_document(&self, props: &StandingsTabProps) -> Element {
        // Get the current stacked document info
        let doc_info = if let Some(doc_entry) = props.document_stack.last() {
            let msg = match &doc_entry.document {
                super::super::StackedDocument::TeamDetail { abbrev } => {
                    format!("Team Detail: {}\n\n(Document rendering not yet implemented)\n\nPress ESC to go back", abbrev)
                }
                super::super::StackedDocument::PlayerDetail { player_id } => {
                    format!("Player Detail: {}\n\n(Document rendering not yet implemented)\n\nPress ESC to go back", player_id)
                }
                super::super::StackedDocument::Boxscore { game_id } => {
                    format!("Boxscore: {}\n\n(Document rendering not yet implemented)\n\nPress ESC to go back", game_id)
                }
            };
            tracing::debug!("RENDER: Rendering stacked document with message: {}", msg);
            msg
        } else {
            tracing::warn!("RENDER: render_stacked_document called but document_stack is empty!");
            "No document".to_string()
        };

        Element::Widget(Box::new(StackedDocumentWidget {
            message: doc_info,
        }))
    }
}

/// Loading widget - shows a simple loading or error message
struct LoadingWidget {
    message: String,
}

impl ElementWidget for LoadingWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, _config: &DisplayConfig) {
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

/// Stacked document widget placeholder
struct StackedDocumentWidget {
    message: String,
}

impl ElementWidget for StackedDocumentWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, _config: &DisplayConfig) {
        let widget = Paragraph::new(self.message.as_str())
            .block(Block::default().borders(Borders::ALL).title("Document View"));
        ratatui::widgets::Widget::render(widget, area, buf);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(StackedDocumentWidget {
            message: self.message.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::renderer::Renderer;
    use crate::tui::testing::{assert_buffer, create_test_standings};
    use ratatui::{buffer::Buffer, layout::Rect};
    const RENDER_WIDTH: u16 = 120;
    const RENDER_HEIGHT: u16 = 40;

    #[test]
    fn test_standings_tab_renders_with_no_standings() {
        let standings_tab = StandingsTab;
        let props = StandingsTabProps {
            standings: Arc::new(None),
            document_stack: Vec::new(),
            focused: false,
            config: Config::default(),
        };

        let element = standings_tab.view(&props, &StandingsTabState::default());

        match element {
            Element::Container { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected container element"),
        }
    }

    #[test]
    fn test_standings_tab_renders_league_view() {
        let standings_tab = StandingsTab;
        let standings = create_test_standings();

        let props = StandingsTabProps {
            standings: Arc::new(Some(standings)),
            document_stack: Vec::new(),
            focused: false,
            config: Config::default(),
        };

        // This should not panic - verifies TableWidget can be created
        let element = standings_tab.view(&props, &StandingsTabState::default());

        match element {
            Element::Container { children, .. } => {
                assert_eq!(children.len(), 2); // Tab bar + content
            }
            _ => panic!("Expected container element"),
        }
    }

    // === Rendering Tests ===

    /// Helper to render element to buffer
    fn render_element_to_buffer(
        element: &Element,
        width: u16,
        height: u16,
        config: &DisplayConfig,
    ) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        let mut renderer = Renderer::new();
        renderer.render(element.clone(), buf.area, &mut buf, config);
        buf
    }

    #[test]
    fn test_league_view_full_render() {
        let standings_tab = StandingsTab;
        let standings = create_test_standings();

        let props = StandingsTabProps {
            standings: Arc::new(Some(standings)),
            document_stack: Vec::new(),
            focused: false,
            config: Config::default(),
        };

        let state = StandingsTabState {
            view: GroupBy::League,
            ..Default::default()
        };

        let element = standings_tab.view(&props, &state);
        let config = DisplayConfig::default();
        let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

        assert_buffer(&buf, &[
            "Wildcard │ Division │ Conference │ League",
            "─────────┴──────────┴────────────┴──────────────────────────────────────────────────────────────────────────────────────",
            "  Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────",
            "  Panthers                      19    14    3    2     30",
            "  Bruins                        18    13    4    1     27",
            "  Maple Leafs                   19    12    5    2     26",
            "  Lightning                     18    11    6    1     23",
            "  Canadiens                     18    10    5    3     23",
            "  Senators                      18     9    7    2     20",
            "  Red Wings                     18     8    8    2     18",
            "  Sabres                        18     6   10    2     14",
            "  Devils                        18    15    2    1     31",
            "  Hurricanes                    19    14    3    2     30",
            "  Rangers                       18    12    5    1     25",
            "  Penguins                      19    11    6    2     24",
            "  Capitals                      18    10    7    1     21",
            "  Islanders                     18     9    7    2     20",
            "  Flyers                        18     8    9    1     17",
            "  Blue Jackets                  18     5   11    2     12",
            "  Avalanche                     19    16    2    1     33",
            "  Stars                         20    14    4    2     30",
            "  Jets                          19    13    5    1     27",
            "  Wild                          19    11    6    2     24",
            "  Predators                     19    10    7    2     22",
            "  Blues                         19     8    8    3     19",
            "  Blackhawks                    18     7   10    1     15",
            "  Coyotes                       18     4   13    1      9",
            "  Golden Knights                19    15    3    1     31",
            "  Oilers                        20    14    4    2     30",
            "  Kings                         19    12    6    1     25",
            "  Kraken                        19    11    6    2     24",
            "  Canucks                       19    10    7    2     22",
            "  Flames                        19     9    8    2     20",
            "  Ducks                         19     7   10    2     16",
            "  Sharks                        18     5   12    1     11",
            "",
            "",
            "",
            "",
        ]);
    }

    #[test]
    fn test_division_view_full_render() {
        let standings_tab = StandingsTab;
        let standings = create_test_standings();

        let props = StandingsTabProps {
            standings: Arc::new(Some(standings)),
            document_stack: Vec::new(),
            focused: false,
            config: Config::default(),
        };

        let state = StandingsTabState {
            view: GroupBy::Division,
            ..Default::default()
        };

        let element = standings_tab.view(&props, &state);
        let config = DisplayConfig::default();
        let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

        // Division view now uses document system with two Groups in a Row
        // Layout: Atlantic + Metropolitan on left, Central + Pacific on right
        // (when western_first = false, which is the default)
        // Section titles have no underline
        assert_buffer(&buf, &[
            "Wildcard │ Division │ Conference │ League",
            "─────────┴──────────┴────────────┴──────────────────────────────────────────────────────────────────────────────────────",
            "  Atlantic                                                     Central",
            "",
            "  Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
            "  Panthers                      19    14    3    2     30      Avalanche                     19    16    2    1     33",
            "  Bruins                        18    13    4    1     27      Stars                         20    14    4    2     30",
            "  Maple Leafs                   19    12    5    2     26      Jets                          19    13    5    1     27",
            "  Lightning                     18    11    6    1     23      Wild                          19    11    6    2     24",
            "  Canadiens                     18    10    5    3     23      Predators                     19    10    7    2     22",
            "  Senators                      18     9    7    2     20      Blues                         19     8    8    3     19",
            "  Red Wings                     18     8    8    2     18      Blackhawks                    18     7   10    1     15",
            "  Sabres                        18     6   10    2     14      Coyotes                       18     4   13    1      9",
            "",
            "  Metropolitan                                                 Pacific",
            "",
            "  Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
            "  Devils                        18    15    2    1     31      Golden Knights                19    15    3    1     31",
            "  Hurricanes                    19    14    3    2     30      Oilers                        20    14    4    2     30",
            "  Rangers                       18    12    5    1     25      Kings                         19    12    6    1     25",
            "  Penguins                      19    11    6    2     24      Kraken                        19    11    6    2     24",
            "  Capitals                      18    10    7    1     21      Canucks                       19    10    7    2     22",
            "  Islanders                     18     9    7    2     20      Flames                        19     9    8    2     20",
            "  Flyers                        18     8    9    1     17      Ducks                         19     7   10    2     16",
            "  Blue Jackets                  18     5   11    2     12      Sharks                        18     5   12    1     11",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ]);
    }

    #[test]
    fn test_conference_view_full_render() {
        let standings_tab = StandingsTab;
        let standings = create_test_standings();

        let props = StandingsTabProps {
            standings: Arc::new(Some(standings)),
            document_stack: Vec::new(),
            focused: false,
            config: Config::default(),
        };

        let state = StandingsTabState {
            view: GroupBy::Conference,
            ..Default::default()
        };

        let element = standings_tab.view(&props, &state);
        let config = DisplayConfig::default();
        let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

        // Conference view now uses document system with teams sorted by points
        assert_buffer(&buf, &[
            "Wildcard │ Division │ Conference │ League",
            "─────────┴──────────┴────────────┴──────────────────────────────────────────────────────────────────────────────────────",
            "  Eastern                                                      Western",
            "",
            "  Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
            "  Devils                        18    15    2    1     31      Avalanche                     19    16    2    1     33",
            "  Panthers                      19    14    3    2     30      Golden Knights                19    15    3    1     31",
            "  Hurricanes                    19    14    3    2     30      Stars                         20    14    4    2     30",
            "  Bruins                        18    13    4    1     27      Oilers                        20    14    4    2     30",
            "  Maple Leafs                   19    12    5    2     26      Jets                          19    13    5    1     27",
            "  Rangers                       18    12    5    1     25      Kings                         19    12    6    1     25",
            "  Penguins                      19    11    6    2     24      Wild                          19    11    6    2     24",
            "  Lightning                     18    11    6    1     23      Kraken                        19    11    6    2     24",
            "  Canadiens                     18    10    5    3     23      Predators                     19    10    7    2     22",
            "  Capitals                      18    10    7    1     21      Canucks                       19    10    7    2     22",
            "  Senators                      18     9    7    2     20      Flames                        19     9    8    2     20",
            "  Islanders                     18     9    7    2     20      Blues                         19     8    8    3     19",
            "  Red Wings                     18     8    8    2     18      Ducks                         19     7   10    2     16",
            "  Flyers                        18     8    9    1     17      Blackhawks                    18     7   10    1     15",
            "  Sabres                        18     6   10    2     14      Sharks                        18     5   12    1     11",
            "  Blue Jackets                  18     5   11    2     12      Coyotes                       18     4   13    1      9",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ]);
    }

    #[test]
    fn test_wildcard_view_full_render() {
        let standings_tab = StandingsTab;
        let standings = create_test_standings();
        let props = StandingsTabProps {
            standings: Arc::new(Some(standings)),
            document_stack: Vec::new(),
            focused: false,
            config: Config::default(),
        };

        let element = standings_tab.view(&props, &StandingsTabState::default());
        let config = DisplayConfig::default();
        let buf = render_element_to_buffer(&element, RENDER_WIDTH, RENDER_HEIGHT, &config);

        assert_buffer(&buf, &[
            "Wildcard │ Division │ Conference │ League",
            "─────────┴──────────┴────────────┴──────────────────────────────────────────────────────────────────────────────────────",
            "  Atlantic                                                     Central",
            "",
            "  Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
            "  Panthers                      19    14    3    2     30      Avalanche                     19    16    2    1     33",
            "  Bruins                        18    13    4    1     27      Stars                         20    14    4    2     30",
            "  Maple Leafs                   19    12    5    2     26      Jets                          19    13    5    1     27",
            "",
            "  Metropolitan                                                 Pacific",
            "",
            "  Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
            "  Devils                        18    15    2    1     31      Golden Knights                19    15    3    1     31",
            "  Hurricanes                    19    14    3    2     30      Oilers                        20    14    4    2     30",
            "  Rangers                       18    12    5    1     25      Kings                         19    12    6    1     25",
            "",
            "  Wildcard                                                     Wildcard",
            "",
            "  Team                          GP     W    L   OT    PTS      Team                          GP     W    L   OT    PTS",
            "  ───────────────────────────────────────────────────────      ───────────────────────────────────────────────────────",
            "  Penguins                      19    11    6    2     24      Wild                          19    11    6    2     24",
            "  Lightning                     18    11    6    1     23      Kraken                        19    11    6    2     24",
            "  Canadiens                     18    10    5    3     23      Predators                     19    10    7    2     22",
            "  Capitals                      18    10    7    1     21      Canucks                       19    10    7    2     22",
            "  Senators                      18     9    7    2     20      Flames                        19     9    8    2     20",
            "  Islanders                     18     9    7    2     20      Blues                         19     8    8    3     19",
            "  Red Wings                     18     8    8    2     18      Ducks                         19     7   10    2     16",
            "  Flyers                        18     8    9    1     17      Blackhawks                    18     7   10    1     15",
            "  Sabres                        18     6   10    2     14      Sharks                        18     5   12    1     11",
            "  Blue Jackets                  18     5   11    2     12      Coyotes                       18     4   13    1      9",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ]);
    }

    /// Regression test: focusable_positions must be rebuilt when switching views
    ///
    /// Bug: When switching between standings views (League, Division, Conference, Wildcard),
    /// the focusable_positions were not being updated. This caused autoscroll to use
    /// stale position data from the previous view, resulting in incorrect scrolling behavior.
    ///
    /// For example, switching from Conference view (positions [5-20, 5-20] for two columns)
    /// to League view (positions [2-33] for single column) would use the wrong positions.
    #[test]
    fn test_cycle_view_triggers_rebuild_focusable_metadata() {
        use crate::tui::action::Action;
        use crate::tui::component::{Component, Effect};

        let mut standings_tab = StandingsTab;
        let mut state = StandingsTabState {
            view: GroupBy::Wildcard,
            ..Default::default()
        };

        // Cycle view left should return Effect::Action to rebuild metadata
        let effect = standings_tab.update(StandingsTabMsg::CycleViewLeft, &mut state);

        // View should change
        assert_eq!(state.view, GroupBy::League);

        // Focus/scroll should be reset
        assert_eq!(state.doc_nav.focus_index, None);
        assert_eq!(state.doc_nav.scroll_offset, 0);

        // Effect should trigger RebuildStandingsFocusable
        match effect {
            Effect::Action(Action::RebuildStandingsFocusable) => {
                // Good - this is the fix for the regression
            }
            _ => panic!("Expected RebuildStandingsFocusable action, got {:?}", effect),
        }
    }

    #[test]
    fn test_cycle_view_right_triggers_rebuild_focusable_metadata() {
        use crate::tui::action::Action;
        use crate::tui::component::{Component, Effect};

        let mut standings_tab = StandingsTab;
        let mut state = StandingsTabState {
            view: GroupBy::League,
            ..Default::default()
        };

        // Cycle view right should return Effect::Action to rebuild metadata
        let effect = standings_tab.update(StandingsTabMsg::CycleViewRight, &mut state);

        // View should change
        assert_eq!(state.view, GroupBy::Wildcard);

        // Focus/scroll should be reset
        assert_eq!(state.doc_nav.focus_index, None);
        assert_eq!(state.doc_nav.scroll_offset, 0);

        // Effect should trigger RebuildStandingsFocusable
        match effect {
            Effect::Action(Action::RebuildStandingsFocusable) => {
                // Good - this is the fix for the regression
            }
            _ => panic!("Expected RebuildStandingsFocusable action, got {:?}", effect),
        }
    }

    #[test]
    fn test_activate_team_pushes_team_detail_document() {
        use crate::tui::action::Action;
        use crate::tui::component::{Component, Effect};
        use crate::tui::document::LinkTarget;
        use crate::tui::types::StackedDocument;

        let mut standings_tab = StandingsTab;
        let mut state = StandingsTabState {
            view: GroupBy::League,
            ..Default::default()
        };

        // Set link targets for teams (what table cells now use)
        state.doc_nav.link_targets = vec![
            Some(LinkTarget::Action("team:TOR".to_string())),
            Some(LinkTarget::Action("team:BOS".to_string())),
            Some(LinkTarget::Action("team:MTL".to_string())),
        ];

        // Set focus to second team (BOS)
        state.doc_nav.focus_index = Some(1);

        // ActivateTeam should push TeamDetail document
        let effect = standings_tab.update(StandingsTabMsg::ActivateTeam, &mut state);

        match effect {
            Effect::Action(Action::PushDocument(StackedDocument::TeamDetail { abbrev })) => {
                assert_eq!(abbrev, "BOS");
            }
            _ => panic!("Expected PushDocument(TeamDetail) action, got {:?}", effect),
        }
    }

    #[test]
    fn test_activate_team_without_focus_does_nothing() {
        use crate::tui::component::{Component, Effect};
        use crate::tui::document::LinkTarget;

        let mut standings_tab = StandingsTab;
        let mut state = StandingsTabState {
            view: GroupBy::League,
            ..Default::default()
        };

        // Set link targets for teams
        state.doc_nav.link_targets = vec![
            Some(LinkTarget::Action("team:TOR".to_string())),
        ];

        // No focus set
        state.doc_nav.focus_index = None;

        let effect = standings_tab.update(StandingsTabMsg::ActivateTeam, &mut state);

        assert!(matches!(effect, Effect::None));
    }
}
