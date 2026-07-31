use super::*;

#[test]
fn test_parse_color_named() {
    assert_eq!(parse_color("red"), Some(Color::Red));
    assert_eq!(parse_color("blue"), Some(Color::Blue));
    assert_eq!(parse_color("orange"), Some(Color::Rgb(255, 165, 0)));
    assert_eq!(parse_color("cyan"), Some(Color::Cyan));
    assert_eq!(parse_color("white"), Some(Color::White));
}

#[test]
fn test_parse_color_case_insensitive() {
    assert_eq!(parse_color("RED"), Some(Color::Red));
    assert_eq!(parse_color("Blue"), Some(Color::Blue));
    assert_eq!(parse_color("ORANGE"), Some(Color::Rgb(255, 165, 0)));
}

#[test]
fn test_parse_color_hex_6_digit() {
    assert_eq!(parse_color("#FF6600"), Some(Color::Rgb(255, 102, 0)));
    assert_eq!(parse_color("#ff6600"), Some(Color::Rgb(255, 102, 0)));
    assert_eq!(parse_color("#00FF00"), Some(Color::Rgb(0, 255, 0)));
}

#[test]
fn test_parse_color_hex_3_digit() {
    assert_eq!(parse_color("#F60"), Some(Color::Rgb(255, 102, 0)));
    assert_eq!(parse_color("#f60"), Some(Color::Rgb(255, 102, 0)));
    assert_eq!(parse_color("#0F0"), Some(Color::Rgb(0, 255, 0)));
}

#[test]
fn test_parse_color_rgb_tuple() {
    assert_eq!(parse_color("255,165,0"), Some(Color::Rgb(255, 165, 0)));
    assert_eq!(parse_color("0,255,0"), Some(Color::Rgb(0, 255, 0)));
    assert_eq!(parse_color("255, 102, 0"), Some(Color::Rgb(255, 102, 0))); // with spaces
}

#[test]
fn test_parse_color_invalid() {
    assert_eq!(parse_color("invalid"), None);
    assert_eq!(parse_color("#ZZZ"), None);
    assert_eq!(parse_color("256,0,0"), None); // RGB values too high
    assert_eq!(parse_color("#GGGGGG"), None);
}

#[test]
fn test_serialize_color_rgb() {
    let color = Color::Rgb(255, 165, 0);
    assert_eq!(format_color(&color), "255,165,0");
}

#[test]
fn test_serialize_color_named() {
    assert_eq!(format_color(&Color::Red), "red");
    assert_eq!(format_color(&Color::Blue), "blue");
    assert_eq!(format_color(&Color::Cyan), "cyan");
}

