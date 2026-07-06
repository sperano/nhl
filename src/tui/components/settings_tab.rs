//! Settings tab component - displays configuration settings
//!
//! Uses the document system to display settings as focusable links.
//! Settings can be toggled (booleans) or edited (strings/numbers).

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::component_message_impl;
use crate::config::{Config, RenderContext};
use crate::tui::component::{Component, Effect, Element, ElementWidget};
use crate::tui::components::{SettingsDocument, TabItem, TabbedPanel, TabbedPanelProps};
use crate::tui::document::DocumentView;
#[cfg(test)]
use crate::tui::document::FocusableId;
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};
use crate::tui::settings_helpers::ModalOption;
use crate::tui::tab_component::{handle_common_message, CommonTabMessage, TabMessage, TabState};
use crate::tui::SettingsCategory;

/// Props for SettingsTab component
#[derive(Clone)]
pub struct SettingsTabProps {
    // Arc'd by the caller so cloning props each render is a pointer bump, not a
    // deep copy of the underlying Config.
    pub config: Arc<Config>,
    pub selected_category: SettingsCategory,
    pub focused: bool,
}

/// Modal navigation messages
#[derive(Clone, Debug, PartialEq)]
pub enum ModalMsg {
    Up,
    Down,
    Confirm,
    Cancel,
}

/// Modal state for list selections (log_level, theme)
#[derive(Debug, Clone)]
pub struct ModalState {
    pub options: Vec<ModalOption>,
    pub selected_index: usize,
    pub setting_key: String,
    pub position_x: u16,
    pub position_y: u16,
}

/// State for SettingsTab component
#[derive(Debug, Clone, Default)]
pub struct SettingsTabState {
    /// Document navigation state for the current category
    pub doc_nav: DocumentNavState,
    /// Modal state for list selections (log_level, theme)
    pub modal: Option<ModalState>,
}

impl TabState for SettingsTabState {
    fn doc_nav(&self) -> &DocumentNavState {
        &self.doc_nav
    }

    fn doc_nav_mut(&mut self) -> &mut DocumentNavState {
        &mut self.doc_nav
    }
}

/// Messages that can be sent to the Settings tab
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SettingsTabMsg {
    /// Key event when this tab is focused
    Key(KeyEvent),

    /// Navigate up request (ESC closes modal or clears item focus)
    NavigateUp,

    /// Document navigation
    DocNav(DocumentNavMsg),
    /// Update viewport height
    UpdateViewportHeight(u16),
    /// Change category (triggered by tab navigation)
    SetCategory(SettingsCategory),
    /// Activate the currently focused setting (includes config for modal initialization)
    ActivateSetting(Config),
    /// Modal navigation
    Modal(ModalMsg),
}

impl TabMessage for SettingsTabMsg {
    fn as_common(&self) -> Option<CommonTabMessage<'_>> {
        match self {
            Self::DocNav(msg) => Some(CommonTabMessage::DocNav(msg)),
            Self::UpdateViewportHeight(h) => Some(CommonTabMessage::UpdateViewportHeight(*h)),
            // Note: NavigateUp is NOT handled by common - SettingsTab has special modal logic
            _ => None,
        }
    }

    fn from_doc_nav(msg: DocumentNavMsg) -> Self {
        Self::DocNav(msg)
    }
}

// Use macro to eliminate ComponentMessageTrait boilerplate
component_message_impl!(SettingsTabMsg, SettingsTab, SettingsTabState);

/// SettingsTab component - displays settings with category tabs
#[derive(Default)]
pub struct SettingsTab;

impl Component for SettingsTab {
    type Props = SettingsTabProps;
    type State = SettingsTabState;
    type Message = SettingsTabMsg;

    fn init(props: &Self::Props) -> Self::State {
        use crate::tui::components::SettingsDocument;
        use crate::tui::document::Document;

        // Create document and populate focusable metadata
        let doc = SettingsDocument::new(props.selected_category, props.config.clone());
        let mut state = SettingsTabState::default();
        state.doc_nav.focusable_positions = doc.focusable_positions();
        state.doc_nav.focusable_ids = doc.focusable_ids();
        state.doc_nav.focusable_row_positions = doc.focusable_row_positions();
        state.doc_nav.link_targets = doc.focusable_link_targets();
        state
    }

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        use crate::tui::action::{Action, SettingsAction};

        // Handle common tab messages (DocNav, UpdateViewportHeight)
        // Note: NavigateUp is NOT in common for SettingsTab due to special modal handling
        if let Some(effect) = handle_common_message(msg.as_common(), state) {
            return effect;
        }

