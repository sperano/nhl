Ask the user:
1. **What file/function to test?** (e.g., "src/tui/components/scores_tab.rs::render_content")
2. **What scenario to test?** (e.g., "empty schedule", "multiple games", "with selection")

Then:

**Step 1: Read the target file and analyze existing tests**
- Look for similar test patterns in the same file
- Identify test data structures being used
- Note any test helper functions available

**Step 2: Generate the appropriate test template**

For **rendering tests** (components/widgets), use assert_buffer:
```rust
#[test]
fn test_{function}_{scenario}() {
    // Setup
    let config = test_config();

    // Create test data (suggest based on existing tests)

    // Render
    let buf = render_widget(&widget, width, height, &config);

    // Assert
    assert_buffer(&buf, &[
        "expected line 1",
        "expected line 2",
    ]);
}
```

For **reducer tests**:
```rust
#[test]
fn test_{action}_{scenario}() {
    let mut state = AppState::default();
    // Setup state (suggest based on existing tests)

    let action = Action::...;
    let (new_state, effect) = reduce(state, action);

    // Assertions
    assert_eq!(new_state.field, expected_value);
    assert!(matches!(effect, Effect::None));
}
```

For **helper function tests**:
```rust
#[test]
fn test_{function}_{scenario}() {
    // Arrange (suggest test data)

    // Act
    let result = function_name(args);

    // Assert
    assert_eq!(result, expected);
}
```

**Step 3: Add the test to the file**
- Place it in the appropriate #[cfg(test)] mod tests block
- Use proper indentation matching the file

**Step 4: Run the new test**
```bash
cargo test --lib {module_path}::tests::test_{function}_{scenario} -- --nocapture
```

**Step 5: Report results**
- ✅ If passes: "Test created and passing!"
- ❌ If fails: Show error and offer to fix it
- Show the test code that was added
