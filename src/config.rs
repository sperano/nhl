use phf::phf_map;
use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const DARKENING_FACTOR: f32 = 0.5;

/// Default refresh interval in seconds for background data fetching
pub const DEFAULT_REFRESH_INTERVAL_SECONDS: u32 = 60;

/// Style modifier for selected items (reversed and bold)
pub const SELECTION_STYLE_MODIFIER: Modifier = Modifier::REVERSED.union(Modifier::BOLD);

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Config {
    pub log_level: String,
    pub log_file: String,
    pub refresh_interval: u32,
    pub display_standings_western_first: bool,
    pub time_format: String,
    pub display: DisplayConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Theme {
    #[serde(skip)]
    pub name: &'static str,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub fg1: Color,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub fg2: Color,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub fg3: Color,
    #[serde(skip)]
    pub fg2_dim: OnceLock<Color>,
    #[serde(skip)]
    pub fg3_dim: OnceLock<Color>,
}

pub static THEME_ID_ORANGE: &str = "orange";
pub static THEME_ID_GREEN: &str = "green";
pub static THEME_ID_BLUE: &str = "blue";
pub static THEME_ID_PURPLE: &str = "purple";
pub static THEME_ID_WHITE: &str = "white";
pub static THEME_ID_RED: &str = "red";
pub static THEME_ID_YELLOW: &str = "yellow";
pub static THEME_ID_CYAN: &str = "cyan";

pub static THEME_ORANGE: Theme = Theme {
    name: "Orange",
    fg1: Color::Rgb(255, 214, 128),
    fg2: Color::Rgb(255, 175, 64),
    fg3: Color::Rgb(226, 108, 34),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_GREEN: Theme = Theme {
    name: "Green",
    fg1: Color::Rgb(175, 255, 135),
    fg2: Color::Rgb(95, 255, 175),
    fg3: Color::Rgb(0, 255, 0),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_BLUE: Theme = Theme {
    name: "Blue",
    fg1: Color::Rgb(175, 255, 255),
    fg2: Color::Rgb(95, 135, 255),
    fg3: Color::Rgb(0, 95, 255),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_PURPLE: Theme = Theme {
    name: "Purple",
    fg1: Color::Rgb(255, 175, 255),
    fg2: Color::Rgb(175, 135, 255),
    fg3: Color::Rgb(135, 95, 175),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_WHITE: Theme = Theme {
    name: "White",
    fg1: Color::Rgb(255, 255, 255),
    fg2: Color::Rgb(192, 192, 192),
    fg3: Color::Rgb(128, 128, 128),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_RED: Theme = Theme {
    name: "Red",
    fg1: Color::Rgb(255, 175, 175),
    fg2: Color::Rgb(255, 95, 95),
    fg3: Color::Rgb(255, 0, 0),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_YELLOW: Theme = Theme {
    name: "Yellow",
    fg1: Color::Rgb(255, 255, 175),
    fg2: Color::Rgb(255, 255, 95),
    fg3: Color::Rgb(255, 215, 0),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEME_CYAN: Theme = Theme {
    name: "Cyan",
    fg1: Color::Rgb(175, 255, 255),
    fg2: Color::Rgb(95, 255, 255),
    fg3: Color::Rgb(0, 255, 255),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};

pub static THEMES: phf::Map<&'static str, &Theme> = phf_map! {
    "orange" => &THEME_ORANGE,
    "green"  => &THEME_GREEN,
    "blue"   => &THEME_BLUE,
    "purple" => &THEME_PURPLE,
    "white"  => &THEME_WHITE,
    "red"    => &THEME_RED,
    "yellow" => &THEME_YELLOW,
    "cyan"   => &THEME_CYAN,
};

impl Default for Theme {
    fn default() -> Self {
        THEME_WHITE.clone()
    }
}

impl Theme {
    /// Get a 50% darker version of fg2, computed lazily and cached
    pub fn fg2_dark(&self) -> Color {
        *self
            .fg2_dim
            .get_or_init(|| darken_color(self.fg2, DARKENING_FACTOR))
    }

    /// Get a 50% darker version of fg3, computed lazily and cached
    pub fn fg3_dark(&self) -> Color {
        *self
            .fg3_dim
            .get_or_init(|| darken_color(self.fg3, DARKENING_FACTOR))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct DisplayConfig {
    pub use_unicode: bool,
    #[serde(rename = "theme")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_name: Option<String>,
    #[serde(skip)]
    pub theme: Option<Theme>,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub error_fg: Color,
    #[serde(skip)]
    pub box_chars: crate::formatting::BoxChars,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: "info".to_string(),
            log_file: "/dev/null".to_string(),
            refresh_interval: DEFAULT_REFRESH_INTERVAL_SECONDS,
            display_standings_western_first: false,
            time_format: "%H:%M:%S".to_string(),
            display: DisplayConfig::default(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            use_unicode: true,
            theme_name: None,
            theme: None,
            error_fg: Color::Rgb(255, 0, 0), // Red
            box_chars: crate::formatting::BoxChars::unicode(),
        }
    }
}

impl DisplayConfig {
    /// Apply theme from theme_name by looking it up in THEMES map
    pub fn apply_theme(&mut self) {
        self.theme = self
            .theme_name
            .as_ref()
            .and_then(|name| THEMES.get(name.as_str()))
            .map(|theme| (*theme).clone());
    }

    /// Get the default text style using fg2 from theme
    ///
    /// This is the primary text color for normal content.
    pub fn text_style(&self) -> ratatui::style::Style {
        self.theme
            .as_ref()
            .map(|t| ratatui::style::Style::default().fg(t.fg2))
            .unwrap_or_default()
    }

    /// Get the secondary/muted text style using fg3 from theme
    ///
    /// This is for separators, borders, and less prominent text.
    pub fn muted_style(&self) -> ratatui::style::Style {
        self.theme
            .as_ref()
            .map(|t| ratatui::style::Style::default().fg(t.fg3))
            .unwrap_or_default()
    }

    /// Get a heading style with bold modifier
    pub fn heading_style(&self, level: u8) -> ratatui::style::Style {
        let base = self.text_style();
        match level {
            1 | 2 => base.add_modifier(Modifier::BOLD),
            _ => base.add_modifier(Modifier::UNDERLINED),
        }
    }
}

/// Darken a color by a given factor (0.0 = black, 1.0 = original)
fn darken_color(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let r = (r as f32 * factor) as u8;
            let g = (g as f32 * factor) as u8;
            let b = (b as f32 * factor) as u8;
            Color::Rgb(r, g, b)
        }
        // For named colors, return them as-is (could convert to RGB if needed)
        other => other,
    }
}

/// Deserialize a color from a string (supports named colors, RGB hex, or RGB tuple)
fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_color(&s).ok_or_else(|| serde::de::Error::custom(format!("Invalid color: {}", s)))
}

// Deserialize an optional color from a string
// fn deserialize_color_optional<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     let s: Option<String> = Option::deserialize(deserializer)?;
//     match s {
//         Some(color_str) => {
//             let color = parse_color(&color_str)
//                 .ok_or_else(|| serde::de::Error::custom(format!("Invalid color: {}", color_str)))?;
//             Ok(Some(color))
//         }
//         None => Ok(None),
//     }
// }

/// Serialize a color to a string
fn serialize_color<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format_color(color))
}

// Serialize an optional color to a string
// fn serialize_color_optional<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
// where
//     S: serde::Serializer,
// {
//     match color {
//         Some(c) => serializer.serialize_str(&format_color(c)),
//         None => serializer.serialize_none(),
//     }
// }

/// Format a color as a string (RGB format for serialization)
fn format_color(color: &Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("{},{},{}", r, g, b),
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Gray => "gray".to_string(),
        Color::DarkGray => "darkgray".to_string(),
        Color::LightRed => "lightred".to_string(),
        Color::LightGreen => "lightgreen".to_string(),
        Color::LightYellow => "lightyellow".to_string(),
        Color::LightBlue => "lightblue".to_string(),
        Color::LightMagenta => "lightmagenta".to_string(),
        Color::LightCyan => "lightcyan".to_string(),
        Color::White => "white".to_string(),
        _ => "white".to_string(), // fallback for indexed colors
    }
}

/// Parse a color string into a ratatui Color
/// Supports:
/// - Named colors: "red", "blue", "cyan", "orange", etc.
/// - Hex colors: "#FF6600", "#f60"
/// - RGB tuples: "255,165,0"
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();

    // Named colors
    match s.as_str() {
        "black" => return Some(Color::Black),
        "red" => return Some(Color::Red),
        "green" => return Some(Color::Green),
        "yellow" => return Some(Color::Yellow),
        "blue" => return Some(Color::Blue),
        "magenta" => return Some(Color::Magenta),
        "cyan" => return Some(Color::Cyan),
        "gray" | "grey" => return Some(Color::Gray),
        "darkgray" | "darkgrey" => return Some(Color::DarkGray),
        "lightred" => return Some(Color::LightRed),
        "lightgreen" => return Some(Color::LightGreen),
        "lightyellow" => return Some(Color::LightYellow),
        "lightblue" => return Some(Color::LightBlue),
        "lightmagenta" => return Some(Color::LightMagenta),
        "lightcyan" => return Some(Color::LightCyan),
        "white" => return Some(Color::White),
        "orange" => return Some(Color::Rgb(255, 165, 0)),
        "seafoam" => return Some(Color::Rgb(159, 226, 191)),
        "deepred" | "deep red" => return Some(Color::Rgb(226, 74, 74)),
        "coral" => return Some(Color::Rgb(255, 107, 107)),
        "burntorange" | "burnt orange" => return Some(Color::Rgb(255, 140, 66)),
        "amber" => return Some(Color::Rgb(255, 200, 87)),
        "goldenrod" => return Some(Color::Rgb(232, 185, 35)),
        "olive" => return Some(Color::Rgb(166, 166, 89)),
        "chartreuse" => return Some(Color::Rgb(140, 207, 77)),
        "greenapple" | "green apple" => return Some(Color::Rgb(88, 196, 114)),
        "emerald" => return Some(Color::Rgb(46, 184, 114)),
        "teal" => return Some(Color::Rgb(42, 168, 118)),
        "cyansky" | "cyan sky" => return Some(Color::Rgb(77, 208, 225)),
        "azure" => return Some(Color::Rgb(33, 150, 243)),
        "cobaltblue" | "cobalt blue" => return Some(Color::Rgb(61, 90, 254)),
        "indigo" => return Some(Color::Rgb(92, 107, 192)),
        "violet" => return Some(Color::Rgb(126, 87, 194)),
        "orchid" => return Some(Color::Rgb(186, 104, 200)),
        "hotpink" | "hot pink" => return Some(Color::Rgb(255, 119, 169)),
        "salmon" => return Some(Color::Rgb(255, 158, 157)),
        "beige" => return Some(Color::Rgb(234, 210, 172)),
        "coolgray" | "cool gray" => return Some(Color::Rgb(159, 168, 176)),
        "slate" => return Some(Color::Rgb(96, 125, 139)),
        "charcoal" => return Some(Color::Rgb(55, 71, 79)),
        _ => {}
    }

    // Hex colors (#FF6600 or #f60)
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }

    // RGB tuples "255,165,0"
    if s.contains(',') {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }

    None
}