        // Handle tab-specific messages
        match msg {
            SettingsTabMsg::Key(key) => self.handle_key(key, state),

            SettingsTabMsg::NavigateUp => {
                // Priority 1: Close modal if open
                if state.modal.is_some() {
                    state.modal = None;
                    return Effect::Handled;
                }
                // Priority 2: Clear item focus if active
                if state.has_item_focus() {
                    state.clear_item_focus();
                    return Effect::Handled;
                }
                // Otherwise let it bubble up
                Effect::None
            }

            SettingsTabMsg::SetCategory(_category) => {
                // Category changed - reset document state and rebuild focusable metadata
                *state.doc_nav_mut() = DocumentNavState::default();

                // Need to get config - but we don't have props here!
                // This will be populated by the reducer which has access to the config
                // For now, just reset the state - the reducer will trigger a re-init
                Effect::None
            }
            SettingsTabMsg::ActivateSetting(config) => {
                use crate::tui::document::LinkTarget;
                use crate::tui::settings_helpers::{
                    find_initial_modal_index, get_setting_modal_options,
                };

                let Some(focus_idx) = state.doc_nav().focus_index else {
                    return Effect::None;
                };

                // The setting's own link declares whether Enter should toggle
                // it directly or open a selection modal (see settings_document.rs);
                // this used to be re-derived here from a hardcoded list of key
                // names kept in sync by hand.
                match state.doc_nav().focused_link_target().cloned() {
                    Some(LinkTarget::ToggleSetting(key)) => {
                        Effect::Action(Action::SettingsAction(SettingsAction::ToggleBoolean(key)))
                    }
                    Some(LinkTarget::EditSetting(key)) => {
                        let options = get_setting_modal_options(&key);
                        let selected_index = find_initial_modal_index(&config, &key);

                        let position_y = state
                            .doc_nav()
                            .focusable_positions
                            .get(focus_idx)
                            .copied()
                            .unwrap_or(0);
                        let position_x = 10;

                        state.modal = Some(ModalState {
                            options,
                            selected_index,
                            setting_key: key,
                            position_x,
                            position_y,
                        });

                        Effect::None
                    }
                    _ => Effect::None,
                }
            }
            SettingsTabMsg::Modal(modal_msg) => {
                if let Some(modal) = &mut state.modal {
                    match modal_msg {
                        ModalMsg::Up => {
                            modal.selected_index = modal.selected_index.saturating_sub(1);
                            Effect::None
                        }
                        ModalMsg::Down => {
                            modal.selected_index = (modal.selected_index + 1)
                                .min(modal.options.len().saturating_sub(1));
                            Effect::None
                        }
                        ModalMsg::Cancel => {
                            state.modal = None;
                            Effect::None
                        }
                        ModalMsg::Confirm => {
                            // Get the selected option's ID (not display name)
                            let selected_id = modal
                                .options
                                .get(modal.selected_index)
                                .map(|opt| opt.id.clone())
                                .unwrap_or_default();
                            let setting_key = modal.setting_key.clone();

                            // Close the modal
                            state.modal = None;

                            // Dispatch action to update the setting
                            Effect::Action(Action::SettingsAction(SettingsAction::UpdateSetting {
                                key: setting_key,
                                value: selected_id,
                            }))
                        }
                    }
                } else {
                    Effect::None
                }
            }

            // Common messages already handled above
            SettingsTabMsg::DocNav(_) | SettingsTabMsg::UpdateViewportHeight(_) => {
                unreachable!("Common messages should be handled by handle_common_message")
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        // Create tabs for each category
        let tabs = vec![
            TabItem::new(
                "logging",
                "Logging",
                self.render_settings_document(SettingsCategory::Logging, props, state),
            ),
            TabItem::new(
                "display",
                "Display",
                self.render_settings_document(SettingsCategory::Display, props, state),
            ),
            TabItem::new(
                "data",
                "Data",
                self.render_settings_document(SettingsCategory::Data, props, state),
            ),
        ];

        // Active key based on selected category
        let active_key = self.category_to_key(props.selected_category);

        // Create the base element (tabbed panel)
        let base_element = TabbedPanel.view(
            &TabbedPanelProps {
                active_key,
                tabs,
                focused: props.focused && !state.has_item_focus(),
                content_has_focus: props.focused && state.has_item_focus(),
            },
            &(),
        );

        // If modal is open, wrap in a widget that renders both the base and the modal
        if let Some(modal) = &state.modal {
            // Extract display names for the modal widget
            let display_names: Vec<String> = modal
                .options
                .iter()
                .map(|opt| opt.display_name.clone())
                .collect();

            Element::Widget(Box::new(SettingsTabWithModal {
                base_element,
                modal_options: display_names,
                modal_selected_index: modal.selected_index,
                modal_position_x: modal.position_x,
                modal_position_y: modal.position_y,
            }))
        } else {
            base_element
        }
    }
}

impl SettingsTab {
    /// Handle key events when this tab is focused
    fn handle_key(&mut self, key: KeyEvent, state: &mut SettingsTabState) -> Effect {
        use crate::tui::action::{Action, SettingsAction};
        use crate::tui::nav_handler::key_to_nav_msg;

        // If modal is open, handle modal navigation
        if state.modal.is_some() {
            return match key.code {
                KeyCode::Up => self.update(SettingsTabMsg::Modal(ModalMsg::Up), state),
                KeyCode::Down => self.update(SettingsTabMsg::Modal(ModalMsg::Down), state),
                KeyCode::Enter => self.update(SettingsTabMsg::Modal(ModalMsg::Confirm), state),
                KeyCode::Esc => self.update(SettingsTabMsg::Modal(ModalMsg::Cancel), state),
                _ => Effect::None,
            };
        }

        // Check if an item has focus
        let has_item_focus = state.doc_nav.focus_index.is_some();

        if has_item_focus {
            // Item focus mode - navigate settings

            // Handle Escape to clear item focus
            if key.code == KeyCode::Esc {
                return self.update(SettingsTabMsg::NavigateUp, state);
            }

            // Try standard navigation first (handles Tab, arrows, PageUp/Down, etc.)
            if let Some(nav_msg) = key_to_nav_msg(key) {
                return crate::tui::document_nav::handle_message(&mut state.doc_nav, &nav_msg);
            }

            // Handle Enter to activate the setting
            match key.code {
                KeyCode::Enter => {
                    // We need access to config, which is in props
                    // This will be handled by dispatching an action
                    Effect::Action(Action::SettingsAction(SettingsAction::ToggleBoolean(
                        "placeholder".to_string(),
                    )))
                }
                _ => Effect::None,
            }
        } else {
            // Category selection mode
            match key.code {
                KeyCode::Left => {
                    Effect::Action(Action::SettingsAction(SettingsAction::NavigateCategoryLeft))
                }
                KeyCode::Right => Effect::Action(Action::SettingsAction(
                    SettingsAction::NavigateCategoryRight,
                )),
                KeyCode::Down | KeyCode::Enter => {
                    // Enter browse mode
                    if !state.doc_nav.focusable_positions.is_empty() {
                        state.doc_nav.focus_index = Some(0);
                    }
                    Effect::None
                }
                _ => Effect::None,
            }
        }
    }

