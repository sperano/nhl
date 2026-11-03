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

#[test]
fn test_format_optional_present_value_uses_display() {
    assert_eq!(format_optional(Some(42)), "42");
    assert_eq!(format_optional(Some("12:34")), "12:34");
}

#[test]
fn test_format_optional_absent_value_uses_placeholder() {
    assert_eq!(format_optional(None::<i32>), MISSING_STAT);
}
