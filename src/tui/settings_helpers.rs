/// Shared helper functions for settings management
///
/// This module provides common functionality used by the settings tab
/// for managing settings modals.
use crate::config::{
    Config, THEMES, THEME_ID_BLUE, THEME_ID_CYAN, THEME_ID_GREEN, THEME_ID_NORTH_STARS,
    THEME_ID_ORANGE, THEME_ID_PURPLE, THEME_ID_RED, THEME_ID_WHITE, THEME_ID_YELLOW,
    THEME_ID_HABS, THEME_ID_SABRES,
};

const THEME_IDS: &[&str] = &[
    THEME_ID_ORANGE,
    THEME_ID_GREEN,
    THEME_ID_BLUE,
    THEME_ID_PURPLE,
    THEME_ID_WHITE,
    THEME_ID_RED,
    THEME_ID_YELLOW,
    THEME_ID_CYAN,
    THEME_ID_NORTH_STARS,
    THEME_ID_HABS,
    THEME_ID_SABRES,
];

/// Modal option with ID and display name
#[derive(Debug, Clone)]
pub struct ModalOption {
    pub id: String,
    pub display_name: String,
}

/// Get modal options (ID + display name pairs) for a setting
pub fn get_setting_modal_options(key: &str) -> Vec<ModalOption> {
    match key {
        "log_level" => vec![
            ModalOption {
                id: "trace".to_string(),
                display_name: "Trace".to_string(),
            },
            ModalOption {
                id: "debug".to_string(),
                display_name: "Debug".to_string(),
            },
            ModalOption {
                id: "info".to_string(),
                display_name: "Info".to_string(),
            },
            ModalOption {
                id: "warn".to_string(),
                display_name: "Warn".to_string(),
            },
            ModalOption {
                id: "error".to_string(),
                display_name: "Error".to_string(),
            },
        ],
        "theme" => {
            let mut options = vec![ModalOption {
                id: "none".to_string(),
                display_name: "None".to_string(),
            }];
            for id in THEME_IDS {
                if let Some(theme) = THEMES.get(id) {
                    options.push(ModalOption {
                        id: id.to_string(),
                        display_name: theme.name.to_string(),
                    });
                }
            }
            options
        }
        _ => vec![], // Empty for non-list settings
    }
}

/// Get the list of valid values for a setting that has a fixed set of options
fn get_setting_values(key: &str) -> Vec<&'static str> {
    match key {
        "log_level" => vec!["trace", "debug", "info", "warn", "error"],
        "theme" => {
            let mut values = vec!["none"];
            values.extend(THEME_IDS.iter().copied());
            values
        }
        _ => vec![], // Empty for non-list settings
    }
}

/// Get the current value of a setting from the config
fn get_current_setting_value(config: &Config, key: &str) -> String {
    match key {
        "log_level" => config.log_level.clone(),
        "theme" => config
            .display
            .theme_name
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "none".to_string()),
        _ => String::new(),
    }
}

/// Find the initial index for a modal based on the current setting value
pub fn find_initial_modal_index(config: &Config, key: &str) -> usize {
    let current_value = get_current_setting_value(config, key);
    let values = get_setting_values(key);

    values.iter().position(|&v| v == current_value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_initial_modal_index_theme_none() {
        let config = Config::default();
        assert_eq!(find_initial_modal_index(&config, "theme"), 0);
    }

    #[test]
    fn test_find_initial_modal_index_theme_blue() {
        use crate::config::THEME_ID_BLUE;
        let mut config = Config::default();
        config.display.theme_name = Some(THEME_ID_BLUE.to_string());
        // "blue" is at index 3 (after "none", "orange", "green")
        assert_eq!(find_initial_modal_index(&config, "theme"), 3);
    }

    #[test]
    fn test_find_initial_modal_index_log_level_info() {
        let mut config = Config::default();
        config.log_level = "info".to_string();
        // Log level values: ["trace", "debug", "info", "warn", "error"]
        // "info" is at index 2
        assert_eq!(find_initial_modal_index(&config, "log_level"), 2);
    }

    #[test]
    fn test_find_initial_modal_index_unknown_value_returns_zero() {
        let mut config = Config::default();
        config.log_level = "unknown_level".to_string();
        // Should return 0 when value not found in list
        assert_eq!(find_initial_modal_index(&config, "log_level"), 0);
    }

    #[test]
    fn test_get_setting_modal_options_log_level() {
        let options = get_setting_modal_options("log_level");
        assert_eq!(options.len(), 5);
        assert_eq!(options[0].id, "trace");
        assert_eq!(options[0].display_name, "Trace");
        assert_eq!(options[4].id, "error");
    }

    #[test]
    fn test_get_setting_modal_options_theme() {
        let options = get_setting_modal_options("theme");
        assert!(options.len() >= 2); // At least "none" + some themes
        assert_eq!(options[0].id, "none");
        assert_eq!(options[0].display_name, "None");
    }

    #[test]
    fn test_get_setting_modal_options_unknown() {
        let options = get_setting_modal_options("unknown");
        assert!(options.is_empty());
    }
}
