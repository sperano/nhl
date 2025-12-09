# Orphaned Code Analysis: src/tui/ Module

## Executive Summary

The TUI module contains **significant orphaned code** due to an incomplete migration from the original production TUI (`mod.rs`) to an experimental React-like TUI (`mod_experimental.rs`). The production entry point calls `run_experimental()`, leaving the original `run()` function and its entire supporting infrastructure dead.

**Key Finding**: Two parallel TUI implementations exist, but only the experimental one is used in production.

---

## Critical Orphaned Items

### 1. PRIMARY DEAD CODE: `src/tui/mod.rs` - 873 Lines

**Location**: `src/tui/mod.rs:646-746`

```rust
pub async fn run(shared_data: SharedDataHandle, refresh_tx: mpsc::Sender<()>) -> Result<(), io::Error> {
    // ... 100 lines of production TUI code
}
```

**Status**: COMPLETELY ORPHANED - Never called

**Evidence**:
- `main.rs:207` calls `tui::run_experimental()` instead
- No imports of this function anywhere in the codebase
- This is the entire original production TUI implementation

**Code Size**: 873 lines of production-quality rendering and event handling code

**Impact**: 
- Entire `tui::mod.rs` file is largely dead except for utility functions
- All rendering logic, event loop, and state management in this file is unused
- Every public export from `mod.rs` that supported this function is now orphaned

---

## Orphaned Modules & Their Dependencies

### 2. Production TUI Infrastructure (All Orphaned)

These modules are **only** used by the dead `run()` function in `mod.rs`:

| Module | Lines | Status | Used By |
|--------|-------|--------|---------|
| `src/tui/app.rs` | 200+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/common/` | 500+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/scores/` | 1000+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/standings/` | 800+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/stats/` | 500+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/players/` | 400+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/settings/` | 600+ | ORPHANED | Only `mod.rs::run()` |
| `src/tui/browser/` | 400+ | ORPHANED | Only `mod.rs::run()` |

**Total Orphaned Code**: 4,400+ lines

---

### 3. `src/tui/context.rs` - 296 Lines

**Status**: SEMI-ORPHANED

**Exports**:
- `pub trait NavigationContextProvider`
- `pub trait BreadcrumbProvider`
- `pub struct ScoresBreadcrumbProvider<'a>`
- `pub enum NavigationCommand`
- `pub struct SearchableItem`

**Usage**:
- Exported from `mod.rs:17` but never imported anywhere
- Only used internally within `mod.rs` in rendering functions (lines 410-441, 469)
- Implemented by tab state structs in orphaned modules (scores, standings, etc.)

**Verdict**: Orphaned export; internal usage within dead `run()` function

---

### 4. `src/tui/layout.rs` - 300+ Lines

**Status**: ORPHANED

**Usage**: Only used in `mod.rs:21` and `mod.rs:507-520` within the dead `run()` function

**Evidence**: 
```rust
use layout::{Layout as LayoutManager};
// ... used only in render_frame() which is only called by run()
```

**Verdict**: Dead code supporting the dead `run()` function

---

## Experimental (React-like) Implementation

### 5. `src/tui/mod_experimental.rs` - 189 Lines

**Status**: ACTIVE - This is what's actually used

**Entry Point**: `main.rs:207` → `tui::run_experimental()`

**What it Uses**:
- `super::framework::Runtime`
- `super::framework::Action`
- `super::framework::DataEffects`
- `super::framework::Renderer`
- `super::framework::keys::key_to_action`

**Clean Architecture**: This represents the new implementation path

---

### 6. `src/tui/framework/` - ~2000 Lines

**Status**: ACTIVE (only used in experimental path)

**Modules**:
- `action.rs` - Action types and handlers
- `component.rs` - Component trait and Element type
- `effects.rs` - DataEffects for API calls
- `reducer.rs` - State reducer logic
- `renderer.rs` - Virtual element to ratatui rendering
- `runtime.rs` - Event loop and state management
- `state.rs` - AppState definition
- `keys.rs` - Keyboard event mapping
- `experimental_tests.rs` - TEST ONLY
- `integration_tests.rs` - TEST ONLY

**Used Only By**: `mod_experimental.rs`

**Architecture**: Proper React-like pattern with Actions → Reducer → State → Renderer

---

### 7. `src/tui/components/` - 500+ Lines

**Status**: SEMI-ORPHANED - Only used in experimental/test code

**Exports**:
- `pub struct App` - **Root component, IS USED** (runtime.rs:172)
- `pub struct ScoresTab` - Only used in App hierarchy
- `pub struct StandingsTab` - Only used in App hierarchy
- `pub struct SettingsTab` - Only used in App hierarchy
- `pub struct BoxscorePanel` - Only used in App hierarchy
- `pub struct TabbedPanel` - Only used in App hierarchy
- `pub struct StatusBar` - Only used in App hierarchy

