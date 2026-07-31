# Size-rule compliance tickets — audit reference

Imported into Vikunja project 5 on 2026-07-31: #132 (CLAUDE.md align),
#134 (test-module moves), #133 (file splits, blocked by #134),
#135 (function decomposition). Kept as the detailed audit reference the
tickets point to.

Decisions recorded 2026-07-30:
1. The **global** ~/.claude/CLAUDE.md rule governs: functions ≤~50 lines,
   files ≤~500 lines, justified exceptions OK. (Project CLAUDE.md's
   "under 100 lines" is superseded.)
2. `src/fixtures.rs` mock-data builders are a justified exception (literal
   struct data, dev-only).
3. Inline `#[cfg(test)] mod tests` blocks move to sibling test files.

---

## Ticket 1 — Align project CLAUDE.md size rules with global standard (priority 2)

The project CLAUDE.md says "Functions under 100 lines when possible", which
contradicts the global rule (functions ≤~50, files ≤~500) and is what the
codebase was written to. Update the Requirements section to the global
numbers and record the agreed exceptions: `src/fixtures.rs` builders, and
match-over-all-variants dispatchers near the limit (e.g. the DocumentElement
`Debug` impl, `key_to_action`) where splitting fights the enum shape.

## Ticket 2 — Move inline test modules to sibling files (priority 1)

25 of the 31 files exceeding 500 lines are pushed over by inline
`#[cfg(test)] mod tests` blocks (e.g. `src/tui/document/handlers.rs`: 59
production lines in a 1,269-line file). Move test modules to sibling files
via `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;` (or the
`keys/tests.rs` precedent already in-repo: a separate file declared with
`#[cfg(test)] mod tests;` from `keys/mod.rs`). Constraints: tests must stay
in the lib crate so `cargo tarpaulin --lib` coverage and the CI floor are
unaffected; `cargo test --features development` count must not change.
Worst offenders by test-block size: elements/render.rs (1241 test lines),
document/handlers.rs (1210), document/mod.rs (586), factory.rs (514),
tabbed_panel.rs, status_bar.rs, standings_documents/mod.rs, table/mod.rs,
standings_tab.rs.

## Ticket 3 — Split production files exceeding 500 lines (priority 1)

After Ticket 2, five files still exceed 500 lines of production code
(fixtures.rs is exempt per decision 2):
- src/tui/document/elements/render.rs (732) — split per element family
  (text/heading/link primitives vs table/boxscore composites vs clipping+row
  layout helpers)
- src/config.rs (903) — split theme definitions from config load/save/merge
- src/tui/document/mod.rs (605) — DocumentRenderCache + DocumentView could
  move to a sibling module, leaving trait + FocusContext
- src/tui/components/settings_tab.rs (560) — pairs with the update() split
  in Ticket 4
- src/commands/tail.rs (519)

## Ticket 4 — Decompose functions exceeding ~50 lines (priority 1)

66 production functions exceed 50 lines (91% compliant). Real decomposition
targets, worst first: settings_tab::update (135 — extract ActivateSetting /
modal handling), score_box::render (107), tui/mod.rs::run (106 — extract
event-loop body phases), commands/boxscore.rs::format_boxscore (103),
elements/behavior.rs::collect_focusable (108), render_team_boxscore (100),
then the 51–100 tail (~48 functions; see audit). Justified-exception
candidates to leave alone (document in code or CLAUDE.md rather than split):
fixtures.rs builders, DocumentElement Debug fmt (101), keys/dispatch.rs::
key_to_action (97) and similar exhaustive dispatchers. Test helpers over 50
(handlers.rs::test_boxscore etc.) are fixture data — leave.
