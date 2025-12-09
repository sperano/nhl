# Implementation Plan: Command Palette Navigation & Widget Migration

## Overview
This plan implements Proposal 4 (Command Palette Navigation) while completing the widget system migration for the NHL TUI application.

## Agent Assignments

### Primary Agent: `rust-code-writer`
Handles all code implementation tasks.

### Secondary Agent: `integration-tester`
Tests after each phase completion.

### Optional Agent: `idiomatic-rust`
Reviews code after major implementations (Phases 1, 3, and 5).

---

## Phase 1: Foundation Widgets [rust-code-writer - PARALLEL]

### Step 1.1: Create ActionBar Widget
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Create:** `src/tui/widgets/action_bar.rs`

**Implementation Requirements:**
```rust
pub struct ActionBar {
    pub actions: Vec<Action>,
}

pub struct Action {
    pub key: String,      // e.g., "Enter", "G", "T"
    pub label: String,    // e.g., "View Team", "Game Log", "Team Page"
    pub enabled: bool,
}

impl RenderableWidget for ActionBar {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Render format: Actions: [Enter] View Team │ [G] Game Log │ [T] Team Page
        // - Use division_header_fg for labels
        // - Use selection_fg for enabled keys
        // - Use unfocused_selection_fg for disabled keys
        // - Center horizontally
        // - Add │ separators between actions
    }
}
```

**Add to:** `src/tui/widgets/mod.rs` (export the new widget)

---

### Step 1.2: Create EnhancedBreadcrumb Widget
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Create:** `src/tui/widgets/enhanced_breadcrumb.rs`

**Implementation Requirements:**
```rust
pub struct EnhancedBreadcrumb {
    pub items: Vec<String>,
    pub separator: String,  // Default: " ▸ "
    pub icon: Option<String>, // Default: Some("📍")
}

impl Default for EnhancedBreadcrumb {
    fn default() -> Self {
        Self {
            items: vec![],
            separator: " ▸ ".to_string(),
            icon: Some("📍".to_string()),
        }
    }
}

impl RenderableWidget for EnhancedBreadcrumb {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Render format: 📍 Standings ▸ Division ▸ Maple Leafs ▸ Matthews
        // - Icon is optional (check config.show_breadcrumb_icon)
        // - Use selection_fg for the last item (current location)
        // - Use normal fg for previous items
        // - Truncate with "..." if too long for area
    }
}
```

**Add to:** `src/tui/widgets/mod.rs` (export the new widget)

---

### Step 1.3: Create CommandPalette Widget
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Create:** `src/tui/widgets/command_palette.rs`

**Implementation Requirements:**
```rust
pub struct CommandPalette {
    pub input: String,
    pub cursor_position: usize,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub is_visible: bool,
}

pub struct SearchResult {
    pub label: String,      // e.g., "Mitchell Marner"
    pub category: String,   // e.g., "Player", "Team", "Game"
    pub navigation_path: Vec<String>, // Path to navigate to this item
    pub icon: Option<String>, // Optional icon for the category
}

impl RenderableWidget for CommandPalette {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        if !self.is_visible { return; }

        // Calculate modal area (centered, 50% width, 40% height)
        // Draw border with shadow effect
        // Render search input at top with cursor
        // List filtered results below
        // Highlight selected result with selection_fg
        // Show category labels with division_header_fg
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            results: vec![],
            selected_index: 0,
            is_visible: false,
        }
    }

    pub fn update_search(&mut self, query: &str) {
        self.input = query.to_string();
        // This will be connected to search logic later
    }
}
```

**Add to:** `src/tui/widgets/mod.rs` (export the new widget)

---

### PAUSE POINT 1
**Test Agent:** `integration-tester`
- Verify each widget compiles
- Create unit tests for each widget
- Test rendering in isolation

**Review Agent:** `idiomatic-rust` (optional)
- Review the three new widgets for Rust idioms

---

## Phase 2: Widget Migration [rust-code-writer - PARALLEL]

### Step 2.1: Migrate TabBar to Widget System
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Move:** `src/tui/common/tab_bar.rs` → `src/tui/widgets/tab_bar.rs`