pub fn get_config_path() -> Option<PathBuf> {
    let pgm = env!("CARGO_PKG_NAME");

    // On Unix, use XDG-style ~/.config for backward compatibility
    // On Windows, use the native config directory
    #[cfg(unix)]
    let config_dir = dirs::home_dir()?.join(".config");

    #[cfg(windows)]
    let config_dir = dirs::config_dir()?;

    Some(config_dir.join(pgm).join("config.toml"))
}

pub fn read() -> Config {
    let config_path = match get_config_path() {
        Some(path) => path,
        None => return Config::default(),
    };

    // Check if file exists
    if !config_path.exists() {
        return Config::default();
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(_) => return Config::default(),
    };

    let mut config: Config = toml::from_str(&content).unwrap_or_else(|_| Config::default());

    // Initialize box_chars based on use_unicode (since it's not serialized)
    config.display.box_chars =
        crate::formatting::BoxChars::from_use_unicode(config.display.use_unicode);

    // Apply theme based on theme_name (since it's not serialized)
    config.display.apply_theme();

    config
}

/// Write a config to the config file
pub fn write(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path().ok_or("Failed to get config path")?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Serialize config to TOML
    let toml_string = toml::to_string_pretty(config)?;

    // Write to file
    fs::write(&config_path, toml_string)?;

    Ok(())
}

