use phf::phf_map;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[path = "config_themes_color.rs"]
mod config_themes_color;
pub(super) use config_themes_color::parse_color;

/// Default darkening factor for unfocused elements
const DEFAULT_DARKENING_FACTOR: f32 = 0.5;
/// Darkening factor for themes with bright backgrounds (like Habs)
const BRIGHT_BG_DARKENING_FACTOR: f32 = 0.85;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Theme {
    #[serde(skip)]
    pub name: &'static str,
    #[serde(deserialize_with = "deserialize_color_optional")]
    #[serde(serialize_with = "serialize_color_optional")]
    pub bg: Option<Color>,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub emphasis_fg: Color,
    #[serde(skip)]
    emphasis_fg_dim: OnceLock<Color>,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub fg: Color,
    #[serde(skip)]
    fg_dim: OnceLock<Color>,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub boxchar_fg: Color,
    /// Factor for darkening colors when unfocused (0.0 = black, 1.0 = no change)
    pub darkening_factor: f32,
    #[serde(skip)]
    boxchar_fg_dim: OnceLock<Color>,
    #[serde(skip)]
    bg_dim: OnceLock<Option<Color>>,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub selection_text_fg: Color,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(serialize_with = "serialize_color")]
    pub selection_text_bg: Color,
    #[serde(skip)]
    selection_text_fg_dim: OnceLock<Color>,
    #[serde(skip)]
    selection_text_bg_dim: OnceLock<Color>,
}

impl Theme {
    /// Create a new theme with the given colors
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: &'static str,
        bg: Option<Color>,
        emphasis_fg: Color,
        fg: Color,
        boxchar_fg: Color,
        selection_text_fg: Color,
        selection_text_bg: Color,
        darkening_factor: f32,
    ) -> Self {
        Self {
            name,
            bg,
            emphasis_fg,
            emphasis_fg_dim: OnceLock::new(),
            fg,
            fg_dim: OnceLock::new(),
            boxchar_fg,
            darkening_factor,
            boxchar_fg_dim: OnceLock::new(),
            bg_dim: OnceLock::new(),
            selection_text_fg,
            selection_text_bg,
            selection_text_fg_dim: OnceLock::new(),
            selection_text_bg_dim: OnceLock::new(),
        }
    }

    /// Get a darkened version of fg, computed lazily and cached
    pub fn fg_dark(&self) -> Color {
        *self
            .fg_dim
            .get_or_init(|| darken_color(self.fg, self.darkening_factor))
    }

    /// Get a darkened version of boxchar_fg, computed lazily and cached
    pub fn boxchar_fg_dark(&self) -> Color {
        *self
            .boxchar_fg_dim
            .get_or_init(|| darken_color(self.boxchar_fg, self.darkening_factor))
    }

    /// Get a darkened version of bg, computed lazily and cached
    pub fn bg_dark(&self) -> Option<Color> {
        *self
            .bg_dim
            .get_or_init(|| self.bg.map(|c| darken_color(c, self.darkening_factor)))
    }

    /// Get a darkened version of selection_text_fg, computed lazily and cached
    pub fn selection_text_fg_dark(&self) -> Color {
        *self
            .selection_text_fg_dim
            .get_or_init(|| darken_color(self.selection_text_fg, self.darkening_factor))
    }

    /// Get a darkened version of selection_text_bg, computed lazily and cached
    pub fn selection_text_bg_dark(&self) -> Color {
        *self
            .selection_text_bg_dim
            .get_or_init(|| darken_color(self.selection_text_bg, self.darkening_factor))
    }

    /// Get a darkened version of emphasis_fg, computed lazily and cached
    pub fn emphasis_fg_dark(&self) -> Color {
        *self
            .emphasis_fg_dim
            .get_or_init(|| darken_color(self.emphasis_fg, self.darkening_factor))
    }
}

