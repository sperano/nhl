use crate::config::DisplayConfig;
use std::fmt::Display;

/// Box-drawing characters for table borders
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxChars {
    // Single-line characters
    pub horizontal: &'static str,
    pub vertical: &'static str,
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
    pub top_junction: &'static str,
    pub bottom_junction: &'static str,
    pub left_junction: &'static str,
    pub right_junction: &'static str,
    pub cross: &'static str,

    // Double-line characters
    pub double_horizontal: &'static str,
    pub double_vertical: &'static str,
    pub double_top_left: &'static str,
    pub double_top_right: &'static str,
    pub double_bottom_left: &'static str,
    pub double_bottom_right: &'static str,
    pub double_top_junction: &'static str,
    pub double_bottom_junction: &'static str,

    // Mixed double-vertical/single-horizontal junctions
    pub mixed_left_junction: &'static str,
    pub mixed_right_junction: &'static str,

    // Mixed double-horizontal/single-vertical (for TeamBoxscore borders)
    pub mixed_dh_top_left: &'static str,
    pub mixed_dh_top_right: &'static str,
    pub mixed_dh_bottom_left: &'static str,
    pub mixed_dh_bottom_right: &'static str,
    pub mixed_dh_left_t: &'static str,
    pub mixed_dh_right_t: &'static str,

    // Other characters
    pub connector2: &'static str,
    pub connector3: &'static str,
    pub selector: &'static str,
    pub breadcrumb_separator: &'static str,
    pub checkmark: &'static str,
}

impl BoxChars {
    pub fn unicode() -> Self {
        Self {
            // Single-line
            horizontal: "─",
            vertical: "│",
            top_left: "╭",
            top_right: "╮",
            bottom_left: "╰",
            bottom_right: "╯",
            top_junction: "┬",
            bottom_junction: "┴",
            left_junction: "├",
            right_junction: "┤",
            cross: "┼",

            // Double-line
            double_horizontal: "═",
            double_vertical: "║",
            double_top_left: "╔",
            double_top_right: "╗",
            double_bottom_left: "╚",
            double_bottom_right: "╝",
            double_top_junction: "╤",
            double_bottom_junction: "╧",

            // Mixed (double vertical, single horizontal)
            mixed_left_junction: "╟",
            mixed_right_junction: "╢",

            // Mixed (double horizontal, single vertical)
            mixed_dh_top_left: "╒",
            mixed_dh_top_right: "╕",
            mixed_dh_bottom_left: "╘",
            mixed_dh_bottom_right: "╛",
            mixed_dh_left_t: "╞",
            mixed_dh_right_t: "╡",

            // Other
            connector2: "┴",
            connector3: "┬",
            selector: "▶",
            breadcrumb_separator: "▶",
            checkmark: "✓",
        }
    }

    pub fn ascii() -> Self {
        Self {
            // Single-line
            horizontal: "-",
            vertical: "|",
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            top_junction: "+",
            bottom_junction: "+",
            left_junction: "+",
            right_junction: "+",
            cross: "+",

            // Double-line
            double_horizontal: "=",
            double_vertical: "|",
            double_top_left: "+",
            double_top_right: "+",
            double_bottom_left: "+",
            double_bottom_right: "+",
            double_top_junction: "+",
            double_bottom_junction: "+",

            // Mixed
            mixed_left_junction: "+",
            mixed_right_junction: "+",

            // Mixed (double horizontal, single vertical)
            mixed_dh_top_left: "+",
            mixed_dh_top_right: "+",
            mixed_dh_bottom_left: "+",
            mixed_dh_bottom_right: "+",
            mixed_dh_left_t: "+",
            mixed_dh_right_t: "+",

            // Other
            connector2: "-",
            connector3: "-",
            selector: ">",
            breadcrumb_separator: ">",
            checkmark: "*",
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

/// Placeholder shown for a stat the API did not send.
pub const MISSING_STAT: &str = "-";

/// Format an optional stat, showing [`MISSING_STAT`] when it is absent.
pub fn format_optional<T: Display>(value: Option<T>) -> String {
    value.map_or_else(|| MISSING_STAT.to_string(), |v| v.to_string())
}

#[cfg(test)]
#[path = "formatting_tests.rs"]
mod tests;
