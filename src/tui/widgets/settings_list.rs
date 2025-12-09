use crate::config::{Config, DisplayConfig};
use crate::tui::component::ElementWidget;
use crate::tui::SettingsCategory;
/// SettingsListWidget - displays a read-only list of settings with their current values
///
/// This widget renders a simple key-value table for configuration settings.
/// Settings are displayed as "Setting Name: Current Value" pairs.
use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line};

/// Widget for displaying settings list
#[derive(Debug, Clone)]
pub struct SettingsListWidget {
    /// Category of settings to display
    pub category: SettingsCategory,
    /// Configuration to display settings from
    pub config: Config,
    /// Left margin for indentation
    pub margin: u16,
    /// Index of selected setting (for highlighting)
    pub selected_index: Option<usize>,
    /// Whether we're in settings navigation mode
    pub settings_mode: bool,
    /// Whether we're editing a setting
    pub editing: bool,
    /// Edit buffer for current edit
    pub edit_buffer: String,
}

impl SettingsListWidget {
    /// Create a new SettingsListWidget
    pub fn new(
        category: SettingsCategory,
        config: Config,
        margin: u16,
        selected_index: Option<usize>,
        settings_mode: bool,
        editing: bool,
        edit_buffer: String,
    ) -> Self {
        Self {
            category,
            config,
            margin,
            selected_index,
            settings_mode,
            editing,
            edit_buffer,
        }
    }

    /// Get the settings to display for this category
    fn get_settings(&self) -> Vec<(String, String)> {
        match self.category {
            SettingsCategory::Logging => vec![
                ("Log Level".to_string(), self.config.log_level.clone()),
                ("Log File".to_string(), self.config.log_file.clone()),
            ],
            SettingsCategory::Display => vec![
                (
                    "Theme".to_string(),
                    self.config
                        .display
                        .theme
                        .as_ref()
                        .map(|t| t.name.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                (
                    "Use Unicode".to_string(),
                    self.config.display.use_unicode.to_string(),
                ),
                (
                    "Error Color".to_string(),
                    format_color(&self.config.display.error_fg),
                ),
            ],
            SettingsCategory::Data => vec![
                (
                    "Refresh Interval".to_string(),
                    format!("{} seconds", self.config.refresh_interval),
                ),
                (
                    "Western Teams First".to_string(),
                    self.config.display_standings_western_first.to_string(),
                ),
                ("Time Format".to_string(), self.config.time_format.clone()),
            ],
        }
    }
}

impl ElementWidget for SettingsListWidget {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        let settings = self.get_settings();
        let x = area.x + self.margin;
        let mut y = area.y + 1;

        // Calculate max key length (including colon) for alignment
        let max_key_len = settings
            .iter()
            .map(|(key, _)| key.len() + 1) // +1 for the colon
            .max()
            .unwrap_or(0);

        for (index, (key, value)) in settings.iter().enumerate() {
            if y >= area.y + area.height {
                break; // Stop if we run out of space
            }

            // Check if this setting is being edited
            let is_editing =
                self.editing && self.settings_mode && self.selected_index == Some(index);

            // Format value with cursor if editing
            let display_value = if is_editing {
                format!("{}█", &self.edit_buffer)
            } else {
                value.clone()
            };

            // Check if this setting is selected
            let is_selected = self.settings_mode && self.selected_index == Some(index);

            // Format selector indicator
            let selector = if is_selected {
                format!("{} ", config.box_chars.selector)
            } else {
                "  ".to_string()
            };

            // Format as "Key:  Value" with padding for alignment
            let key_with_colon = format!("{}:", key);
            let line_text = format!(
                "{}{:width$}  {}",
                selector,
                key_with_colon,
                display_value,
                width = max_key_len
            );

            // Apply fg2 style from theme (or default if no theme), with REVERSED and BOLD for selection
            let style = if let Some(theme) = &config.theme {
                Style::default().fg(theme.fg2)
            } else {
                Style::default()
            };

            let line = Line::from(line_text).style(style);

            // Render the line
            let line_width = line.width() as u16;
            if x + line_width <= area.x + area.width {
                buf.set_line(x, y, &line, line_width);
            }

            y += 1;
        }
    }

    fn clone_box(&self) -> Box<dyn ElementWidget> {
        Box::new(self.clone())
    }

    fn preferred_height(&self) -> Option<u16> {
        Some(self.get_settings().len() as u16 + 1)
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
    use crate::tui::testing::{assert_buffer, RENDER_WIDTH};
    use ratatui::{buffer::Buffer, layout::Rect};

    #[test]
    fn test_settings_list_logging_category() {
        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config.clone(),
            2,
            None,
            false,
            false,
            String::new(),
        );

        let area = Rect::new(0, 0, RENDER_WIDTH, 3);
        let mut buf = Buffer::empty(area);
        let display_config = DisplayConfig::default();

        widget.render(area, &mut buf, &display_config);

        // Verify both settings are rendered with aligned values (margin + selector space)
        assert_buffer(
            &buf,
            &[
                "",
                &format!("    Log Level:  {}", config.log_level),
                &format!("    Log File:   {}", config.log_file),
            ],
        );
    }

    #[test]
    fn test_settings_list_display_category() {
        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Display,
            config,
            2,
            None,
            false,
            false,
            String::new(),
        );

        let settings = widget.get_settings();
        assert_eq!(settings.len(), 3);
        assert_eq!(settings[0].0, "Theme");
        assert_eq!(settings[1].0, "Use Unicode");
        assert_eq!(settings[2].0, "Error Color");
    }

