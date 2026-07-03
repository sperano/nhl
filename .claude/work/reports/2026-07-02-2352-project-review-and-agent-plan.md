# nhl-dev Project Review & Agent Improvement Plan

**Date:** 2026-07-02 23:52
**Scope:** Full review across architecture, performance, UX, Rust idiomaticity, testing, and repo hygiene. Five parallel review agents examined the codebase (~32.5k lines of Rust); every finding below carries file references from actual code reads, not guesses. Cross-referenced against prior analyses in `.claude/work/reports/` (Nov 2025) and `TODO.md`.

---

## Executive summary

The codebase is in better shape than most TUI projects of this size: clippy is nearly clean, the no-panic/anyhow discipline is well respected, `Arc`-based state with `std::mem::take` dispatch is genuinely good, 578 tests pass fast, and the intended unidirectional data flow (key → ComponentMessage → `message.apply` → component update → Effect) is sound and clean **in its live path**.

The problems cluster into five themes:

1. **CRITICAL: No version control.** `.git` is an orphaned worktree pointer to `/Users/eric/code/nhl/.git/worktrees/nhl-dev`, which no longer exists. Every git command fails. Nothing else on this list matters until this is fixed.
2. **The performance architecture exists but is disconnected.** Renderer tree-diffing, `DocumentView.full_buffer` caching, `should_update` memoization — all built, none wired. Result: the whole UI is rebuilt ~10×/sec while idle, and each visible document is built 3× per frame.
3. **The UI promises things the code doesn't do.** Auto-refresh never fires (the status bar counts down and lies), `/` is a dead key, `time_format` is never read, some Settings rows are inert, `scores` renders a fake period grid, `config.toml.example` documents a schema that doesn't exist.
4. **Migration residue.** A large layer of dead code from the panel→document refactor (dead render path, dead action variants, dead message variants, unused traits) plus docs frozen at the pre-migration design. One genuinely fragile subsystem survives: the manually-synced focusable-metadata cache, duplicated across ~6 sites, which already needed a regression test.
5. **CI is red and has a blind spot.** `fmt` and `clippy -D warnings` both fail today; CI tests default features only, which is exactly why the `game_stats` feature bit-rotted into a compile error.

---

## 0. Critical / blocking

| # | Finding | Evidence |
|---|---------|----------|
| 0.1 | **Git is broken — orphaned worktree.** `.git` contains `gitdir: /Users/eric/code/nhl/.git/worktrees/nhl-dev`; that path is gone. `git status` → `fatal: not a git repository`. No history, no tracking, no branches. | Verified directly, not just by agent |
| 0.2 | **CI would be red** even if git worked: `cargo fmt --check` fails (`src/commands/player_stats.rs:58,93`); `cargo clippy -- -D warnings` fails on 2 `too_many_arguments` (`src/tui/widgets/big_score.rs:78`, `src/tui/document/elements/mod.rs:1034`). | `.github/workflows/ci.yml` |
| 0.3 | **`game_stats` feature doesn't compile** (`cannot find value BOXSCORE_STAT_BAR_WIDTH` via `src/commands/boxscore.rs`). CI never builds it, so it rotted silently. | `cargo check --features game_stats` |

**Git recovery options** (user decision required — do not do this automatically):
- If `/Users/eric/code/nhl` was moved: restore it or edit `.git` to point at the new location, then `git worktree repair`.
- If the parent repo is gone for good: `rm .git && git init`, re-add the GitHub remote if one exists, and make a fresh initial commit. History is lost locally but may survive on the remote.

---

## 1. Performance (theme: caching built but never connected)