**Modifications Required:**
```rust
// Change from standalone function to widget struct
pub struct TabBar {
    pub tabs: Vec<Tab>,
    pub current_tab: usize,
    pub focused: bool,
}

pub struct Tab {
    pub label: String,
    pub shortcut: Option<char>, // '1', '2', '3', '4', '5'
}

impl TabBar {
    pub fn new(current_tab: CurrentTab, focused: bool) -> Self {
        let tabs = vec![
            Tab { label: "Scores".to_string(), shortcut: Some('1') },
            Tab { label: "Standings".to_string(), shortcut: Some('2') },
            Tab { label: "Stats".to_string(), shortcut: Some('3') },
            Tab { label: "Players".to_string(), shortcut: Some('4') },
            Tab { label: "Settings".to_string(), shortcut: Some('5') },
        ];

        let current_tab = match current_tab {
            CurrentTab::Scores => 0,
            CurrentTab::Standings => 1,
            CurrentTab::Stats => 2,
            CurrentTab::Players => 3,
            CurrentTab::Settings => 4,
        };

        Self { tabs, current_tab, focused }
    }
}

impl RenderableWidget for TabBar {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Migrate existing render logic from tab_bar::render()
        // Keep the same visual style with box drawing characters
    }
}
```

**Update:** `src/tui/widgets/mod.rs` (export TabBar)
**Update:** `src/tui/mod.rs` (use new TabBar widget instead of function)

---

### Step 2.2: Migrate StatusBar to Widget System
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Move:** `src/tui/common/status_bar.rs` → `src/tui/widgets/status_bar.rs`

**Modifications Required:**
```rust
pub struct StatusBar {
    pub last_refresh: Option<SystemTime>,
    pub next_refresh_in: Option<Duration>,
    pub error_message: Option<String>,
    pub hints: Vec<KeyHint>,
}

pub struct KeyHint {
    pub key: String,
    pub action: String,
    pub style: KeyHintStyle,
}

pub enum KeyHintStyle {
    Normal,
    Important,  // Use selection_fg
    Subtle,     // Use unfocused_selection_fg
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            last_refresh: None,
            next_refresh_in: None,
            error_message: None,
            hints: vec![
                KeyHint {
                    key: "?".to_string(),
                    action: "Help".to_string(),
                    style: KeyHintStyle::Normal,
                },
                KeyHint {
                    key: "ESC".to_string(),
                    action: "Back".to_string(),
                    style: KeyHintStyle::Important,
                },
                KeyHint {
                    key: "/".to_string(),
                    action: "Jump to...".to_string(),
                    style: KeyHintStyle::Normal,
                },
            ],
        }
    }

    pub fn with_context(mut self, context: &dyn NavigationContextProvider) -> Self {
        self.hints = context.get_keyboard_hints();
        self
    }
}

impl RenderableWidget for StatusBar {
    fn render(&self, area: Rect, buf: &mut Buffer, config: &DisplayConfig) {
        // Format: Last refresh: 14:23:15 │ Next: 45s │ [?] Help │ [ESC] Back │ [/] Jump to...
        // If error_message exists, show it with red background instead
        // Migrate existing render logic
    }
}
```

**Update:** `src/tui/widgets/mod.rs` (export StatusBar)
**Update:** `src/tui/mod.rs` (use new StatusBar widget)

---

### PAUSE POINT 2
**Test Agent:** `integration-tester`
- Run full TUI application
- Verify tab bar still works
- Verify status bar still shows refresh times
- Ensure no visual regressions

---

## Phase 3: Layout Integration [rust-code-writer - SEQUENTIAL]

### Step 3.1: Create Unified Layout Manager
**Agent:** `rust-code-writer`

**Create:** `src/tui/layout.rs`

