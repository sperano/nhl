# Fresh Analysis — Low-Hanging Fruit & Unsolicited Opinions

Date: 2026-07-06 22:12
Scope: full codebase sweep (~35k lines of Rust across `src/`), clippy, test suite, CI config, docs spot-check.

## Baseline health

The project is in genuinely good shape — better than most codebases this size:

- `cargo test --features development` passes cleanly (593 `#[test]` + 29 `#[tokio::test]`, 0 failures).
- Only 6 TODO/FIXME markers in the entire source tree.
- Almost no `unwrap`/`panic` in production paths (3 guarded unwraps, 3 `expect`, 1 `panic!`).
- Docs spot-check found **zero stale references** — `docs/*.md` still matches the code. That is rare.
- CI runs tests + clippy `-D warnings` across all three feature matrices.

The findings below are polish, not rescue.

---

## Low-hanging fruit — delegatable to a lower model

Ordered by ROI. "Model" = the cheapest tier that can do it safely.

### Tier 1: trivial, near-zero risk (Haiku-class, or `clippy --fix`)

| # | Item | Where | Effort |
|---|------|-------|--------|
| 1 | Fix 28 clippy warnings (16× `field_reassign_with_default`, 4× `map_or` simplification, 2× `useless_vec`, 2× bool `assert_eq!`, misc) — 8 are auto-fixable via `cargo clippy --fix --lib -p nhl --tests` | mostly test code | trivial |
| 2 | Remove unused `criterion` dev-dependency (no `benches/` dir, no `[[bench]]` target exists) | `Cargo.toml:29` | trivial |
| 3 | Hoist `DATE_WINDOW_SIZE=5` from inside a match arm to a module const; it's duplicated as hardcoded `2` ("middle of 5") and "stays at 4" edge comments | `src/tui/components/scores_tab.rs:36,153` | trivial |
| 4 | Name the magic render offsets in score_box (`x+19`, `x+20`, `x+24`, `.repeat(18)`, `.repeat(4)`) — only `TEAM_NAME_WIDTH=17` is named | `src/tui/widgets/score_box.rs:191-227` | small |
| 5 | Audit 5 `#[allow(dead_code)]` markers; `LeagueStandingsDocument::config` is held-but-unused | `standings_documents/league.rs:15`, `table.rs:300`, `tab_component.rs:268`, `tabbed_panel.rs:107`, `testing.rs:166` | trivial |
| 6 | Fix path-style inconsistencies (`crate::tui::action::Action` inline vs imported — CLAUDE.md says use imports) | `standings_tab.rs:166,190` | trivial |
| 7 | Decide on the shipped placeholder text "TODO: Player statistics will be displayed here." in the live Demo tab | `demo_tab.rs:234` | trivial |

### Tier 2: mechanical dedup (Sonnet-class, one focused session each)