    #[test]
    fn test_settings_list_data_category() {
        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Data,
            config.clone(),
            2,
            None,
            false,
            false,
            String::new(),
        );

        let settings = widget.get_settings();
        assert_eq!(settings.len(), 3);
        assert_eq!(settings[0].0, "Refresh Interval");
        assert_eq!(
            settings[0].1,
            format!("{} seconds", config.refresh_interval)
        );
    }

    #[test]
    fn test_format_color() {
        use ratatui::style::Color;

        assert_eq!(format_color(&Color::Red), "red");
        assert_eq!(format_color(&Color::Rgb(255, 165, 0)), "rgb(255, 165, 0)");
        assert_eq!(format_color(&Color::Cyan), "cyan");
    }

    #[test]
    fn test_settings_list_editing_mode() {
        use crate::tui::testing::assert_buffer;

        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config.clone(),
            2,
            Some(1), // Select "Log File"
            true,    // settings_mode = true
            true,    // editing = true
            "/tmp/test.log".to_string(),
        );

        let area = Rect::new(0, 0, RENDER_WIDTH, 3);
        let mut buf = Buffer::empty(area);
        let display_config = DisplayConfig::default();

        widget.render(area, &mut buf, &display_config);

        // Log File should show with edit cursor
        assert_buffer(
            &buf,
            &[
                "",
                &format!("    Log Level:  {}", config.log_level),
                "  ▶ Log File:   /tmp/test.log█",
            ],
        );
    }

    #[test]
    fn test_settings_list_not_editing() {
        use crate::tui::testing::assert_buffer;

        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config.clone(),
            2,
            Some(1), // Select "Log File"
            true,    // settings_mode = true
            false,   // editing = false
            String::new(),
        );

        let area = Rect::new(0, 0, RENDER_WIDTH, 3);
        let mut buf = Buffer::empty(area);
        let display_config = DisplayConfig::default();

        widget.render(area, &mut buf, &display_config);

        // Log File should show without edit cursor
        assert_buffer(
            &buf,
            &[
                "",
                &format!("    Log Level:  {}", config.log_level),
                &format!("  ▶ Log File:   {}", config.log_file),
            ],
        );
    }

    #[test]
    fn test_settings_list_empty_edit_buffer() {
        use crate::tui::testing::assert_buffer;

        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config,
            2,
            Some(1),       // Select "Log File"
            true,          // settings_mode = true
            true,          // editing = true
            String::new(), // Empty buffer
        );

        let area = Rect::new(0, 0, RENDER_WIDTH, 3);
        let mut buf = Buffer::empty(area);
        let display_config = DisplayConfig::default();

        widget.render(area, &mut buf, &display_config);

        // Log File should show with just cursor
        assert_buffer(&buf, &["", "    Log Level:  info", "  ▶ Log File:   █"]);
    }

    #[test]
    fn test_settings_list_truncates_when_area_too_small() {
        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config,
            2,
            None,
            false,
            false,
            String::new(),
        );

        // Very small area - only 1 line
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        let display_config = DisplayConfig::default();

        widget.render(area, &mut buf, &display_config);

        // Should render only first setting without panicking
        // Logging has 2 settings, but only 1 should render
    }

    #[test]
    fn test_settings_list_preferred_height() {
        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config,
            2,
            None,
            false,
            false,
            String::new(),
        );

        // Logging category has 2 settings + 1 top margin
        assert_eq!(widget.preferred_height(), Some(3));
    }

    #[test]
    fn test_format_color_all_variants() {
        use ratatui::style::Color;

        assert_eq!(format_color(&Color::Black), "black");
        assert_eq!(format_color(&Color::Red), "red");
        assert_eq!(format_color(&Color::Green), "green");
        assert_eq!(format_color(&Color::Yellow), "yellow");
        assert_eq!(format_color(&Color::Blue), "blue");
        assert_eq!(format_color(&Color::Magenta), "magenta");
        assert_eq!(format_color(&Color::Cyan), "cyan");
        assert_eq!(format_color(&Color::Gray), "gray");
        assert_eq!(format_color(&Color::DarkGray), "darkgray");
        assert_eq!(format_color(&Color::LightRed), "lightred");
        assert_eq!(format_color(&Color::LightGreen), "lightgreen");
        assert_eq!(format_color(&Color::LightYellow), "lightyellow");
        assert_eq!(format_color(&Color::LightBlue), "lightblue");
        assert_eq!(format_color(&Color::LightMagenta), "lightmagenta");
        assert_eq!(format_color(&Color::LightCyan), "lightcyan");
        assert_eq!(format_color(&Color::White), "white");
        assert_eq!(format_color(&Color::Rgb(128, 64, 32)), "rgb(128, 64, 32)");
    }

    #[test]
    fn test_settings_list_clone_box() {
        let config = Config::default();
        let widget = SettingsListWidget::new(
            SettingsCategory::Logging,
            config,
            2,
            None,
            false,
            false,
            String::new(),
        );

        let _cloned: Box<dyn ElementWidget> = widget.clone_box();
        // If we get here, clone_box() worked
    }
}