**Status**: App component IS used in `framework/runtime.rs:172`, but all others are only used within the App component tree, which itself is not imported in `mod.rs`.

**Verdict**: Component library orphaned from production (only used in experimental branch)

---

## Test-Only Code

### 8. Framework Test Files

**Files**:
- `src/tui/framework/experimental_tests.rs` - 5.5 KB
- `src/tui/framework/integration_tests.rs` - 6.5 KB

**Status**: Test-only code with `#[cfg(test)]`

**Note**: These are legitimate test files, but they test code that isn't used in production

---

### 9. `src/tui/widgets/testing.rs` - 250+ Lines

**Status**: Test utility library

**Exports** (all test helpers):
- `pub fn test_config() -> DisplayConfig`
- `pub fn test_config_ascii() -> DisplayConfig`
- `pub fn render_widget()`
- `pub fn buffer_to_string()`
- `pub fn assert_buffer()`
- etc.

**Usage**: Only imported in test modules (correct location for test utilities)

**Verdict**: Legitimately test-only; no issue here

---

## Unused Public Exports from `mod.rs`

```rust
// src/tui/mod.rs - Lines 1-18
pub mod navigation;          // Used in: standings, scores, players, stats, settings, browser, common
pub mod widgets;             // Used in: 170+ call sites (ACTIVE)
pub mod command_palette;     // Used in: mod.rs only (1 usage)
pub mod framework;           // Used in: mod_experimental.rs only (experimental branch)
pub mod components;          // Used in: framework/runtime.rs only (experimental branch)

pub use context::{NavigationContextProvider, BreadcrumbProvider};
// Used in: mod.rs only, but mod.rs is dead
```

**Summary**:
- `framework` - Active but only in experimental branch
- `components` - Active but only in experimental branch
- `context` - Exported but only used internally in dead code
- `navigation` - Active and used by several tab modules (production code)
- `widgets` - Active and widely used (production code)
- `command_palette` - Active but minimal usage

---

## Module Structure Diagram

```
main.rs
  └─→ tui::run_experimental()  ✓ ACTIVE
      └─→ framework::Runtime
          └─→ framework::reducer
          └─→ framework::effects
          └─→ framework::renderer
          └─→ components::App  ✓ USED
              ├─→ ScoresTab
              ├─→ StandingsTab
              ├─→ SettingsTab
              ├─→ BoxscorePanel
              └─→ TabbedPanel

      └─→ framework::keys::key_to_action

ORPHANED DEAD BRANCH:
tui::run()  ✗ NEVER CALLED
  └─→ layout::Layout
  └─→ app::AppState
  └─→ scores::  (all code)
  └─→ standings::  (all code)
  └─→ stats::  (all code)
  └─→ players::  (all code)
  └─→ settings::  (all code)
  └─→ browser::  (all code)
  └─→ common::  (all code)
  └─→ context::NavigationContextProvider  (exported but unused)
```

---

## Detailed Orphaned Items List

### Tier 1: Critical Orphaned Functions

1. **`src/tui/mod.rs:646`** - `pub async fn run()`
   - 100 lines, never called
   - Entire event loop and rendering logic
   - Dependency: SharedDataHandle, mpsc channel

### Tier 2: Orphaned Modules (Used Only in Dead Path)

2. **`src/tui/app.rs`** - AppState (old production state struct)
   - Used only by `mod.rs::run()`
   - Not to be confused with `framework::state::AppState` (new version)

3. **`src/tui/common/`** - Tab bar, status bar, panels, rendering helpers
   - All used only by `mod.rs::run()`

4. **`src/tui/scores/`** - Complete scores tab module
   - State, view, handler, game_details submodule
   - ~1000 lines of production code
   - Used only by `mod.rs`

5. **`src/tui/standings/`** - Complete standings tab module
   - State, view, handler, layout
   - ~800 lines of production code
   - Used only by `mod.rs`

6. **`src/tui/stats/`** - Stats tab module
   - ~500 lines
   - Used only by `mod.rs`

7. **`src/tui/players/`** - Players tab module
   - ~400 lines
   - Used only by `mod.rs`

8. **`src/tui/settings/`** - Settings tab module
   - ~600 lines
   - Used only by `mod.rs`

9. **`src/tui/browser/`** - Browser tab module
   - ~400 lines
   - Used only by `mod.rs`

10. **`src/tui/layout.rs`** - Layout manager for chrome elements
    - ~300 lines
    - Only used in `mod.rs::render_frame()`

### Tier 3: Semi-Orphaned Exports

11. **`src/tui/context.rs`** - NavigationContextProvider trait and related types
    - Exported from mod.rs but only used internally
    - 296 lines with tests
    - Implemented by all orphaned tab modules

