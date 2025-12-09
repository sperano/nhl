# Dynamic Theming Implementation Plan

## Goal
Implement dynamic theming where `selection_fg` and other colors reference a central theme object in the config, so changes are immediately reflected in the UI without copying strings/colors around.

## Current Architecture Summary

### Current State
- **ThemeConfig struct** exists in `src/config.rs` with:
  - `selection_fg: Color` - main selection color
  - `unfocused_selection_fg: Option<Color>` - optional unfocused color (auto-darkens 50% if not set)
  - Full serialization/deserialization support for config file
  - Color parsing from named colors, hex, and RGB tuples

- **Config contains theme**: `Config` struct has `theme: ThemeConfig`
- **SharedData stores config**: `SharedData.config: Config` in RwLock
- **RenderData clones theme**: Extracts `theme: ThemeConfig` via clone each frame

### Current Color Flow (Problem)
1. Config File → `SharedData.config.theme`
2. Each frame: SharedData → **Clone** → `RenderData.theme`
3. `render_frame()` extracts individual colors from theme
4. Colors passed as **separate parameters** to ALL render functions
5. Result: Multiple color copies, verbose function signatures

### Files Using Colors

**Core Infrastructure:**
- `/src/config.rs` - ThemeConfig definition
- `/src/tui/common/styling.rs` - Style helper functions
- `/src/tui/mod.rs` - Main render_frame() that distributes colors

**Render Functions Receiving Colors:**
- `/src/tui/common/tab_bar.rs` - `render(..., selection_fg, unfocused_selection_fg)`
- `/src/tui/scores/view.rs` - `render_subtabs()` and `render_content()`
- `/src/tui/standings/view.rs` - `render_subtabs()` and `render_content()`
- `/src/tui/common/breadcrumb.rs` - `render_breadcrumb()` functions

## Implementation Plan

### Phase 1: Wrap Theme in Arc
**Goal**: Make theme cheaply cloneable without copying color data

**Changes to `src/tui/mod.rs`:**
```rust
// Before
struct RenderData {
    theme: ThemeConfig,  // Full clone each frame
}

// After
struct RenderData {
    theme: Arc<ThemeConfig>,  // Cheap Arc clone
}
```

**Update RenderData construction (line ~358):**
```rust
let render_data = {
    let data = shared_data.read().await;
    RenderData {
        theme: Arc::new(data.config.theme.clone()), // One clone, then Arc refs
        // ... other fields
    }
};
```

### Phase 2: Refactor Function Signatures
**Goal**: Pass theme reference instead of individual colors

**Pattern to apply everywhere:**
```rust
// Before
pub fn render(
    f: &mut Frame,
    area: Rect,
    // ... other params
    selection_fg: Color,
    unfocused_selection_fg: Color,
) {
    let style = selection_style(..., selection_fg, unfocused_selection_fg);
}

// After
pub fn render(
    f: &mut Frame,
    area: Rect,
    // ... other params
    theme: &Arc<ThemeConfig>,
) {
    let style = selection_style(..., theme.selection_fg, theme.unfocused_selection_fg());
}
```

### Phase 3: Update All Call Sites

**In `src/tui/mod.rs::render_frame()`:**
```rust
// Before (6 places)
common::tab_bar::render(..., data.theme.selection_fg, data.theme.unfocused_selection_fg());

// After
common::tab_bar::render(..., &data.theme);
```

### Phase 4: Optional Theme Module
**Create `src/tui/theme.rs`** for future extensibility:

```rust
use std::sync::Arc;
use crate::config::ThemeConfig;

pub trait ThemeProvider {
    fn theme(&self) -> &Arc<ThemeConfig>;
}

impl ThemeProvider for Arc<ThemeConfig> {
    fn theme(&self) -> &Arc<ThemeConfig> {
        self
    }
}
```

## Implementation Steps

1. **Modify RenderData** to use `Arc<ThemeConfig>`
2. **Update render_frame()** to pass `&data.theme` instead of individual colors
3. **Update tab_bar::render()** signature and implementation
4. **Update breadcrumb functions** (2 functions)
5. **Update scores/view.rs** functions (2 render functions + helper)
6. **Update standings/view.rs** functions (2 render functions)
7. **Update styling.rs** if needed for new signatures
8. **Test** all rendering still works
9. **Add unit tests** for theme reference system

## Files to Modify

**Core changes (required):**
- `src/tui/mod.rs` - RenderData struct, render_frame() calls (9 changes)
- `src/tui/common/tab_bar.rs` - render() signature (1 change)
- `src/tui/common/breadcrumb.rs` - 2 function signatures
- `src/tui/scores/view.rs` - 2-3 function signatures
- `src/tui/standings/view.rs` - 2 function signatures

**Optional additions:**
- `src/tui/theme.rs` - New module for theme utilities
- `src/config.rs` - Add more theme colors in future

## Benefits

1. **No Color Copying**: Arc reference instead of cloning colors
2. **Cleaner Signatures**: One theme parameter instead of multiple colors
3. **Dynamic Updates**: If theme changes, all references see it immediately
4. **Extensible**: Easy to add new theme colors without changing signatures
5. **Type Safety**: Theme is a single source of truth
6. **Performance**: Reduced memory allocations and copies

## Testing Strategy

1. Verify all render functions receive theme reference
2. Check no color values are cloned/copied
3. Test theme changes are reflected immediately
4. Ensure selection highlighting still works
5. Test unfocused selection color calculation

## Future Enhancements

Once dynamic theming is in place, we can:
- Add live theme switching/reloading
- Support multiple theme presets
- Add more colors (error, success, borders, etc.)
- Implement theme inheritance/variants
- Add per-tab theme overrides