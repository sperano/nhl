# Panel Stack to Document Stack Refactoring - COMPLETED

## Summary

Successfully converted the panel stack system to a document stack system. All "panel" terminology has been eliminated from the codebase (except for `TabbedPanel` which is unrelated to drill-down navigation).

## Naming Decisions
- `Panel` enum → `StackedDocument` (emphasizes push/pop stack nature)
- `TabbedPanel` component stays as-is (it's for tabs UI, not drill-down)

## Completed Changes

### Core Type Renames
- ✅ `Panel` enum → `StackedDocument` in `types.rs`
- ✅ `PanelState` → `DocumentStackEntry` in `state.rs`
- ✅ `panel_stack` → `document_stack` in `state.rs`

### Action Renames
- ✅ `PushPanel` → `PushDocument` in `action.rs`
- ✅ `PopPanel` → `PopDocument` in `action.rs`
- ✅ `PanelSelectNext/Previous/Item` → `DocumentSelectNext/Previous/Item`

### Reducer Updates
- ✅ Renamed `reducers/panels.rs` → `reducers/document_stack.rs`
- ✅ Renamed `reduce_panels` → `reduce_document_stack`
- ✅ Updated all panel references in reducer functions

### Component File Renames
- ✅ `boxscore_panel.rs` → `boxscore_document.rs`
- ✅ `team_detail_panel.rs` → `team_detail_document.rs`
- ✅ `player_detail_panel.rs` → `player_detail_document.rs`

### Component Type Renames
- ✅ `BoxscorePanel` → `BoxscoreDocument`
- ✅ `BoxscorePanelProps` → `BoxscoreDocumentProps`
- ✅ `BoxscorePanelWidget` → `BoxscoreDocumentWidget`
- ✅ `TeamDetailPanel` → `TeamDetailDocument`
- ✅ `TeamDetailPanelProps` → `TeamDetailDocumentProps`
- ✅ `PlayerDetailPanel` → `PlayerDetailDocument`
- ✅ `PlayerDetailPanelProps` → `PlayerDetailDocumentProps`

### Other Files Updated
- ✅ `runtime.rs` - Document stack detection and data fetching
- ✅ `keys.rs` - Key handling for document navigation
- ✅ `reducers/navigation.rs` - Tab navigation clears document stack
- ✅ `components/app.rs` - Rendering stacked documents
- ✅ `components/mod.rs` - Module exports
- ✅ `effects.rs` - Test state creation
- ✅ `widgets/mod.rs` - Documentation comments
- ✅ `CLAUDE.md` - Full documentation update

### Tests
- ✅ All test terminology updated
- ✅ All tests passing (`cargo test` passes)

## Architecture After Refactoring

### Document Stack
```rust
pub struct DocumentStackEntry {
    pub document: StackedDocument,
    pub selected_index: Option<usize>,
}

pub enum StackedDocument {
    Boxscore { game_id: i64 },
    TeamDetail { abbrev: String },
    PlayerDetail { player_id: i64 },
}
```

### Navigation State
```rust
pub struct NavigationState {
    pub current_tab: Tab,
    pub document_stack: Vec<DocumentStackEntry>,  // formerly panel_stack
    pub content_focused: bool,
}
```

### Actions
```rust
pub enum Action {
    // Document stack management
    PushDocument(StackedDocument),  // formerly PushPanel
    PopDocument,                     // formerly PopPanel
    DocumentSelectNext,              // formerly PanelSelectNext
    DocumentSelectPrevious,          // formerly PanelSelectPrevious
    DocumentSelectItem,              // formerly PanelSelectItem
    // ...
}
```

## Files Changed (Complete List)

1. `src/tui/types.rs`
2. `src/tui/state.rs`
3. `src/tui/action.rs`
4. `src/tui/reducer.rs`
5. `src/tui/reducers/mod.rs`
6. `src/tui/reducers/document_stack.rs` (renamed from panels.rs)
7. `src/tui/reducers/navigation.rs`
8. `src/tui/reducers/scores.rs`
9. `src/tui/runtime.rs`
10. `src/tui/keys.rs`
11. `src/tui/effects.rs`
12. `src/tui/navigation.rs`
13. `src/tui/components/mod.rs`
14. `src/tui/components/app.rs`
15. `src/tui/components/boxscore_document.rs` (renamed from boxscore_panel.rs)
16. `src/tui/components/team_detail_document.rs` (renamed from team_detail_panel.rs)
17. `src/tui/components/player_detail_document.rs` (renamed from player_detail_panel.rs)
18. `src/tui/components/breadcrumb.rs`
19. `src/tui/components/standings_tab.rs`
20. `src/tui/widgets/mod.rs`
21. `CLAUDE.md`

## Verification

- Build: `cargo build` passes with no errors
- Tests: `cargo test` passes with all tests passing
- Documentation: CLAUDE.md fully updated with new terminology

## Original Plan (For Reference)

The original plan was to convert the panel stack system to use document terminology:

### Phase 1: Convert Panels to Documents
Convert existing panel components to document implementations (direct replacement).

### Phase 2: Rename Concepts
- Types: `Panel` → `StackedDocument`
- State: `PanelState` → `DocumentStackEntry`, `panel_stack` → `document_stack`
- Actions: `PushPanel/PopPanel` → `PushDocument/PopDocument`
- Reducers: `panels.rs` → `document_stack.rs`
- Keys: Panel navigation → Document navigation
- Components: `*_panel.rs` → `*_document.rs`

### Phase 3: Update Navigation System
Document stack entry state with `DocumentNavState`.

### Phase 4: File Changes
All files updated as listed above.

### Phase 5: Testing
All tests passing.

---

**Completed: 2025-11-28**