**Implementation Requirements:**
```rust
use crate::tui::widgets::{TabBar, EnhancedBreadcrumb, ActionBar, StatusBar, CommandPalette};

pub struct Layout {
    pub tab_bar: TabBar,
    pub breadcrumb: Option<EnhancedBreadcrumb>,
    pub action_bar: Option<ActionBar>,
    pub status_bar: StatusBar,
    pub command_palette: Option<CommandPalette>,
}

pub struct LayoutAreas {
    pub tab_bar: Rect,
    pub breadcrumb: Option<Rect>,
    pub content: Rect,
    pub action_bar: Option<Rect>,
    pub status_bar: Rect,
    pub command_palette: Option<Rect>,
}

impl Layout {
    pub fn calculate_areas(&self, terminal_area: Rect) -> LayoutAreas {
        let mut constraints = vec![];
        let mut areas_map = std::collections::HashMap::new();

        // Tab bar: 2 lines
        constraints.push(Constraint::Length(2));
        areas_map.insert("tab_bar", constraints.len() - 1);

        // Breadcrumb: 2 lines (if present)
        if self.breadcrumb.is_some() {
            constraints.push(Constraint::Length(2));
            areas_map.insert("breadcrumb", constraints.len() - 1);
        }

        // Content: remaining space minus bottom bars
        constraints.push(Constraint::Min(0));
        areas_map.insert("content", constraints.len() - 1);

        // Action bar: 2 lines (if present)
        if self.action_bar.is_some() {
            constraints.push(Constraint::Length(2));
            areas_map.insert("action_bar", constraints.len() - 1);
        }

        // Status bar: 1 line
        constraints.push(Constraint::Length(1));
        areas_map.insert("status_bar", constraints.len() - 1);

        let chunks = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(terminal_area);

        LayoutAreas {
            tab_bar: chunks[*areas_map.get("tab_bar").unwrap()],
            breadcrumb: areas_map.get("breadcrumb").map(|i| chunks[*i]),
            content: chunks[*areas_map.get("content").unwrap()],
            action_bar: areas_map.get("action_bar").map(|i| chunks[*i]),
            status_bar: chunks[*areas_map.get("status_bar").unwrap()],
            command_palette: self.command_palette.as_ref()
                .filter(|cp| cp.is_visible)
                .map(|_| centered_rect(50, 40, terminal_area)),
        }
    }

    pub fn render(&self, frame: &mut Frame, areas: LayoutAreas, config: &DisplayConfig) {
        // Render each component in its area
        let mut tab_buf = Buffer::empty(areas.tab_bar);
        self.tab_bar.render(areas.tab_bar, &mut tab_buf, config);
        frame.render_widget(Widget::from(tab_buf), areas.tab_bar);

        if let (Some(breadcrumb), Some(area)) = (&self.breadcrumb, areas.breadcrumb) {
            let mut buf = Buffer::empty(area);
            breadcrumb.render(area, &mut buf, config);
            frame.render_widget(Widget::from(buf), area);
        }

        // Similar for action_bar and status_bar

        // Command palette renders on top (last)
        if let (Some(palette), Some(area)) = (&self.command_palette, areas.command_palette) {
            if palette.is_visible {
                let mut buf = Buffer::empty(area);
                palette.render(area, &mut buf, config);
                frame.render_widget(Widget::from(buf), area);
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

---

### Step 3.2: Integrate Layout into Main TUI Loop
**Agent:** `rust-code-writer`

**Modify:** `src/tui/mod.rs`

**Changes Required:**
```rust
// Add to imports
use crate::tui::layout::{Layout, LayoutAreas};

// In the render loop, replace current rendering with:
let layout = Layout {
    tab_bar: TabBar::new(app_state.current_tab.clone(), !app_state.is_subtab_focused()),
    breadcrumb: create_breadcrumb(&app_state),
    action_bar: create_action_bar(&app_state),
    status_bar: create_status_bar(&shared_data, &app_state),
    command_palette: app_state.command_palette.clone(),
};

let areas = layout.calculate_areas(terminal.size()?);
layout.render(&mut frame, areas, &config);

// Render tab-specific content in areas.content
match app_state.current_tab {
    CurrentTab::Scores => {
        scores::render_content(&mut frame, areas.content, &app_state.scores, &data, &config);
    }
    CurrentTab::Standings => {
        standings::render_content(&mut frame, areas.content, &app_state.standings, &data, &config);
    }
    // ... other tabs
}