    /// Convert category to tab key
    fn category_to_key(&self, category: SettingsCategory) -> String {
        match category {
            SettingsCategory::Logging => "logging".to_string(),
            SettingsCategory::Display => "display".to_string(),
            SettingsCategory::Data => "data".to_string(),
        }
    }

    /// Render the settings document for a category
    fn render_settings_document(
        &self,
        category: SettingsCategory,
        props: &SettingsTabProps,
        state: &SettingsTabState,
    ) -> Element {
        // Document is only focused when in browse mode (navigating within the document)
        Element::Widget(Box::new(SettingsTabWidget {
            category,
            config: props.config.clone(),
            focus_index: state.doc_nav.focus_index,
            scroll_offset: state.doc_nav.scroll_offset,
            viewport_height: state.doc_nav.viewport_height,
            focused: props.focused && state.has_item_focus(),
        }))
    }
}

/// Widget for rendering the Settings tab with modal overlay
struct SettingsTabWithModal {
    base_element: Element,
    modal_options: Vec<String>,
    modal_selected_index: usize,
    modal_position_x: u16,
    modal_position_y: u16,
}

impl ElementWidget for SettingsTabWithModal {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        use crate::tui::renderer::Renderer;
        use crate::tui::widgets::ListModalWidget;

        // Render the base element first
        let mut renderer = Renderer::new();
        renderer.render(self.base_element.clone(), area, buf, ctx);

