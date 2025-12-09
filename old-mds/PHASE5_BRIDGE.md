# Phase 5 Bridge Architecture

## Overview

The Phase 5 Bridge provides a seamless integration layer between the existing imperative TUI code and the new React-like framework. This allows for **gradual migration** without rewriting everything at once.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Main Application                         │
│                                                              │
│  ┌──────────────┐                                           │
│  │   main.rs    │                                           │
│  │              │                                           │
│  │ NHL_EXPERIMENTAL? ──Yes──┐                               │
│  │      │                    │                               │
│  │      No                   │                               │
│  │      │                    ▼                               │
│  │      │         ┌──────────────────┐                      │
│  │      │         │ run_experimental │                      │
│  │      │         │  (mod_experimental)                     │
│  │      │         └────────┬─────────┘                      │
│  │      │                  │                                 │
│  │      │                  ▼                                 │
│  │      │         ┌──────────────────┐                      │
│  │      │         │  BridgeRuntime   │                      │
│  │      │         │   (bridge.rs)    │                      │
│  │      │         │                  │                      │
│  │      │         │  • sync_from_    │                      │
│  │      │         │    shared_data() │                      │
│  │      │         │  • handle_key()  │                      │
│  │      │         │  • render()      │                      │
│  │      │         │  • process_      │                      │
│  │      │         │    actions()     │                      │
│  │      │         └────────┬─────────┘                      │
│  │      │                  │                                 │
│  │      │         ┌────────┴──────────┐                     │
│  │      │         │                   │                     │
│  │      │         ▼                   ▼                     │
│  │      │   ┌──────────┐      ┌──────────────┐             │
│  │      │   │ Runtime  │      │ SharedData   │             │
│  │      │   │          │◄────►│ (temporary)  │             │
│  │      │   │ • State  │ sync │              │             │
│  │      │   │ • Reducer│      │ API Data     │             │
│  │      │   │ • Effects│      └──────────────┘             │
│  │      │   └────┬─────┘                                    │
│  │      │        │                                          │
│  │      │        ▼                                          │
│  │      │   ┌──────────┐                                    │
│  │      │   │Component │                                    │
│  │      │   │   Tree   │                                    │
│  │      │   │          │                                    │
│  │      │   │ App      │                                    │
│  │      │   │  ├─TabBar                                     │
│  │      │   │  ├─Content                                    │
│  │      │   │  │  ├─ScoresTab                               │
│  │      │   │  │  ├─StandingsTab                            │
│  │      │   │  │  └─SettingsTab                             │
│  │      │   │  └─StatusBar                                  │
│  │      │   └──────────┘                                    │
│  │      │                                                    │
│  │      ▼                                                    │
│  │  ┌────────┐                                              │
│  │  │  run() │  (Legacy imperative mode)                    │
│  │  │        │                                              │
│  │  │ Old TUI│                                              │
│  │  │  Code  │                                              │
│  │  └────────┘                                              │
│  │                                                          │
│  │           Both modes share:                              │
│  │           • background::fetch_data_loop()                │
│  │           • SharedData (API responses)                   │
│  │           • Client (NHL API)                             │
│  └──────────────────────────────────────────────────────────┘
```

## Key Components

### 1. BridgeRuntime (`src/tui/framework/bridge.rs`)

The central integration point that:

**State Management:**
- Wraps the new `Runtime` (action/reducer/state)
- Maintains bidirectional sync with `SharedData`
- `sync_from_shared_data()`: Pulls API data into `AppState`
- `sync_to_shared_data()`: Pushes UI state changes back (for refresh triggers)

**Event Handling:**
- `key_to_action(KeyEvent)`: Maps keyboard input to Actions
- `handle_key(KeyEvent)`: Dispatches actions through reducer
- Supports all navigation keys, tab switching, and subtab navigation

**Rendering:**
- `render()`: Builds component tree and renders to buffer
- Uses new virtual element tree from component system
- `process_actions()`: Handles async effects from runtime

### 2. Experimental Mode (`src/tui/mod_experimental.rs`)

Parallel TUI implementation:

```rust
pub async fn run_experimental(
    client: Arc<Client>,
    shared_data: SharedDataHandle,
    refresh_tx: mpsc::Sender<()>,
) -> Result<(), io::Error>
```

**Event Loop:**
1. Sync state from SharedData
2. Render via BridgeRuntime (component tree)
3. Process queued actions from effects
4. Poll for keyboard events
5. Handle keys → dispatch actions
6. Repeat

**Key Features:**
- Full terminal management (raw mode, alternate screen)
- Crossterm event handling
- Graceful quit handling
- Same background data fetching as legacy mode

### 3. State Syncing Strategy

**Temporary Hybrid Approach:**

```
┌─────────────┐         ┌──────────────┐
│ SharedData  │◄───────►│  AppState    │
│ (Old)       │  Sync   │  (New)       │
└─────────────┘         └──────────────┘
     │                        │
     │ API Responses          │ UI State
     │ • standings            │ • current_tab
     │ • schedule             │ • selected_index
     │ • game_details         │ • subtab_focused
     │ • errors               │ • team_mode
     │ • loading              │
     └────────────────────────┘
```

**Migration Path:**
1. **Phase 5a** (Current): SharedData = source of truth for API data
2. **Phase 5b**: Move API data fetching to effects system
3. **Phase 5c**: Remove SharedData entirely, AppState is sole source of truth

### 4. Action Flow

```
KeyEvent (crossterm)
    │
    ▼
key_to_action()
    │
    ▼
