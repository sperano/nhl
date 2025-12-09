//! Link system for document navigation
//!
//! Provides types for representing links within documents that can navigate
//! to other documents, anchors within the current document, or trigger actions.

use crate::commands::standings::GroupBy;
use nhl_api::GameDate;

/// Target of a document link
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Navigate to another document
    Document(DocumentLink),

    /// Navigate to a specific position in current document
    Anchor(String),

    /// External action (e.g., open modal, trigger command)
    Action(String),
}

/// Link to another document
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentLink {
    /// Document type
    pub doc_type: DocumentType,
    /// Parameters for the document
    pub params: LinkParams,
}

impl DocumentLink {
    /// Create a new document link
    pub fn new(doc_type: DocumentType, params: LinkParams) -> Self {
        Self { doc_type, params }
    }

    /// Create a link to a team document
    pub fn team(abbrev: impl Into<String>) -> Self {
        Self {
            doc_type: DocumentType::Team,
            params: LinkParams::Team {
                abbrev: abbrev.into(),
            },
        }
    }

    /// Create a link to a player document
    pub fn player(id: i64) -> Self {
        Self {
            doc_type: DocumentType::Player,
            params: LinkParams::Player { id },
        }
    }

    /// Create a link to a game document
    pub fn game(id: i64) -> Self {
        Self {
            doc_type: DocumentType::Game,
            params: LinkParams::Game { id },
        }
    }

    /// Create a link to standings with a specific view
    pub fn standings(group_by: GroupBy) -> Self {
        Self {
            doc_type: DocumentType::Standings,
            params: LinkParams::StandingsView { group_by },
        }
    }

    /// Create a link to schedule for a specific date
    pub fn schedule(date: GameDate) -> Self {
        Self {
            doc_type: DocumentType::Schedule,
            params: LinkParams::ScheduleDate { date },
        }
    }
}

/// Types of documents that can be linked to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentType {
    Team,
    Player,
    Game,
    Standings,
    Schedule,
}

/// Parameters for different document types
#[derive(Debug, Clone, PartialEq)]
pub enum LinkParams {
    Team { abbrev: String },
    Player { id: i64 },
    Game { id: i64 },
    StandingsView { group_by: GroupBy },
    ScheduleDate { date: GameDate },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_target_document() {
        let link = DocumentLink::team("BOS");
        let target = LinkTarget::Document(link.clone());

        match target {
            LinkTarget::Document(doc_link) => {
                assert_eq!(doc_link.doc_type, DocumentType::Team);
                match doc_link.params {
                    LinkParams::Team { abbrev } => assert_eq!(abbrev, "BOS"),
                    _ => panic!("Expected Team params"),
                }
            }
            _ => panic!("Expected Document target"),
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
    fn test_link_target_action() {
        let target = LinkTarget::Action("open_modal".to_string());

        match target {
            LinkTarget::Action(action) => assert_eq!(action, "open_modal"),
            _ => panic!("Expected Action target"),
        }
    }

    #[test]
    fn test_document_link_team() {
        let link = DocumentLink::team("TOR");

        assert_eq!(link.doc_type, DocumentType::Team);
        match link.params {
            LinkParams::Team { abbrev } => assert_eq!(abbrev, "TOR"),
            _ => panic!("Expected Team params"),
        }
    }

    #[test]
    fn test_document_link_player() {
        let link = DocumentLink::player(8478402);

        assert_eq!(link.doc_type, DocumentType::Player);
        match link.params {
            LinkParams::Player { id } => assert_eq!(id, 8478402),
            _ => panic!("Expected Player params"),
        }
    }

    #[test]
    fn test_document_link_game() {
        let link = DocumentLink::game(2024020001);

        assert_eq!(link.doc_type, DocumentType::Game);
        match link.params {
            LinkParams::Game { id } => assert_eq!(id, 2024020001),
            _ => panic!("Expected Game params"),
        }
    }

    #[test]
    fn test_document_link_standings() {
        let link = DocumentLink::standings(GroupBy::Division);

        assert_eq!(link.doc_type, DocumentType::Standings);
        match link.params {
            LinkParams::StandingsView { group_by } => assert_eq!(group_by, GroupBy::Division),
            _ => panic!("Expected StandingsView params"),
        }
    }

    #[test]
    fn test_document_link_schedule() {
        let date = GameDate::from_ymd(2024, 11, 15).unwrap();
        let expected_date = GameDate::from_ymd(2024, 11, 15).unwrap();
        let link = DocumentLink::schedule(date);

        assert_eq!(link.doc_type, DocumentType::Schedule);
        match link.params {
            LinkParams::ScheduleDate { date: d } => assert_eq!(d, expected_date),
            _ => panic!("Expected ScheduleDate params"),
        }
    }

    #[test]
    fn test_document_link_new() {
        let link = DocumentLink::new(DocumentType::Player, LinkParams::Player { id: 12345 });

        assert_eq!(link.doc_type, DocumentType::Player);
        match link.params {
            LinkParams::Player { id } => assert_eq!(id, 12345),
            _ => panic!("Expected Player params"),
        }
    }

    #[test]
    fn test_link_target_equality() {
        let target1 = LinkTarget::Document(DocumentLink::team("BOS"));
        let target2 = LinkTarget::Document(DocumentLink::team("BOS"));
        let target3 = LinkTarget::Document(DocumentLink::team("TOR"));

        assert_eq!(target1, target2);
        assert_ne!(target1, target3);
    }

    #[test]
    fn test_document_type_equality() {
        assert_eq!(DocumentType::Team, DocumentType::Team);
        assert_ne!(DocumentType::Team, DocumentType::Player);
        assert_ne!(DocumentType::Game, DocumentType::Schedule);
    }

    #[test]
    fn test_link_params_equality() {
        let params1 = LinkParams::Team {
            abbrev: "BOS".to_string(),
        };
        let params2 = LinkParams::Team {
            abbrev: "BOS".to_string(),
        };
        let params3 = LinkParams::Team {
            abbrev: "TOR".to_string(),
        };

        assert_eq!(params1, params2);
        assert_ne!(params1, params3);
    }
}
