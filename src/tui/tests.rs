use super::*;

#[test]
fn test_is_quit_action_with_quit() {
    assert!(is_quit_action(&Action::Quit));
}

#[test]
fn test_is_quit_action_with_non_quit_actions() {
    assert!(!is_quit_action(&Action::RefreshData));
    assert!(!is_quit_action(&Action::NavigateTab(Tab::Scores)));
    assert!(!is_quit_action(&Action::FocusNext));
    assert!(!is_quit_action(&Action::SelectGame(12345)));
}

#[test]
fn test_is_quit_action_with_document_stack_actions() {
    assert!(!is_quit_action(&Action::PopDocument));
    assert!(!is_quit_action(&Action::RefreshData));
}

#[test]
#[cfg(feature = "development")]
#[ignore = "changes working directory - run with --test-threads=1"]
fn test_get_next_screenshot_counter_with_existing_files() {
    use std::env;
    use std::fs;

    // Create a temporary directory for testing
    let temp_dir = env::temp_dir().join(format!("nhl_screenshot_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir); // Remove if exists from previous run
    fs::create_dir(&temp_dir).unwrap();
    let original_dir = env::current_dir().unwrap();

    // Change to temp directory
    env::set_current_dir(&temp_dir).unwrap();

    // Test with no existing files
    assert_eq!(get_next_screenshot_counter(), 1);

    // Create test files
    fs::write("nhl-screenshot-001.txt", "test").unwrap();
    fs::write("nhl-screenshot-005.txt", "test").unwrap();
    fs::write("nhl-screenshot-010.txt", "test").unwrap();

    // Should return 11 (max + 1)
    assert_eq!(get_next_screenshot_counter(), 11);

    // Restore original directory before cleanup
    env::set_current_dir(&original_dir).unwrap();

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
#[cfg(feature = "development")]
#[ignore = "changes working directory - run with --test-threads=1"]
fn test_get_next_screenshot_counter_ignores_old_format() {
    use std::env;
    use std::fs;

    // Create a temporary directory for testing
    let temp_dir = env::temp_dir().join(format!("nhl_screenshot_test_2_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir); // Remove if exists from previous run
    fs::create_dir(&temp_dir).unwrap();
    let original_dir = env::current_dir().unwrap();

    // Change to temp directory
    env::set_current_dir(&temp_dir).unwrap();

    // Create a counter-based file and an old timestamp-based file
    fs::write("nhl-screenshot-002.txt", "test").unwrap();
    fs::write("nhl-screenshot-20251120-135010.txt", "test").unwrap();

    // Should return 3, ignoring the timestamp-based file
    assert_eq!(get_next_screenshot_counter(), 3);

    // Restore original directory before cleanup
    env::set_current_dir(&original_dir).unwrap();

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);
}
