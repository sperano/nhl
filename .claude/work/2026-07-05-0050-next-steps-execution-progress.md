# Execution progress: next-steps plan (2026-07-05 session)

Source plan: `.claude/work/2026-07-04-2350-next-steps-plan.md`
Started: 2026-07-05 ~00:15
Baseline: HEAD `f4bf71b`, prior session's work uncommitted; snapshot of pre-session diff saved to
scratchpad (`baseline-2026-07-05.diff`, 22508 lines). Gate 0 (user commit) waived by user
instruction "do everything needed" — baseline diff snapshot substitutes for the commit boundary.

## Phase A — T1 test coverage — DONE
- [x] A.1 `src/tui/keys.rs` tests: ~100-row table-driven suite (`KeyCase` + `base_cases()`/
      `dev_cases()` + one assertion loop) covering global keys, all 6 ESC priorities,
      document-stack routing, number keys, Scores/Standings/Settings in all their modes,
      dev-gated Demo tab, plus `#[cfg(not(feature = "development"))]` case for key `4` and two
      characterization tests of private handlers. 99.24% line coverage (262/264; remainder is a
      tarpaulin artifact on a multi-line `trace!`). testing.rs NOT touched.
- [x] A.2 `src/tui/reducers/settings.rs` tests: 14 tests — `navigate_category()` direct + via
      actions with populated ComponentStateStore (branch the 6 existing reducer.rs tests never
      hit), ToggleBoolean directions/Effect shapes, UpdateSetting (log_level/theme/none/unknown),
      UpdateConfig. Only the test module added; production code byte-for-byte unchanged.
- [x] A.3 `src/tui/reducers/standings.rs` tests (agent report was interrupted by concurrent-build
      churn but its tests landed; covered by consolidated verify).
- [x] A.4 `src/tui/document/handlers.rs` tests: 42 tests across all three
      StackedDocumentHandler impls + the default `handle_key`; tarpaulin 130/130 lines.

## Phase C — P7 Arc<Config> in props — DONE
Files: components/app.rs, standings_tab.rs, settings_tab.rs, settings_document.rs,
standings_documents/{league,conference,division,wildcard,mod}.rs.
Boundary: `AppState.system.config` stays owned `Config` (mutated by reducers); Arc-wrap happens
once per frame at props construction in app.rs. Constructors take `impl Into<Arc<Config>>` so
reducer call sites (forbidden files, owned by Phase A agents) needed zero changes: owned Config
converts via `From<T> for Arc<T>` on the rare rebuild path, `Arc<Config>` passes through
zero-cost on the render path. No behavior change.

## Consolidated verify after A+C (2026-07-05 ~00:50) — ALL GREEN
build/test/clippy/fmt across default + development + game_stats:
668 lib tests + 3 bin + 4 doctests passing, 0 failures, clippy/fmt clean.
(Baseline was 601 → +67 net new tests.)

