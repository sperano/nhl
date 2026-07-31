//! Demo tab component showcasing the document system
//!
//! This tab demonstrates the document system capabilities including:
//! - Document elements (headings, text, links, separators)
//! - Viewport-based scrolling
//! - Tab/Shift-Tab focus navigation through focusable elements
//! - Autoscrolling to keep focused elements visible
//! - Embedded tables (league standings) rendered at natural height

use std::borrow::Cow;
use std::sync::Arc;

use nhl_api::Standing;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::component_message_impl;
use crate::config::RenderContext;
use crate::tui::action::Action;
use crate::tui::component::{Component, Effect, Element, ElementWidget};
use crate::tui::components::create_standings_table_with_selection;
use crate::tui::document::{
    Document, DocumentBuilder, DocumentElement, DocumentView, FocusContext, FocusableId, LinkTarget,
};
use crate::tui::document_nav::{DocumentNavMsg, DocumentNavState};
use crate::tui::helpers::StandingsSorting;
use crate::tui::tab_component::{
    activate_focused_link, enter_item_focus, exit_item_focus, handle_common_message,
    CommonTabMessage, TabMessage,
};
use crate::tui::types::StackedDocument;

/// Props for the Demo tab
#[derive(Clone)]
pub struct DemoTabProps {
    /// Whether this tab has focus
    pub focused: bool,
    /// Standings data for demonstrating embedded tables
    pub standings: Arc<Option<Vec<Standing>>>,
}

/// Messages that can be sent to the Demo tab
#[derive(Clone, Debug)]
pub enum DemoTabMsg {
    /// Navigate up request (ESC in browse mode, returns to tab bar otherwise)
    /// Returns Effect::Handled if consumed, Effect::None if should bubble up
    NavigateUp,

    /// Document navigation
    DocNav(DocumentNavMsg),
    /// Update viewport height
    UpdateViewportHeight(u16),
    /// Activate the currently focused link (team or player)
    ActivateLink,
    /// Enter focus mode (from tab bar) - focuses first item and sets content focus
    EnterFocus,
    /// Exit focus mode (via Escape) - clears selection and returns to tab bar
    ExitFocus,
}

impl TabMessage for DemoTabMsg {
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
component_message_impl!(DemoTabMsg, DemoTab, DocumentNavState);

/// Demo tab component
#[derive(Default)]
pub struct DemoTab;

impl Component for DemoTab {
    type Props = DemoTabProps;
    type State = crate::tui::document_nav::DocumentNavState;
    type Message = DemoTabMsg;

    fn init(props: &Self::Props) -> Self::State {
        // Build initial state with focusable metadata from the document
        let standings = props.standings.as_ref().clone();
        let doc = DemoDocument::new(standings);
        crate::tui::document_nav::DocumentNavState {
            focusables: doc.focusables(&FocusContext::default()),
            ..Default::default()
        }
    }

    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        // Handle common tab messages (DocNav, UpdateViewportHeight, NavigateUp)
        if let Some(effect) = handle_common_message(msg.as_common(), state) {
            return effect;
        }

        // Handle tab-specific messages
        match msg {
            DemoTabMsg::ActivateLink => activate_focused_link(state),

            // Focus first item (global focus_in_content already set by reducer)
            DemoTabMsg::EnterFocus => enter_item_focus(state),

            // Clear selection and return to tab bar
            DemoTabMsg::ExitFocus => {
                exit_item_focus(state, Effect::Action(Action::ExitContentFocus))
            }

            // Common messages already handled above
            DemoTabMsg::DocNav(_)
            | DemoTabMsg::UpdateViewportHeight(_)
            | DemoTabMsg::NavigateUp => {
                unreachable!("Common messages should be handled by handle_common_message")
            }
        }
    }

    fn view(&self, props: &Self::Props, state: &Self::State) -> Element {
        Element::Widget(Box::new(DemoTabWidget {
            focused: props.focused,
            focused_id: state.focused_id(),
            has_item_focus: state.focus_index.is_some(),
            scroll_offset: state.scroll_offset,
            standings: props.standings.clone(),
            tab_selections: state.doc_tab_selections.clone(),
        }))
    }
}

/// ID for the tabs element in the demo document
const DEMO_TABS_ID: &str = "demo_tabs";

/// Widget for rendering the Demo tab
struct DemoTabWidget {
    focused: bool,
    focused_id: Option<FocusableId>,
    /// Whether item-level focus is active (drives dim/bright rendering even
    /// when the focus index doesn't resolve to a known focusable).
    has_item_focus: bool,
    scroll_offset: u16,
    standings: Arc<Option<Vec<Standing>>>,
    tab_selections: std::collections::HashMap<String, usize>,
}

impl ElementWidget for DemoTabWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, ctx: &RenderContext) {
        // Create document view with state from AppState
        let standings = (*self.standings).clone();
        let doc = Arc::new(DemoDocument::new(standings));
        let mut view = DocumentView::new(doc, area.height);