#[cfg(test)]
mod tests {
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
        let mut config = Config::default();
        config.refresh_interval = 30;
        config.log_level = "debug".to_string();
        config.display_standings_western_first = true;
        config.display.use_unicode = false;

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
    fn test_roundtrip_serialization() {
        let mut config = Config::default();
        config.display.use_unicode = false;
        config.refresh_interval = 45;
        config.display_standings_western_first = true;

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Deserialize back
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.display.use_unicode, false);
        assert_eq!(deserialized.refresh_interval, 45);
        assert_eq!(deserialized.display_standings_western_first, true);
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
        assert_eq!(theme.fg1, THEME_ORANGE.fg1);
        assert_eq!(theme.fg2, THEME_ORANGE.fg2);
        assert_eq!(theme.fg3, THEME_ORANGE.fg3);
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
        // Test fg2_dark returns 50% darker (50% of original)
        let orange_fg2 = THEME_ORANGE.fg2;
        let orange_fg2_dark = THEME_ORANGE.fg2_dark();

        match (orange_fg2, orange_fg2_dark) {
            (Color::Rgb(r, g, b), Color::Rgb(rd, gd, bd)) => {
                assert_eq!(rd, (r as f32 * DARKENING_FACTOR) as u8);
                assert_eq!(gd, (g as f32 * DARKENING_FACTOR) as u8);
                assert_eq!(bd, (b as f32 * DARKENING_FACTOR) as u8);
            }
            _ => panic!("Expected RGB colors"),
        }

        // Test fg3_dark returns 50% darker (50% of original)
        let orange_fg3 = THEME_ORANGE.fg3;
        let orange_fg3_dark = THEME_ORANGE.fg3_dark();

        match (orange_fg3, orange_fg3_dark) {
            (Color::Rgb(r, g, b), Color::Rgb(rd, gd, bd)) => {
                assert_eq!(rd, (r as f32 * DARKENING_FACTOR) as u8);
                assert_eq!(gd, (g as f32 * DARKENING_FACTOR) as u8);
                assert_eq!(bd, (b as f32 * DARKENING_FACTOR) as u8);
            }
            _ => panic!("Expected RGB colors"),
        }
    }

    #[test]
    fn test_theme_dark_colors_cached() {
        // Call twice to verify it returns the same value (cached)
        let first_call = THEME_GREEN.fg2_dark();
        let second_call = THEME_GREEN.fg2_dark();
        assert_eq!(first_call, second_call);

        let first_call = THEME_GREEN.fg3_dark();
        let second_call = THEME_GREEN.fg3_dark();
        assert_eq!(first_call, second_call);
    }
}
