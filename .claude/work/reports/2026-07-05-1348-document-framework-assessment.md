# Document framework assessment — simplicity & DRY

Date: 2026-07-05 13:48
Scope: `src/tui/document/` (~8,400 lines incl. tests), `document_nav.rs`, `tab_component.rs`,
plus their consumers (components, handlers, reducers).

## The verdict in one paragraph

The framework's concepts are right — declarative element trees, typed focusable identities,
hyperlink navigation, a document stack. The problem is that the *same core idea* (focus +
scroll + "what is at this focus index") is implemented three times in parallel, kept in sync by
discipline instead of by construction. Nearly every latent bug found this month (PlayerDetail
activation mismatch, cross-tab bleed's phantom-highlight half, the A9 "riskiest refactor"
metadata cache) is a symptom of one of these duplications. Collapsing them removes on the
order of 1,000+ lines and makes the invariants structural.

## Finding 1 — Two parallel navigation engines (the big one)

There are two complete implementations of focus/scroll/wrap/autoscroll/paging:

| Concern | Engine A: `DocumentView`+`Viewport`+`FocusManager` | Engine B: `document_nav.rs` |
|---|---|---|
| focus next/prev with wrap | `DocumentView::focus_next/prev` | `focus_next/prev(state)` |
| autoscroll to focus | `autoscroll_to_focused` + `Viewport::smart_padding` | `autoscroll_to_focus` + `AUTOSCROLL_PADDING = 3` |
| page up/down | `PAGE_OVERLAP_LINES = 2` | `MIN_PAGE_SIZE = 10` |
| scroll ops | `Viewport::scroll_*` | `scroll_*(state)` |

**Production reality**: Engine B (`DocumentNavState`) is the single source of truth. Engine A
is a per-frame throwaway — `DocumentElementWidget::render` (widget.rs:66) constructs a fresh
`DocumentView` and immediately *injects* focus_index and scroll_offset back into it from
`DocumentNavState`. `DocumentView`'s own navigation methods (`focus_next/prev`, `page_up/down`,
`scroll_*`) are called only from tests. Two engines means constants and semantics that can
silently diverge (they already do: page overlap 2 vs min-page 10, different padding rules).

**Fix**: one engine. Reduce `DocumentView` to a pure render shim — `(document, focus_index,
scroll_offset, area) → buffer` — or delete it entirely and let `DocumentElementWidget` do the
three steps itself. All *navigation* semantics live only in `document_nav.rs`. Port the
handful of Engine-A-only behaviors worth keeping (wrap-to-bottom scroll on backward wrap) into
Engine B once, delete the rest with their tests.

## Finding 2 — The focusable-metadata cache: 5 parallel Vecs, synced by hand at 6+ sites

`DocumentNavState` carries `focusable_positions`, `focusable_heights`, `focusable_ids`,
`focusable_row_positions`, `link_targets` — five index-aligned Vecs (struct-of-arrays) that are
a manual copy of what `FocusManager::from_elements` already computes as `Vec<FocusableElement>`
(one struct with y/height/id/row_position/link_target — the exact type already exists in
focus.rs).

Every sync site does it differently, and several are wrong-by-omission:
- `handlers.rs` Boxscore: builds elements once, `collect_focusable`, fills **all 5** fields. ✔
- `handlers.rs` TeamDetail/PlayerDetail: call four separate `Document` trait methods
  (`focusable_positions()`, `focusable_heights()`, `focusable_ids()`,
  `focusable_link_targets()`) — **each one internally rebuilds the entire document**, so four
  full builds per keypress — and **skip `focusable_row_positions`** entirely.
- `settings_tab.rs:126`: sets positions + ids only (two builds; no heights/targets/rows).
- `reducers/data_loading.rs` (schedule→scores, demo), `reducer.rs`
  `rebuild_standings_focusable_metadata`: more variants of the same copy.

**Fix**:
1. Replace the five Vecs with one `focusables: Vec<FocusableElement>` (accessors for the old
   shapes during migration if needed).
2. One constructor: `DocumentNavState::sync_from(doc: &dyn Document, ctx: &FocusContext)` —
   build once, collect once, fill everything. Every sync site becomes one line; a site that
   forgets a field becomes impossible.
3. On the `Document` trait, replace the five `focusable_*()` methods (each of which calls
   `build()` again) with a single `focusables(&self, ctx) -> Vec<FocusableElement>` default —
   or drop them from the trait entirely since `FocusManager::from_elements` covers it.

This is A9 ("the single riskiest refactor") made safe: instead of unifying six bespoke sync
sites in place, you delete the possibility of divergence.

## Finding 3 — Activation ignores the link system (biggest DRY + correctness win)

The framework already attaches a `LinkTarget` to every focusable during build and copies it
into `nav.link_targets`; `DocumentNavState::focused_link_target()` exists and works — the
Standings tab already activates through it.