12. **`src/tui/mod.rs` - Internal helper functions** (lines 61-644)
    - `save_screenshot()` - Feature-gated development utility
    - `log_widget_tree()` - Feature-gated debug function
    - `handle_esc_key()` - Event handler (only used by dead `run()`)
    - `handle_number_keys()` - Event handler (only used by dead `run()`)
    - `handle_enter_subtab_mode()` - Event handler (only used by dead `run()`)
    - `handle_arrow_and_enter_keys()` - Event handler (only used by dead `run()`)
    - `handle_key_event()` - Main dispatcher (only used by dead `run()`)
    - `create_breadcrumb()` - Rendering helper (only used by dead `run()`)
    - `create_action_bar()` - Rendering helper (only used by dead `run()`)
    - `create_status_bar()` - Rendering helper (only used by dead `run()`)
    - `render_frame()` - Main render function (only used by dead `run()`)
    - RenderData struct - Helper struct (only used by dead `run()`)

### Tier 4: Orphaned Exports That Are "Functional" But Unused

13. **`src/tui/mod.rs:17`** - `pub use context::{NavigationContextProvider, BreadcrumbProvider}`
    - Exported but never imported outside mod.rs
    - Used internally only by dead code

14. **`src/tui/framework/experimental_tests.rs`** and **`integration_tests.rs`**
    - Test-only files (legitimate)
    - But they test code that's not in the production path

---

## The Two TUI Implementations: Comparison

| Aspect | Original (mod.rs) | Experimental (mod_experimental.rs) |
|--------|-------------------|-------------------------------------|
| **Status** | ORPHANED | ACTIVE |
| **Lines** | 873 | 189 |
| **Entry Point** | Never called | main.rs:207 |
| **Architecture** | Event-driven, monolithic | React-like components |
| **State Management** | SharedData + local AppState | Reducer + virtual tree |
| **Tab Implementation** | Separate handler/state/view per tab | Components in hierarchy |
| **Tab Count** | 6 tabs (Scores, Standings, Stats, Players, Settings, Browser) | Same 6 tabs (in framework) |
| **Rendering** | Direct to ratatui | Virtual tree → ratatui |

---

## Why This Code Exists

The project is in the middle of a **React-like migration** (PHASE 5-6 based on PHASE5_COMPLETE.md and PHASE6_PLAN.md):

- **Old implementation** (mod.rs) - Classic Elm-like architecture, direct ratatui rendering
- **New implementation** (mod_experimental.rs) - React-like components, virtual tree, reducer pattern

The production code uses the new experimental path, leaving the old one completely unused.

---

## Recommendations for Cleanup

### Critical Priority: Remove Dead Code

1. **Delete entire old production TUI** (estimated 4,400+ lines):
   - Delete or archive `src/tui/mod.rs::run()` function
   - Delete `src/tui/app.rs` (old AppState)
   - Delete `src/tui/common/` (old tab bar, status bar, rendering)
   - Delete `src/tui/scores/` (old implementation)
   - Delete `src/tui/standings/` (old implementation)
   - Delete `src/tui/stats/` (old implementation)
   - Delete `src/tui/players/` (old implementation)
   - Delete `src/tui/settings/` (old implementation)
   - Delete `src/tui/browser/` (old implementation)
   - Delete `src/tui/layout.rs` (only used by old implementation)
   - Remove `pub use context::*` from `mod.rs`

2. **Move experimental to production**:
   - Rename `mod_experimental.rs` → move into `mod.rs` as the main implementation
   - Or keep it as is if the "experimental" name is intentional

3. **Clean up exports**:
   - Remove public exports of `framework` and `components` from `mod.rs` if not needed externally
   - Or document why they're public if part of the library API

### Medium Priority: Consolidate Test Code

4. **Move test utilities** from `framework/` to a dedicated test module if tests are stable

### Low Priority: Documentation

5. **Document the architecture transition** - make it clear which path is active

---

## Statistical Summary

| Metric | Count |
|--------|-------|
| **Completely Orphaned Modules** | 8 (scores, standings, stats, players, settings, browser, common, app) |
| **Orphaned Lines of Code** | ~4,400+ |
| **Unused Public Functions** | 1 (run) |
| **Semi-Orphaned Exports** | 5 (context traits, framework mod, components mod) |
| **Active Modules** | framework, components (via runtime), mod_experimental |
| **Modules with No Production Use** | layout (300 lines), context (296 lines with tests) |

---

## Key Findings

1. **One non-dead function** (`run_experimental`) is the only active TUI entry point
2. **Multiple parallel implementations** exist without clear deprecation
3. **4,400+ lines** of production-quality but unused code
4. **Component library** (components/) is correctly factored but only consumed by framework
5. **Test code** is properly isolated but tests unused implementation
6. **Migration is incomplete** - old code was not removed when experimental version was completed

---

## References in Codebase

- `PHASE5_COMPLETE.md` - Documents transition to React-like architecture
- `PHASE6_PLAN.md` - Plans for further improvements
- `main.rs:207` - Only call site: `tui::run_experimental(client, config).await`