#[test]
fn test_config_to_toml() {
    let config = Config {
        refresh_interval: 30,
        log_level: "debug".to_string(),
        display_standings_western_first: true,
        display: DisplayConfig {
            use_unicode: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let toml_str = toml::to_string_pretty(&config).unwrap();

    let expected = r#"log_level = "debug"
log_file = "/dev/null"
refresh_interval = 30
display_standings_western_first = true
time_format = "%H:%M:%S"

[display]
use_unicode = false
error_fg = "255,0,0"
"#;
    assert_eq!(toml_str.trim(), expected.trim());
}

#[test]
fn test_config_toml_example_parses() {
    // Guards against config.toml.example drifting from the real Config schema.
    let example = include_str!("../config.toml.example");
    let config: Config = toml::from_str(example)
        .expect("config.toml.example should parse into Config without unknown/missing keys");

    assert_eq!(config.log_level, "info");
    assert_eq!(config.refresh_interval, 60);
    assert!(config.display.use_unicode);
    assert_eq!(config.display.theme_name, Some("orange".to_string()));
}

#[test]
fn test_roundtrip_serialization() {
    let mut config = Config::default();
    config.display.use_unicode = false;
    config.refresh_interval = 45;
    config.display_standings_western_first = true;

    // Serialize to TOML
    let toml_str = toml::to_string_pretty(&config).unwrap();

    // Deserialize back
    let deserialized: Config = toml::from_str(&toml_str).unwrap();

    assert!(!deserialized.display.use_unicode);
    assert_eq!(deserialized.refresh_interval, 45);
    assert!(deserialized.display_standings_western_first);
}

#[test]
fn test_theme_auto_loading_with_valid_theme() {
    let toml_str = r#"
log_level = "info"
log_file = "/dev/null"
refresh_interval = 60
display_standings_western_first = false
time_format = "%H:%M:%S"

[display]
theme = "orange"
    "#;

    let mut config: Config = toml::from_str(toml_str).unwrap();

    // Manually apply theme loading logic (simulating what read() does)
    config.display.theme = config
        .display
        .theme_name
        .as_ref()
        .and_then(|name| THEMES.get(name.as_str()))
        .map(|theme| (*theme).clone());

    assert_eq!(config.display.theme_name, Some("orange".to_string()));
    assert!(config.display.theme.is_some());

    let theme = config.display.theme.unwrap();
    assert_eq!(theme.emphasis_fg, THEME_ORANGE.emphasis_fg);
    assert_eq!(theme.fg, THEME_ORANGE.fg);
    assert_eq!(theme.boxchar_fg, THEME_ORANGE.boxchar_fg);
}

#[test]
fn test_theme_auto_loading_with_invalid_theme() {
    let toml_str = r#"
log_level = "info"
log_file = "/dev/null"
refresh_interval = 60
display_standings_western_first = false
time_format = "%H:%M:%S"

[display]
theme = "invalid_theme_name"
    "#;

    let mut config: Config = toml::from_str(toml_str).unwrap();

    // Manually apply theme loading logic
    config.display.theme = config
        .display
        .theme_name
        .as_ref()
        .and_then(|name| THEMES.get(name.as_str()))
        .map(|theme| (*theme).clone());

    assert_eq!(
        config.display.theme_name,
        Some("invalid_theme_name".to_string())
    );
    assert!(config.display.theme.is_none());
}

#[test]
fn test_theme_auto_loading_with_no_theme() {
    let toml_str = r#"
log_level = "info"
log_file = "/dev/null"
refresh_interval = 60
display_standings_western_first = false
time_format = "%H:%M:%S"
    "#;

    let mut config: Config = toml::from_str(toml_str).unwrap();

    // Manually apply theme loading logic
    config.display.theme = config
        .display
        .theme_name
        .as_ref()
        .and_then(|name| THEMES.get(name.as_str()))
        .map(|theme| (*theme).clone());

    assert_eq!(config.display.theme_name, None);
    assert!(config.display.theme.is_none());
}

#[test]
fn test_theme_auto_loading_all_themes() {
    let theme_names = vec!["orange", "green", "blue", "purple", "white"];

    for theme_name in theme_names {
        let toml_str = format!(
            r#"
log_level = "info"
log_file = "/dev/null"
refresh_interval = 60
display_standings_western_first = false
time_format = "%H:%M:%S"

[display]
theme = "{}"
        "#,
            theme_name
        );

        let mut config: Config = toml::from_str(&toml_str).unwrap();

        // Apply theme loading logic
        config.display.apply_theme();

        assert_eq!(config.display.theme_name, Some(theme_name.to_string()));
        assert!(
            config.display.theme.is_some(),
            "Theme '{}' should load",
            theme_name
        );
    }
}

#[test]
fn test_theme_dark_colors() {
    // Test fg_dark returns darkened color based on theme's darkening_factor
    let orange_fg = THEME_ORANGE.fg;
    let orange_fg_dark = THEME_ORANGE.fg_dark();
    let factor = THEME_ORANGE.darkening_factor;

    match (orange_fg, orange_fg_dark) {
        (Color::Rgb(r, g, b), Color::Rgb(rd, gd, bd)) => {
            assert_eq!(rd, (r as f32 * factor) as u8);
            assert_eq!(gd, (g as f32 * factor) as u8);
            assert_eq!(bd, (b as f32 * factor) as u8);
        }
        _ => panic!("Expected RGB colors"),
    }

    // Test boxchar_fg_dark returns darkened color based on theme's darkening_factor
    let orange_boxchar_fg = THEME_ORANGE.boxchar_fg;
    let orange_boxchar_fg_dark = THEME_ORANGE.boxchar_fg_dark();

    match (orange_boxchar_fg, orange_boxchar_fg_dark) {
        (Color::Rgb(r, g, b), Color::Rgb(rd, gd, bd)) => {
            assert_eq!(rd, (r as f32 * factor) as u8);
            assert_eq!(gd, (g as f32 * factor) as u8);
            assert_eq!(bd, (b as f32 * factor) as u8);
        }
        _ => panic!("Expected RGB colors"),
    }
}

#[test]
fn test_theme_dark_colors_cached() {
    // Call twice to verify it returns the same value (cached)
    let first_call = THEME_GREEN.fg_dark();
    let second_call = THEME_GREEN.fg_dark();
    assert_eq!(first_call, second_call);

    let first_call = THEME_GREEN.boxchar_fg_dark();
    let second_call = THEME_GREEN.boxchar_fg_dark();
    assert_eq!(first_call, second_call);
}