1. **Extract a shared `render_document_widget()` helper.** The `render()` + `clone_box()` bodies of `team_detail_document.rs:258`, `player_detail_document.rs:311`, and `boxscore_document.rs:396` are near-identical (child RenderContext → loading animation → `document.clone().unwrap()` → `DocumentView` → focus/scroll → render). One extraction simultaneously kills: the 3 guarded unwraps (§ CLAUDE.md `anyhow::Result` rule), the boxscore-only "clone before area check" wasted work, the `selected_index`/`focus_index` naming split, and the two `clone_box` styles. **Highest single ROI in the codebase.**
2. **Fold duplicated tab match arms into `tab_component` helpers.** The `Activate* → PushDocument` arm and the Enter/Exit-focus arms are copy-pasted verbatim across `scores_tab.rs:166-188`, `standings_tab.rs:180-189`, `demo_tab.rs:103-115`.
3. **Collapse `CycleViewLeft`/`CycleViewRight`** — two exact mirror-image `match state.view` blocks; a `next()`/`prev()` on the enum halves it (`standings_tab.rs:156-179`).
4. **Return `&'static str` from constant `title()`/`id()` methods** — 8 methods across `standings_documents/*.rs` allocate a `String` per call for literals.
5. **CI gaps (2-line fixes):** add `--all-targets` to the CI clippy invocation (currently 25 of the 28 warnings are invisible to CI because they're in test code), and drop `--lib` from CI's `cargo test` so doc tests and integration tests actually run in CI (16 doc tests currently only run locally).

### Tier 3: small-to-medium, still delegatable with a clear spec

1. **Test the untested `commands/` layer** — `tail.rs` (517 lines), `boxscore.rs`, `player_stats.rs`, `franchises.rs` have zero tests. Same for `standings_documents/{wildcard,division,conference,league}.rs` (non-trivial grouping logic) and `document/elements/render.rs` (565 lines — highest-value single gap). Fixture infra already exists in `src/fixtures.rs`, so this is mostly labor.
2. **Split the >100-line functions:** `tui/mod.rs:96 run()` (210 lines), `tui/reducer.rs:59 reduce()` (136), `commands/boxscore.rs:99` (129), `commands/player_stats.rs:12 run()` (124).
3. **Replace `.expect("State type mismatch")` ×3** in `component_store.rs:40,53,66` and the `panic!` in `table.rs:166` clone with graceful handling or documented invariants.

### Not low-hanging (don't delegate blindly)

- **The per-frame deep document clone.** All three document widgets deep-clone the entire built document on every render tick because `DocumentView::new` takes ownership. The fix (`Rc`/borrowing) touches the `DocumentView` API and rendering lifetimes — worth doing, but it's an architecture change for you or a top-tier model, not fruit.
- The parallel grouping logic across the four standings-document variants is structurally similar but not identical; unifying it needs design judgment.

---

## What you're not asking, but I'd say anyway

1. **Your 90%-coverage rule is aspirational, not enforced.** CLAUDE.md mandates 90% coverage for new code, but the entire CLI `commands/` layer is untested and nothing in CI measures coverage. Either wire `cargo tarpaulin` into CI with a threshold (you already have a `/test-coverage` skill) or soften the rule — a rule that's written down but never checked trains everyone (including your agents) to ignore CLAUDE.md.

2. **CI is quietly weaker than it looks.** `-D warnings` in CI while `--all-targets` shows 28 warnings locally means the gate exists but the door is open. Same with `--lib`-only tests. Both are one-line fixes (Tier 2 #5) and I'd do them before any of the code cleanups — otherwise delegated cleanups can regress unnoticed.

3. **There's tooling rot around the edges.** `criterion` with `html_reports` is declared but there are no benchmarks, while a `/benchmark` skill exists that presumably can't run anything. You have ~30 skills and ~15 agent definitions for a project with 6 TODOs — some of that scaffolding is describing a project that doesn't exist. A 30-minute pass deleting or fixing dead skills would keep the agent ecosystem trustworthy, which matters more as you delegate more.

4. **The codebase has earned the boring compliment: it's maintained.** Docs match code, tests pass, TODOs are scarce, error handling is disciplined. The main real debt is performance-shaped (per-frame document clones, `Vec<Standing>` cloned out of `Arc` on each build) rather than correctness-shaped — invisible in a TUI at human timescales, so only worth fixing if you enjoy it or plan bigger documents.

5. **Growth-point watch:** `document/elements/mod.rs` (1703 lines) and `keys.rs` (1662) are the two files trending toward unmaintainable. Neither is urgent, but the next feature that touches them is the right moment to split, not after.

---

## Suggested delegation batch

If you want one command to hand to a cheaper model: Tier 1 items 1–7 plus Tier 2 item 5 (CI) as a single batch — all mechanical, all verifiable by `cargo clippy --features development --all-targets -- -D warnings && cargo test --features development`. Tier 2 items 1–3 as a second batch with the extraction spec above.