        // Render the modal on top
        let modal = ListModalWidget::new(
            self.modal_options.clone(),
            self.modal_selected_index,
            self.modal_position_x,
            self.modal_position_y,
        );
        modal.render(area, buf, ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(SettingsTabWithModal {
            base_element: self.base_element.clone(),
            modal_options: self.modal_options.clone(),
            modal_selected_index: self.modal_selected_index,
            modal_position_x: self.modal_position_x,
            modal_position_y: self.modal_position_y,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

/// Widget for rendering the Settings tab content
struct SettingsTabWidget {
    category: SettingsCategory,
    config: Arc<Config>,
    focus_index: Option<usize>,
    scroll_offset: u16,
    viewport_height: u16,
    /// Whether this widget has focus (affects dim/bright rendering)
    focused: bool,
}

impl ElementWidget for SettingsTabWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create document for the current category
        let doc = Arc::new(SettingsDocument::new(self.category, self.config.clone()));
        let mut view = DocumentView::new(doc, area.height);

        // Apply focus state
        if let Some(idx) = self.focus_index {
            view.focus_by_index(idx);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        let child_ctx = RenderContext::new(ctx.config, self.focused);

        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(SettingsTabWidget {
            category: self.category,
            config: self.config.clone(),
            focus_index: self.focus_index,
            scroll_offset: self.scroll_offset,
            viewport_height: self.viewport_height,
            focused: self.focused,
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

/// Helper to get focusable IDs for a settings category (for testing)
///
/// Reads focusable IDs from the real `SettingsDocument` rather than a hardcoded
/// list, so this can't drift from actual navigation behavior.
#[cfg(test)]
fn get_focusable_ids_for_category(category: SettingsCategory) -> Vec<FocusableId> {
    use crate::tui::document::Document;

    SettingsDocument::new(category, Arc::new(Config::default())).focusable_ids()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::testing::assert_buffer;

    #[test]
    fn test_data_settings_navigation_skips_inert_rows() {
        // Data category renders "Refresh Interval" (inert), then "Western
        // Teams First" (focusable), then "Time Format" (inert). Focus index 0
        // refers to the first *focusable* row, so the selector marker should
        // land directly on "Western Teams First" and never on the two inert,
        // display-only rows.
        let widget = SettingsTabWidget {
            category: SettingsCategory::Data,
            config: Arc::new(Config::default()),
            focus_index: Some(0),
            scroll_offset: 0,
            viewport_height: 8,
            focused: true,
        };

        let area = Rect::new(0, 0, 60, 8);
        let mut buf = Buffer::empty(area);
        let display_config = Config::default().display;
        let ctx = RenderContext::focused(&display_config);
        widget.render(area, &mut buf, &ctx);

        assert_buffer(
            &buf,
            &[
                "",
                " Refresh Interval:      60 seconds",
                "",
                " ▶ Western Teams First:   false",
                "",
                " Time Format:           %H:%M:%S",
                "",
                "",
            ],
        );
    }

    #[test]
    fn test_settings_tab_init() {
        let props = SettingsTabProps {
            config: Arc::new(Config::default()),
            selected_category: SettingsCategory::Logging,
            focused: false,
        };
        let state = SettingsTab::init(&props);

        assert_eq!(state.doc_nav.focus_index, None);
        assert_eq!(state.doc_nav.scroll_offset, 0);
    }

    #[test]
    fn test_settings_tab_renders() {
        let settings_tab = SettingsTab;
        let props = SettingsTabProps {
            config: Arc::new(Config::default()),
            selected_category: SettingsCategory::Logging,
            focused: false,
        };
        let state = SettingsTabState::default();

        let element = settings_tab.view(&props, &state);

        // Should create a container element (from TabbedPanel's vertical layout)
        match element {
            Element::Container { children, .. } => {
                // Container created successfully with children
                assert_eq!(children.len(), 2); // Tab bar + content
            }
            _ => panic!("Expected container element"),
        }
    }

    #[test]
    fn test_category_to_key() {
        let settings_tab = SettingsTab;

        assert_eq!(
            settings_tab.category_to_key(SettingsCategory::Logging),
            "logging"
        );
        assert_eq!(
            settings_tab.category_to_key(SettingsCategory::Display),
            "display"
        );
        assert_eq!(settings_tab.category_to_key(SettingsCategory::Data), "data");
    }

    #[test]
    fn test_doc_nav_message_handling() {
        use crate::tui::document_nav::DocumentNavMsg;

        let mut component = SettingsTab;
        let mut state = SettingsTabState::default();

        // Set up some focusable elements
        state.doc_nav.focusable_positions = vec![0, 2, 4];
        state.doc_nav.focus_index = Some(0);

        let effect = component.update(
            SettingsTabMsg::DocNav(DocumentNavMsg::FocusNext),
            &mut state,
        );

        assert_eq!(state.doc_nav.focus_index, Some(1));
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_update_viewport_height() {
        let mut component = SettingsTab;
        let mut state = SettingsTabState::default();

        let effect = component.update(SettingsTabMsg::UpdateViewportHeight(50), &mut state);

        assert_eq!(state.doc_nav.viewport_height, 50);
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_set_category_resets_state() {
        let mut component = SettingsTab;
        let mut state = SettingsTabState::default();

        // Set some state
        state.doc_nav.focus_index = Some(2);
        state.doc_nav.scroll_offset = 10;

        let effect = component.update(
            SettingsTabMsg::SetCategory(SettingsCategory::Display),
            &mut state,
        );

        // State should be reset
        assert_eq!(state.doc_nav.focus_index, None);
        assert_eq!(state.doc_nav.scroll_offset, 0);
        assert!(matches!(effect, Effect::None));
    }

    #[test]
    fn test_get_focusable_ids_logging() {
        // "log_file" is display-only (not editable via the UI), so only
        // "log_level" is focusable.
        let ids = get_focusable_ids_for_category(SettingsCategory::Logging);
        assert_eq!(ids, vec![FocusableId::Link("log_level".to_string())]);
    }

    #[test]
    fn test_get_focusable_ids_display() {
        let ids = get_focusable_ids_for_category(SettingsCategory::Display);
        assert_eq!(
            ids,
            vec![
                FocusableId::Link("theme".to_string()),
                FocusableId::Link("use_unicode".to_string()),
            ]
        );
    }

    #[test]
    fn test_get_focusable_ids_data() {
        // "refresh_interval" and "time_format" are display-only (not editable
        // via the UI), so only "western_teams_first" is focusable.
        let ids = get_focusable_ids_for_category(SettingsCategory::Data);
        assert_eq!(
            ids,
            vec![FocusableId::Link("western_teams_first".to_string())]
        );
    }

    #[test]
    fn test_activate_setting_toggle_dispatches_toggle_boolean() {
        use crate::tui::action::{Action, SettingsAction};
        use crate::tui::document::LinkTarget;

        let mut settings_tab = SettingsTab;
        let props = SettingsTabProps {
            config: Arc::new(Config::default()),
            selected_category: SettingsCategory::Data,
            focused: true,
        };
        let mut state = SettingsTab::init(&props);
        state.doc_nav.focus_index = Some(0);

        // Sanity check: the "western_teams_first" row is declared as a
        // ToggleSetting link by settings_document.rs.
        assert_eq!(
            state.doc_nav.link_targets,
            vec![Some(LinkTarget::ToggleSetting(
                "western_teams_first".to_string()
            ))]
        );

        let effect = settings_tab.update(
            SettingsTabMsg::ActivateSetting(props.config.as_ref().clone()),
            &mut state,
        );

        match effect {
            Effect::Action(Action::SettingsAction(SettingsAction::ToggleBoolean(key))) => {
                assert_eq!(key, "western_teams_first");
            }
            _ => panic!("Expected ToggleBoolean action, got {:?}", effect),
        }
        assert!(state.modal.is_none());
    }

    #[test]
    fn test_activate_setting_edit_opens_modal() {
        let mut settings_tab = SettingsTab;
        let props = SettingsTabProps {
            config: Arc::new(Config::default()),
            selected_category: SettingsCategory::Logging,
            focused: true,
        };
        let mut state = SettingsTab::init(&props);
        state.doc_nav.focus_index = Some(0);

        let effect = settings_tab.update(
            SettingsTabMsg::ActivateSetting(props.config.as_ref().clone()),
            &mut state,
        );

        assert!(matches!(effect, Effect::None));
        let modal = state.modal.expect("expected modal to open for log_level");
        assert_eq!(modal.setting_key, "log_level");
    }

    #[test]
    fn test_activate_setting_without_focus_does_nothing() {
        let mut settings_tab = SettingsTab;
        let props = SettingsTabProps {
            config: Arc::new(Config::default()),
            selected_category: SettingsCategory::Logging,
            focused: true,
        };
        let mut state = SettingsTab::init(&props);
        // No focus set.

        let effect = settings_tab.update(
            SettingsTabMsg::ActivateSetting(props.config.as_ref().clone()),
            &mut state,
        );

        assert!(matches!(effect, Effect::None));
        assert!(state.modal.is_none());
    }
}