The app is not laggy at NHL data sizes, but burns CPU/battery continuously and scales O(document size) per frame.

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| P1 | HIGH | No dirty flag: full element tree rebuilt + redrawn every loop iteration, ~10 Hz while completely idle (20 Hz while animating). ratatui's back-buffer suppresses flicker but not the build cost. | `src/tui/mod.rs:119-228` |
| P2 | HIGH | `Renderer::new()` inside the draw closure every frame → `previous_tree` always `None` → tree diffing is dead code. | `src/tui/mod.rs:154`, `renderer.rs:18,35-46` |
| P3 | HIGH | Even with P2 fixed, diffing can never skip: `trees_equal` returns `false` for any widget pair ("conservative: assume widgets are always different") and every leaf is a widget. | `renderer.rs:211-215` |
| P4 | HIGH | Every visible document built **3× per frame** (calculate_height → FocusManager build → render_full), because every widget constructs a fresh `DocumentView` on each render. The `full_buffer`/`cached_height`/`invalidate_cache` fields designed to prevent this are dead. | `document/mod.rs:172-177,336,340,535`; widgets at `document/widget.rs:66`, `scores_tab.rs:364`, `standings_documents/mod.rs:109`, `boxscore_document.rs:415`, etc. |
| P5 | HIGH | Full-document-height `Buffer` allocated + per-cell cloned into the viewport every frame. | `document/mod.rs:228,556-577` |
| P6 | MED | All tabs' contents built every frame; `TabbedPanel::view` keeps one, discards the rest. Standings content deep-clones the full standings vec + `Config` even when not visible. | `components/app.rs:184-186`, `tabbed_panel.rs:56-61`, `standings_tab.rs:310-367` |
| P7 | MED | `TabbedPanel` deep-clones the active `Element` subtree each frame. | `tabbed_panel.rs:60` |
| P8 | MED | Focusable-metadata extraction rebuilds the document 4× on data load and once per navigation keypress. | `document/mod.rs:183-220,281` |
| P9 | LOW | `club_stats_season` uncached → team detail always costs 2 sequential round-trips. | `effects.rs:92` |
| P10 | LOW | criterion is a dev-dep but there is no `benches/` dir — `cargo bench` is a no-op. | `Cargo.toml` |

