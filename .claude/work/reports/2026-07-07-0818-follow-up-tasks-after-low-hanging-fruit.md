# Follow-Up Tasks — Skipped Items After Low-Hanging-Fruit Batches

Date: 2026-07-07 08:18
Source: `.claude/work/reports/2026-07-06-2212-fresh-analysis-low-hanging-fruit.md`
Context: Tier 1 (items 1–7) and Tier 2 (items 1–5) were completed on 2026-07-06/07 across two
delegated batches, verified by build + 605 passing tests + an integration pass over all 11 CLI
commands. Everything below was deliberately left out of scope and remains to be done.
All completed work is still **uncommitted** in the working tree.

---

## 1. Tier 3 — delegatable with a clear spec (from the original report)

### 1.1 Test the untested `commands/` layer
Zero tests today in:
- `src/commands/tail.rs` (517 lines)
- `src/commands/boxscore.rs`
- `src/commands/player_stats.rs`
- `src/commands/franchises.rs`

Also untested:
- `src/tui/components/standings_documents/{wildcard,division,conference,league}.rs` — non-trivial
  grouping logic
- `src/tui/document/elements/render.rs` (565 lines) — the highest-value single coverage gap

Fixture infrastructure already exists in `src/fixtures.rs`, so this is mostly labor.
Project rule: 90% coverage for new code; `assert_buffer` for rendering tests.

### 1.2 Split the >100-line functions
- `src/tui/mod.rs:96` — `run()` (210 lines)
- `src/tui/reducer.rs:59` — `reduce()` (136 lines)
- `src/commands/boxscore.rs:99` (129 lines)
- `src/commands/player_stats.rs:12` — `run()` (124 lines)

(Line numbers are from the 2026-07-06 analysis; re-verify before starting since the two
batches shifted some files.)

### 1.3 Replace panicking invariants with graceful handling
- `src/tui/component_store.rs:40,53,66` — three `.expect("State type mismatch")`
- `src/tui/table.rs:166` — `panic!` in clone

Either handle gracefully (per the `anyhow::Result` rule) or document the invariant explicitly.

---

## 2. Architecture-level (explicitly "not fruit" — needs design judgment, top-tier model or human)

### 2.1 Per-frame document rebuild (corrected 2026-07-07)
The original analysis called this a "deep clone caused by `DocumentView::new` taking ownership".
That was wrong: `DocumentView::new` takes `Arc<dyn Document>` and the per-frame
`document.clone()` in `render_document_widget()` is an `Arc` refcount bump — already shallow.
Switching to a borrow (`&dyn Document`) is trivial (the view never outlives a frame) but saves
nothing meaningful.

The real per-frame cost in `src/tui/document/mod.rs`:
1. `DocumentView::new` calls `document.build()` for height + `FocusManager` (line ~227).
2. `render()` → `render_full()` calls `build()` **again** (line ~177), then renders the entire
   document to a full-height offscreen buffer and copies out the visible slice.
3. `full_buffer`/`cached_height` are write-only caches — the view is reconstructed every frame,
   so the cache never survives to a second frame.
4. `Vec<Standing>`-cloned-out-of-`Arc` is paid inside each `build()`, i.e. twice per frame.

Fixes, ascending ambition:
- **Merge the two builds** (have `new()` keep its elements, or share one `build()` per frame) —
  localized, no lifetime changes, halves build cost. This part IS delegatable fruit.
- **Persist the `DocumentView`/built elements/full buffer across frames** in component state,
  invalidated on data/width/focus change (`build()` takes `FocusContext`, so focus moves
  invalidate). This is the real architecture change needing design judgment.

Performance-shaped debt, invisible at human timescales — only worth it if planning bigger documents.

### 2.2 Unify grouping logic across the four standings-document variants
`standings_documents/{wildcard,division,conference,league}.rs` are structurally similar but not
identical; unifying needs design judgment, don't delegate blindly.

---

## 3. Meta / tooling issues (the report's "unsolicited opinions", none addressed)

### 3.1 Enforce or soften the 90%-coverage rule
CLAUDE.md mandates 90% coverage for new code but nothing in CI measures it, and the whole
`commands/` layer is untested. Either wire `cargo tarpaulin` into CI with a threshold
(a `/test-coverage` skill already exists) or soften the written rule.

### 3.2 Prune dead skills/agents scaffolding
~30 skills and ~15 agent definitions for a project with 6 TODOs. Known-dead example:
`/benchmark` skill has nothing to run (the `criterion` dep was removed in Batch 1 and no
`benches/` dir ever existed — decide whether to delete the skill or add real benchmarks).
Estimated ~30-minute pass to delete or fix dead entries.

### 3.3 Growth-point watch (act when next touched, not before)
- `src/tui/document/elements/mod.rs` (1703 lines)
- `src/tui/keys.rs` (1662 lines)
Split these the next time a feature touches them.

---

## 4. New items surfaced during the batches (not in the original report)

### 4.1 Pre-existing `cargo fmt` drift
`cargo fmt --check` fails on four files neither batch touched:
- `src/commands/scores_format.rs`
- `src/fixtures.rs`
- `src/tui/components/boxscore_document.rs`
- `src/tui/components/score_boxes_document.rs`

Baseline repo state. Fix is `cargo fmt`, but decide first whether to also add
`cargo fmt --check` to CI (currently not gated, which is how the drift accumulated).

### 4.2 Explicit unit tests for `GroupBy::next()` / `GroupBy::prev()`
Integration-tester recommendation: the cycling behavior (Wildcard → Division → Conference →
League → wrap) in `src/commands/standings.rs` is exercised indirectly via `standings_tab.rs`
tests but has no direct unit tests documenting the cycle order.

### 4.3 Manual TUI smoke check
The integration pass could not drive the interactive TUI (no terminal in the agent
environment). Unit tests cover render logic via `assert_buffer`, but a quick manual
`cargo run --features development -- --mock` pass over the refactored paths
(view cycling, opening team/player/boxscore documents, enter/exit focus) would close the gap.
Alternative: the report's suggestion of visual regression screenshots in CI
(a `/mock-screenshot` skill exists).

---

## Suggested batching

1. **Quick batch (delegatable now):** 4.1 fmt + CI gate decision, 4.2 GroupBy tests, 1.3 panics.
2. **Test-writing batch (delegatable, largest):** 1.1 — one agent per module group
   (`commands/`, `standings_documents/`, `document/elements/render.rs`).
3. **Refactor batch (delegatable with care):** 1.2 function splits, after batch 2 so tests exist first.
4. **Human/top-tier decisions:** 2.1, 2.2, 3.1, 3.2, 3.3.
