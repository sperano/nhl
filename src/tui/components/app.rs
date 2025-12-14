use crate::tui::component::{vertical, Component, Constraint, Element};
use crate::tui::component_store::ComponentStateStore;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::state::{AppState, LoadingKey};

#[cfg(feature = "development")]
use super::demo_tab::DemoTabProps;
#[cfg(feature = "development")]
use super::DemoTab;
use super::{
    boxscore_document::{BoxscoreDocument, BoxscoreDocumentProps, TeamView},
    player_detail_document::PlayerDetailDocumentProps,
    scores_tab::ScoresTabProps,
    settings_tab::SettingsTabProps,
    standings_tab::StandingsTabProps,
    team_detail_document::TeamDetailDocumentProps,
    BreadcrumbWidget, PlayerDetailDocument, ScoresTab, SettingsTab, StandingsTab, StatusBar,
    TabItem, TabbedPanel, TabbedPanelProps, TeamDetailDocument,
};
use crate::tui::state::DocumentStackEntry;
use crate::tui::types::StackedDocument;

/// Root App component
///
/// This is the top-level component that renders the entire application.
/// It uses the global AppState as props and delegates rendering to child components.
pub struct App;

impl Component for App {
    type Props = AppState;
    type State = ();
    type Message = ();

    fn view(&self, props: &Self::Props, _state: &Self::State) -> Element {
        vertical(
            [Constraint::Min(0), Constraint::Length(2)],
            vec![
                self.render_main_tabs_without_states(props),
                StatusBar.view(&props.system, &()),
            ],
        )
    }
}

impl App {
    pub fn build_with_component_states(
        &self,
        state: &AppState,
        component_states: &mut ComponentStateStore,
    ) -> Element {
        vertical(
            [Constraint::Min(0), Constraint::Length(2)],
            vec![
                self.render_main_tabs_with_states(state, component_states),
                StatusBar.view(&state.system, &()),
            ],
        )
    }

    fn render_main_tabs_without_states(&self, state: &AppState) -> Element {
        use crate::tui::Tab;

        let active_key = match state.navigation.current_tab {
            Tab::Scores => "scores",
            Tab::Standings => "standings",
            Tab::Settings => "settings",
            #[cfg(feature = "development")]
            Tab::Demo => "demo",
        };

        let mut scores_content = Element::None;
        let mut standings_content = Element::None;
        let mut settings_content = Element::None;
        #[cfg(feature = "development")]
        let mut demo_content = Element::None;

        if let Some(doc_entry) = state.navigation.document_stack.last() {
            let doc_element = self.render_stacked_document(state, doc_entry);
            let breadcrumb_element = self.render_breadcrumb(state);

            let content_with_breadcrumb = vertical(
                [Constraint::Length(2), Constraint::Min(0)],
                vec![breadcrumb_element, doc_element],
            );

            match state.navigation.current_tab {
                Tab::Scores => scores_content = content_with_breadcrumb,
                Tab::Standings => standings_content = content_with_breadcrumb,
                Tab::Settings => settings_content = content_with_breadcrumb,
                #[cfg(feature = "development")]
                Tab::Demo => demo_content = content_with_breadcrumb,
            }
        } else {
            scores_content = self.render_scores_tab(state);
            standings_content = self.render_standings_tab(state);
            settings_content = self.render_settings_tab(state);
            #[cfg(feature = "development")]
            {
                demo_content = DemoTab.view(
                    &DemoTabProps {
                        focused: state.navigation.focus_in_content,
                        standings: state.data.standings.clone(),
                    },
                    &Default::default(),
                );
            }
        }

        #[cfg(feature = "development")]
        let tabs = vec![
            TabItem::new("scores", "Scores", scores_content),
            TabItem::new("standings", "Standings", standings_content),
            TabItem::new("settings", "Settings", settings_content),
            TabItem::new("demo", "Demo", demo_content),
        ];
        #[cfg(not(feature = "development"))]
        let tabs = vec![
            TabItem::new("scores", "Scores", scores_content),
            TabItem::new("standings", "Standings", standings_content),
            TabItem::new("settings", "Settings", settings_content),
        ];

        TabbedPanel.view(
            &TabbedPanelProps {
                active_key: active_key.into(),
                tabs,
                focused: !state.navigation.focus_in_content
                    && state.navigation.document_stack.is_empty(),
                content_has_focus: state.navigation.focus_in_content,
            },
            &(),
        )
    }