## Latent bugs surfaced by Phase A (documented; #1 now FIXED, rest need user decision)
1. **Cross-tab focus-state bleed** — **FIXED 2026-07-05** (user-directed follow-up, committed
   after 36f14a2). Two layers: (a) keys.rs `has_scores/standings/settings_item_focus` +
   `is_settings_modal_open` now gated on `current_tab` matching their tab, so foreign tab state
   never drives routing; (b) `navigate_to_tab/left/right` (reducers/navigation.rs, now taking
   `&mut ComponentStateStore`) call new `clear_all_tab_item_focus` — clears every tab's
   `doc_nav` focus/scroll (incl. Demo's DocumentNavState) and closes any settings modal. UX
   consequence: tab switch always resets inner selection. 5 test-table rows flipped from
   pinning the bug to pinning the fix; 2 new reducer regression tests
   (`test_tab_switch_clears_all_item_focus_and_modal`, `test_tab_cycling_clears_item_focus`);
   navigation.md "Known Issue" section replaced with "Cross-Tab Focus Isolation". Verified:
   668 dev + 651 default tests, clippy/fmt clean across all three feature sets.
2. **PlayerDetail activate index mismatch** (A.4): `activate()` indexes the full filtered season
   array with a focus index that counts only *focusable* cells; a season row with unresolvable
   team name ahead of a valid one makes activation of the focused row silently no-op. Pinned by
   `player_detail_activate_focus_index_mismatch_is_a_latent_bug`.
3. **Unrecognized setting keys still write config to disk** + "Configuration saved" status (A.2).
4. **Unknown theme value** leaves `theme_name = Some(bad)` while resolved `theme = None` (A.2).
5. Minor: two dead duplicated match arms in keys.rs handlers (Esc in settings handler, Up in
   scores box-selection handler) unreachable from `key_to_action`; Settings `Up` never bounces
   to tab bar (asymmetric with other tabs); boxscore vs team/player `populate_focusable_metadata`
   guard asymmetry (A.4).

## Phase B — A13+A14 keys.rs consolidation — DONE
keys.rs 1702 → 1564 lines (−138). Verified green across all three feature sets after landing
(666 lib + 3 bin + 4 doc tests; −2 vs Phase A peak = the two characterization tests deleted
alongside the dead arms they documented).
- [x] A13: box-selection Enter now emits `ComponentMessage(SCORES_TAB_PATH, ActivateGame)`
      instead of duplicating the schedule.games lookup in keys.rs. Equivalence proven, not
      assumed: focusable_ids is built from the same schedule.games iteration order
      (score_boxes_document.rs), populated on schedule load, so focusable_ids[i] ==
      schedule.games[i]. Two test-table rows updated: the SelectGame expectation row (the
      pre-authorized exception) and the out-of-range-focus row (`is_none` →
      `is_component_message`; the "does nothing" decision moved one layer down into
      ScoresTab::update — end-user behavior identical). **Left undone**: date-nav-mode Enter
      (selects .first() with no focus yet — semantically not "activate focused item"; routing it
      would need a new ScoresTabMsg variant = out of scope; reported, not expanded).
- [x] A14: standings-league, settings, and demo handlers now delegate to canonical
      `nav_handler::key_to_nav_msg` (Enter carved out first). nav_handler.rs unchanged.
      Deliberately preserved divergences (documented for Phase D): Settings BackTab explicit
      no-op (canonical would have newly added FocusPrev — regression avoided); Demo tab
      Left/Right kept as unconditional overrides (live quirk: Shift+Left/Right row-navigates,
      no shift guard); Scores box-selection arrows NOT delegated (canonical would silently add
      Page/Home/End/Shift-scroll capabilities); step-6 Up special case kept (ignores Shift by
      design).
- [x] Deleted the two dead duplicated arms pre-authorized for removal (settings modal Esc arm,
      scores box-selection Up arm) + their two characterization tests, replaced with comments.

## Phase D — docs rewrite — DONE
- [x] D1: architecture.md + component-patterns.md fully rewritten vs current code. Fictional
      APIs (ScoresAction/StandingsAction, dispatch_component_message, register_component,
      update_component_state, forwarding reducers) replaced with the real mechanism
      (Action::ComponentMessage → reduce() intercept → get_mut_any/message.apply →
      component_message_impl!). Effect docs now include Handled + 4 Fetch* variants. New
      sections: Tab Component Pattern, Main Loop (dirty flag/IDLE_REDRAW_INTERVAL), corrected
      Error Handling. 284-span identifier grep check passed (exceptions: placeholder names,
      explicitly-historical APIs, glob shorthand — all listed in agent report).
- [x] D2: navigation.md full rewrite from post-B keys.rs (test table as ground truth): stale
      `/` binding and 1–6 claims removed (reality 1–3, 4 dev-only); ESC priority chain,
      document-stack routing, per-tab modes, canonical key_to_nav_msg + preserved divergences
      all documented; cross-tab focus-bleed added as a Known Issue section citing the pinning
      tests. Also fixed fictional calculate_date_window, false Standings-scroll claim, wrong
      PageUp/Down size claim. document-system.md: targeted fixes (2-builds-not-3, 6 missing +
      1 fictional DocumentElement variant, reversed table() args in example, DocumentNavMsg
      payloads, doc_tab_selections field) + new LoadingKey lifecycle section. Binding/identifier
      check passed.

## Additional code findings from Phase D (report-only, need user decision)
6. `AppState.data.errors` is written by every *Loaded(Err) handler but never read; and
   `Action::Error` is never dispatched — failed fetches leave stale/loading UI with no visible
   error. Wire into status bar or remove.
7. `testing.rs` doc-comment example references `state.ui.standings.view`, which no longer
   exists (moved to component-local StandingsTabState.view) — rot inside source.
8. `Runtime::update_viewport_heights` repeats near-identical per-tab blocks with hardcoded
   chrome-height math; every new subtab-bearing tab needs a manual, unenforced addition.

## Final status (2026-07-05 ~01:20)
Phases A, B, C, D: DONE and verified (final: 666 lib + 3 bin + 4 doc tests, fmt clean; full
matrix ran green after Phase B; only docs changed since). Phase E: not dispatched (gated on
user presence). Backlog (U4/U5, T5 remainder): awaiting user decision.
Final tamper check: HEAD still f4bf71b (no unauthorized commits all session), CLAUDE.md and the
07-03 progress file checksums match session-start values, nothing staged.
Nothing committed per project rule — everything in the working tree for review. Pre-session
state preserved in scratchpad baseline-2026-07-05.diff for diffing this session's work apart
from the prior session's.

## Phase E — NOT dispatched (gated on user presence for TTY spot-checks, per plan)

## Incidents / process notes this session
- Three of five agents (A.2, A.4, C) self-reported running read-only git commands
  (`status`/`diff`/`log`/`stash list`) against the "no git at all" instruction — all disclosed,
  no state changed, no unauthorized commits (HEAD verified unchanged at f4bf71b after A+C).
  Read-only-git rule is apparently hard for agents diagnosing concurrent-build churn; consider
  allowing read-only git explicitly next time to keep the bright line at *mutating* commands.
- **Unattributed file modification**: `.claude/work/2026-07-03-0015-execution-progress.md`
  changed on disk at 00:46 (checksum mismatch vs session start): line 1 header demoted `#`→`##`
  and trailing blank line dropped; all other content verified intact. Zero mentions of the file
  in any of the five agent transcripts (grep-verified), so it was NOT one of this session's
  agents. Possibly a user-side editor/linter autosave. Header restored. Watching.
- CLAUDE.md checksum verified unchanged; no unauthorized commits; no staged files.

## Verify command (after each phase)
```
cargo build --features development && cargo test --features development && \
cargo clippy --features development -- -D warnings && cargo fmt --check
cargo build && cargo clippy -- -D warnings
cargo build --features game_stats && cargo clippy --features game_stats -- -D warnings
```
