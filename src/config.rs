use anyhow::Context;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use themes::{deserialize_color, serialize_color};
#[cfg(test)]
use themes::{format_color, parse_color};

#[path = "config_themes.rs"]
mod themes;

pub use themes::{
    Theme, THEME_ID_BLUE, THEME_ID_BRUINS, THEME_ID_CYAN, THEME_ID_FLAMES, THEME_ID_GREEN,
    THEME_ID_HABS, THEME_ID_ISLANDERS, THEME_ID_NORTH_STARS, THEME_ID_ORANGE, THEME_ID_PURPLE,
    THEME_ID_RED, THEME_ID_RED_WINGS, THEME_ID_SABRES, THEME_ID_SHARKS, THEME_ID_WHITE,
    THEME_ID_YELLOW, THEMES, THEME_BLUE, THEME_BRUINS, THEME_CYAN, THEME_FLAMES, THEME_GREEN,
    THEME_HABS, THEME_ISLANDERS, THEME_NORTH_STARS, THEME_ORANGE, THEME_PURPLE, THEME_RED,
    THEME_RED_WINGS, THEME_SABRES, THEME_SHARKS, THEME_WHITE, THEME_YELLOW,
};

/// Default refresh interval in seconds for background data fetching
pub const DEFAULT_REFRESH_INTERVAL_SECONDS: u32 = 60;

/// Style modifier for selected items (reversed and bold)
pub const SELECTION_STYLE_MODIFIER: Modifier = Modifier::BOLD;
pub const THEMELESS_SELECTION_STYLE_MODIFIER: Modifier = Modifier::REVERSED.union(Modifier::BOLD);

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

    /// Get the base style with just background color if theme specifies one
    pub fn base_style(&self) -> Style {
        self.theme
            .as_ref()
            .and_then(|t| t.bg)
            .map(|bg| Style::default().bg(bg))
            .unwrap_or_default()
    }

    /// Get the default text style using fg2 from theme
    ///
    /// This is the primary text color for normal content.
    pub fn text_style(&self) -> Style {
        self.theme
            .as_ref()
            .map(|t| {
                let style = Style::default().fg(t.fg);
                match t.bg {
                    Some(bg) => style.bg(bg),
                    None => style,
                }
            })
            .unwrap_or_default()
    }

    /// This is for separators, borders, etc
    pub fn boxchar_style(&self) -> Style {
        self.theme
            .as_ref()
            .map(|t| {
                let style = Style::default().fg(t.boxchar_fg);
                match t.bg {
                    Some(bg) => style.bg(bg),
                    None => style,
                }
            })
            .unwrap_or_default()
    }

    /// Dimmed version of boxchar_style for unfocused elements
    pub fn boxchar_style_dim(&self) -> Style {
        self.theme
            .as_ref()
            .map(|t| {
                let style = Style::default().fg(t.boxchar_fg_dark());
                match t.bg_dark() {
                    Some(bg) => style.bg(bg),
                    None => style,
                }
            })
            .unwrap_or_default()
    }

    /// Get a heading style with bold modifier
    pub fn heading_style(&self, level: u8) -> Style {
        let base = self.text_style();
        match level {
            1 | 2 => base.add_modifier(Modifier::BOLD),
            _ => base.add_modifier(Modifier::UNDERLINED),
        }
    }

    /// Dimmed version of base_style for unfocused elements
    pub fn base_style_dim(&self) -> Style {
        self.theme
            .as_ref()
            .and_then(|t| t.bg_dark())
            .map(|bg| Style::default().bg(bg))
            .unwrap_or_default()
    }

    /// Dimmed version of text_style for unfocused elements
    pub fn text_style_dim(&self) -> Style {
        self.theme
            .as_ref()
            .map(|t| {
                let style = Style::default().fg(t.fg_dark());
                match t.bg_dark() {
                    Some(bg) => style.bg(bg),
                    None => style,
                }
            })
            .unwrap_or_default()
    }

    /// Dimmed version of heading_style for unfocused elements
    pub fn heading_style_dim(&self, level: u8) -> Style {
        let base = self.text_style_dim();
        match level {
            1 | 2 => base.add_modifier(Modifier::BOLD),
            _ => base.add_modifier(Modifier::UNDERLINED),
        }
    }

    /// Get the emphasis style (bold with emphasis_fg color)
    ///
    /// Used for section titles and other emphasized text.
    pub fn emphasis_style(&self) -> Style {
        self.theme
            .as_ref()
            .map(|t| {
                let style = Style::default()
                    .fg(t.emphasis_fg)
                    .add_modifier(Modifier::BOLD);
                match t.bg {
                    Some(bg) => style.bg(bg),
                    None => style,
                }
            })
            .unwrap_or_else(|| Style::default().add_modifier(Modifier::BOLD))
    }

    /// Dimmed version of emphasis_style for unfocused elements
    pub fn emphasis_style_dim(&self) -> Style {
        self.theme
            .as_ref()
            .map(|t| {
                let style = Style::default()
                    .fg(t.emphasis_fg_dark())
                    .add_modifier(Modifier::BOLD);
                match t.bg_dark() {
                    Some(bg) => style.bg(bg),
                    None => style,
                }
            })
            .unwrap_or_else(|| Style::default().add_modifier(Modifier::BOLD))
    }
}