pub static THEME_ID_ORANGE: &str = "orange";
pub static THEME_ID_GREEN: &str = "green";
pub static THEME_ID_BLUE: &str = "blue";
pub static THEME_ID_PURPLE: &str = "purple";
pub static THEME_ID_WHITE: &str = "white";
pub static THEME_ID_RED: &str = "red";
pub static THEME_ID_YELLOW: &str = "yellow";
pub static THEME_ID_CYAN: &str = "cyan";
pub static THEME_ID_NORTH_STARS: &str = "north_stars";
pub static THEME_ID_HABS: &str = "habs";
pub static THEME_ID_SABRES: &str = "sabres";
pub static THEME_ID_SHARKS: &str = "sharks";
pub static THEME_ID_BRUINS: &str = "bruins";
pub static THEME_ID_ISLANDERS: &str = "islanders";
pub static THEME_ID_FLAMES: &str = "flames";
pub static THEME_ID_RED_WINGS: &str = "red_wings";

pub static THEME_ORANGE: Theme = Theme::new(
    "Orange",
    None,
    Color::Rgb(255, 214, 128),
    Color::Rgb(255, 175, 64),
    Color::Rgb(226, 108, 34),
    Color::Rgb(0, 0, 0),
    Color::Rgb(255, 175, 64),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_GREEN: Theme = Theme::new(
    "Green",
    None,
    Color::Rgb(175, 255, 135),
    Color::Rgb(95, 255, 175),
    Color::Rgb(0, 255, 0),
    Color::Rgb(0, 0, 0),
    Color::Rgb(95, 255, 175),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_BLUE: Theme = Theme::new(
    "Blue",
    None,
    Color::Rgb(175, 255, 255),
    Color::Rgb(95, 135, 255),
    Color::Rgb(0, 95, 255),
    Color::White,
    Color::Rgb(0, 95, 255),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_PURPLE: Theme = Theme::new(
    "Purple",
    None,
    Color::Rgb(255, 175, 255),
    Color::Rgb(175, 135, 255),
    Color::Rgb(135, 95, 175),
    Color::Rgb(0, 0, 0),
    Color::Rgb(175, 135, 255),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_WHITE: Theme = Theme::new(
    "White",
    None,
    Color::Rgb(255, 255, 255),
    Color::Rgb(192, 192, 192),
    Color::Rgb(128, 128, 128),
    Color::Rgb(0, 0, 0),
    Color::Rgb(192, 192, 192),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_RED: Theme = Theme::new(
    "Red",
    None,
    Color::Rgb(255, 175, 175),
    Color::Rgb(255, 95, 95),
    Color::Rgb(255, 0, 0),
    Color::Rgb(0, 0, 0),
    Color::Rgb(255, 95, 95),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_YELLOW: Theme = Theme::new(
    "Yellow",
    None,
    Color::Rgb(255, 255, 175),
    Color::Rgb(255, 255, 95),
    Color::Rgb(255, 215, 0),
    Color::Rgb(0, 0, 0),
    Color::Rgb(255, 255, 95),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_CYAN: Theme = Theme::new(
    "Cyan",
    None,
    Color::Rgb(175, 255, 255),
    Color::Rgb(95, 255, 255),
    Color::Rgb(0, 255, 255),
    Color::Rgb(0, 0, 0),
    Color::Rgb(95, 255, 255),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_NORTH_STARS: Theme = Theme::new(
    "North Stars",
    None,
    Color::Rgb(240, 240, 240),
    Color::Rgb(198, 146, 20),
    Color::Rgb(0, 122, 51),
    Color::Rgb(0, 0, 0),
    Color::Rgb(198, 146, 20),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_HABS: Theme = Theme::new(
    "Habs",
    Some(Color::Rgb(175, 30, 45)),
    Color::Rgb(255, 255, 255),
    Color::Rgb(255, 255, 255),
    Color::Rgb(45, 53, 124),
    Color::Rgb(255, 255, 255),
    Color::Rgb(45, 53, 124),
    BRIGHT_BG_DARKENING_FACTOR,
);

pub static THEME_SABRES: Theme = Theme::new(
    "Sabres",
    None,
    Color::Rgb(255, 255, 255),
    Color::Rgb(255, 184, 28),
    Color::Rgb(0, 48, 135),
    Color::Rgb(0, 0, 0),
    Color::Rgb(255, 184, 28),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_SHARKS: Theme = Theme::new(
    "Sharks",
    None,
    Color::Rgb(255, 255, 255),
    Color::Rgb(0, 109, 117),
    Color::Rgb(234, 114, 0),
    Color::Rgb(255, 255, 255),
    Color::Rgb(0, 109, 117),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_BRUINS: Theme = Theme::new(
    "Bruins",
    None,
    Color::Rgb(255, 255, 255),
    Color::Rgb(252, 181, 20),
    Color::Rgb(196, 196, 196),
    Color::Rgb(0, 0, 0),
    Color::Rgb(252, 181, 20),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_ISLANDERS: Theme = Theme::new(
    "Islanders",
    None,
    Color::Rgb(255, 255, 255),
    Color::Rgb(252, 76, 2),
    Color::Rgb(0, 58, 162),
    Color::Rgb(0, 0, 0),
    Color::Rgb(252, 76, 2),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_FLAMES: Theme = Theme::new(
    "Flames",
    None,
    Color::Rgb(255, 255, 255),
    Color::Rgb(200, 16, 46),
    Color::Rgb(241, 190, 72),
    Color::Rgb(255, 255, 255),
    Color::Rgb(200, 16, 46),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEME_RED_WINGS: Theme = Theme::new(
    "Red Wings",
    None,
    Color::Rgb(255, 82, 102),
    Color::Rgb(206, 17, 38),
    Color::Rgb(255, 255, 255),
    Color::Rgb(255, 255, 255),
    Color::Rgb(206, 17, 38),
    DEFAULT_DARKENING_FACTOR,
);

pub static THEMES: phf::Map<&'static str, &Theme> = phf_map! {
    "orange" => &THEME_ORANGE,
    "green"  => &THEME_GREEN,
    "blue"   => &THEME_BLUE,
    "purple" => &THEME_PURPLE,
    "white"  => &THEME_WHITE,
    "red"    => &THEME_RED,
    "yellow" => &THEME_YELLOW,
    "cyan"   => &THEME_CYAN,
    "north_stars" => &THEME_NORTH_STARS,
    "habs" => &THEME_HABS,
    "sabres" => &THEME_SABRES,
    "sharks" => &THEME_SHARKS,
    "bruins" => &THEME_BRUINS,
    "islanders" => &THEME_ISLANDERS,
    "flames" => &THEME_FLAMES,
    "red_wings" => &THEME_RED_WINGS,
};

impl Default for Theme {
    fn default() -> Self {
        THEME_WHITE.clone()
    }
}

/// Darken a color by a given factor (0.0 = black, 1.0 = original)
fn darken_color(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ),
        // For named colors, return them as-is (could convert to RGB if needed)
        other => other,
    }
}

/// Deserialize a color from a string (supports named colors, RGB hex, or RGB tuple)
pub(super) fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_color(&s).ok_or_else(|| serde::de::Error::custom(format!("Invalid color: {}", s)))
}

/// Deserialize an optional color from a string
fn deserialize_color_optional<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(color_str) => {
            let color = parse_color(&color_str)
                .ok_or_else(|| serde::de::Error::custom(format!("Invalid color: {}", color_str)))?;
            Ok(Some(color))
        }
        None => Ok(None),
    }
}

/// Serialize a color to a string
pub(super) fn serialize_color<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format_color(color))
}

/// Serialize an optional color to a string
fn serialize_color_optional<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match color {
        Some(c) => serializer.serialize_str(&format_color(c)),
        None => serializer.serialize_none(),
    }
}

/// Format a color as a string (RGB format for serialization)
pub(super) fn format_color(color: &Color) -> String {
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
