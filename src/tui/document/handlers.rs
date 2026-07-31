//! Key handling for stacked documents (Boxscore, TeamDetail, PlayerDetail)
//!
//! Stacked documents used to each own a `StackedDocumentHandler` impl with
//! per-type `populate_focusable_metadata()` (and, before F3, per-type
//! `activate()`). With activation generic over `LinkTarget::Push` (F3),
//! focusable metadata collapsed into one `Vec<FocusableElement>` synced by
//! one function (F2), and document construction collapsed into
//! [`super::build_stacked_document`] (F4), there is no per-type logic left:
//! every stacked document is handled by the same sequence of steps. This
//! module holds that single free function.

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::action::Action;
use crate::tui::component::Effect;
use crate::tui::document_nav::{handle_message, DocumentNavState};
use crate::tui::nav_handler::key_to_nav_msg;
use crate::tui::state::DataState;
use crate::tui::types::StackedDocument;

use super::{build_stacked_document, FocusContext, LinkTarget};

/// Handle a key event routed to the document on top of the document stack.
///
/// Syncs focusable metadata on demand (via the same factory the render path
/// uses, so the two can't disagree about what's on screen), then handles
/// navigation via [`key_to_nav_msg`] and delegates `Enter` to activating the
/// focused element's [`LinkTarget`].
///
/// If the document's backing data hasn't loaded yet, `nav`'s existing
/// metadata is left untouched (there is nothing new to sync); navigation
/// simply has nothing to move through until the data arrives.
pub fn handle_stacked_document_key(
    doc: &StackedDocument,
    key: KeyEvent,
    nav: &mut DocumentNavState,
    data: &DataState,
    width: u16,
) -> Effect {
    if let Some(document) = build_stacked_document(doc, data) {
        let ctx = FocusContext::default().with_width(width);
        nav.sync_focusables(document.as_ref(), &ctx);
    }

    if let Some(nav_msg) = key_to_nav_msg(key) {
        return handle_message(nav, &nav_msg);
    }

    if key.code == KeyCode::Enter {
        return match nav.focused_link_target() {
            Some(LinkTarget::Push(target)) => Effect::Action(Action::PushDocument(target.clone())),
            _ => Effect::None,
        };
    }

    Effect::None
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
