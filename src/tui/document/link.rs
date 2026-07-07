//! Link system for document navigation
//!
//! Provides the single typed vocabulary for links within documents: pushing a
//! new stacked document, editing/toggling a setting, or jumping to an anchor
//! within the current document.

use crate::tui::types::StackedDocument;

/// Target of a document link
///
/// This is the only link vocabulary in the document system. Consumers match
/// on it directly instead of parsing strings, so the compiler checks link
/// payloads and activation can read the destination straight off the focused
/// element rather than re-deriving it from application data.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Push a stacked document onto the navigation stack (the app's typed
    /// page vocabulary, defined in `tui::types`)
    Push(StackedDocument),

    /// Open the editor for a setting (replaces the old stringly-typed
    /// `edit:` + key prefix format)
    EditSetting(String),

    /// Toggle a boolean setting (replaces the old stringly-typed `toggle:` +
    /// key prefix format)
    ToggleSetting(String),

    /// Navigate to a specific position in the current document (currently
    /// unused in production, kept for future intra-document navigation)
    Anchor(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_target_push() {
        let doc = StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
        };
        let target = LinkTarget::Push(doc.clone());

        match target {
            LinkTarget::Push(StackedDocument::TeamDetail { abbrev }) => {
                assert_eq!(abbrev, "BOS")
            }
            _ => panic!("Expected Push(TeamDetail)"),
        }
    }

    #[test]
    fn test_link_target_edit_setting() {
        let target = LinkTarget::EditSetting("log_level".to_string());

        match target {
            LinkTarget::EditSetting(key) => assert_eq!(key, "log_level"),
            _ => panic!("Expected EditSetting"),
        }
    }

    #[test]
    fn test_link_target_toggle_setting() {
        let target = LinkTarget::ToggleSetting("use_unicode".to_string());

        match target {
            LinkTarget::ToggleSetting(key) => assert_eq!(key, "use_unicode"),
            _ => panic!("Expected ToggleSetting"),
        }
    }

    #[test]
    fn test_link_target_anchor() {
        let target = LinkTarget::Anchor("section-1".to_string());

        match target {
            LinkTarget::Anchor(anchor) => assert_eq!(anchor, "section-1"),
            _ => panic!("Expected Anchor target"),
        }
    }

    #[test]
    fn test_link_target_equality() {
        let target1 = LinkTarget::Push(StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
        });
        let target2 = LinkTarget::Push(StackedDocument::TeamDetail {
            abbrev: "BOS".to_string(),
        });
        let target3 = LinkTarget::Push(StackedDocument::TeamDetail {
            abbrev: "TOR".to_string(),
        });

        assert_eq!(target1, target2);
        assert_ne!(target1, target3);
    }
}
