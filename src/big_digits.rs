/// Big digit font - each digit is represented as 4 lines of text
/// Each line is exactly 4 characters wide
pub const BIG_DIGITS: [[&str; 4]; 10] = [
    // 0
    ["▟▀▀▙", "█  █", "█  █", "▜▄▄▛"],
    // 1
    ["▗█  ", " █  ", " █  ", "▗█▖ "],
    // 2
    ["▟▀▀▙", "  ▗▛", " ▗▛ ", "▄█▄▄"],
    // 3
    ["▟▀▀▙", " ▄▄▛", "   █", "▜▄▄▛"],
    // 4
    [" ▗█ ", "▗▘█ ", "▙▄█▄", "  █ "],
    // 5
    ["█▀▀▀", "█▄▄▖", "   █", "▜▄▄▛"],
    // 6
    ["▗▛▀▘", "█▄▄▖", "█  █", "▜▄▄▛"],
    // 7
    ["█▀▀█", "  ▟▘", " ▟▘ ", " █  "],
    // 8
    ["▟▀▀▙", "▜▄▄▛", "█  █", "▜▄▄▛"],
    // 9
    ["▟▀▀▙", "▜▄▄█", "  ▗▛", "▗▄▛ "],
];

/// Width of each big digit in characters
pub const BIG_DIGIT_WIDTH: u16 = 4;

/// Height of each big digit in lines
pub const BIG_DIGIT_HEIGHT: u16 = 4;

/// Get a big digit by its numeric value (0-9)
pub fn get_digit(n: u8) -> &'static [&'static str; 4] {
    &BIG_DIGITS[n as usize % 10]
}
