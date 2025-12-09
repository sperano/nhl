# Browser Tab Implementation Plan for rust-code-writer

## Overview
Implement a new "Browser" tab (6th tab) in the NHL TUI application that allows navigating between players and teams using a hyperlink-like system. The implementation must achieve at least 95% code coverage.

## Core Requirements

### 1. Data Structures

#### 1.1 Create Target Enum (`src/tui/browser/target.rs`)
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Team { id: String },    // Team abbreviation (e.g., "MTL", "BOS")
    Player { id: u32 },     // Player ID from NHL API
}
```

#### 1.2 Create Link Struct (`src/tui/browser/link.rs`)
```rust
#[derive(Debug, Clone)]
pub struct Link {
    pub display: String,    // Display text (e.g., "Nick Suzuki")
    pub target: Target,     // Where the link points to
    pub start: usize,       // Start position in the line
    pub end: usize,         // End position in the line
}
```

#### 1.3 Create BrowserContent Struct (`src/tui/browser/content.rs`)
```rust
#[derive(Debug)]
pub struct BrowserContent {
    pub lines: Vec<String>,           // Raw text lines
    pub links: Vec<Link>,             // All links in the content
}
```

### 2. Browser Tab Module Structure

Create the following file structure:
```
src/tui/browser/
├── mod.rs         # Module exports
├── state.rs       # BrowserState struct
├── view.rs        # Rendering logic
├── handler.rs     # Event handling
├── target.rs      # Target enum
├── link.rs        # Link struct
├── content.rs     # BrowserContent and parsing
└── tests/
    ├── mod.rs
    ├── state_test.rs
    ├── view_test.rs
    ├── handler_test.rs
    ├── target_test.rs
    ├── link_test.rs
    └── content_test.rs
```

### 3. Implementation Details

#### 3.1 BrowserState (`src/tui/browser/state.rs`)
```rust
pub struct BrowserState {
    pub content: BrowserContent,
    pub selected_link_index: Option<usize>,  // None if no links, Some(index) for selected link
    pub scroll_offset: u16,                  // For future scrolling support
}

impl BrowserState {
    pub fn new() -> Self {
        // Initialize with the demo content
        let content = Self::create_demo_content();
        let selected = if content.links.is_empty() { None } else { Some(0) };
        Self {
            content,
            selected_link_index: selected,
            scroll_offset: 0,
        }
    }

    fn create_demo_content() -> BrowserContent {
        // Create the demo text with Nick Suzuki, Canadiens, and Golden Knights links
        // The text: "Nick Suzuki plays for the Canadiens, and was drafted by the Golden Knights."
    }

    pub fn select_next_link(&mut self) {
        // Move to next link, wrap around if necessary
    }

    pub fn select_previous_link(&mut self) {
        // Move to previous link, wrap around if necessary
    }

    pub fn get_selected_link(&self) -> Option<&Link> {
        // Return the currently selected link
    }
}
```

#### 3.2 View Rendering (`src/tui/browser/view.rs`)
```rust
pub fn render(frame: &mut Frame, area: Rect, state: &BrowserState, config: &Config) {
    // Render the browser content with links
    // Active link uses config.theme.selection_fg
    // Inactive links use default color
}

fn render_line_with_links(
    line: &str,
    links_in_line: &[Link],
    selected_link: Option<&Link>,
    config: &Config,
) -> Line {
    // Parse the line and create styled spans
    // Apply selection_fg to selected link
}
```

#### 3.3 Event Handler (`src/tui/browser/handler.rs`)
```rust
pub async fn handle_key(
    key: KeyCode,
    state: &mut BrowserState,
    shared_data: &SharedDataHandle,
) -> EventResult {
    match key {
        KeyCode::Down => {
            state.select_next_link();
            EventResult::Consumed
        }
        KeyCode::Up => {
            state.select_previous_link();
            EventResult::Consumed
        }
        KeyCode::Enter => {
            // Get selected link and update status message
            if let Some(link) = state.get_selected_link() {
                let message = format_link_activation(&link);
                update_status_message(shared_data, message).await;
            }
            EventResult::Consumed
        }
        _ => EventResult::Ignored,
    }
}

fn format_link_activation(link: &Link) -> String {
    match &link.target {
        Target::Team { id } => format!("Team: {} ({})", link.display, id),
        Target::Player { id } => format!("Player: {} (ID: {})", link.display, id),
    }
}
```

#### 3.4 Content Parser (`src/tui/browser/content.rs`)
Implement a builder pattern for creating content with embedded links:

```rust
impl BrowserContent {
    pub fn builder() -> BrowserContentBuilder { ... }
}

pub struct BrowserContentBuilder {
    current_line: String,
    current_position: usize,
    links: Vec<Link>,
    lines: Vec<String>,
}