    /// Render main navigation tabs using TabbedPanel (with component states)
    fn render_main_tabs_with_states(
        &self,
        state: &AppState,
        component_states: &mut ComponentStateStore,
    ) -> Element {
        use crate::tui::Tab;

        // Convert Tab enum to string key
        let active_key = match state.navigation.current_tab {
            Tab::Scores => "scores",
            Tab::Standings => "standings",
            Tab::Settings => "settings",
            #[cfg(feature = "development")]
            Tab::Demo => "demo",
        };

        let mut scores_content = Element::None;
        let mut standings_content = Element::None;
        let mut settings_content = Element::None;
        #[cfg(feature = "development")]
        let mut demo_content = Element::None;

        // Determine content for active tab - if document is open, show document instead
        if let Some(doc_entry) = state.navigation.document_stack.last() {
            // Document is open - render it with breadcrumb in the active tab's content area
            let doc_element = self.render_stacked_document(state, doc_entry);
            let breadcrumb_element = self.render_breadcrumb(state);

            // Wrap document with breadcrumb
            let content_with_breadcrumb = vertical(
                [
                    Constraint::Length(2), // Breadcrumb (2 lines: text + divider)
                    Constraint::Min(0),    // Document content
                ],
                vec![breadcrumb_element, doc_element],
            );

            match state.navigation.current_tab {
                Tab::Scores => scores_content = content_with_breadcrumb,
                Tab::Standings => standings_content = content_with_breadcrumb,
                Tab::Settings => settings_content = content_with_breadcrumb,
                #[cfg(feature = "development")]
                Tab::Demo => demo_content = content_with_breadcrumb,
            }
        } else {
            // No panel - render normal tab content
            scores_content = self.render_scores_tab_with_states(state, component_states);
            standings_content = self.render_standings_tab_with_states(state, component_states);
            settings_content = self.render_settings_tab_with_states(state, component_states);
            #[cfg(feature = "development")]
            {
                // Build Demo tab content
                let demo_props = DemoTabProps {
                    focused: state.navigation.focus_in_content,
                    standings: state.data.standings.clone(),
                };
                let demo_state =
                    component_states.get_or_init::<DemoTab>(DEMO_TAB_PATH, &demo_props);
                demo_content = DemoTab.view(&demo_props, demo_state);
            }
        }

        #[cfg(feature = "development")]
        let tabs = vec![
            TabItem::new("scores", "Scores", scores_content),
            TabItem::new("standings", "Standings", standings_content),
            TabItem::new("settings", "Settings", settings_content),
            TabItem::new("demo", "Demo", demo_content),
        ];
        #[cfg(not(feature = "development"))]
        let tabs = vec![
            TabItem::new("scores", "Scores", scores_content),
            TabItem::new("standings", "Standings", standings_content),
            TabItem::new("settings", "Settings", settings_content),
        ];

        TabbedPanel.view(
            &TabbedPanelProps {
                active_key: active_key.into(),
                tabs,
                focused: !state.navigation.focus_in_content
                    && state.navigation.document_stack.is_empty(),
                content_has_focus: state.navigation.focus_in_content,
            },
            &(),
        )
    }

    fn render_stacked_document(&self, state: &AppState, doc_entry: &DocumentStackEntry) -> Element {
        match &doc_entry.document {
            StackedDocument::Boxscore { game_id, .. } => {
                let props = BoxscoreDocumentProps {
                    game_id: *game_id,
                    boxscore: state.data.boxscores.get(game_id).cloned(),
                    loading: state.data.loading.contains(&LoadingKey::Boxscore(*game_id)),
                    team_view: TeamView::Away,
                    selected_index: doc_entry.nav.focus_index,
                    scroll_offset: doc_entry.nav.scroll_offset,
                    focused: true, // Document has focus when it's on the stack
                    animation_frame: state.system.animation_frame,
                };
                BoxscoreDocument.view(&props, &())
            }
            StackedDocument::TeamDetail { abbrev } => {
                // Find the standing for this team
                let standing = state
                    .data
                    .standings
                    .as_ref()
                    .as_ref()
                    .and_then(|standings| {
                        standings
                            .iter()
                            .find(|s| s.team_abbrev.default == *abbrev)
                            .cloned()
                    });
                //
                let props = TeamDetailDocumentProps {
                    team_abbrev: abbrev.clone(),
                    standing,
                    club_stats: state.data.team_roster_stats.get(abbrev).cloned(),
                    loading: state
                        .data
                        .loading
                        .contains(&LoadingKey::TeamRosterStats(abbrev.clone())),
                    selected_index: doc_entry.nav.focus_index,
                    scroll_offset: doc_entry.nav.scroll_offset,
                    animation_frame: state.system.animation_frame,
                    focused: true, // Stacked documents are always focused
                };
                TeamDetailDocument.view(&props, &())
            }
            StackedDocument::PlayerDetail { player_id, .. } => {
                let props = PlayerDetailDocumentProps {
                    player_id: *player_id,
                    player_data: state.data.player_data.get(player_id).cloned(),
                    loading: state
                        .data
                        .loading
                        .contains(&LoadingKey::PlayerStats(*player_id)),
                    selected_index: doc_entry.nav.focus_index,
                    scroll_offset: doc_entry.nav.scroll_offset,
                    animation_frame: state.system.animation_frame,
                    focused: true, // Stacked documents are always focused
                };
                PlayerDetailDocument.view(&props, &())
            }
        }
    }
    /// Render Scores tab content using component state store
    fn render_scores_tab_with_states(
        &self,
        state: &AppState,
        component_states: &mut ComponentStateStore,
    ) -> Element {
        use crate::tui::components::scores_tab::ScoresTab;

        let props = ScoresTabProps {
            schedule: state.data.schedule.clone(),
            game_info: state.data.game_info.clone(),
            period_scores: state.data.period_scores.clone(),
            focused: state.navigation.focus_in_content,
            animation_frame: state.system.animation_frame,
        };

        // Get or initialize component state from the component store
        let scores_state = component_states.get_or_init::<ScoresTab>(SCORES_TAB_PATH, &props);
        ScoresTab.view(&props, scores_state)
    }

