# Document System

The document system (`src/tui/document/`) provides scrollable, focusable content views for content that exceeds viewport height.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Document Trait                          │
│  - build(focus) -> Vec<DocumentElement>                     │
│  - calculate_height() -> u16            (default, via build)│
│  - focusables(ctx) -> Vec<FocusableElement> (default, via   │
│    build) -- one pass, one call, the single source of       │
│    focus metadata (position/height/id/row/link target)      │
│  - render_full(width, ctx, focus) -> (Buffer, u16)          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    DocumentView                              │
│  - Holds Arc<dyn Document>                                   │
│  - Render-only shim: applies a focus index + scroll offset   │
│    (both computed by document_nav.rs) and renders            │
│  - No navigation methods of its own -- see                   │
│    "Generic Document Navigation (document_nav.rs)" below     │
└─────────────────────────────────────────────────────────────┘
```

`DocumentView::new()` builds nothing: at construction time neither the real
content width (known only when `render()` sees its area) nor the focused
element (applied afterwards via `focus_id`) are known, so it just records
them as pending state. `render()` then obtains the full-height buffer and
copies the visible slice into the output buffer. Building happens exactly
**once** per changed frame: focus highlighting is baked into the tree by
`build()`, and the focused element's `FocusableId` is supplied directly by
the navigation layer (`DocumentNavState::focused_id`), so no preliminary
unfocused build is needed.

### DocumentRenderCache (cross-frame reuse)

The TUI run loop owns a `DocumentRenderCache` and threads it down to every
`DocumentView` through `RenderContext` (propagated by `RenderContext::child`,
so widgets that derive child contexts keep it). Each entry stores a
document's full-height rendered buffer together with every input it depended
on: data identity, focus index, content width, focused flag, tab selections,
and a config fingerprint (theme name, unicode, box chars, error color).

While those inputs are unchanged, re-renders skip `Document::build` and the
offscreen render entirely — scrolling and the idle 1 Hz status-bar redraw
become a pure viewport copy, because the scroll offset only selects which
slice of the cached buffer is blitted. Any changed input rebuilds exactly
once and replaces the entry.

Data identity works two ways:

- **Persistent documents** (team/player/boxscore — built once when data
  arrives and stored in state) hit via `Arc` pointer equality.
- **Per-frame documents** (the standings documents, recreated each frame
  around persistent data `Arc`s) implement `Document::cache_token()`,
  returning the addresses of the `Arc`s their `build()` output depends on.
  Equal tokens mean identical content because each cache entry holds the
  previously rendered document — keeping those allocations alive, so an
  address cannot be reused by different data while the entry exists.
  Documents that don't implement `cache_token()` (settings, score boxes,
  demo) simply rebuild every frame, exactly as before.

CLI paths and tests that construct a `RenderContext` without a cache render
uncached, with identical output.

## Document Trait

```rust
pub trait Document: Send + Sync {
    /// Build the element tree (called on each render, and by both default
    /// methods below)
    fn build(&self, focus: &FocusContext) -> Vec<DocumentElement>;

    /// Document title for navigation/history
    fn title(&self) -> String;

    /// Unique document ID
    fn id(&self) -> String;

    // Default implementations provided, each calling build() once with a
    // FocusContext:
    fn calculate_height(&self) -> u16;

    /// The single source of "what's focusable in this document, with what
    /// metadata": one `build()` call, one collection pass, every field
    /// (position, height, ID, row membership, link target) gathered
    /// together so a sync site can no longer update some and forget others.
    fn focusables(&self, ctx: &FocusContext) -> Vec<FocusableElement>;

    fn render_full(&self, width: u16, ctx: &RenderContext, focus: &FocusContext) -> (Buffer, u16);
}
```

## DocumentElement Types

`DocumentElement` (`src/tui/document/elements/mod.rs`) has more variants than
a quick glance at the builder API suggests:

```rust
pub enum DocumentElement {
    Text { content: String, style: Option<Style> },
    Heading { level: u8, content: String },
    SectionTitle { content: String, underline: bool },
    Link { display: String, target: LinkTarget, id: String, focused: bool },
    Separator,
    Spacer { height: u16 },
    Group { children: Vec<DocumentElement>, style: Option<Style> },
    Custom { render_fn: fn(Rect, &mut Buffer, &RenderContext), height: u16, focusable: Vec<FocusableElement> },
    Table { widget: TableWidget, focusable: Vec<FocusableElement> },
    Row { children: Vec<DocumentElement>, gap: u16, align: RowAlignment },
    ScoreBoxElement { id: String, game_id: i64, score_box: ScoreBox, focused: bool },
    Indented { element: Box<DocumentElement>, margin: u16 },
    TeamBoxscore { team_name: String, forwards_table: TableWidget, defense_table: TableWidget, goalies_table: TableWidget, focusable: Vec<FocusableElement> },
    BigScoreElement { big_score: BigScore },
    Tabs { id: String, tabs: Vec<DocTabDef>, active_index: usize },
}
```

**Key types:**
- **Link**: Focusable element with a `LinkTarget` for activation.
- **Table**: Embeds a `TableWidget`; focusable cells are extracted from
  `PlayerLink`/`TeamLink` cell values (non-link cells are skipped entirely).
- **Row**: Horizontal layout enabling left/right navigation between children,
  laid out per `RowAlignment` (`Left`, `Spread` (default), or `Center`).
- **Group**: Vertical container for nested elements.
- **Tabs**: An in-document tabbed panel; only the active tab's children
  contribute to height and focusable-element collection. Active tab index is
  read from `FocusContext::tab_selections` (populated from
  `DocumentNavState::doc_tab_selections`), not stored on the element itself
  when built via `DocumentElement::tabs_from_context` /
  `DocumentBuilder::tabs_with_focus`.
- **TeamBoxscore** / **ScoreBoxElement** / **BigScoreElement**: composite
  widgets specific to boxscore/score rendering, each contributing their own
  focusable metadata.

There is no `Blank` variant - vertical spacing is `Spacer { height }`.

## Row Navigation (Left/Right)

The `Row` element enables horizontal navigation between side-by-side content:

```rust
DocumentBuilder::new()
    .row(vec![
        DocumentElement::table("left", left_table),
        DocumentElement::table("right", right_table),
    ])
