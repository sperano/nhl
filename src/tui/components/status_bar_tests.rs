use super::*;
use crate::config::{Config, DisplayConfig, RenderContext};
use crate::tui::testing::{assert_buffer, RENDER_WIDTH};
use ratatui::buffer::Buffer;

#[test]
fn test_status_bar_renders_loading() {
    let status_bar = StatusBar;
    let system_state = SystemState {
        last_refresh: None,
        config: Config::default(),
        status_message: None,
        status_is_error: false,
        terminal_width: 80,
        animation_frame: 0,
    };

    let element = status_bar.view(&system_state, &());

    match element {
        Element::Widget(widget) => {
            let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
            let config = DisplayConfig::default();
            let ctx = RenderContext::focused(&config);
            widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);
            assert_buffer(&buf, &[
                "────────────────────────────────────────────────────────────────────┬───────────",
                "                                                                    │ Loading...",
            ]);
        }
        _ => panic!("Expected widget element"),
    }
}

#[test]
fn test_status_bar_renders() {
    let status_bar = StatusBar;
    let system_state = SystemState {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        config: Config::default(),
        status_message: None,
        status_is_error: false,
        terminal_width: 80,
        animation_frame: 0,
    };

    let element = status_bar.view(&system_state, &());

    match element {
        Element::Widget(widget) => {
            let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
            let config = DisplayConfig::default();
            let ctx = RenderContext::focused(&config);
            widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);
            assert_buffer(&buf, &[
                "────────────────────────────────────────────────────────────────┬───────────────",
                "                                                                │ Updated 5s ago",
            ]);
        }
        _ => panic!("Expected widget element"),
    }
}

#[test]
fn test_status_bar_with_error_message() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("ERROR: Network timeout".to_string()),
        is_error: true,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Error message should appear on left side
    let line2 = (0..RENDER_WIDTH)
        .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
        .collect::<String>();

    assert!(
        line2.contains("ERROR: Network timeout"),
        "Error message not found in: {}",
        line2
    );
}

#[test]
fn test_status_bar_shows_elapsed_time_at_refresh_interval_boundary() {
    // Elapsed time exactly equal to the interval is not yet stale
    // (stale threshold is a multiple of the interval, see STALE_THRESHOLD_MULTIPLIER).
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(60)),
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    let line2 = (0..RENDER_WIDTH)
        .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
        .collect::<String>();

    assert!(
        line2.contains("Updated 60s ago") && !line2.contains("stale"),
        "Expected non-stale elapsed indicator in: {}",
        line2
    );
}

#[test]
fn test_status_bar_flags_stale_data_past_threshold() {
    // Regression test: previously the status bar showed a "Refresh in Ns"
    // countdown that implied an imminent refresh even though `Tick` never
    // re-dispatched `RefreshData`, so scores froze silently. Now the bar
    // reflects the real last-refresh timestamp and flags stale data.
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(200)),
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    let line2 = (0..RENDER_WIDTH)
        .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
        .collect::<String>();

    assert!(
        line2.contains("Updated 200s ago (stale)"),
        "Stale indicator not found in: {}",
        line2
    );
}

#[test]
fn test_status_bar_future_time() {
    // Test with a future time (should handle time calculation error)
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() + std::time::Duration::from_secs(100)),
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Should show "Updated ?s ago" when duration_since fails
    let line2 = (0..RENDER_WIDTH)
        .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
        .collect::<String>();

    assert!(
        line2.contains("Updated ?s ago"),
        "Error fallback not found in: {}",
        line2
    );
}

#[test]
fn test_status_bar_clone_box() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now()),
        refresh_interval: 60,
        status_message: Some("Test".to_string()),
        is_error: false,
    };

    let _cloned: Box<dyn ElementWidget> = widget.clone_box();
    // If we get here, clone_box() worked
}

#[test]
fn test_status_bar_preferred_height() {
    let widget = StatusBarWidget {
        last_refresh: None,
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    assert_eq!(widget.preferred_height(), Some(2));
}

#[test]
fn test_status_bar_with_success_message() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Configuration saved".to_string()),
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    assert_buffer(
        &buf,
        &[
            "────────────────────────────────────────────────────────────────┬───────────────",
            " Configuration saved                                            │ Updated 5s ago",
        ],
    );

    // Verify success message is NOT styled red
    if let Some(cell) = buf.cell((1, 1)) {
        assert_ne!(cell.fg, Color::Red, "Success message should not be red");
    }
}

#[test]
fn test_status_bar_error_message_has_red_color() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Failed to save config".to_string()),
        is_error: true,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    assert_buffer(
        &buf,
        &[
            "────────────────────────────────────────────────────────────────┬───────────────",
            " Failed to save config                                          │ Updated 5s ago",
        ],
    );

    // Verify error message IS styled red
    if let Some(cell) = buf.cell((1, 1)) {
        assert_eq!(cell.fg, Color::Red, "Error message should be red");
    }
}

#[test]
fn test_status_bar_clears_previous_status_message() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    assert_buffer(
        &buf,
        &[
            "────────────────────────────────────────────────────────────────┬───────────────",
            "                                                                │ Updated 5s ago",
        ],
    );
}

