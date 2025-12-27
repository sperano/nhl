//! Settings document - displays settings in a document-based layout
//!
//! This module provides the document-based implementation for settings display.

use crate::config::Config;
use crate::tui::document::{Document, DocumentBuilder, DocumentElement, FocusContext, LinkTarget};
use crate::tui::SettingsCategory;

/// Settings document for a specific category
pub struct SettingsDocument {
    category: SettingsCategory,
    config: Config,
}

impl SettingsDocument {
    pub fn new(category: SettingsCategory, config: Config) -> Self {
        Self { category, config }
    }

    /// Build the logging settings section
    fn build_logging_settings(
        &self,
        builder: DocumentBuilder,
        focus: &FocusContext,
    ) -> DocumentBuilder {
        // Align labels: "Log Level:" is 10 chars (longest)
        const LABEL_WIDTH: usize = 10;

        builder
            .spacer(1)
            .link_with_focus(
                "log_level",
                format!(
                    "{:width$}   {}",
                    "Log Level:",
                    self.config.log_level,
                    width = LABEL_WIDTH
                ),
                LinkTarget::Action("edit:log_level".to_string()),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "log_file",
                format!(
                    "{:width$}   {}",
                    "Log File:",
                    self.config.log_file,
                    width = LABEL_WIDTH
                ),
                LinkTarget::Action("edit:log_file".to_string()),
                focus,
            )
    }

    /// Build the display settings section
    fn build_display_settings(
        &self,
        builder: DocumentBuilder,
        focus: &FocusContext,
    ) -> DocumentBuilder {
        // Align labels: "Use Unicode:" and "Error Color:" are 12 chars (longest)
        const LABEL_WIDTH: usize = 12;

        let theme_name = self
            .config
            .display
            .theme
            .as_ref()
            .map(|t| t.name.to_string())
            .unwrap_or_else(|| "none".to_string());

        builder
            .spacer(1)
            .link_with_focus(
                "theme",
                format!("{:width$}   {}", "Theme:", theme_name, width = LABEL_WIDTH),
                LinkTarget::Action("edit:theme".to_string()),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "use_unicode",
                format!(
                    "{:width$}   {}",
                    "Use Unicode:",
                    self.config.display.use_unicode,
                    width = LABEL_WIDTH
                ),
                LinkTarget::Action("toggle:use_unicode".to_string()),
                focus,
            )
            .spacer(1)
            .text(format!(
                "{:width$}   {}",
                "Error Color:",
                format_color(&self.config.display.error_fg),
                width = LABEL_WIDTH
            ))
    }

    /// Build the data settings section
    fn build_data_settings(
        &self,
        builder: DocumentBuilder,
        focus: &FocusContext,
    ) -> DocumentBuilder {
        // Align labels: "Western Teams First:" is 20 chars (longest)
        const LABEL_WIDTH: usize = 20;

        builder
            .spacer(1)
            .link_with_focus(
                "refresh_interval",
                format!(
                    "{:width$}   {} seconds",
                    "Refresh Interval:",
                    self.config.refresh_interval,
                    width = LABEL_WIDTH
                ),
                LinkTarget::Action("edit:refresh_interval".to_string()),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "western_teams_first",
                format!(
                    "{:width$}   {}",
                    "Western Teams First:",
                    self.config.display_standings_western_first,
                    width = LABEL_WIDTH
                ),
                LinkTarget::Action("toggle:western_teams_first".to_string()),
                focus,
            )
            .spacer(1)
            .link_with_focus(
                "time_format",
                format!(
                    "{:width$}   {}",
                    "Time Format:",
                    self.config.time_format,
                    width = LABEL_WIDTH
                ),
                LinkTarget::Action("edit:time_format".to_string()),
                focus,
            )
    }
}

impl Document for SettingsDocument {
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
        let builder = DocumentBuilder::new();

        let builder = match self.category {
            SettingsCategory::Logging => self.build_logging_settings(builder, focus),
            SettingsCategory::Display => self.build_display_settings(builder, focus),
            SettingsCategory::Data => self.build_data_settings(builder, focus),
        };

        builder.build()
    }

    fn title(&self) -> String {
        match self.category {
            SettingsCategory::Logging => "Logging Settings".to_string(),
            SettingsCategory::Display => "Display Settings".to_string(),
            SettingsCategory::Data => "Data Settings".to_string(),
        }
    }

    fn id(&self) -> String {
        match self.category {
            SettingsCategory::Logging => "settings_logging".to_string(),
            SettingsCategory::Display => "settings_display".to_string(),
            SettingsCategory::Data => "settings_data".to_string(),
        }
    }
}

/// Format a ratatui Color as a readable string
fn format_color(color: &ratatui::style::Color) -> String {
    use ratatui::style::Color;
    match color {
        Color::Rgb(r, g, b) => format!("rgb({}, {}, {})", r, g, b),
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
        _ => "custom".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_document_builds() {
        let config = Config::default();
        let doc = SettingsDocument::new(SettingsCategory::Logging, config);
        let elements = doc.build(&FocusContext::default());

        // Should have heading + settings
        assert!(elements.len() > 3);
    }

    #[test]
    fn test_display_document_builds() {
        let config = Config::default();
        let doc = SettingsDocument::new(SettingsCategory::Display, config);
        let elements = doc.build(&FocusContext::default());

        // Should have heading + settings
        assert!(elements.len() > 5);
    }

    #[test]
    fn test_data_document_builds() {
        let config = Config::default();
        let doc = SettingsDocument::new(SettingsCategory::Data, config);
        let elements = doc.build(&FocusContext::default());

        // Should have heading + settings
        assert!(elements.len() > 3);
    }

    #[test]
    fn test_document_titles() {
        let config = Config::default();

        let logging_doc = SettingsDocument::new(SettingsCategory::Logging, config.clone());
        assert_eq!(logging_doc.title(), "Logging Settings");

        let display_doc = SettingsDocument::new(SettingsCategory::Display, config.clone());
        assert_eq!(display_doc.title(), "Display Settings");

        let data_doc = SettingsDocument::new(SettingsCategory::Data, config);
        assert_eq!(data_doc.title(), "Data Settings");
    }

    #[test]
    fn test_format_color() {
        use ratatui::style::Color;

        assert_eq!(format_color(&Color::Red), "red");
        assert_eq!(format_color(&Color::Rgb(255, 165, 0)), "rgb(255, 165, 0)");
        assert_eq!(format_color(&Color::Cyan), "cyan");
    }
}