// Helper functions
fn create_breadcrumb(app_state: &AppState) -> Option<EnhancedBreadcrumb> {
    // Check if any tab has navigation depth > 0
    // If so, get breadcrumb items from current tab's context
    None // Placeholder for now
}

fn create_action_bar(app_state: &AppState) -> Option<ActionBar> {
    // Get available actions from current tab's context
    None // Placeholder for now
}

fn create_status_bar(shared_data: &SharedData, app_state: &AppState) -> StatusBar {
    StatusBar::new()
        .with_refresh_info(shared_data.last_refresh, calculate_next_refresh(shared_data))
        .with_error(shared_data.error_message.clone())
        // Context hints will be added in Phase 4
}
```

---

### PAUSE POINT 3
**Test Agent:** `integration-tester`
- Verify layout renders correctly
- Check that all components appear in correct positions
- Test with different terminal sizes
- Ensure content area sizing is correct

---

## Phase 4: Navigation Context System [rust-code-writer - SEQUENTIAL]

### Step 4.1: Create Context Provider Trait
**Agent:** `rust-code-writer`

**Create:** `src/tui/context.rs`

**Implementation Requirements:**
```rust
use crate::tui::widgets::{Action, KeyHint, KeyHintStyle};

pub trait NavigationContextProvider {
    fn get_breadcrumb_items(&self) -> Vec<String>;
    fn get_available_actions(&self) -> Vec<Action>;
    fn get_keyboard_hints(&self) -> Vec<KeyHint>;
    fn get_searchable_items(&self) -> Vec<SearchableItem>;
}

pub struct SearchableItem {
    pub label: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub navigation_command: NavigationCommand,
}

#[derive(Debug, Clone)]
pub enum NavigationCommand {
    GoToTab(CurrentTab),
    GoToTeam(String),           // Team abbreviation
    GoToPlayer(i64),            // Player ID
    GoToGame(i64),              // Game ID
    GoToDate(GameDate),         // Navigate to date in scores
    GoToStandingsView(GroupBy), // Change standings view
    GoToSettings(String),       // Settings category
}

// Helper struct to build contexts
pub struct ContextBuilder {
    breadcrumb: Vec<String>,
    actions: Vec<Action>,
    hints: Vec<KeyHint>,
    searchable: Vec<SearchableItem>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            breadcrumb: vec![],
            actions: vec![],
            hints: vec![],
            searchable: vec![],
        }
    }

    pub fn with_breadcrumb(mut self, items: Vec<String>) -> Self {
        self.breadcrumb = items;
        self
    }

    pub fn add_action(mut self, key: &str, label: &str, enabled: bool) -> Self {
        self.actions.push(Action {
            key: key.to_string(),
            label: label.to_string(),
            enabled,
        });
        self
    }

    pub fn add_hint(mut self, key: &str, action: &str, style: KeyHintStyle) -> Self {
        self.hints.push(KeyHint {
            key: key.to_string(),
            action: action.to_string(),
            style,
        });
        self
    }

    pub fn build(self) -> Box<dyn NavigationContextProvider> {
        Box::new(SimpleContext {
            breadcrumb: self.breadcrumb,
            actions: self.actions,
            hints: self.hints,
            searchable: self.searchable,
        })
    }
}

struct SimpleContext {
    breadcrumb: Vec<String>,
    actions: Vec<Action>,
    hints: Vec<KeyHint>,
    searchable: Vec<SearchableItem>,
}

impl NavigationContextProvider for SimpleContext {
    fn get_breadcrumb_items(&self) -> Vec<String> { self.breadcrumb.clone() }
    fn get_available_actions(&self) -> Vec<Action> { self.actions.clone() }
    fn get_keyboard_hints(&self) -> Vec<KeyHint> { self.hints.clone() }
    fn get_searchable_items(&self) -> Vec<SearchableItem> { self.searchable.clone() }
}
```

---

### Step 4.2: Implement Context for Each Tab
**Agent:** `rust-code-writer`

**Modify:** `src/tui/scores/state.rs`

```rust
use crate::tui::context::{NavigationContextProvider, ContextBuilder, SearchableItem, NavigationCommand};

