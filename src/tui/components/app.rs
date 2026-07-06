use std::sync::Arc;

use crate::tui::component::{vertical, Component, Constraint, Element};
use crate::tui::component_store::ComponentStateStore;
#[cfg(feature = "development")]
use crate::tui::constants::DEMO_TAB_PATH;
use crate::tui::constants::{SCORES_TAB_PATH, SETTINGS_TAB_PATH, STANDINGS_TAB_PATH};
use crate::tui::document::build_stacked_document;
use crate::tui::state::{AppState, LoadingKey};

#[cfg(feature = "development")]
use super::demo_tab::DemoTabProps;
#[cfg(feature = "development")]
use super::DemoTab;
use super::{
    boxscore_document::{BoxscoreDocument, BoxscoreDocumentProps},
    player_detail_document::PlayerDetailDocumentProps,
    scores_tab::ScoresTabProps,
    settings_tab::SettingsTabProps,
    standings_tab::StandingsTabProps,
    team_detail_document::TeamDetailDocumentProps,
    BreadcrumbWidget, PlayerDetailDocument, SettingsTab, StatusBar, TabItem, TabbedPanel,
    TabbedPanelProps, TeamDetailDocument,
};
use crate::tui::state::DocumentStackEntry;
use crate::tui::types::StackedDocument;

/// Root App component
///
/// This is the top-level component that renders the entire application.
/// It uses the global AppState as props and delegates rendering to child components.
pub struct App;

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
            // No panel - render only the active tab's content. The other tabs are hidden by
            // TabbedPanel regardless (it keeps the active TabItem and discards the rest), so
            // building their content every frame was pure waste - including deep-cloning
            // standings/config data for tabs nobody can see.
            match state.navigation.current_tab {
                Tab::Scores => {
                    scores_content = self.render_scores_tab_with_states(state, component_states);
                }
                Tab::Standings => {
                    standings_content =
                        self.render_standings_tab_with_states(state, component_states);
                }
                Tab::Settings => {
                    settings_content =
                        self.render_settings_tab_with_states(state, component_states);
                }
                #[cfg(feature = "development")]
                Tab::Demo => {
                    let demo_props = DemoTabProps {
                        focused: state.navigation.focus_in_content,
                        standings: state.data.standings.clone(),
                    };
                    let demo_state =
                        component_states.get_or_init::<DemoTab>(DEMO_TAB_PATH, &demo_props);
                    demo_content = DemoTab.view(&demo_props, demo_state);
                }
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
        // Single construction path: the same factory the input-handling path
        // uses to sync focus metadata, so render and input can't disagree
        // about which document is on screen.
        let document = build_stacked_document(&doc_entry.document, &state.data);

        match &doc_entry.document {
            StackedDocument::Boxscore { game_id, .. } => {
                let props = BoxscoreDocumentProps {
                    document,
                    loading: state.data.loading.contains(&LoadingKey::Boxscore(*game_id)),
                    selected_index: doc_entry.nav.focus_index,
                    scroll_offset: doc_entry.nav.scroll_offset,
                    focused: true, // Document has focus when it's on the stack
                    animation_frame: state.system.animation_frame,
                };
                BoxscoreDocument.view(&props, &())
            }
            StackedDocument::TeamDetail { abbrev } => {
                let props = TeamDetailDocumentProps {
                    document,
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
                    document,
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
            // Arc-wrap here so every downstream `.clone()` (props, widgets,
            // documents) is a cheap pointer bump instead of a deep Config clone.
            config: Arc::new(state.system.config.clone()),
            animation_frame: state.system.animation_frame,
        };

        let standings_state =
            component_states.get_or_init::<StandingsTab>(STANDINGS_TAB_PATH, &props);
        StandingsTab.view(&props, standings_state)
    }

    /// Render Settings tab content with component state management
    fn render_settings_tab_with_states(
        &self,
        state: &AppState,
        component_states: &mut ComponentStateStore,
    ) -> Element {
        let props = SettingsTabProps {
            // Arc-wrap here so every downstream `.clone()` (props, widgets,
            // documents) is a cheap pointer bump instead of a deep Config clone.
            config: Arc::new(state.system.config.clone()),
            focused: state.navigation.focus_in_content,
        };

        let settings_state = component_states.get_or_init::<SettingsTab>(SETTINGS_TAB_PATH, &props);
        SettingsTab.view(&props, settings_state)
    }

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
    use crate::tui::Tab;
    //
    #[test]
    fn test_app_renders_with_default_state() {
        let app = App;
        let state = AppState::default();
        let mut component_states = ComponentStateStore::new();
        //
        let element = app.build_with_component_states(&state, &mut component_states);
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

    #[test]
    fn test_build_only_initializes_active_tab_component_state() {
        // Only the active tab's content should be built each frame - the others are hidden by
        // TabbedPanel regardless, so building them is pure waste (deep-cloning standings/config
        // data nobody can see). Verify this by checking that only the active tab's component
        // state gets initialized, not the inactive ones.
        let app = App;
        let mut state = AppState::default();
        state.navigation.current_tab = Tab::Scores;
        let mut component_states = ComponentStateStore::new();

        app.build_with_component_states(&state, &mut component_states);

        assert!(component_states
            .get::<crate::tui::components::scores_tab::ScoresTabState>(SCORES_TAB_PATH)
            .is_some());
        assert!(component_states
            .get::<crate::tui::components::standings_tab::StandingsTabState>(STANDINGS_TAB_PATH)
            .is_none());
        assert!(component_states
            .get::<crate::tui::components::settings_tab::SettingsTabState>(SETTINGS_TAB_PATH)
            .is_none());
    }
}