Yet all three `StackedDocumentHandler::activate()` implementations ignore it and re-derive
"what did the user activate" via **index arithmetic against re-sorted/re-filtered data**:
- Boxscore: `get_player_info_at_index` — 70 lines of six-section boundary math that must
  mirror the document's section order forever.
- TeamDetail: re-clones and re-sorts skaters/goalies "the same way as display" (a comment is
  the only thing enforcing that).
- PlayerDetail: re-filters/re-sorts seasons — and its index space **already diverges** from
  the focusable index space when a season's team name doesn't resolve (the pinned latent bug:
  visually-focused row silently fails to activate).

**Fix**: have `build()` attach the real destination to each link (player rows →
`DocumentLink::player(id)` etc. — carrying `sweater_number`/`last_name` payloads either in
`LinkParams` or by resolving them at push time), then `activate()` becomes one default trait
method: `nav.focused_link_target() → Effect`. The three handler structs lose ~200 lines, and
the index-mismatch class of bug becomes unrepresentable — the target is computed in the same
pass that made the element focusable.

**Related dead weight**: the typed link vocabulary (`LinkTarget::Document`, `DocumentLink`,
`DocumentType`, `LinkParams` — ~100 lines + ~140 lines of tests) is constructed **nowhere** in
production. The living system is stringly-typed: `LinkTarget::Action("team:TOR")`,
`"edit:log_level"`, `"open_boxscore_{id}"`, parsed ad hoc at each activation site. Keep exactly
one: recommend the **typed** one (consumers stop string-parsing; the compiler checks link
payloads), migrating the `"verb:arg"` strings to variants. Deleting the string grammar also
removes the `demo_tab`/`standings_tab`/`settings` parsers.

## Finding 4 — Documents are constructed twice per type

For each stacked document type, the *content document* is built in two unrelated places:
- Render path: `app.rs` constructs `BoxscoreDocumentContent` (etc.) for display.
- Input path: `handlers.rs` `populate_focusable_metadata` constructs the same document again
  from the same `DataState` to extract metadata.

Two construction sites per type = another sync-by-discipline seam (the boxscore handler
already has to remember `TeamView::Away` + width to match what render did). With Findings 2+3
done, the only per-type logic left in handlers is exactly this construction.

**Fix**: one factory — `fn build_document(doc: &StackedDocument, data: &DataState) ->
Option<Arc<dyn Document>>` — used by both app.rs (render) and the input path. At that point
`StackedDocumentHandler` + `get_stacked_document_handler` (a hand-rolled vtable over an enum
that already exists) can collapse to plain functions on `StackedDocument`, and `handlers.rs`
mostly disappears into it.

## Finding 5 — Smaller cleanups

- **`FocusContext` is misnamed and repetitive**: it's really a build context (width, unicode,
  box_chars, tab_selections, focused_id). Its five constructors repeat the same field defaults;
  `from_id`/`with_link`/`with_table_cell` are `Default::default()` + one field. ~40 lines.
- **`Runtime::update_viewport_heights`** hardcodes per-tab chrome math (`BASE_CHROME_LINES`,
  `SUBTAB_CHROME_LINES`) and repeats a near-identical `get_mut::<XState>` block per tab; every
  new tab needs a manual, unenforced addition. Chrome height belongs to the component (or a
  `TabState` method), not the runtime.
- **Per-tab key-semantic divergences** (Settings BackTab no-op, Demo Shift+Left/Right
  row-navigating, Scores box-selection missing Page/Home/End) are preserved quirks, not
  designs. Converging every tab's content mode on the canonical `key_to_nav_msg` mapping would
  delete the remaining carve-outs in keys.rs — but it's a user-visible behavior change, so it
  needs a product decision, not just a refactor.
- `AppState.data.errors` written but never read / `Action::Error` never dispatched (from D1) —
  outside the document system but same "two mechanisms, one dead" smell.

## Suggested order (each step independently shippable, TTY spot-check after 2–4)

1. **F3 typed link activation** — highest value/risk ratio; handlers now have 42 tests, and the
   pinned PlayerDetail-mismatch test flips from documenting the bug to proving the fix.
2. **F2 single metadata struct + single sync fn** — mechanical once F3 removed the index math;
   kills the A9 cache problem structurally.
3. **F4 one document factory** — collapses handlers.rs; render and input provably see the same
   document.
4. **F1 single nav engine** — delete DocumentView's navigation surface (or DocumentView
   itself); port wrap-scroll niceties into document_nav once.
5. **F5 cleanups** — FocusContext rename/builder, viewport-height ownership.
6. (Product decision) converge per-tab key semantics on the canonical mapping.

Estimated net deletion across 1–5: ~1,000+ lines of production code and tests-of-duplicates,
with every "must match display" invariant becoming enforced by construction instead of comment.