impl NavigationContextProvider for State {
    fn get_breadcrumb_items(&self) -> Vec<String> {
        let mut items = vec!["Scores".to_string()];

        // Add date if in subtab mode
        if self.subtab_focused {
            items.push(format!("{}", self.game_date.format("%b %d, %Y")));
        }

        // Add game if selected
        if let Some(selected_game) = self.selected_game_id {
            items.push(format!("Game #{}", selected_game));
        }

        items
    }

    fn get_available_actions(&self) -> Vec<Action> {
        let mut actions = vec![];

        if self.subtab_focused && self.selected_game_id.is_some() {
            actions.push(Action {
                key: "Enter".to_string(),
                label: "View Boxscore".to_string(),
                enabled: true,
            });
            actions.push(Action {
                key: "T".to_string(),
                label: "Home Team".to_string(),
                enabled: true,
            });
            actions.push(Action {
                key: "V".to_string(),
                label: "Away Team".to_string(),
                enabled: true,
            });
        }

        actions
    }

    fn get_keyboard_hints(&self) -> Vec<KeyHint> {
        let mut hints = vec![];

        if self.subtab_focused {
            hints.push(KeyHint {
                key: "←→".to_string(),
                action: "Change Date".to_string(),
                style: KeyHintStyle::Important,
            });
        } else {
            hints.push(KeyHint {
                key: "↓".to_string(),
                action: "Select Date".to_string(),
                style: KeyHintStyle::Important,
            });
        }

        hints.push(KeyHint {
            key: "ESC".to_string(),
            action: "Back".to_string(),
            style: KeyHintStyle::Normal,
        });

        hints
    }

    fn get_searchable_items(&self) -> Vec<SearchableItem> {
        // This will be populated with games from SharedData
        vec![]
    }
}
```

**Similarly modify:**
- `src/tui/standings/state.rs`
- `src/tui/settings/state.rs`
- `src/tui/stats/state.rs` (if exists)
- `src/tui/players/state.rs` (if exists)

---

### PAUSE POINT 4
**Test Agent:** `integration-tester`
- Navigate through all tabs
- Verify breadcrumbs update correctly
- Check action bar shows appropriate actions
- Verify keyboard hints change based on context

---

## Phase 5: Command Palette Implementation [rust-code-writer - SEQUENTIAL]

### Step 5.1: Add Command Palette to AppState
**Agent:** `rust-code-writer`

**Modify:** `src/tui/app.rs`

```rust
use crate::tui::widgets::CommandPalette;
use crate::tui::context::NavigationCommand;

