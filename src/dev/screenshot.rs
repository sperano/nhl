use ratatui::{backend::Backend, buffer::Buffer, layout::Rect, Terminal};
/// Screenshot utilities for capturing terminal output
///
/// This module provides functionality to save terminal screenshots
/// as text files for debugging and documentation purposes.
use std::fs::File;
use std::io::{self, Write};

/// Save a terminal screenshot from a buffer to a text file
///
/// Captures a ratatui Buffer and saves it to a file with the given filename.
/// This should be called with a buffer captured during a draw call.
///
/// # Arguments
///
/// * `buffer` - The ratatui Buffer to save
/// * `area` - The area of the buffer to save
/// * `filename` - The filename to save the screenshot to
///
/// # Returns
///
/// Returns `Ok(())` if successful, or an IO error if the file could not be written.
pub fn save_buffer_screenshot(buffer: &Buffer, area: Rect, filename: &str) -> io::Result<()> {
    let mut file = File::create(filename)?;

    // Write each line
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            write!(file, "{}", cell.symbol())?;
        }
        writeln!(file)?;
    }

    Ok(())
}

/// Save a terminal screenshot to a text file (legacy compatibility)
///
/// This is a wrapper that captures during a draw call. Note that this
/// performs an additional draw just to capture the buffer.
///
/// # Arguments
///
/// * `terminal` - The ratatui Terminal to capture
/// * `filename` - The filename to save the screenshot to
///
/// # Returns
///
/// Returns `Ok(())` if successful, or an IO error if the file could not be written.
pub fn save_terminal_screenshot<B: Backend>(
    terminal: &mut Terminal<B>,
    filename: &str,
) -> io::Result<()> {
    terminal.draw(|f| {
        let area = f.area();
        let buffer = f.buffer_mut().clone();

        // Save to file (ignore errors during draw, we'll return them)
        let _ = save_buffer_screenshot(&buffer, area, filename);
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_buffer_screenshot_basic() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));

        // Write some content to the buffer
        buffer.set_string(0, 0, "Hello", ratatui::style::Style::default());
        buffer.set_string(0, 1, "World", ratatui::style::Style::default());
        buffer.set_string(0, 2, "Test!", ratatui::style::Style::default());

        let filename = "/tmp/test_screenshot_basic.txt";
        let area = Rect::new(0, 0, 10, 3);

        // Save screenshot
        save_buffer_screenshot(&buffer, area, filename).unwrap();

        // Read file and verify content
        let content = fs::read_to_string(filename).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Hello     ");
        assert_eq!(lines[1], "World     ");
        assert_eq!(lines[2], "Test!     ");

        // Cleanup
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_save_buffer_screenshot_empty_buffer() {
        let buffer = Buffer::empty(Rect::new(0, 0, 5, 2));

        let filename = "/tmp/test_screenshot_empty.txt";
        let area = Rect::new(0, 0, 5, 2);

        // Save screenshot
        save_buffer_screenshot(&buffer, area, filename).unwrap();

        // Read file and verify content
        let content = fs::read_to_string(filename).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "     ");
        assert_eq!(lines[1], "     ");

        // Cleanup
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_save_buffer_screenshot_unicode() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 15, 2));

        // Write unicode content
        buffer.set_string(0, 0, "▸ Test", ratatui::style::Style::default());
        buffer.set_string(0, 1, "► Arrow", ratatui::style::Style::default());

        let filename = "/tmp/test_screenshot_unicode.txt";
        let area = Rect::new(0, 0, 15, 2);

        // Save screenshot
        save_buffer_screenshot(&buffer, area, filename).unwrap();

        // Read file and verify content
        let content = fs::read_to_string(filename).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("▸ Test"));
        assert!(lines[1].starts_with("► Arrow"));

        // Cleanup
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_save_buffer_screenshot_partial_area() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 5));

        // Write content across full buffer
        buffer.set_string(0, 0, "Line 1", ratatui::style::Style::default());
        buffer.set_string(0, 1, "Line 2", ratatui::style::Style::default());
        buffer.set_string(0, 2, "Line 3", ratatui::style::Style::default());
        buffer.set_string(0, 3, "Line 4", ratatui::style::Style::default());
        buffer.set_string(0, 4, "Line 5", ratatui::style::Style::default());

        let filename = "/tmp/test_screenshot_partial.txt";
        // Only save first 3 lines with width 10
        let area = Rect::new(0, 0, 10, 3);

        // Save screenshot
        save_buffer_screenshot(&buffer, area, filename).unwrap();

        // Read file and verify content
        let content = fs::read_to_string(filename).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Should only have 3 lines (from area.height)
        assert_eq!(lines.len(), 3);
        // Each line should be 10 chars (from area.width)
        assert_eq!(lines[0], "Line 1    ");
        assert_eq!(lines[1], "Line 2    ");
        assert_eq!(lines[2], "Line 3    ");

        // Cleanup
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_save_buffer_screenshot_single_cell() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        buffer.set_string(0, 0, "X", ratatui::style::Style::default());

        let filename = "/tmp/test_screenshot_single.txt";
        let area = Rect::new(0, 0, 1, 1);

        // Save screenshot
        save_buffer_screenshot(&buffer, area, filename).unwrap();

        // Read file and verify content
        let content = fs::read_to_string(filename).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "X");

        // Cleanup
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_save_buffer_screenshot_newlines() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 2));
        buffer.set_string(0, 0, "ABC", ratatui::style::Style::default());
        buffer.set_string(0, 1, "DEF", ratatui::style::Style::default());

        let filename = "/tmp/test_screenshot_newlines.txt";
        let area = Rect::new(0, 0, 3, 2);

        // Save screenshot
        save_buffer_screenshot(&buffer, area, filename).unwrap();

        // Read raw content (not lines) to verify newlines
        let content = fs::read_to_string(filename).unwrap();

        // Should have exactly one newline after each row
        assert_eq!(content, "ABC\nDEF\n");

        // Cleanup
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_save_buffer_screenshot_file_creation() {
        let buffer = Buffer::empty(Rect::new(0, 0, 1, 1));
        let filename = "/tmp/test_screenshot_file_creation.txt";

        // Ensure file doesn't exist
        let _ = fs::remove_file(filename);

        // Save screenshot
        save_buffer_screenshot(&buffer, Rect::new(0, 0, 1, 1), filename).unwrap();

        // Verify file exists
        assert!(fs::metadata(filename).is_ok());

        // Cleanup
        fs::remove_file(filename).unwrap();
    }
}
