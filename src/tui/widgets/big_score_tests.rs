use super::*;
use crate::tui::testing::assert_buffer;
use crate::tui::widgets::testing::{render_widget_with_config, test_config};

fn final_status() -> ScoreBoxStatus {
    ScoreBoxStatus::Final {
        overtime: false,
        shootout: false,
    }
}

#[test]
fn test_single_digit_scores() {
    // Layout: 20 (away box) + 2 (gap) + 4 (digit) + 6 (sep) + 4 (digit) + 2 (gap) + 20 (home box) = 58
    let widget = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 3,
        home_score: 2,
        away_sog: 30,
        home_sog: 25,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    let config = test_config();
    let buf = render_widget_with_config(&widget, 58, 9, &config);

    assert_buffer(
        &buf,
        &[
            "                          Final                          ",
            "                                                          ",
            "                      ▟▀▀▙      ▟▀▀▙                      ",
            "              Devils   ▄▄▛  ▄▄    ▗▛  Sabres              ",
            "                         █       ▗▛                       ",
            "                      ▜▄▄▛      ▄█▄▄                      ",
            "                                                          ",
            "                       SOG: 30 - 25                       ",
            "                        TD Garden                         ",
        ],
    );
}

#[test]
fn test_score_10_4() {
    // 10-4: away=9 (4+1+4), home=4, imbalance=5, home_box=25
    // Width: 20 + 2 + 9 + 6 + 4 + 2 + 25 = 68
    let widget = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 10,
        home_score: 4,
        away_sog: 40,
        home_sog: 20,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    let config = test_config();
    let buf = render_widget_with_config(&widget, 68, 9, &config);

    assert_buffer(
        &buf,
        &[
            "                               Final                                ",
            "                                                                    ",
            "                      ▗█   ▟▀▀▙       ▗█                            ",
            "              Devils   █   █  █  ▄▄  ▗▘█   Sabres                   ",
            "                       █   █  █      ▙▄█▄                           ",
            "                      ▗█▖  ▜▄▄▛        █                            ",
            "                                                                    ",
            "                            SOG: 40 - 20                            ",
            "                             TD Garden                              ",
        ],
    );
}

#[test]
fn test_score_4_10() {
    // 4-10: away=4, home=9, imbalance=5, away_box=25
    // Width: 25 + 2 + 4 + 6 + 9 + 2 + 20 = 68
    let widget = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 4,
        home_score: 10,
        away_sog: 20,
        home_sog: 40,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    let config = test_config();
    let buf = render_widget_with_config(&widget, 68, 9, &config);

    assert_buffer(
        &buf,
        &[
            "                               Final                                ",
            "                                                                    ",
            "                            ▗█       ▗█   ▟▀▀▙                      ",
            "                   Devils  ▗▘█   ▄▄   █   █  █  Sabres              ",
            "                           ▙▄█▄       █   █  █                      ",
            "                             █       ▗█▖  ▜▄▄▛                      ",
            "                                                                    ",
            "                            SOG: 20 - 40                            ",
            "                             TD Garden                              ",
        ],
    );
}

#[test]
fn test_score_10_10() {
    // 10-10: both=9, balanced
    // Width: 20 + 2 + 9 + 6 + 9 + 2 + 20 = 68
    let widget = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 10,
        home_score: 10,
        away_sog: 35,
        home_sog: 35,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    let config = test_config();
    let buf = render_widget_with_config(&widget, 68, 9, &config);

    assert_buffer(
        &buf,
        &[
            "                               Final                                ",
            "                                                                    ",
            "                      ▗█   ▟▀▀▙      ▗█   ▟▀▀▙                      ",
            "              Devils   █   █  █  ▄▄   █   █  █  Sabres              ",
            "                       █   █  █       █   █  █                      ",
            "                      ▗█▖  ▜▄▄▛      ▗█▖  ▜▄▄▛                      ",
            "                                                                    ",
            "                            SOG: 35 - 35                            ",
            "                             TD Garden                              ",
        ],
    );
}

#[test]
fn test_score_digits() {
    assert_eq!(BigScore::score_digits(0), vec![0]);
    assert_eq!(BigScore::score_digits(5), vec![5]);
    assert_eq!(BigScore::score_digits(10), vec![1, 0]);
    assert_eq!(BigScore::score_digits(99), vec![9, 9]);
}

#[test]
fn test_preferred_dimensions() {
    // Single digit scores: 20 + 2 + 4 + 6 + 4 + 2 + 20 = 58
    let widget = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 3,
        home_score: 2,
        away_sog: 30,
        home_sog: 25,
        status: final_status(),
        venue: "KeyBank Center".to_string(),
    });
    assert_eq!(widget.preferred_height(), Some(9)); // 1 status + 1 blank + 4 digits + 1 blank + 1 SOG + 1 venue
    assert_eq!(widget.preferred_width(), Some(58));

    // 10-4: away=9 (4+1+4), home=4, imbalance=5, home_box=25
    // Width: 20 + 2 + 9 + 6 + 4 + 2 + 25 = 68
    let widget_10_4 = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 10,
        home_score: 4,
        away_sog: 40,
        home_sog: 20,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    assert_eq!(widget_10_4.preferred_width(), Some(68));

    // 4-10: away=4, home=9, imbalance=5, away_box=25
    // Width: 25 + 2 + 4 + 6 + 9 + 2 + 20 = 68
    let widget_4_10 = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 4,
        home_score: 10,
        away_sog: 20,
        home_sog: 40,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    assert_eq!(widget_4_10.preferred_width(), Some(68));

    // 10-10: both=9, balanced
    // Width: 20 + 2 + 9 + 6 + 9 + 2 + 20 = 68
    let widget_10_10 = BigScore::new(BigScoreParams {
        away_name: "Devils".to_string(),
        home_name: "Sabres".to_string(),
        away_score: 10,
        home_score: 10,
        away_sog: 35,
        home_sog: 35,
        status: final_status(),
        venue: "TD Garden".to_string(),
    });
    assert_eq!(widget_10_10.preferred_width(), Some(68));
}
