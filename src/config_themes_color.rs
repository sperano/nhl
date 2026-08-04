//! Color string parsing for theme configuration: named colors, hex codes, and RGB
//! tuples. Split out of `config_themes.rs` to keep that file within the project's
//! size guideline.

use ratatui::style::Color;

/// Parse a color string into a ratatui Color.
/// Supports named colors ("red"), hex ("#FF6600", "#f60"), and RGB tuples ("255,165,0").
pub(crate) fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();

    if let Some(color) = parse_named_color(&s) {
        return Some(color);
    }

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if s.contains(',') {
        return parse_rgb_tuple_color(&s);
    }

    None
}

/// Look up a named color (e.g. "red", "burntorange"). Declarative name->Color table.
fn parse_named_color(s: &str) -> Option<Color> {
    match s {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "orange" => Some(Color::Rgb(255, 165, 0)),
        "seafoam" => Some(Color::Rgb(159, 226, 191)),
        "deepred" | "deep red" => Some(Color::Rgb(226, 74, 74)),
        "coral" => Some(Color::Rgb(255, 107, 107)),
        "burntorange" | "burnt orange" => Some(Color::Rgb(255, 140, 66)),
        "amber" => Some(Color::Rgb(255, 200, 87)),
        "goldenrod" => Some(Color::Rgb(232, 185, 35)),
        "olive" => Some(Color::Rgb(166, 166, 89)),
        "chartreuse" => Some(Color::Rgb(140, 207, 77)),
        "greenapple" | "green apple" => Some(Color::Rgb(88, 196, 114)),
        "emerald" => Some(Color::Rgb(46, 184, 114)),
        "teal" => Some(Color::Rgb(42, 168, 118)),
        "cyansky" | "cyan sky" => Some(Color::Rgb(77, 208, 225)),
        "azure" => Some(Color::Rgb(33, 150, 243)),
        "cobaltblue" | "cobalt blue" => Some(Color::Rgb(61, 90, 254)),
        "indigo" => Some(Color::Rgb(92, 107, 192)),
        "violet" => Some(Color::Rgb(126, 87, 194)),
        "orchid" => Some(Color::Rgb(186, 104, 200)),
        "hotpink" | "hot pink" => Some(Color::Rgb(255, 119, 169)),
        "salmon" => Some(Color::Rgb(255, 158, 157)),
        "beige" => Some(Color::Rgb(234, 210, 172)),
        "coolgray" | "cool gray" => Some(Color::Rgb(159, 168, 176)),
        "slate" => Some(Color::Rgb(96, 125, 139)),
        "charcoal" => Some(Color::Rgb(55, 71, 79)),
        _ => None,
    }
}

/// Parse a 6-digit or 3-digit hex color string (without the leading '#').
fn parse_hex_color(hex: &str) -> Option<Color> {
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}

/// Parse a comma-separated "r,g,b" tuple string.
fn parse_rgb_tuple_color(s: &str) -> Option<Color> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].trim().parse::<u8>().ok()?;
    let g = parts[1].trim().parse::<u8>().ok()?;
    let b = parts[2].trim().parse::<u8>().ok()?;
    Some(Color::Rgb(r, g, b))
}