    /// Render Scores tab content (old method - kept for compatibility during migration)
    #[allow(dead_code)]
    fn render_scores_tab(&self, state: &AppState) -> Element {
        use crate::tui::components::scores_tab::ScoresTabState;

        let props = ScoresTabProps {
            schedule: state.data.schedule.clone(),
            game_info: state.data.game_info.clone(),
            period_scores: state.data.period_scores.clone(),
            focused: state.navigation.focus_in_content,
            animation_frame: state.system.animation_frame,
        };
        let component_state = ScoresTabState::default();
        ScoresTab.view(&props, &component_state)
    }
    /// Render Standings tab content using component state store
    fn render_standings_tab_with_states(
        &self,
        state: &AppState,
        component_states: &mut ComponentStateStore,
    ) -> Element {
        use crate::tui::components::standings_tab::StandingsTab;

        let props = StandingsTabProps {
            standings: state.data.standings.clone(),
            document_stack: state.navigation.document_stack.clone(),
            focused: state.navigation.focus_in_content,
            config: state.system.config.clone(),
            animation_frame: state.system.animation_frame,
        };

        let standings_state =
            component_states.get_or_init::<StandingsTab>(STANDINGS_TAB_PATH, &props);
        StandingsTab.view(&props, standings_state)
    }

    /// Render Standings tab content (old method - kept for compatibility during migration)
    #[allow(dead_code)]
    fn render_standings_tab(&self, state: &AppState) -> Element {
        use crate::tui::components::standings_tab::StandingsTabState;

        let props = StandingsTabProps {
            standings: state.data.standings.clone(),
            document_stack: state.navigation.document_stack.clone(),
            focused: state.navigation.focus_in_content,
            config: state.system.config.clone(),
            animation_frame: state.system.animation_frame,
        };
        let component_state = StandingsTabState::default();
        StandingsTab.view(&props, &component_state)
    }
    //
    /// Render Settings tab content with component state management
    fn render_settings_tab_with_states(
        &self,
        state: &AppState,
        component_states: &mut ComponentStateStore,
    ) -> Element {
        let props = SettingsTabProps {
            config: state.system.config.clone(),
            selected_category: state.ui.settings.selected_category,
            focused: state.navigation.focus_in_content,
        };

        let settings_state = component_states.get_or_init::<SettingsTab>(SETTINGS_TAB_PATH, &props);
        SettingsTab.view(&props, settings_state)
    }

    /// Render Settings tab content (legacy - without state management)
    fn render_settings_tab(&self, state: &AppState) -> Element {
        use crate::tui::components::SettingsTabState;
        let props = SettingsTabProps {
            config: state.system.config.clone(),
            selected_category: state.ui.settings.selected_category,
            focused: state.navigation.focus_in_content,
        };
        let component_state = SettingsTabState::default();
        SettingsTab.view(&props, &component_state)
    }
    //
    /// Render breadcrumb navigation
    fn render_breadcrumb(&self, state: &AppState) -> Element {
        Element::Widget(Box::new(BreadcrumbWidget::new(
            state.navigation.current_tab,
            state.navigation.document_stack.clone(),
        )))
    }
}
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::AppState;
    //
    #[test]
    fn test_app_renders_with_default_state() {
        let app = App;
        let state = AppState::default();
        //
        let element = app.view(&state, &());
        //
        // Should render a vertical container with 2 children (TabbedPanel + StatusBar)
        match element {
            Element::Container {
                children, layout, ..
            } => {
                assert_eq!(children.len(), 2);
                match layout {
                    crate::tui::component::ContainerLayout::Vertical(constraints) => {
                        assert_eq!(constraints.len(), 2);
                    }
                    _ => panic!("Expected vertical layout"),
                }
            }
            _ => panic!("Expected container element"),
        }
    }
}
