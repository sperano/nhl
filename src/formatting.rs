use crate::config::DisplayConfig;

/// Box-drawing characters for table borders
#[derive(Debug, Clone, PartialEq)]
pub struct BoxChars {
    // Single-line characters
    pub horizontal: String,
    pub vertical: String,
    pub top_left: String,
    pub top_right: String,
    pub bottom_left: String,
    pub bottom_right: String,
    pub top_junction: String,
    pub bottom_junction: String,
    pub left_junction: String,
    pub right_junction: String,
    pub cross: String,

    // Double-line characters
    pub double_horizontal: String,
    pub double_vertical: String,
    pub double_top_left: String,
    pub double_top_right: String,
    pub double_bottom_left: String,
    pub double_bottom_right: String,
    pub double_top_junction: String,
    pub double_bottom_junction: String,

    // Mixed double-vertical/single-horizontal junctions
    pub mixed_left_junction: String,
    pub mixed_right_junction: String,

    // Mixed double-horizontal/single-vertical (for TeamBoxscore borders)
    pub mixed_dh_top_left: String,
    pub mixed_dh_top_right: String,
    pub mixed_dh_bottom_left: String,
    pub mixed_dh_bottom_right: String,
    pub mixed_dh_left_t: String,
    pub mixed_dh_right_t: String,

    // Other characters
    pub connector2: String,
    pub connector3: String,
    pub selector: String,
    pub breadcrumb_separator: String,
}

impl BoxChars {
    pub fn unicode() -> Self {
        Self {
            // Single-line
            horizontal: "─".to_string(),
            vertical: "│".to_string(),
            top_left: "╭".to_string(),
            top_right: "╮".to_string(),
            bottom_left: "╰".to_string(),
            bottom_right: "╯".to_string(),
            top_junction: "┬".to_string(),
            bottom_junction: "┴".to_string(),
            left_junction: "├".to_string(),
            right_junction: "┤".to_string(),
            cross: "┼".to_string(),

            // Double-line
            double_horizontal: "═".to_string(),
            double_vertical: "║".to_string(),
            double_top_left: "╔".to_string(),
            double_top_right: "╗".to_string(),
            double_bottom_left: "╚".to_string(),
            double_bottom_right: "╝".to_string(),
            double_top_junction: "╤".to_string(),
            double_bottom_junction: "╧".to_string(),

            // Mixed (double vertical, single horizontal)
            mixed_left_junction: "╟".to_string(),
            mixed_right_junction: "╢".to_string(),

            // Mixed (double horizontal, single vertical)
            mixed_dh_top_left: "╒".to_string(),
            mixed_dh_top_right: "╕".to_string(),
            mixed_dh_bottom_left: "╘".to_string(),
            mixed_dh_bottom_right: "╛".to_string(),
            mixed_dh_left_t: "╞".to_string(),
            mixed_dh_right_t: "╡".to_string(),

            // Other
            connector2: "┴".to_string(),
            connector3: "┬".to_string(),
            selector: "▶".to_string(),
            breadcrumb_separator: "▶".to_string(),
        }
    }

    pub fn ascii() -> Self {
        Self {
            // Single-line
            horizontal: "-".to_string(),
            vertical: "|".to_string(),
            top_left: "+".to_string(),
            top_right: "+".to_string(),
            bottom_left: "+".to_string(),
            bottom_right: "+".to_string(),
            top_junction: "+".to_string(),
            bottom_junction: "+".to_string(),
            left_junction: "+".to_string(),
            right_junction: "+".to_string(),
            cross: "+".to_string(),

            // Double-line
            double_horizontal: "=".to_string(),
            double_vertical: "|".to_string(),
            double_top_left: "+".to_string(),
            double_top_right: "+".to_string(),
            double_bottom_left: "+".to_string(),
            double_bottom_right: "+".to_string(),
            double_top_junction: "+".to_string(),
            double_bottom_junction: "+".to_string(),

            // Mixed
            mixed_left_junction: "+".to_string(),
            mixed_right_junction: "+".to_string(),

            // Mixed (double horizontal, single vertical)
            mixed_dh_top_left: "+".to_string(),
            mixed_dh_top_right: "+".to_string(),
            mixed_dh_bottom_left: "+".to_string(),
            mixed_dh_bottom_right: "+".to_string(),
            mixed_dh_left_t: "+".to_string(),
            mixed_dh_right_t: "+".to_string(),

            // Other
            connector2: "-".to_string(),
            connector3: "-".to_string(),
            selector: ">".to_string(),
            breadcrumb_separator: ">".to_string(),
        }
    }

    pub fn from_use_unicode(use_unicode: bool) -> Self {
        if use_unicode {
            Self::unicode()
        } else {
            Self::ascii()
        }
    }
}

/// Format a header with text and underline
///
/// # Arguments
/// * `text` - The header text to display
/// * `double_line` - If true, uses double-line (═/=), otherwise single-line (─/-)
/// * `display` - Display configuration to determine unicode vs ASCII
///
/// # Returns
/// A formatted string with the header text and underline separator matching the text length
pub fn format_header(text: &str, double_line: bool, display: &DisplayConfig) -> String {
    let separator_char = if double_line {
        &display.box_chars.double_horizontal
    } else {
        &display.box_chars.horizontal
    };
    format!("{}\n{}\n", text, separator_char.repeat(text.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_header_single_line_unicode() {
        let display = DisplayConfig {
            use_unicode: true,
            ..Default::default()
        };
        let result = format_header("Test Header", false, &display);
        assert_eq!(result, "Test Header\n───────────\n");
    }

    #[test]
    fn test_format_header_double_line_unicode() {
        let display = DisplayConfig {
            use_unicode: true,
            ..Default::default()
        };
        let result = format_header("Test Header", true, &display);
        assert_eq!(result, "Test Header\n═══════════\n");
    }

    #[test]
    fn test_format_header_single_line_ascii() {
        let mut display = DisplayConfig {
            use_unicode: false,
            ..Default::default()
        };
        display.box_chars = BoxChars::ascii();
        let result = format_header("Test Header", false, &display);
        assert_eq!(result, "Test Header\n-----------\n");
    }

    #[test]
    fn test_format_header_double_line_ascii() {
        let mut display = DisplayConfig {
            use_unicode: false,
            ..Default::default()
        };
        display.box_chars = BoxChars::ascii();
        let result = format_header("Test Header", true, &display);
        assert_eq!(result, "Test Header\n===========\n");
    }

    #[test]
    fn test_empty_header() {
        let display = DisplayConfig {
            use_unicode: true,
            ..Default::default()
        };
        let result = format_header("", false, &display);
        assert_eq!(result, "\n\n");
    }

    #[test]
    fn test_long_header() {
        let display = DisplayConfig {
            use_unicode: true,
            ..Default::default()
        };
        let result = format_header("This is a very long header text", true, &display);
        assert_eq!(
            result,
            "This is a very long header text\n═══════════════════════════════\n"
        );
    }
}