```

(`DocumentElement::table` takes `(name, widget)`, in that order.)

### How it works

Each element in a Row gets a `RowPosition`:

```rust
pub struct RowPosition {
    pub row_y: u16,           // Y position identifying the Row
    pub child_idx: usize,     // 0 = leftmost, 1 = next, etc.
    pub idx_within_child: usize, // Position within that child
}
```

Left/Right arrows find the element with matching `row_y` and (as close as
possible to) the same `idx_within_child` in the adjacent `child_idx`
(`document_nav::find_row_sibling`) - if the exact `idx_within_child` doesn't
exist in the target column (e.g. one team has fewer players than the other),
it falls back to the closest available index rather than failing.

**Wrapping**: Left at leftmost wraps to rightmost; Right at rightmost wraps to leftmost.

**Example:**
```
Row with two tables (5 rows each):
┌─────────────┐  ┌─────────────┐
│ Table Left  │  │ Table Right │
├─────────────┤  ├─────────────┤
│ Row 0 ◄─────┼──┼─► Row 0     │  ← Left/Right moves between tables
│ Row 1 ◄─────┼──┼─► Row 1     │     preserving row position
│ Row 2       │  │   Row 2     │
└─────────────┘  └─────────────┘
```

## Focus Navigation

**Up/Down:**
- Cycles through all focusable elements in document order
- Wraps from last to first (Down) or first to last (Up)
- Autoscrolls viewport to keep focused element visible

**Left/Right (within Rows):**
- Only works when focused element is inside a Row
- Moves to same relative position in adjacent child
- Wraps around at edges

**Enter:**
- Activates the focused element
- Returns `LinkTarget` for navigation actions

See `docs/navigation.md` for the full key-to-message mapping
(`nav_handler::key_to_nav_msg`) that drives this, including where individual
tabs and `handle_stacked_document_key` deliberately diverge from it.

## DocumentBuilder

Declarative API for constructing documents:

```rust
let doc = DocumentBuilder::new()
    .heading(1, "Player Stats")
    .spacer(1)
    .text("Top scorers this season:")
    .table("scorers", stats_table)
    .separator()
    .link_with_id(
        "team_bos",
        "Boston Bruins",
        LinkTarget::Push(StackedDocument::TeamDetail { abbrev: "BOS".into(), season: None }),
    )
    .when(show_details, |b| b.text("Additional details..."))
    .for_each(players.iter(), |b, player| b.text(format!("- {}", player.name)))
    .build();