        // Apply focus state from AppState
        if let Some(id) = self.focused_id.clone() {
            view.focus_id(id);
        }

        // Apply scroll offset from AppState
        view.set_scroll_offset(self.scroll_offset);

        // Create child RenderContext with our focus state
        // Document is only focused when navigating items within the document
        let child_ctx = ctx
            .child(self.focused && self.has_item_focus)
            .with_tab_selections(self.tab_selections.clone());

        view.render(area, buf, &child_ctx);
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(DemoTabWidget {
            focused: self.focused,
            focused_id: self.focused_id.clone(),
            has_item_focus: self.has_item_focus,
            scroll_offset: self.scroll_offset,
            standings: self.standings.clone(),
            tab_selections: self.tab_selections.clone(),
        })
    }

    fn preferred_height(&self) -> Option<u16> {
        None // Fills available space
    }
}

/// Demo document showcasing all document element types
pub struct DemoDocument {
    standings: Option<Vec<Standing>>,
}

impl DemoDocument {
    pub fn new(standings: Option<Vec<Standing>>) -> Self {
        Self { standings }
    }

    /// Build the content for the Standings tab
    fn build_standings_tab_content(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        const TABLE_NAME: &str = "standings";

        let builder = DocumentBuilder::new()
            .heading(2, "League Standings")
            .spacer(1)
            .text("This demonstrates the shared standings table embedded in a document:");

        match &self.standings {
            Some(standings) if !standings.is_empty() => {
                // Sort by points (highest first) using the shared sorting trait
                let mut sorted = standings.clone();
                sorted.sort_by_points_desc();

                // Use the shared standings table component with focus state
                let table = create_standings_table_with_selection(
                    sorted,
                    focus.focused_table_row(TABLE_NAME),
                );

                builder.spacer(1).table(TABLE_NAME, table).build()
            }
            _ => builder
                .text("(No standings data loaded - try refreshing)")
                .build(),
        }
    }

    /// Build the content for the Players tab
    fn build_players_tab_content(&self) -> Vec<DocumentElement> {
        DocumentBuilder::new()
            .heading(2, "Player Stats")
            .spacer(1)
            .text("Player statistics placeholder.")
            .build()
    }
}

impl Document for DemoDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        // Build the Standings tab content
        let standings_content = self.build_standings_tab_content(focus);

        // Build the Players tab content
        let players_content = self.build_players_tab_content();

        DocumentBuilder::new()
            .heading(1, "Document System Demo")
            .spacer(1)
            .text("This tab demonstrates the new document system for the NHL TUI.")
            .text("Press Tab/Shift-Tab to navigate, Left/Right to switch tabs.")
            .spacer(1)
            .separator()
            .spacer(1)
            // Embedded tabs demonstrating tabs-within-documents
            .tabs_with_focus(
                DEMO_TABS_ID,
                vec![
                    ("standings", "Standings", standings_content),
                    ("players", "Players", players_content),
                ],
                focus,
            )
            .spacer(1)
            .separator()
            .spacer(1)
            .heading(2, "Features")
            .text("- Viewport-based scrolling for unlimited content height")
            .text("- Tab/Shift-Tab navigation cycles through focusable elements")
            .text("- Left/Right arrows switch between embedded tabs")
            .text("- Autoscrolling keeps the focused element visible")
            .text("- Smart padding positions elements comfortably in view")
            .spacer(1)
            .heading(2, "Example Links")
            .text("These links demonstrate focusable elements:")
            .spacer(1)
            .link_with_focus(
                "link_bos",
                "Boston Bruins",
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "BOS".to_string(),
                    season: None,
                }),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "link_tor",
                "Toronto Maple Leafs",
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "TOR".to_string(),
                    season: None,
                }),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "link_nyr",
                "New York Rangers",
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "NYR".to_string(),
                    season: None,
                }),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "link_mtl",
                "Montreal Canadiens",
                LinkTarget::Push(StackedDocument::TeamDetail {
                    abbrev: "MTL".to_string(),
                    season: None,
                }),
                focus,
            )
            .spacer(1)
            .separator()
            .spacer(1)
            .heading(2, "Implementation Notes")
            .text("The document system consists of several modules:")
            .spacer(1)
            .text("- viewport.rs: Manages scroll position and visible range")
            .text("- focus.rs: Tracks focusable elements and navigation")
            .text("- elements.rs: Document element types (text, headings, links)")
            .text("- builder.rs: Fluent API for building documents")
            .text("- mod.rs: Document trait and DocumentView container")
            .spacer(1)
            .text("Each document implements the Document trait to define its")
            .text("content structure. DocumentView manages the viewport and")
            .text("focus state for rendering and interaction.")
            .spacer(1)
            .text("End of demo document.")
            .build()
    }

    fn title(&self) -> Cow<'static, str> {
        Cow::Borrowed("Document System Demo")
    }

    fn id(&self) -> Cow<'static, str> {
        Cow::Borrowed("demo")
    }
}

#[cfg(test)]
#[path = "demo_tab_tests.rs"]
mod tests;