**Already good (don't touch):** `std::mem::take` dispatch (`runtime.rs:85,108`), `Arc` data state with `Arc::make_mut` (`state.rs:94-100`), concurrent startup fetches via `Effect::Batch`, cache TTLs in `src/cache.rs` are sensible, no blocking I/O on the runtime.

**Recommended order:** P1 (dirty flag — biggest win, least code) → P4+P5 (persist `DocumentView`/`full_buffer` in component state) → P2+P3 (persist `Renderer` + widget equality hook, or delete the diffing machinery) → P6/P7 (only build active tab; `Arc<Config>`) → P8/P9 → P10 (benches to lock in gains).

---

## 2. Architecture (theme: sound core, migration residue, one fragile subsystem)

### Dead code (delete — low risk, high clarity; several hundred lines)

| # | Finding | Location |
|---|---------|----------|
| A1 | `Element::Component` + `ComponentWrapper` trait: only constructed in a renderer unit test. | `component.rs:191-200`, `renderer.rs:192,279,517` |
| A2 | `Component::should_update`/`did_update`: defined, documented as React memoization, never called. | `component.rs:42-49` |
| A3 | `Action::NavigateUp` + `navigate_up()`: never produced by `key_to_action`; ESC ladder reimplements it in `keys.rs`. Test-only. | `navigation.rs:24,124-151`, `action.rs:41-48` |
| A4 | `Action::FocusNext`/`FocusPrevious`: unreferenced. | `action.rs:71-72` |
| A5 | `ScoresTabMsg::Key`/`StandingsTabMsg::Key` + `handle_key()` methods: never constructed; unreachable (~70 lines). | `scores_tab.rs:55,293-331`, `standings_tab.rs:61,209-236` |
| A6 | Entire `App::view` → `render_main_tabs_without_states` path (~130 lines) is test-only; production uses `_with_states`. Near-duplicate of the live path. | `app.rs:36-45,62-135` |
| A7 | `StandingsTab.view` document-stack branch unreachable (stack always empty there); includes a "(Document rendering not yet implemented)" placeholder widget. | `standings_tab.rs:194-201,375-456` |
| A8 | `LoadingKey`/`DataState.loading`: nothing inserts in production — `loading` props always false, "needs animation" check always false. Either wire inserts at fetch time or delete. | `state.rs:103-117`, `app.rs:231-283`, `mod.rs:203` |

### Coupling / structural

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| A9 | HIGH | **Focusable-metadata cache is manually synced at ~6 sites** (standings, schedule, demo×2, settings×2) and is where autoscroll regressions live (regression test exists at `standings_tab.rs:804-835`). Stacked documents already solve this cleanly on-demand via `populate_focusable_metadata`. Two competing patterns for one concern. This matches `TODO.md`'s own "FocusableId vs LinkTarget is redundant" and "review document_nav" items. | `reducers/data_loading.rs:73-172`, `reducers/settings.rs:28-66`, `reducers/standings.rs`, `document/mod.rs:255-292` |
| A10 | HIGH | `Runtime::dispatch` bypasses the reducer: `RefreshSchedule` directly mutates `game_date` and clears schedule state, splitting transition logic across two layers. | `runtime.rs:81-114` |
| A11 | MED | `game_date` dual source of truth (global `ui.scores` + `ScoresTabState`), synced by hand. | `state.rs:135-146`, `scores_tab.rs:23,135-163` |
| A12 | MED | Settings split-brained: `selected_category` in global state via `SettingsAction` while everything else is a ComponentMessage; two near-identical 25-line reducer arms. | `state.rs:149-151`, `reducers/settings.rs:17-69` |
| A13 | MED | `keys.rs` couples input layer to data model (indexes `schedule.games` to synthesize `SelectGame`), duplicating logic the component already has (`ScoresTabMsg::ActivateGame`). | `keys.rs:188-224`, `scores_tab.rs:174-195` |
| A14 | MED | Key→nav mapping duplicated 4× in live handlers (~200 lines) while the canonical `nav_handler::key_to_nav_msg` is used only by dead code and the stacked-doc handler. | `keys.rs:233-461`, `nav_handler.rs:41-96` |
| A15 | LOW | Two parallel focus models: `FocusManager` (stacked docs) vs `DocumentNavState` (tabs). Rendering-side twin of A9. | `document/mod.rs:339-341` |
| A16 | LOW | Large files worth splitting: `document/elements/mod.rs` (1698), `document/mod.rs` (1013), `table/mod.rs` (897), `renderer.rs` (743). | — |

### Documentation drift (docs describe the pre-migration design)

| # | Finding |
|---|---------|
| A17 | `architecture.md`/`component-patterns.md` describe `ScoresAction`/`StandingsAction` nested actions and forwarder reducers that no longer exist; reality is `Action::ComponentMessage { path, message }` + `ComponentMessageTrait::apply`. |
| A18 | Documented Runtime API (`dispatch_component_message`, `update_component_state`, `with_component_state`) doesn't exist; real mechanism is `ComponentStateStore`. |
| A19 | `Effect` enum in docs omits `Handled` + four `Fetch*` variants — core control flow undocumented. |
| A20 | `navigation.md` documents `/` command palette and keys 1-6; only 1-4 exist and `/` is a stub. `architecture.md` module listing includes `reducers/scores.rs` (doesn't exist), omits `reducers/settings.rs`. Note: project `CLAUDE.md` also says `reducers/` while describing key files — verify against actual layout when rewriting. |

---

## 3. User experience (theme: promised behavior not delivered)

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| U1 | HIGH | **TUI auto-refresh never happens.** `RefreshData` dispatched once at startup; `Tick` only advances `animation_frame`. Status bar renders a live countdown → "Refreshing..." that never refreshes. Live scores are frozen, contradicting README's headline feature. | `mod.rs:113`, `reducer.rs:144-149`, `status_bar.rs:58-73` |
| U2 | HIGH | `config.toml.example` documents a `[theme]` table with `selection_fg` keys that don't exist; real schema is `[display]`. Copying the example yields silently-ignored keys. README's config block is the correct one. | `config.toml.example:17-27` vs `config.rs:364-378` |
| U3 | MED | `/` mapped to `ToggleCommandPalette`; reducer arm is a no-op. Dead key, discoverability trap. | `keys.rs:63`, `reducers/navigation.rs:25` |
| U4 | MED | No help overlay or key hints anywhere, despite a large modal keymap (7-level ESC ladder, mode-dependent arrows). No mode indicator either. | `keys.rs:69-133`, `status_bar.rs` |
| U5 | MED | No vim keys (`h/j/k/l`, `g/G`) in a keyboard-centric TUI. | `keys.rs` |
| U6 | MED | Inert Settings rows: `refresh_interval`, `log_file`, `time_format` render as focusable links but Enter hits the `_ => Effect::None` arm — no feedback. | `settings_tab.rs:179-208,504-517` |
| U7 | MED | `time_format` config is never read by anything. `schedule` hardcodes `%I:%M %p`; `scores` prints raw UTC (`...T00:00:00Z`). | `config.rs:27`, `schedule.rs:56`, `scores.rs:206` |
| U8 | MED | `scores` period-by-period grid is fake — always prints `-` placeholders. README advertises it. | `scores.rs:168-187` |
| U9 | MED | `scores` box alignment broken: borders at width 88, content lines hardcode different padding → ragged right edge (visible in mock output). Magic widths bypass the module's own named constants. | `scores.rs:86,189-212` |
| U10 | MED | `config` command output stale (self-flagged TODO): dangling `[theme]` header, `[display]` section missing entirely. | `main.rs:200-231` |
| U11 | MED | CLI `standings` hardcodes `western_first = false`, silently ignoring the config option the TUI honors. | `standings.rs:469` |
| U12 | MED | Malformed config silently falls back to defaults — a typo'd setting looks like it "didn't take." | `config.rs:836-841` |
| U13 | LOW | Missing conveniences: no `--json`, no `--no-color` (`tail` emits raw ANSI when piped), no team filter, no favorite-team config. `-F`(global log-file) vs `-f`(tail follow) fumble-prone. | various |
| U14 | LOW | No stale-data indicator ("as of HH:MM") — compounds U1. | — |

**Good:** `--help` is clean; error messages are actionable (`with_context` used well, `{:#}` flattening); ESC hierarchy is thoughtfully layered; unicode-width handling is robust.

---

## 4. Idiomatic Rust (theme: already strong; targeted fixes)

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| R1 | MED | `panic!` on the live render path ("Unresolved component in render tree"). A malformed tree crashes the TUI. Render an error placeholder instead. (Becomes moot if A1 removes `Element::Component`.) | `renderer.rs:194` |
| R2 | MED | `config::write()` returns `Box<dyn Error>` — the only non-anyhow boundary in `src/`. | `config.rs:854` |
| R3 | MED | Errors stringified at the effect boundary (`Result<_, String>` in `*Loaded` actions) — discards `NHLApiError` variant info (rate-limit vs network vs parse). | `reducers/data_loading.rs:61` |
| R4 | MED | `reduce_data_loading` takes `&Action` and clones the full payload (`Vec<Standing>`/`DailySchedule`/`Boxscore`) on every load; runtime already owns the Action. Take it by value. | `data_loading.rs:28-53` (prior report `dispatch-clone-optimization-plan-2025-11-29.md` covers adjacent ground) |
| R5 | MED | `format_wildcard_view` does 4 full filter+clone passes over the league plus a `to_vec()`. Single grouping pass instead. | `commands/standings.rs:362-429` |
| R6 | MED | `config.rs` spells `ratatui::style::Style` fully-qualified 26× (+ `HashMap` etc.) — violates the project's own imports rule. | `config.rs` |
| R7 | MED | Log-level precedence bug: "was the flag set?" inferred by comparing against the default, so an explicit `-L info` is treated as unset and config wins. Use `Option<String>` clap args. | `main.rs:235-249` |
| R8 | LOW | Cluster of small items: `unwrap()` guarded by `is_none()` (use let-else, `main.rs:327`); `unreachable!` arms as routing smell (`main.rs:265`); `&i32` params (`scores.rs:214`); useless `format!("{}", x)` (`cache.rs:144`); `src/types.rs` is one dead `#[allow(dead_code)]` enum; near-duplicate cfg blocks in `keys.rs:536-558`. | various |
| R9 | LOW | Deps: bump `unicode-width` 0.1→0.2 (Cargo.lock currently resolves BOTH — duplicate compile), `dirs` 5→6, optionally `toml`/`cached`. **Keep** `crossterm 0.28.1` (exactly what ratatui 0.29 wants), **keep** `async-trait` (`dyn NHLDataProvider` needs it — native AFIT has no dyn support), **keep** `cached` (load-bearing in `src/cache.rs`). Edition 2021 is fine. | `Cargo.toml` |

---

## 5. Testing & repo hygiene

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| T1 | HIGH | Zero-test core modules: `keys.rs` (581 lines — the entire input contract, trivially table-testable), `document/handlers.rs` (255), `reducers/settings.rs`, `reducers/standings.rs`, `data_provider.rs`, `commands/{tail,scores,player_stats}.rs`. | — |
| T2 | HIGH | CI blind spot: `test` job runs default features only; project mandates `--features development`; `game_stats` never built at all (which is why it broke). Add a feature matrix. | `.github/workflows/ci.yml` |
| T3 | MED | Assertion-free test `test_refresh_data_triggers_loading_state` passes unconditionally ("For now, we just verify the action was dispatched" — nothing is checked). Sleep-based waits (`sleep(100ms)`) are a flakiness risk. | `integration_tests.rs` |
| T4 | MED | Two parallel stale-doc trees at root: `old-mds/` (29 files of completed migration plans) and `new-doc/` (8 more). Neither is authoritative; newcomers can't tell them from `docs/`. Archive to `docs/archive/` or delete. `TODO.md` is scratch notes, untracked by `.gitignore`. | repo root |
| T5 | LOW | README lacks build/install, testing, `--features development` requirement, mock mode, and a `docs/` pointer. No pre-commit hooks despite fmt/clippy being red. No `rustfmt.toml`. | `README.md` |
| T6 | — | **Good:** 578 tests pass in ~0.14s; `assert_buffer` used in 17 files per project rule; `fixtures.rs` (1315 lines) is a well-gated single source of truth serving tests + mock mode; `test_config_to_toml` correctly guards the struct↔TOML mapping (it does NOT guard `config.toml.example` — consider a test that parses the example file). | — |

---

## 6. Prioritized roadmap

**Phase 0 — Unblock (user decision + trivial fixes)**
1. Restore git (0.1) — user decides recovery strategy.
2. Green CI: `cargo fmt`; fix 2 clippy `too_many_arguments` (params struct or scoped allow); fix or remove `game_stats`; add `--features development` + feature matrix to CI (0.2, 0.3, T2).

**Phase 1 — User-visible correctness (highest value per line changed)**
3. Wire auto-refresh (U1) + stale-data indicator (U14).
4. Regenerate `config.toml.example` + fix `config` command + warn on malformed config (U2, U10, U12); add an example-parses test.
5. Fix `scores` alignment + real or removed period grid; unify time handling via `time_format` (U7-U9); honor `western_first` in CLI (U11).
6. Resolve `/` dead key; make inert Settings rows work or non-focusable (U3, U6).

**Phase 2 — Dead code purge (mechanical, enables everything after)**
7. Delete A1-A8 (with A8 decision: wire loading inserts or delete `LoadingKey`). Renderer panic R1 likely falls out with A1.

**Phase 3 — Performance rewiring (the big architectural win)**
8. Dirty flag (P1); persist `DocumentView`/`full_buffer` in component state (P4, P5); persist `Renderer` + widget equality or delete diffing (P2, P3); active-tab-only builds + `Arc<Config>` (P6, P7). Add criterion benches first to measure (P10).

**Phase 4 — Structural consolidation**
9. Unify focus metadata on on-demand `populate_focusable_metadata`; delete the manual cache + rebuild sites (A9, A15, P8) — also resolves TODO.md items.
10. Move `RefreshSchedule` mutation into reducer (A10); single `game_date` owner (A11); settings via ComponentMessage (A12); route live key handlers through `key_to_nav_msg` (A13, A14).

**Phase 5 — Idiomatic + polish**
11. R2-R9 cluster; typed errors in actions (R3) pairs well with Phase 4.

**Phase 6 — Tests, docs, hygiene**
12. Tests for `keys.rs`, reducers, handlers (T1); fix assertion-free test (T3).
13. Rewrite `docs/` to match reality (A17-A20); fix project CLAUDE.md's `reducers/` reference; archive `old-mds/`/`new-doc/` (T4); README expansion + help overlay (U4) + pre-commit hooks (T5).

Sequencing rationale: dead-code deletion (Phase 2) before performance (Phase 3) so you're not optimizing code about to be deleted; performance before focus-model consolidation (Phase 4) because persisting `DocumentView` changes where focus state naturally lives; docs last because every earlier phase changes what the docs should say.

---

## 7. Agent plan

Rough agent definitions to execute the roadmap. Model choice logic: **Haiku** for mechanical, well-specified, low-blast-radius work; **Sonnet** for standard implementation with clear specs and good test feedback loops; **Opus** for work where a plausible-but-wrong change is likely and expensive (frame-loop caching semantics, focus-model unification, reducer-boundary moves). Existing project agents (`rust-code-writer`, `idiomatic-rust`, `integration-tester`, `code-simplifier`, `rust-documenter`) can serve as several of these with a task-specific prompt.

Each implementation agent should be paired with a verify step: `cargo build --features development && cargo test --features development && cargo clippy --features development -- -D warnings && cargo fmt --check`, plus a mock-mode smoke run for UX-touching changes. Run agents **sequentially per phase** (they touch overlapping files); parallelize only within a phase when file sets are disjoint.

### Phase 0
| Agent | Model | Mission |
|-------|-------|---------|
| `ci-greener` | **Haiku** | Run `cargo fmt`; fix the two `too_many_arguments` (introduce a `BigScoreParams` struct — preferred over `#[allow]` since CLAUDE.md favors real fixes); verify clippy clean. |
| `feature-matrix-fixer` | **Sonnet** | Fix `game_stats` compile error (missing `BOXSCORE_STAT_BAR_WIDTH` import); extend `ci.yml` with a feature matrix (`default`, `development`, `game_stats`); OR, if the user says the feature is abandoned, strip it. Needs a user decision first: repair vs remove. |

### Phase 1
| Agent | Model | Mission |
|-------|-------|---------|
| `auto-refresh-implementer` | **Sonnet** | Wire `Tick` → elapsed-vs-`refresh_interval` check → `RefreshData` dispatch in the main loop or reducer; add "data as of HH:MM" staleness cue to status bar; regression test that the countdown actually triggers a refresh (use the mock provider, no sleeps). Files: `mod.rs`, `reducer.rs`, `status_bar.rs`. |
| `config-truth-fixer` | **Haiku** | Regenerate `config.toml.example` from the real `Config`; fix `handle_config_command` output; add `tracing::warn!` on config parse failure; add a test that parses `config.toml.example` into `Config`; update `test_config_to_toml` if new attributes appear (project rule). |
| `cli-output-fixer` | **Sonnet** | `scores`: fix box alignment using the module's named constants; wire real linescore data into the period grid or remove it (ask user which); localize times and honor `time_format` across `scores`/`schedule`; honor `western_first` in CLI standings. Update README claims to match. |
| `settings-ux-fixer` | **Sonnet** | Remove `/`→`ToggleCommandPalette` binding + stub variant (or implement palette — user decision; removal is the cheap default); make `refresh_interval`/`log_file`/`time_format` Settings rows either editable or non-focusable. |

### Phase 2
| Agent | Model | Mission |
|-------|-------|---------|
| `dead-code-reaper` | **Sonnet** | Delete A1-A7 in one pass (dead variants, traits, `handle_key` methods, `_without_states` render path, placeholder widget); port the few tests that exercised dead paths to the live path (`build_with_component_states`). Replace `renderer.rs:194` panic if any trace remains. Large diff but mechanical; Sonnet + full test suite is sufficient. |
| `loading-state-decider` | **Opus** (small task, but semantic) | A8: decide-and-implement wiring `LoadingKey` inserts at fetch-dispatch time so loading animation works, vs deleting the machinery. Requires understanding effect lifecycle; a wrong call here reintroduces silent no-op state. Could also be Sonnet with an explicit spec if the user pre-decides direction. |

### Phase 3
| Agent | Model | Mission |
|-------|-------|---------|
| `bench-baseliner` | **Haiku** | Add `benches/render.rs` (criterion): `runtime.build()` + full `DocumentView` render for standings and boxscore docs from fixtures. Record baseline numbers in a report. |
| `frame-loop-surgeon` | **Opus** | P1+P2+P3: dirty-flag the main loop (set on action/key/tick/resize); hoist `Renderer` so `previous_tree` persists; give widgets an equality/content-hash hook or excise the diffing machinery. This is the exact "plausible-but-wrong is expensive" territory: a missed dirty-set means frozen UI, an over-eager skip means stale rendering. Must verify with mock-mode interaction, not just unit tests. |
| `document-view-cacher` | **Opus** | P4+P5+P8: persist `DocumentView` (built elements + FocusManager + `full_buffer`) in component state; invalidate on data/width/focus change; collapse the 3 builds into 1; single-pass focusable metadata extraction. Interacts with Phase 4's focus model — same agent or tightly sequenced. |
| `clone-trimmer` | **Sonnet** | P6+P7+R4+R5+M-class clones: active-tab-only builds, `Arc<Config>` in props, pass standings `Arc` through instead of `to_vec()`, `Action` by value in `reduce_data_loading`, single-pass wildcard grouping. Re-run benches; report deltas vs baseline. |

### Phase 4
| Agent | Model | Mission |
|-------|-------|---------|
| `focus-model-unifier` | **Opus** | A9+A15: standardize tabs on on-demand `populate_focusable_metadata`; delete `rebuild_standings_focusable_metadata`, the demo/schedule/settings rebuild blocks, `RebuildStandingsFocusable`; keep the autoscroll regression test green. Also address TODO.md's FocusableId/LinkTarget redundancy (string links → enum). The single riskiest refactor in the plan — fragile subsystem, prior regressions. |
| `state-boundary-cleaner` | **Sonnet** | A10-A14: move `RefreshSchedule` mutation into a reducer arm; single-owner `game_date`; `selected_category` → ComponentMessage; key handlers emit intents and route through `key_to_nav_msg`; dedupe the cfg'd Up-key blocks. Well-specified moves along existing patterns. |

### Phase 5
| Agent | Model | Mission |
|-------|-------|---------|
| `idiom-polisher` | **Haiku** | R2 (anyhow in `config::write`), R6 (imports in config.rs), R8 cluster (let-else, `&i32`→`i32`, useless `format!`, delete `src/types.rs`), R9 dep bumps (`unicode-width` 0.2, `dirs` 6 — keep crossterm/cached/async-trait). All mechanical with exact locations known. |
| `typed-error-threader` | **Sonnet** | R3+R7: carry `Arc<NHLApiError>` (or a local error enum) through `*Loaded` actions instead of `String`; fix log-level precedence with `Option<String>` clap args. Touches the action layer broadly but the pattern is uniform. |

### Phase 6
| Agent | Model | Mission |
|-------|-------|---------|
| `keymap-tester` | **Sonnet** | T1 priority slice: table-driven tests for `keys.rs` (key+mode → Action) — pure-function testing, no rendering; then `reducers/settings.rs`, `reducers/standings.rs`, `document/handlers.rs`. Target the project's 90% rule for these modules. |
| `test-honesty-fixer` | **Haiku** | T3: give `test_refresh_data_triggers_loading_state` real assertions (or delete); replace `sleep(100ms)` waits with deterministic effect-driving; audit sibling integration tests for the same pattern. |
| `docs-rewriter` | **Sonnet** | A17-A20: rewrite `architecture.md` (ComponentMessage flow, real Effect enum, real Runtime/store API, correct module list), `navigation.md` (real ESC ladder, keys 1-4), `component-patterns.md`; fix CLAUDE.md's `reducers/` reference. Run AFTER phases 2-4 so it documents the end state, not the current one. `rust-documenter` agent fits. |
| `hygiene-janitor` | **Haiku** | T4+T5: move `old-mds/` + `new-doc/` → `docs/archive/` (or delete — user decision); README build/test/dev/mock/docs sections; `.gitignore` check once git works; optional pre-commit hook + `rustfmt.toml`. No git add/commit per project rule — file moves only. |
| `help-overlay-builder` | **Sonnet** | U4+U5: `?` help overlay listing contextual keybindings; mode indicator in status bar; optionally `j/k/g/G` vim aliases in `key_to_nav_msg` (single mapping site after Phase 4). Component work following documented patterns; `assert_buffer` tests. |

### User decisions needed before dispatch
1. **Git recovery**: repair worktree pointer vs fresh `git init` (Phase 0 gate — everything else should land as reviewable commits).
2. **`game_stats`**: repair or remove.
3. **Period grid in `scores`**: wire real linescore data or drop the grid.
4. **`/` command palette**: implement or remove binding.
5. **`old-mds/`/`new-doc/`**: archive or delete.
6. **`LoadingKey`**: wire it up (working loading animations) or delete.

### Cost/shape estimate
- Haiku agents: 5 (mechanical, ~minutes each)
- Sonnet agents: 9 (bulk of the implementation)
- Opus agents: 4 (frame-loop, document caching, loading-state semantics, focus-model unification — the places a subtle mistake costs a day of debugging)
- Suggested cadence: one phase per session, `integration-tester` agent after each phase, benches re-run after Phase 3.