#[test]
fn test_status_bar_separators_use_fg3_when_theme_set() {
    use crate::config::THEME_ORANGE;

    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    let config = DisplayConfig {
        theme: Some(THEME_ORANGE.clone()),
        ..Default::default()
    };
    let ctx = RenderContext::focused(&config);

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Verify separator characters are styled with fg3
    // Check horizontal line character on line 1
    if let Some(cell) = buf.cell((0, 0)) {
        assert_eq!(
            cell.fg, THEME_ORANGE.boxchar_fg,
            "Horizontal separator should use theme fg3"
        );
    }

    // Check connector character (┬) on line 1
    if let Some(cell) = buf.cell((64, 0)) {
        assert_eq!(
            cell.fg, THEME_ORANGE.boxchar_fg,
            "Connector should use theme fg3"
        );
    }

    // Check vertical bar character (│) on line 2
    if let Some(cell) = buf.cell((64, 1)) {
        assert_eq!(
            cell.fg, THEME_ORANGE.boxchar_fg,
            "Vertical bar should use theme fg3"
        );
    }
}

#[test]
fn test_status_bar_separators_unstyled_when_no_theme() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: None,
        is_error: false,
    };

    let config = DisplayConfig::default(); // No theme set
    let ctx = RenderContext::focused(&config);

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Verify separator characters use default color (Reset)
    // Check horizontal line character on line 1
    if let Some(cell) = buf.cell((0, 0)) {
        assert_eq!(
            cell.fg,
            Color::Reset,
            "Separator should use default color when no theme"
        );
    }

    // Check vertical bar character (│) on line 2
    if let Some(cell) = buf.cell((64, 1)) {
        assert_eq!(
            cell.fg,
            Color::Reset,
            "Vertical bar should use default color when no theme"
        );
    }
}

#[test]
fn test_status_bar_text_uses_fg2_when_theme_set() {
    use crate::config::THEME_ORANGE;

    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Configuration saved".to_string()),
        is_error: false,
    };

    let config = DisplayConfig {
        theme: Some(THEME_ORANGE.clone()),
        ..Default::default()
    };
    let ctx = RenderContext::focused(&config);

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Verify success message uses fg2
    if let Some(cell) = buf.cell((1, 1)) {
        assert_eq!(
            cell.fg, THEME_ORANGE.fg,
            "Success message should use theme fg2"
        );
    }

    // Verify refresh text uses fg2
    // Find the refresh text (right side of the vertical bar)
    if let Some(cell) = buf.cell((66, 1)) {
        assert_eq!(
            cell.fg, THEME_ORANGE.fg,
            "Refresh text should use theme fg2"
        );
    }
}

#[test]
fn test_status_bar_error_text_ignores_theme() {
    use crate::config::THEME_ORANGE;

    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Failed to save config".to_string()),
        is_error: true,
    };

    let config = DisplayConfig {
        theme: Some(THEME_ORANGE.clone()),
        ..Default::default()
    };
    let ctx = RenderContext::focused(&config);

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Verify error message still uses Color::Red, not theme fg2
    if let Some(cell) = buf.cell((1, 1)) {
        assert_eq!(
            cell.fg,
            Color::Red,
            "Error message should use Color::Red even with theme set"
        );
        assert_ne!(
            cell.fg, THEME_ORANGE.fg,
            "Error message should NOT use theme fg2"
        );
    }
}

#[test]
fn test_status_bar_text_unstyled_when_no_theme() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Configuration saved".to_string()),
        is_error: false,
    };

    let config = DisplayConfig::default(); // No theme set
    let ctx = RenderContext::focused(&config);

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Verify success message uses default color when no theme
    if let Some(cell) = buf.cell((1, 1)) {
        assert_eq!(
            cell.fg,
            Color::Reset,
            "Success message should use default color when no theme"
        );
    }

    // Verify refresh text uses default color when no theme
    if let Some(cell) = buf.cell((66, 1)) {
        assert_eq!(
            cell.fg,
            Color::Reset,
            "Refresh text should use default color when no theme"
        );
    }
}

#[test]
fn test_status_bar_with_emoji_in_message() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Updated 🏒".to_string()),
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Should render without panic
    // The vertical bar should still be positioned correctly
    let line1 = (0..RENDER_WIDTH)
        .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
        .collect::<String>();

    assert!(
        line1.contains("Updated"),
        "Status message should be visible"
    );
    assert!(line1.contains("│"), "Vertical separator should be present");
}

#[test]
fn test_status_bar_with_cjk_in_message() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("更新完了".to_string()), // "Update complete" in Japanese
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Should render without panic or incorrect layout
    let line1 = (0..RENDER_WIDTH)
        .map(|x| buf.cell((x, 1)).map(|c| c.symbol()).unwrap_or(""))
        .collect::<String>();

    assert!(
        line1.contains("│"),
        "Vertical separator should be present despite CJK characters"
    );
    assert!(
        line1.contains("Updated") && line1.contains("ago"),
        "Refresh text should still be visible"
    );
}

#[test]
fn test_status_bar_with_long_unicode_message() {
    let widget = StatusBarWidget {
        last_refresh: Some(SystemTime::now() - std::time::Duration::from_secs(5)),
        refresh_interval: 60,
        status_message: Some("Loading players データを読み込み中 🏒🥅".to_string()),
        is_error: false,
    };

    let mut buf = Buffer::empty(Rect::new(0, 0, RENDER_WIDTH, 2));
    let config = DisplayConfig::default();
    let ctx = RenderContext::focused(&config);
    widget.render(Rect::new(0, 0, RENDER_WIDTH, 2), &mut buf, &ctx);

    // Should render without panic
    // Layout should still be functional
    assert!(buf.area.width > 0);
    assert!(buf.area.height == 2);
}