Action (enum)
    │
    ▼
dispatch(action)
    │
    ▼
reduce(state, action) → (new_state, effect)
    │
    ├─► Update state
    │
    └─► Queue effect
            │
            ▼
        Effect executor
            │
            ├─► API call (async)
            ├─► Dispatch result action
            └─► Update state
```

## Usage

### Running Experimental Mode

```bash
# Enable experimental mode
NHL_EXPERIMENTAL=1 cargo run

# Or export it
export NHL_EXPERIMENTAL=1
cargo run

# With logging
NHL_EXPERIMENTAL=1 cargo run -- --log-level debug --log-file /tmp/nhl.log
```

### Running Legacy Mode (Default)

```bash
# Just run normally
cargo run
```

### Testing Both Modes

```bash
# Test legacy
cargo run

# Test experimental (should behave identically)
NHL_EXPERIMENTAL=1 cargo run
```

## Current Status

### ✅ Working
- Bridge infrastructure complete
- Action system fully extended
- Reducer handles all new actions
- Component tree structure defined
- Tab navigation (all 6 tabs)
- KeyEvent → Action mapping
- Experimental mode wired up
- Builds successfully

### 🚧 In Progress
- Component rendering (may need fixes)
- Data flow to widgets
- Effect execution
- Async action handling

### ❌ Not Yet Implemented
- ScoresTab full implementation
- StandingsTab full implementation
- Stats/Players/Browser tabs
- Settings tab full implementation
- Panel navigation
- Game selection
- Date navigation refinement

## Testing Strategy

### 1. Smoke Test
```bash
NHL_EXPERIMENTAL=1 cargo run
```

Expected:
- TUI launches
- Tab bar shows all 6 tabs
- Status bar appears
- Can quit with 'q'

### 2. Navigation Test

Test these keys:
- `Left/Right`: Navigate between tabs
- `1-6`: Jump to specific tab
- `q`: Quit application
- `ESC`: Exit (when not in subtab mode)

Expected behavior:
- Tab selection updates visually
- No crashes
- Smooth transitions

### 3. Component Tree Test

Check rendering:
- Tab bar renders at top
- Status bar renders at bottom
- Content area allocated for tab content
- No buffer overflow errors

### 4. Action Flow Test

With debug logging:
```bash
NHL_EXPERIMENTAL=1 cargo run -- -L debug -F /tmp/nhl.log
```

Check log for:
- Actions being dispatched
- State updates occurring
- Effects being queued
- No panic messages

## Migration Guide

### For Future Tab Migration

When migrating a tab (e.g., Scores):

1. **Implement Component**
   ```rust
   // src/tui/components/scores_tab.rs
   impl Component for ScoresTab {
       type Props = ScoresTabProps;
       fn view(&self, props: &Props, state: &State) -> Element {
           // Build virtual element tree
       }
   }
   ```

2. **Wire Up in App**
   ```rust
   // src/tui/components/app.rs
   Tab::Scores => {
       let props = ScoresTabProps { /* ... */ };
       ScoresTab.view(&props, &())
   }
   ```

3. **Test in Experimental Mode**
   ```bash
   NHL_EXPERIMENTAL=1 cargo run
   ```

4. **Compare with Legacy**
   ```bash
   # Side by side testing
   cargo run              # Legacy
   NHL_EXPERIMENTAL=1 cargo run  # New
   ```

5. **Remove Legacy Code** (when confident)
   - Delete old `src/tui/scores/` code
   - Remove from `mod.rs`
   - Update `run()` to use new rendering

## Known Limitations

1. **SharedData Dependency**: Still depends on old SharedData for API responses
2. **Incomplete Components**: Most components render placeholder content
3. **No Panel Support**: Panel navigation not implemented yet
4. **Limited Effects**: Data fetching still uses old background loop
5. **No Command Palette**: Toggle action exists but component not implemented

## Future Enhancements

### Short Term (Phase 5)
- Complete ScoresTab component
- Complete StandingsTab component
- Fix any rendering issues
- Add comprehensive tests

### Medium Term (Phase 6)
- Migrate all tabs to new system
- Remove SharedData dependency
- Move data fetching to effects system
- Implement panel navigation

### Long Term (Phase 6+)
- Add context system (React Context equivalent)
- Add hooks system (useState, useEffect)
- Implement time-travel debugging
- Virtual tree diffing for performance
- Component memoization

## Troubleshooting

### "Nothing renders"
- Check that App component builds element tree
- Verify Renderer is called
- Check buffer size matches terminal size

### "Keyboard doesn't work"
- Verify `key_to_action()` returns Some(action)
- Check reducer handles the action
- Verify dispatch() is being called

### "Crashes on startup"
- Check logs for panic messages
- Verify SharedData initializes correctly
- Check Client creation doesn't fail

### "Different behavior than legacy"
- Compare action handling
- Check state sync logic
- Verify component props match old code

## Resources

- **Architecture**: `REACT_PLAN.md`
- **Progress**: `REACT_PLAN_PROGRESS.md`
- **Framework Code**: `src/tui/framework/`
- **Components**: `src/tui/components/`
- **Bridge**: `src/tui/framework/bridge.rs`
- **Experimental**: `src/tui/mod_experimental.rs`

## Contributing

When working on the migration:

1. **Test both modes**: Always verify legacy mode still works
2. **Document changes**: Update REACT_PLAN_PROGRESS.md
3. **Write tests**: Add unit tests for new components
4. **Keep bridge working**: Don't break the integration layer
5. **Gradual migration**: One tab at a time, not all at once