```

Notes:
- `.link(display, target)` auto-generates an ID (`link_0`, `link_1`, ...);
  use `.link_with_id(id, display, target)` when the ID needs to be stable
  (e.g. for focus-by-ID lookups) or `.link_with_focus(id, display, target,
  focus)` to render it focused based on a `FocusContext`.
- There is no `.blank()` method - use `.spacer(height)` for vertical spacing.
- `.tabs(id, tabs, active_index)` / `.tabs_with_focus(id, tabs, focus)` add an
  in-document `Tabs` element from `Vec<(key, title, content)>` tuples.

## Generic Document Navigation (document_nav.rs)

Reusable navigation module that eliminates duplication across components.

### DocumentNavState

```rust
#[derive(Debug, Clone, Default)]
pub struct DocumentNavState {
    pub focus_index: Option<usize>,
    pub scroll_offset: u16,
    pub viewport_height: u16,
    /// All focusable elements in document order, carrying position, height,
    /// ID, row membership, and link target together -- replaces five
    /// index-aligned Vecs that used to be synced by hand at every call site.
    pub focusables: Vec<FocusableElement>,
    /// Active tab selections for Tabs elements (tabs_id -> active_index)
    pub doc_tab_selections: HashMap<String, usize>,
}
```

`DocumentNavState::sync_focusables(&mut self, doc: &dyn Document, ctx:
&FocusContext)` is the single sync path: every call site that needs to
refresh navigation metadata (after data loads, a view/category changes, or
the layout width changes) calls this one method - `self.focusables =
doc.focusables(ctx)` - instead of hand-copying individual fields.

Components embed `DocumentNavState` directly (`ScoresTabState`,
`StandingsTabState`, `SettingsTabState`) and stacked documents embed it
inside `DocumentStackEntry::nav`. `DocumentStackEntry::new()` initializes it
with `focus_index: Some(0)` and `viewport_height: DEFAULT_VIEWPORT_HEIGHT` so
a freshly pushed document starts with its first focusable element selected.

### DocumentNavMsg

```rust
pub enum DocumentNavMsg {
    FocusNext,
    FocusPrev,
    FocusLeft,
    FocusRight,
    ScrollUp(u16),
    ScrollDown(u16),
    ScrollToTop,
    ScrollToBottom,
    PageUp,
    PageDown,
    UpdateViewportHeight(u16),
}
```

### Usage

1. Embed `DocumentNavState` in component state
2. Wrap `DocumentNavMsg` in component messages
3. Delegate to `document_nav::handle_message()` in update

```rust
impl Component for MyTab {
    fn update(&mut self, msg: Self::Message, state: &mut Self::State) -> Effect {
        match msg {
            MyTabMsg::DocNav(nav_msg) => {
                document_nav::handle_message(&nav_msg, &mut state.doc_nav)
                    .unwrap_or(Effect::None)
            }
            // ...
        }
    }
}
```

## Stacked Documents: Loading State and Fetch Guarding

Stacked documents (Boxscore, TeamDetail, PlayerDetail) are the three
`StackedDocument` variants pushed onto `AppState.navigation.document_stack`.
Pushing one is handled by `reduce_document_stack` /
`push_document` (`src/tui/reducers/document_stack.rs`), which:

1. Pushes a `DocumentStackEntry::new(doc)` onto the stack.
2. Checks whether the needed data is already cached (`data.boxscores` /
   `data.team_roster_stats` / `data.player_data`) **and** whether a fetch for
   it is already in flight (`data.loading.contains(&LoadingKey::...)`).
3. If neither is true, inserts the matching `LoadingKey` (`LoadingKey::Boxscore(game_id)`,
   `LoadingKey::TeamRosterStats(abbrev)`, or `LoadingKey::PlayerStats(player_id)`)
   into `AppState.data.loading` and returns the corresponding `Effect::Fetch*`
   action; otherwise it returns `Effect::None`.

Popping a document (`pop_document`) removes the matching `LoadingKey`, so a
document popped mid-fetch doesn't leave a stale loading flag behind (though
the in-flight request itself isn't cancelled - the reducer that eventually
handles the fetch response is still responsible for a safe no-op if the
document is no longer on the stack).

`AppState.data.loading` is what gates the loading spinner: `app.rs`
(`render_stacked_document`) reads `state.data.loading.contains(&LoadingKey::...)`
into each document's `loading: bool` prop, alongside `animation_frame` for the
spinner's frame. This is also what prevents a double-fetch: if the user
presses Enter on the same team/player/game again while its `LoadingKey` is
still present, `push_document` sees the loading flag set and returns
`Effect::None` instead of issuing a second fetch.

## Creating a New Document

1. **Define the document struct:**
   ```rust
   pub struct MyDocument {
       data: Vec<MyData>,
   }
   ```

2. **Implement Document trait:**
   ```rust
   impl Document for MyDocument {
       fn build(&self, focus: &FocusContext) -> Vec<DocumentElement> {
           DocumentBuilder::new()
               .heading(1, "My Document")
               .for_each(self.data.iter(), |b, item| {
                   b.link_with_id(&item.id, &item.name, LinkTarget::Push(item.destination.clone()))
               })
               .build()
       }

       fn title(&self) -> String { "My Document".into() }
       fn id(&self) -> String { "my_doc".into() }
       // `calculate_height` and `focusables` are provided by the trait's
       // default implementations, both built on top of `build()` above.
   }
   ```

3. **Store focusable metadata in state** when data changes, via
   `state.doc_nav.sync_focusables(&doc, &ctx)` (or, for a stacked document,
   this happens automatically on every key event -
   `handle_stacked_document_key` calls it after building the document from
   `build_stacked_document`; see `src/tui/document/handlers.rs` and
   `src/tui/document/factory.rs`).

4. **Handle navigation** via `DocumentNavMsg` in the component, or, if this
   is a stacked document, nothing extra to write: `handle_stacked_document_key`
   (`src/tui/document/handlers.rs`) already wires up `key_to_nav_msg` and
   `Enter` activation (`nav.focused_link_target()`) generically for every
   `StackedDocument` variant.