impl BrowserContentBuilder {
    pub fn text(mut self, text: &str) -> Self { ... }
    pub fn link(mut self, display: &str, target: Target) -> Self { ... }
    pub fn newline(mut self) -> Self { ... }
    pub fn build(mut self) -> BrowserContent { ... }
}
```

### 4. Integration Steps

#### 4.1 Update CurrentTab Enum (`src/tui/app.rs`)
- Add `Browser` variant to `CurrentTab` enum
- Update navigation methods to handle 6 tabs

#### 4.2 Update AppState (`src/tui/app.rs`)
- Add `browser: BrowserState` field
- Initialize in `AppState::new()`

#### 4.3 Update Main TUI Loop (`src/tui/mod.rs`)
- Add case for `CurrentTab::Browser` in render function
- Add case for browser key handling

#### 4.4 Update Tab Bar (`src/tui/common/tab_bar.rs`)
- Add "Browser" to the tab list
- Update tab count from 5 to 6

### 5. Demo Content Implementation

Create exactly this content with proper links:
```
Text: "Nick Suzuki plays for the Canadiens, and was drafted by the Golden Knights."

Links:
1. "Nick Suzuki" -> Target::Player { id: 8480018 }  // Real NHL player ID
2. "Canadiens" -> Target::Team { id: "MTL" }
3. "Golden Knights" -> Target::Team { id: "VGK" }
```

### 6. Testing Requirements (95% Coverage Minimum)

#### 6.1 Unit Tests for Target (`src/tui/browser/tests/target_test.rs`)
- Test Target creation for teams
- Test Target creation for players
- Test Target equality
- Test Debug and Clone implementations

#### 6.2 Unit Tests for Link (`src/tui/browser/tests/link_test.rs`)
- Test Link creation
- Test position calculations
- Test link overlap detection (for validation)

#### 6.3 Unit Tests for BrowserContent (`src/tui/browser/tests/content_test.rs`)
- Test builder pattern
- Test content with multiple links
- Test content with no links
- Test newline handling
- Test link position tracking

#### 6.4 Unit Tests for BrowserState (`src/tui/browser/tests/state_test.rs`)
- Test initial state creation
- Test link navigation (next/previous)
- Test wrap-around behavior
- Test state with no links
- Test get_selected_link

#### 6.5 Integration Tests for Handler (`src/tui/browser/tests/handler_test.rs`)
- Test Up key navigation
- Test Down key navigation
- Test Enter key activation
- Test navigation wrap-around
- Test status message updates
- Test ignored keys

#### 6.6 Rendering Tests (`src/tui/browser/tests/view_test.rs`)
- Test rendering with no links
- Test rendering with selected link (verify selection_fg color)
- Test rendering with unselected links
- Test multi-line content rendering
- Test exact output comparison (80-char width snapshots)

Example test structure for rendering:
```rust
#[test]
fn test_render_with_selected_link() {
    let mut state = BrowserState::new();
    state.selected_link_index = Some(0);

    let area = Rect::new(0, 0, 80, 10);
    let mut buffer = Buffer::empty(area);

    // Render and compare with exact expected output
    render_to_buffer(&mut buffer, area, &state, &config);

    let expected = vec![
        "Nick Suzuki plays for the Canadiens, and was drafted by the Golden Knights.   ",
        "                                                                                ",
        // ... rest of the lines
    ];

    assert_buffer_content(&buffer, &expected);
}
```

### 7. Error Handling

- Handle empty content gracefully
- Handle navigation when no links exist
- Ensure no panics on edge cases
- Add proper error messages in status bar when needed

### 8. Implementation Order

1. Create basic module structure and types (Target, Link, BrowserContent)
2. Implement BrowserState with demo content
3. Add Browser to CurrentTab and integrate with main loop
4. Implement basic rendering (without link highlighting)
5. Add link highlighting with selection_fg
6. Implement keyboard navigation
7. Add Enter key handling with status messages
8. Write comprehensive tests for each component
9. Run coverage report and add tests for any uncovered branches

### 9. Coverage Verification

After implementation, run:
```bash
cargo tarpaulin --out Html --output-dir coverage \
  --exclude-files "*/tests/*" \
  --exclude-files "*/examples/*" \
  --ignore-panics \
  --timeout 120 \
  -- --test-threads=1
```

Ensure all browser-related modules have ≥95% coverage.

### 10. Acceptance Criteria

- [ ] Browser tab appears as 6th tab in the UI
- [ ] Demo content displays correctly
- [ ] Three links are visible and navigable
- [ ] Up/Down arrows cycle through links
- [ ] Selected link shows in selection_fg color
- [ ] Enter key shows link info in status bar
- [ ] No panics or errors during navigation
- [ ] Code coverage ≥95% for all browser modules
- [ ] All tests pass
- [ ] Integration with existing TUI is seamless

## Notes for rust-code-writer

- Follow the existing TUI module pattern (state/view/handler/mod structure)
- Use the existing Config and Theme structures for colors
- Reuse the EventResult enum from existing code
- Follow the project's error handling patterns with anyhow
- Keep functions under 100 lines where possible
- Add regression tests for any bugs found during implementation
- Be Unicode-aware for string length calculations
- Use exact string comparisons in rendering tests (not substring matching)