pub struct AppState {
    // ... existing fields
    pub command_palette: Option<CommandPalette>,
    pub command_palette_active: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            // ... existing initialization
            command_palette: Some(CommandPalette::new()), // Always exists, just not visible
            command_palette_active: false,
        }
    }

    pub fn open_command_palette(&mut self) {
        if let Some(palette) = &mut self.command_palette {
            palette.is_visible = true;
            palette.input.clear();
            palette.results.clear();
            palette.selected_index = 0;
        }
        self.command_palette_active = true;
    }

    pub fn close_command_palette(&mut self) {
        if let Some(palette) = &mut self.command_palette {
            palette.is_visible = false;
        }
        self.command_palette_active = false;
    }

    pub async fn execute_navigation_command(
        &mut self,
        command: NavigationCommand,
        shared_data: &SharedDataHandle,
        refresh_tx: &mpsc::Sender<()>,
    ) -> Result<()> {
        match command {
            NavigationCommand::GoToTab(tab) => {
                self.current_tab = tab;
            }
            NavigationCommand::GoToTeam(abbrev) => {
                self.current_tab = CurrentTab::Standings;
                // Navigate to team in standings
                // Set selected_team_abbrev in shared_data
                let mut data = shared_data.write().await;
                data.selected_team_abbrev = Some(abbrev);
                refresh_tx.send(()).await?;
            }
            NavigationCommand::GoToPlayer(player_id) => {
                // Navigate to player
                let mut data = shared_data.write().await;
                data.selected_player_id = Some(player_id);
                refresh_tx.send(()).await?;
            }
            NavigationCommand::GoToGame(game_id) => {
                self.current_tab = CurrentTab::Scores;
                // Navigate to game
                let mut data = shared_data.write().await;
                data.selected_game_id = Some(game_id);
                refresh_tx.send(()).await?;
            }
            NavigationCommand::GoToDate(date) => {
                self.current_tab = CurrentTab::Scores;
                self.scores.game_date = date;
                self.scores.subtab_focused = true;
            }
            NavigationCommand::GoToStandingsView(view) => {
                self.current_tab = CurrentTab::Standings;
                self.standings.view = view;
                self.standings.subtab_focused = true;
            }
            NavigationCommand::GoToSettings(category) => {
                self.current_tab = CurrentTab::Settings;
                // Navigate to specific settings category
            }
        }

        self.close_command_palette();
        Ok(())
    }
}
```

---

### Step 5.2: Create Command Palette Handler
**Agent:** `rust-code-writer`

**Create:** `src/tui/command_palette/mod.rs`
**Create:** `src/tui/command_palette/handler.rs`
**Create:** `src/tui/command_palette/search.rs`

**handler.rs:**
```rust
use crate::tui::app::AppState;
use crate::tui::SharedDataHandle;
use crossterm::event::{KeyCode, KeyEvent};