/// Render context that wraps DisplayConfig with focus state
///
/// This zero-cost wrapper provides style methods that automatically
/// select normal or dimmed variants based on whether the component is focused.
/// Use this instead of passing DisplayConfig directly to render functions.
pub struct RenderContext<'a> {
    pub config: &'a DisplayConfig,
    pub focused: bool,
    /// Tab selections for embedded tabs in documents (tabs_id -> active_index)
    pub tab_selections: HashMap<String, usize>,
    /// Cross-frame document render cache, owned by the TUI run loop and
    /// threaded down to `DocumentView` via `child()`. `None` (the default,
    /// and always the case in CLI paths and most tests) disables caching.
    pub doc_cache: Option<&'a RefCell<crate::tui::document::DocumentRenderCache>>,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context
    pub fn new(config: &'a DisplayConfig, focused: bool) -> Self {
        Self {
            config,
            focused,
            tab_selections: HashMap::new(),
            doc_cache: None,
        }
    }

    /// Create a focused render context (convenience for the common case)
    pub fn focused(config: &'a DisplayConfig) -> Self {
        Self::new(config, true)
    }

    /// Derive a child context with its own focus flag, preserving the config
    /// and the document render cache (but not tab selections, which are
    /// per-document and set by the widget that owns them).
    pub fn child(&self, focused: bool) -> RenderContext<'a> {
        RenderContext {
            config: self.config,
            focused,
            tab_selections: HashMap::new(),
            doc_cache: self.doc_cache,
        }
    }

    /// Set tab selections for embedded tabs
    pub fn with_tab_selections(mut self, selections: HashMap<String, usize>) -> Self {
        self.tab_selections = selections;
        self
    }

    /// Attach the cross-frame document render cache
    pub fn with_doc_cache(
        mut self,
        cache: &'a RefCell<crate::tui::document::DocumentRenderCache>,
    ) -> Self {
        self.doc_cache = Some(cache);
        self
    }

    /// Get the base style (with background color if theme specifies one)
    pub fn base_style(&self) -> Style {
        if self.focused {
            self.config.base_style()
        } else {
            self.config.base_style_dim()
        }
    }

    /// Get the text style
    pub fn text_style(&self) -> Style {
        if self.focused {
            self.config.text_style()
        } else {
            self.config.text_style_dim()
        }
    }

    /// Get the box character style (for borders, separators)
    pub fn boxchar_style(&self) -> Style {
        if self.focused {
            self.config.boxchar_style()
        } else {
            self.config.boxchar_style_dim()
        }
    }

    /// Get the heading style
    pub fn heading_style(&self, level: u8) -> Style {
        if self.focused {
            self.config.heading_style(level)
        } else {
            self.config.heading_style_dim(level)
        }
    }

    /// Get the emphasis style (for section titles)
    pub fn emphasis_style(&self) -> Style {
        if self.focused {
            self.config.emphasis_style()
        } else {
            self.config.emphasis_style_dim()
        }
    }

    /// Get the box drawing characters
    pub fn box_chars(&self) -> &crate::formatting::BoxChars {
        &self.config.box_chars
    }

    /// Check if unicode is enabled
    pub fn use_unicode(&self) -> bool {
        self.config.use_unicode
    }

    /// Get the theme if one is set
    pub fn theme(&self) -> Option<&Theme> {
        self.config.theme.as_ref()
    }

    /// Get the error foreground color
    pub fn error_fg(&self) -> Color {
        self.config.error_fg
    }
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

    let mut config: Config = toml::from_str(&content).unwrap_or_else(|err| {
        tracing::warn!(
            "Failed to parse config file at {}: {}. Falling back to default configuration.",
            config_path.display(),
            err
        );
        Config::default()
    });

    // Initialize box_chars based on use_unicode (since it's not serialized)
    config.display.box_chars =
        crate::formatting::BoxChars::from_use_unicode(config.display.use_unicode);

    // Apply theme based on theme_name (since it's not serialized)
    config.display.apply_theme();

    config
}

/// Write a config to the config file
pub fn write(config: &Config) -> anyhow::Result<()> {
    let config_path = get_config_path().context("Failed to get config path")?;

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
#[path = "config_tests.rs"]
mod tests;
