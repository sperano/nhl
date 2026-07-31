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
#[cfg(test)]
use crate::tui::document::FocusableElement;
use crate::tui::document::{DocumentView, FocusableId};
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};
use crate::tui::settings_helpers::ModalOption;
use crate::tui::tab_component::{
    handle_common_message, CommonTabMessage, TabMessage, TabState, BASE_CHROME_LINES,
    SUBTAB_CHROME_LINES,
};
use crate::tui::SettingsCategory;

/// Props for SettingsTab component
#[derive(Clone)]
pub struct SettingsTabProps {
    // Arc'd by the caller so cloning props each render is a pointer bump, not a
    // deep copy of the underlying Config.
    pub config: Arc<Config>,
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
    /// Currently selected settings category. Component-local (not global
    /// state): every other tab already owns its selection state this way,
    /// this was the last holdout (see module docs for background).
    pub selected_category: SettingsCategory,
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

    /// Settings has a nested category subtab bar above its document viewport.
    fn chrome_lines() -> u16 {
        BASE_CHROME_LINES + SUBTAB_CHROME_LINES
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
    /// Cycle to the previous category (wrapping) and rebuild focus metadata.
    ///
    /// Carries `Config` because `update()` has no props access - mirrors
    /// `ActivateSetting`'s existing pattern of passing the config needed for a
    /// document rebuild through the message itself.
    NavigateCategoryLeft(Config),
    /// Cycle to the next category (wrapping) and rebuild focus metadata. See
    /// `NavigateCategoryLeft` for why `Config` is carried in the message.
    NavigateCategoryRight(Config),
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
        use crate::tui::document::FocusContext;

        // Create document and populate focusable metadata
        let mut state = SettingsTabState::default();
        let doc = SettingsDocument::new(state.selected_category, props.config.clone());
        state
            .doc_nav
            .sync_focusables(&doc, &FocusContext::default());
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

            SettingsTabMsg::NavigateCategoryLeft(config) => {
                state.selected_category = match state.selected_category {
                    SettingsCategory::Logging => SettingsCategory::Data,
                    SettingsCategory::Display => SettingsCategory::Logging,
                    SettingsCategory::Data => SettingsCategory::Display,
                };
                Self::rebuild_doc_nav_for_category(state, config);
                Effect::None
            }
            SettingsTabMsg::NavigateCategoryRight(config) => {
                state.selected_category = match state.selected_category {
                    SettingsCategory::Logging => SettingsCategory::Display,
                    SettingsCategory::Display => SettingsCategory::Data,
                    SettingsCategory::Data => SettingsCategory::Logging,
                };
                Self::rebuild_doc_nav_for_category(state, config);
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
                            .focusables
                            .get(focus_idx)
                            .map(|f| f.y)
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
        let active_key = self.category_to_key(state.selected_category);

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
                // NOTE: `SettingsTabMsg::Key` (and thus `handle_key`) is never
                // actually dispatched today - real Settings key routing lives in
                // `keys.rs::handle_settings_tab_keys`, which has access to
                // `state.system.config` to build `NavigateCategoryLeft/Right`.
                // This arm exists only to keep this dead path's `match`
                // exhaustive; it intentionally does not fabricate a `Config`.
                KeyCode::Left | KeyCode::Right => Effect::None,
                KeyCode::Down | KeyCode::Enter => {
                    // Enter browse mode
                    if !state.doc_nav.focusables.is_empty() {
                        state.doc_nav.focus_index = Some(0);
                    }
                    Effect::None
                }
                _ => Effect::None,
            }
        }
    }

    /// Rebuild `doc_nav`'s focus metadata for `state.selected_category`,
    /// discarding any prior focus/scroll position from the previous category.
    ///
    /// Ported from the old `reducers/settings.rs::navigate_category`, which had
    /// to look `SettingsTabState` up in the global component store from
    /// outside; now that category selection is component-local, `update()`
    /// already owns `state` directly.
    fn rebuild_doc_nav_for_category(state: &mut SettingsTabState, config: Config) {
        use crate::tui::document::FocusContext;

        let doc = SettingsDocument::new(state.selected_category, config);
        state.doc_nav = DocumentNavState::default();
        state
            .doc_nav
            .sync_focusables(&doc, &FocusContext::default());
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
            focused_id: state.doc_nav.focused_id(),
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
    focused_id: Option<FocusableId>,
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
        if let Some(id) = self.focused_id.clone() {
            view.focus_id(id);
        }

        // Apply scroll offset
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        let child_ctx = ctx.child(self.focused);

        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(SettingsTabWidget {
            category: self.category,
            config: self.config.clone(),
            focused_id: self.focused_id.clone(),
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
    use crate::tui::document::{Document, FocusContext};

    SettingsDocument::new(category, Arc::new(Config::default()))
        .focusables(&FocusContext::default())
        .into_iter()
        .map(|f| f.id)
        .collect()
}

#[cfg(test)]
#[path = "settings_tab_tests.rs"]
mod tests;