pub async fn handle_key(
    app_state: &mut AppState,
    key: KeyEvent,
    shared_data: &SharedDataHandle,
    refresh_tx: &mpsc::Sender<()>,
) -> Result<()> {
    let palette = match &mut app_state.command_palette {
        Some(p) if p.is_visible => p,
        _ => return Ok(()),
    };

    match key.code {
        KeyCode::Char(c) => {
            palette.input.push(c);
            update_search_results(palette, shared_data).await;
        }
        KeyCode::Backspace => {
            palette.input.pop();
            update_search_results(palette, shared_data).await;
        }
        KeyCode::Up => {
            if palette.selected_index > 0 {
                palette.selected_index -= 1;
            }
        }
        KeyCode::Down => {
            if palette.selected_index < palette.results.len().saturating_sub(1) {
                palette.selected_index += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(result) = palette.results.get(palette.selected_index) {
                // Extract navigation command from the selected result
                // This needs to be connected to the search result's navigation_path
                if let Some(command) = parse_navigation_path(&result.navigation_path) {
                    app_state.execute_navigation_command(command, shared_data, refresh_tx).await?;
                }
            }
        }
        KeyCode::Esc => {
            app_state.close_command_palette();
        }
        _ => {}
    }

    Ok(())
}

async fn update_search_results(palette: &mut CommandPalette, shared_data: &SharedDataHandle) {
    let data = shared_data.read().await;
    let query = palette.input.to_lowercase();

    palette.results.clear();

    if query.is_empty() {
        // Show recent/popular items
        return;
    }

    // Search teams
    for standing in data.standings.iter() {
        if standing.team_name.to_lowercase().contains(&query) ||
           standing.team_abbrev.to_lowercase().contains(&query) {
            palette.results.push(SearchResult {
                label: standing.team_name.clone(),
                category: "Team".to_string(),
                navigation_path: vec!["team".to_string(), standing.team_abbrev.clone()],
                icon: Some("🏒".to_string()),
            });
        }
    }

    // Search players (if loaded)
    for (player_id, player) in data.player_info.iter() {
        if let Some(name) = &player.first_name {
            if name.to_lowercase().contains(&query) {
                palette.results.push(SearchResult {
                    label: format!("{} {}", name, player.last_name.as_ref().unwrap_or(&"".to_string())),
                    category: "Player".to_string(),
                    navigation_path: vec!["player".to_string(), player_id.to_string()],
                    icon: Some("👤".to_string()),
                });
            }
        }
    }

    // Limit results
    palette.results.truncate(10);
    palette.selected_index = 0;
}
```

---

### Step 5.3: Integrate Command Palette into Event Loop
**Agent:** `rust-code-writer`

**Modify:** `src/tui/mod.rs`

In the main event handling section:
```rust
// Add check for command palette active state
if app_state.command_palette_active {
    if let Some(palette) = &app_state.command_palette {
        if palette.is_visible {
            command_palette::handler::handle_key(&mut app_state, key, &shared_data, &refresh_tx).await?;
            continue; // Skip normal key handling
        }
    }
}

// Add '/' key handler to open palette
match key.code {
    KeyCode::Char('/') if !app_state.command_palette_active => {
        app_state.open_command_palette();
    }
    // ... existing key handlers
}
```

---

### PAUSE POINT 5
**Test Agent:** `integration-tester`
- Press `/` to open command palette
- Type to search for teams/players
- Navigate results with arrow keys
- Press Enter to navigate to selection
- Press ESC to close palette
- Verify navigation commands work correctly

**Review Agent:** `idiomatic-rust` (optional)
- Review command palette implementation

---

## Phase 6: Final Polish [rust-code-writer - PARALLEL]

### Step 6.1: Add Configuration Options
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Modify:** `src/config.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    // ... existing fields

    #[serde(default = "default_true")]
    pub show_breadcrumb_icon: bool,

    #[serde(default = "default_true")]
    pub show_action_bar: bool,

    #[serde(default = "default_palette_width")]
    pub command_palette_width: u16,  // Percentage

    #[serde(default = "default_palette_height")]
    pub command_palette_height: u16, // Percentage

    #[serde(default = "default_false")]
    pub enable_animations: bool,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_palette_width() -> u16 { 50 }
fn default_palette_height() -> u16 { 40 }
```

Update widgets to use these configuration options.

---

### Step 6.2: Add Visual Enhancements
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

**Modify:** `src/tui/widgets/command_palette.rs`

Add:
- Smooth fade-in effect (if animations enabled)
- Cursor blinking in search input
- Better shadow/border styling
- Scrollbar for long result lists

---

### Step 6.3: Complete Widget Migration
**Agent:** `rust-code-writer`
**Can run in parallel:** YES

Migrate remaining non-widget components:
- Convert game boxes to widgets
- Convert settings items to widgets
- Any other rendering functions to widgets

---

### FINAL TESTING
**Test Agent:** `integration-tester`

Run comprehensive tests:
1. Navigate through all tabs with breadcrumbs
2. Test command palette search for all entity types
3. Verify all keyboard shortcuts in action bar
4. Test with small terminal (80x24) and large (200x60)
5. Verify no regressions in existing functionality
6. Test configuration options
7. Performance test with large datasets

**Review Agent:** `idiomatic-rust`
- Final review of all new code

---

## Execution Order Summary

1. **Phase 1** [rust-code-writer - PARALLEL]
   - Steps 1.1, 1.2, 1.3 in parallel
   - Then: integration-tester + idiomatic-rust

2. **Phase 2** [rust-code-writer - PARALLEL]
   - Steps 2.1, 2.2 in parallel
   - Then: integration-tester

3. **Phase 3** [rust-code-writer - SEQUENTIAL]
   - Step 3.1, then 3.2
   - Then: integration-tester

4. **Phase 4** [rust-code-writer - SEQUENTIAL]
   - Step 4.1, then 4.2
   - Then: integration-tester

5. **Phase 5** [rust-code-writer - SEQUENTIAL]
   - Steps 5.1, 5.2, 5.3 in sequence
   - Then: integration-tester + idiomatic-rust

6. **Phase 6** [rust-code-writer - PARALLEL]
   - Steps 6.1, 6.2, 6.3 in parallel
   - Then: integration-tester + idiomatic-rust

## Success Criteria

- [ ] All widgets render correctly
- [ ] Navigation breadcrumbs update properly
- [ ] Action bar shows context-sensitive actions
- [ ] Command palette searches and navigates
- [ ] Status bar shows dynamic hints
- [ ] No regressions in existing functionality
- [ ] Code follows Rust idioms
- [ ] All tests pass
- [ ] Configuration options work