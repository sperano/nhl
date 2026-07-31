use super::*;
use crate::tui::testing::assert_buffer;
use crate::tui::widgets::testing::{render_widget_with_config, test_config};

#[test]
fn test_score_box_final() {
    let score_box = ScoreBox::new(
        "Golden Knights",
        "Avalanche",
        Some(10),
        Some(3),
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: false,
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " Final                   ",
            "╔══════════════════╤════╗",
            "║ Golden Knights   │ 10 ║",
            "╟──────────────────┼────╢",
            "║ Avalanche        │  3 ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_score_box_live() {
    let score_box = ScoreBox::new(
        "Maple Leafs",
        "Bruins",
        Some(2),
        Some(0),
        ScoreBoxStatus::Live {
            period: "1st".to_string(),
            time: Some("09:27".to_string()),
            intermission: false,
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " 1st 09:27               ",
            "╔══════════════════╤════╗",
            "║ Maple Leafs      │  2 ║",
            "╟──────────────────┼────╢",
            "║ Bruins           │  0 ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_score_box_scheduled() {
    let score_box = ScoreBox::new(
        "Canucks",
        "Blue Jackets",
        None,
        None,
        ScoreBoxStatus::Scheduled {
            start_time: "9PM".to_string(),
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " 9PM                     ",
            "╔══════════════════╤════╗",
            "║ Canucks          │  - ║",
            "╟──────────────────┼────╢",
            "║ Blue Jackets     │  - ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_score_box_final_ot() {
    let score_box = ScoreBox::new(
        "Canadiens",
        "Sabres",
        Some(3),
        Some(2),
        ScoreBoxStatus::Final {
            overtime: true,
            shootout: false,
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " Final (OT)              ",
            "╔══════════════════╤════╗",
            "║ Canadiens        │  3 ║",
            "╟──────────────────┼────╢",
            "║ Sabres           │  2 ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_score_box_intermission() {
    let score_box = ScoreBox::new(
        "Senators",
        "Capitals",
        Some(0),
        Some(0),
        ScoreBoxStatus::Live {
            period: "1st".to_string(),
            time: None,
            intermission: true,
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " 1st int.                ",
            "╔══════════════════╤════╗",
            "║ Senators         │  0 ║",
            "╟──────────────────┼────╢",
            "║ Capitals         │  0 ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_score_box_final_so() {
    let score_box = ScoreBox::new(
        "Rangers",
        "Devils",
        Some(4),
        Some(3),
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: true,
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " Final (SO)              ",
            "╔══════════════════╤════╗",
            "║ Rangers          │  4 ║",
            "╟──────────────────┼────╢",
            "║ Devils           │  3 ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_score_box_long_team_name_truncated() {
    let score_box = ScoreBox::new(
        "Very Long Team Name That Should Be Truncated",
        "Short",
        Some(1),
        Some(2),
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: false,
        },
    );

    let config = test_config();
    let buf = render_widget_with_config(&score_box, 25, 6, &config);

    assert_buffer(
        &buf,
        &[
            " Final                   ",
            "╔══════════════════╤════╗",
            "║ Very Long Team Na│  1 ║",
            "╟──────────────────┼────╢",
            "║ Short            │  2 ║",
            "╚══════════════════╧════╝",
        ],
    );
}

#[test]
fn test_format_score() {
    assert_eq!(ScoreBox::format_score(Some(0)), "  0 ");
    assert_eq!(ScoreBox::format_score(Some(3)), "  3 ");
    assert_eq!(ScoreBox::format_score(Some(10)), " 10 ");
    assert_eq!(ScoreBox::format_score(None), "  - ");
}

#[test]
fn test_format_team_name() {
    assert_eq!(ScoreBox::format_team_name("Bruins"), "Bruins           "); // 17 chars
    assert_eq!(
        ScoreBox::format_team_name("Golden Knights"),
        "Golden Knights   " // 17 chars
    );
    assert_eq!(
        ScoreBox::format_team_name("Very Long Team Name"),
        "Very Long Team Na" // 17 chars truncated
    );
}

#[test]
fn test_status_display() {
    assert_eq!(
        ScoreBoxStatus::Scheduled {
            start_time: "7PM".to_string()
        }
        .display(),
        "7PM"
    );

    assert_eq!(
        ScoreBoxStatus::Live {
            period: "2nd".to_string(),
            time: Some("05:30".to_string()),
            intermission: false
        }
        .display(),
        "2nd 05:30"
    );

    assert_eq!(
        ScoreBoxStatus::Live {
            period: "2nd".to_string(),
            time: None,
            intermission: true
        }
        .display(),
        "2nd int."
    );

    assert_eq!(
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: false
        }
        .display(),
        "Final"
    );

    assert_eq!(
        ScoreBoxStatus::Final {
            overtime: true,
            shootout: false
        }
        .display(),
        "Final (OT)"
    );

    assert_eq!(
        ScoreBoxStatus::Final {
            overtime: false,
            shootout: true
        }
        .display(),
        "Final (SO)"
    );
}
