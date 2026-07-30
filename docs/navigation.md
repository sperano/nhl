# Navigation Behavior Specification

This document specifies keyboard navigation as implemented in `src/tui/keys.rs`
(`key_to_action`) and `src/tui/nav_handler.rs` (`key_to_nav_msg`). The
`#[cfg(test)] mod tests` block in `keys.rs` (`base_cases()` / `dev_cases()`,
~100 table-driven cases) is the executable ground truth this document is kept
in sync with; every binding below corresponds to a row in that table or an
arm in the functions it exercises.

## Key Event Flow

```
KeyEvent -> key_to_action(key, state, component_states) -> Option<Action>
```

`key_to_action` resolves a key in this order (matching the numbered steps in
its own source comments):

1. **Global keys** (`handle_global_keys`): `q` / `Q` -> `Action::Quit`,
   checked before anything else - it fires even with a document open or
   content focused.
2. **ESC** (`handle_esc_key`): a fixed priority chain, see below.
3. **Document stack routing**: if `state.navigation.document_stack` is
   non-empty, every remaining key (including number keys and `Enter`) becomes
   `Action::StackedDocumentKey(key)` and is handled by
   `document::handle_stacked_document_key` instead of anything below.
4. **Number keys** (`handle_number_keys`): direct tab switching, only
   reachable when the document stack is empty.
5. **Tab bar vs. content focus**: if `!state.navigation.focus_in_content`,
   delegate to `handle_tab_bar_navigation`.
6. **Up-key special case**: if the key is `Up` and content is focused, handle
   it here for nested modes that need bespoke Up behavior (see below) before
   falling through to step 7.
7. **Tab-specific handlers**: dispatch on `state.navigation.current_tab` to
   `handle_scores_tab_keys` / `handle_standings_tab_keys` (or
   `handle_standings_league_keys`) / `handle_settings_tab_keys` /
   `handle_demo_tab_keys` (development only).

There is no `/` command-palette binding - it was removed; `handle_global_keys`
only matches `q`/`Q`.

## Global Keys

- **`q` / `Q`**: `Action::Quit`, unconditionally - from the tab bar, with
  content focused, with box-selection active, or with a document on the
  stack.

## ESC Priority Chain

`handle_esc_key` checks these conditions in order and returns on the first
match:

1. **Document stack non-empty** -> `Action::PopDocument`.
2. **Settings modal open** (`is_settings_modal_open`) ->
   `SettingsTabMsg::Modal(ModalMsg::Cancel)`.
3. **Scores tab has item focus** (`has_scores_item_focus`) ->
   `ScoresTabMsg::ExitBoxSelection`.
4. **Standings tab has item focus** (`has_standings_item_focus`) ->
   `StandingsTabMsg::ExitBrowseMode`.
5. **Settings tab has item focus** (`has_settings_item_focus`) ->
   `SettingsTabMsg::NavigateUp` (priority "4.5" in the source comments).
6. **Demo tab has content focus** (`state.navigation.focus_in_content` while
   `current_tab == Tab::Demo`, development feature only) ->
   `DemoTabMsg::ExitFocus` (priority "4.6").
7. **Content is focused** (`state.navigation.focus_in_content`) ->
   `Action::ExitContentFocus` (returns to the tab bar).
8. **Otherwise** (at the tab bar, nothing else focused): no-op - use `q` to
   quit.

Note: every `has_*_item_focus` / modal check is gated on
`state.navigation.current_tab` matching the tab it belongs to, so only the
current tab's focus state can drive ESC routing. Checks 3, 4, and 5 are
therefore mutually exclusive; the priority ordering only ever arbitrates
between same-tab states (e.g. the Settings modal at priority 2 beating
Settings item focus at priority 5). See "Cross-Tab Focus Isolation" at the
end of this file.

## Document Stack Routing

While `state.navigation.document_stack` is non-empty, `key_to_action` returns
`Action::StackedDocumentKey(key)` for anything that isn't `q`/`Q` or `Esc`
(ESC's priority-1 check pops the document instead of routing the key). This
means:

- Number keys (`1`-`4`) do **not** switch tabs while a document is open - they
  are forwarded to the document instead.
- `[` and `]` are intercepted by `reduce_document_stack::stacked_document_key`
  before the generic handler: when the top document is a `TeamDetail` with a
  resolved season, they cycle to the previous/next season in the team's
  regular-season list (clamped at both ends, no wrap), resetting the
  document's focus and scroll and fetching that season's roster if it isn't
  cached yet. On any other document (or before the first roster load
  resolves the season) they are no-ops.
- `Enter` and all arrow/paging keys are forwarded to
  `document::handle_stacked_document_key` (see `docs/document-system.md`), not
  to any tab-specific handler.

## Tab Switching

### Number keys (`handle_number_keys`)

Only reachable when the document stack is empty (step 4).

| Key | Action |
|-----|--------|
| `1` | `Action::NavigateTab(Tab::Scores)` |
| `2` | `Action::NavigateTab(Tab::Standings)` |
| `3` | `Action::NavigateTab(Tab::Settings)` |
| `4` | `Action::NavigateTab(Tab::Demo)` - only compiled in with `--features development` |

Number-key tab switching ignores modifiers (`Shift+1` still switches tabs) and
works regardless of current focus level. Without the `development` feature,
`4` is unmapped and falls through to whatever the current focus-level handler
does with an unrecognized character (a no-op from the tab bar).

### Tab bar arrows (`handle_tab_bar_navigation`, focus not in content)

| Key | Action |
|-----|--------|
| `Left` | `Action::NavigateTabLeft` (cycles Scores -> Demo -> Settings -> Standings -> Scores in dev builds; Scores -> Settings -> Standings -> Scores otherwise) |
| `Right` | `Action::NavigateTabRight` (reverse order) |
| `Down` / `Enter` | `Action::EnterContentFocus` |
| anything else (incl. `Up`) | no-op |

`NavigateTab`, `NavigateTabLeft`, and `NavigateTabRight` all clear
`document_stack` and reset `focus_in_content = false` (see
`src/tui/reducers/navigation.rs`). Entering content focus on the Demo tab
additionally sends `DemoTabMsg::EnterFocus` to focus its first item and sets a
tab-specific status-bar hint; leaving content focus on the Demo tab resets
that status message.

All three tab-switch reducers also clear every tab's item focus and any open
settings modal via `clear_all_tab_item_focus` (`ScoresTabState`,
`StandingsTabState`, `SettingsTabState`, and the Demo tab's `DocumentNavState`
all get `TabState::clear_item_focus()`; `SettingsTabState.modal` is set to
`None`). Switching tabs therefore always lands you at the tab bar with no
inner focus anywhere - see "Cross-Tab Focus Isolation" below.

## The Up-Key Special Case (step 6)

When content is focused and the key is `Up`, `key_to_action` checks nested
modes *before* falling through to the tab-specific handlers:

- **Demo tab** (development only), **Settings tab**, **Standings tab with
  item focus**, or **Scores tab with item focus (box-selection)**: this step
  does nothing and lets the tab-specific handler process `Up` itself (both
  plain and `Shift+Up`).
- **Otherwise**: `Action::ExitContentFocus` - `Up` returns to the tab bar.

The `has_scores_item_focus`/`has_standings_item_focus` checks here are gated
on the current tab actually being Scores/Standings, so they can only fire
while that tab is active.

Scores box-selection used to be special-cased directly in this step
(`key.code == KeyCode::Up` with no modifier check, so `Shift+Up` produced
`FocusPrev` instead of scrolling like every other browse mode). That
divergence was removed: Scores box-selection now joins the fall-through list
like the other three modes, and `handle_scores_tab_keys` handles `Up` itself
via the canonical mapping (plain `Up` -> `FocusPrev`, `Shift+Up` ->
`ScrollUp(1)`), matching Standings/Settings/Demo.

## The Canonical Document-Navigation Mapping (`key_to_nav_msg`)

`nav_handler::key_to_nav_msg` is the shared mapping used, with one remaining
documented exception (Settings `Left`/`Right`, claimed for category
navigation), by Standings browse mode, the Settings tab, the Demo tab, Scores
box-selection mode, and stacked documents
(`document::handle_stacked_document_key`). As of the F6 convergence, this is
the only surviving divergence from the canonical mapping across all of these
callers - see the per-tab sections below.

| Key | Without Shift | With Shift |
|-----|----------------|------------|
| `Tab` | `FocusNext` | `FocusPrev` |
| `BackTab` | `FocusPrev` | (n/a - BackTab carries no Shift state) |
| `Up` | `FocusPrev` | `ScrollUp(1)` |
| `Down` | `FocusNext` | `ScrollDown(1)` |
| `Left` | `FocusLeft` | `ScrollUp(1)` (not left - scrolling wins) |
| `Right` | `FocusRight` | `ScrollDown(1)` (not right - scrolling wins) |
| `PageUp` | `PageUp` | - |
| `PageDown` | `PageDown` | - |
| `Home` | `ScrollToTop` | - |
| `End` | `ScrollToBottom` | - |

`Enter` and `Esc` return `None` from `key_to_nav_msg` - both are handled
separately by each caller (activation and the global ESC chain,
respectively).

## Per-Tab Navigation

### Scores tab (`handle_scores_tab_keys`)

Two modes, distinguished by `has_scores_item_focus`:

**Date-navigation mode** (no item focus):

| Key | Result |
|-----|--------|
| `Left` | `ScoresTabMsg::NavigateLeft` |
| `Right` | `ScoresTabMsg::NavigateRight` |
| `Down` | `ScoresTabMsg::EnterBoxSelection` |
| `Enter` | `Action::SelectGame(id)` for the first game in `state.data.schedule`, or no-op if no schedule is loaded |
| `Up` | `Action::ExitContentFocus` (via the step-6 special case, since there's no nested mode) |
| anything else | no-op |

**Box-selection mode** (item focus active): `Enter` activates the focused
game (`ScoresTabMsg::ActivateGame`); every other key goes through the
canonical `key_to_nav_msg` mapping exactly, wrapped in `ScoresTabMsg::DocNav`
- full parity with Standings browse mode.

| Key | Result |
|-----|--------|
| `Up` | `ScoresTabMsg::DocNav(FocusPrev)` - reached via the step-6 fallthrough into this function, same as Standings/Settings/Demo |
| `Shift+Up` | `ScoresTabMsg::DocNav(ScrollUp(1))` |
| `Down` | `ScoresTabMsg::DocNav(FocusNext)` |
| `Shift+Down` | `ScoresTabMsg::DocNav(ScrollDown(1))` |
| `Left` | `ScoresTabMsg::DocNav(FocusLeft)` |
| `Shift+Left` | `ScoresTabMsg::DocNav(ScrollUp(1))` (not left - scrolling wins) |
| `Right` | `ScoresTabMsg::DocNav(FocusRight)` |
| `Shift+Right` | `ScoresTabMsg::DocNav(ScrollDown(1))` (not right - scrolling wins) |
| `Tab` | `ScoresTabMsg::DocNav(FocusNext)` |
| `Shift+Tab` / `BackTab` | `ScoresTabMsg::DocNav(FocusPrev)` |
| `PageUp` / `PageDown` | `ScoresTabMsg::DocNav(PageUp/PageDown)` |
| `Home` / `End` | `ScoresTabMsg::DocNav(ScrollToTop/ScrollToBottom)` |
| `Enter` | `ScoresTabMsg::ActivateGame` - reads the destination straight off the focused element (`doc_nav.focused_link_target()`), pushing the boxscore document it carries; no focused link target is a no-op |
| `Esc` | `ScoresTabMsg::ExitBoxSelection` (ESC priority 3) |
| anything else | no-op |

Before the F6 convergence, Scores box-selection had no `Tab`/`BackTab`,
`PageUp`/`PageDown`, `Home`/`End`, or `Shift`-scroll support (only
`Up`/`Down`/`Left`/`Right`/`Enter` were wired), and `Up` was special-cased in
`key_to_action` step 6 to ignore Shift entirely (so `Shift+Up` produced
`FocusPrev` there instead of scrolling). Both divergences were removed: this
mode now matches the canonical mapping with no exceptions.

### Standings tab

Two modes, distinguished by `has_standings_item_focus`.

**View-selection mode** (`handle_standings_tab_keys`, no item focus):

| Key | Result |
|-----|--------|
| `Left` | `StandingsTabMsg::CycleViewLeft` |
| `Right` | `StandingsTabMsg::CycleViewRight` |
| `Down` | `StandingsTabMsg::EnterBrowseMode` |
| `Up` | `Action::ExitContentFocus` |
| anything else (incl. `PageUp`/`PageDown`/`Home`/`End`) | no-op |

Manual scrolling (`PageUp`/`PageDown`/`Home`/`End`) is **not** available in
view-selection mode - only in browse mode below.

**Browse mode** (`handle_standings_league_keys`, item focus active): `Enter`
activates the focused team (`StandingsTabMsg::ActivateTeam`, pushes a
`TeamDetail` document); every other key goes through the canonical
`key_to_nav_msg` mapping exactly, wrapped in `StandingsTabMsg::DocNav`. In
particular `Shift+Left`/`Shift+Right` scroll (they do not switch columns) and
`PageUp`/`PageDown`/`Home`/`End` page/jump the viewport. Plain `Left`/`Right`
move focus to the same row position in the adjacent column
(`document_nav::find_row_sibling`), wrapping at the edges.

`Esc` in browse mode is `StandingsTabMsg::ExitBrowseMode` (ESC priority 4).

### Settings tab (`handle_settings_tab_keys`)

**Modal open** (`is_settings_modal_open`): only `Up`, `Down`, and `Enter` are
handled (`Modal(Up)` / `Modal(Down)` / `Modal(Confirm)`); everything else,
including `Left` and `Tab`, is a no-op inside this branch. There is no `Esc`
arm here - ESC priority 2 intercepts it globally before this function is ever
reached, so an `Esc` arm here would be dead code (and was removed in the
A13/A14 refactor along with a corresponding characterization test).

**Normal mode** (no modal):

| Key | Result |
|-----|--------|
| `Left` | `SettingsAction::NavigateCategoryLeft` - **always**, even with item focus active |
| `Right` | `SettingsAction::NavigateCategoryRight` - always |
| `Enter` | `SettingsTabMsg::ActivateSetting(config)`, carrying the current `state.system.config` |
| `BackTab` | `SettingsTabMsg::DocNav(FocusPrev)` - via `key_to_nav_msg`, same as Standings/Demo |
| everything else | delegated to `key_to_nav_msg`, wrapped in `SettingsTabMsg::DocNav` |

One remaining deliberate divergence from the canonical mapping:

- **`Left`/`Right` are claimed for category navigation**, not row/scroll
  navigation - Settings is the only tab where these keys don't reach
  `key_to_nav_msg` at all.

Before the F6 convergence, `BackTab` was explicitly guarded out to a no-op
here (unlike Standings and Demo, which always mapped it to `FocusPrev`). That
guard was removed; `BackTab` now falls through to `key_to_nav_msg` like every
other unclaimed key in this function.

Settings also has a divergence in the *global* Up handling (step 6, not in
this function): the step-6 special case treats Settings as a nested mode
unconditionally (even before any item has focus), so plain `Up` in the
Settings tab is *always* funneled into `handle_settings_tab_keys` ->
`key_to_nav_msg` -> `FocusPrev`, and can never return Settings content focus
to the tab bar - only `Esc` can.

### Demo tab (`handle_demo_tab_keys`, `--features development` only)

`Enter` activates the focused link (`DemoTabMsg::ActivateLink`); every other
key goes through the canonical `key_to_nav_msg` mapping exactly, wrapped in
`DemoTabMsg::DocNav` - full parity with Standings browse mode.

| Key | Result |
|-----|--------|
| `Enter` | `DemoTabMsg::ActivateLink` |
| `Left` | `DemoTabMsg::DocNav(FocusLeft)` |
| `Shift+Left` | `DemoTabMsg::DocNav(ScrollUp(1))` (not left - scrolling wins) |
| `Right` | `DemoTabMsg::DocNav(FocusRight)` |
| `Shift+Right` | `DemoTabMsg::DocNav(ScrollDown(1))` (not right - scrolling wins) |
| everything else | delegated to `key_to_nav_msg`, wrapped in `DemoTabMsg::DocNav` |

Before the F6 convergence, `Left`/`Right` **always row-navigated** in the
Demo tab, even with Shift held - there was no `Shift+Left`/`Shift+Right`
scroll, unlike Standings browse mode. That override was removed; the Demo
tab now matches the canonical mapping with no exceptions.

`src/tui/components/demo_tab.rs` used to also define its own
`DemoTab::handle_key` (matched from a `DemoTabMsg::Key` variant) with the
same Left/Right-always-row-navigates quirk baked in, but nothing in the
codebase ever constructed `DemoTabMsg::Key` - all real Demo tab key routing
goes through `handle_demo_tab_keys` in `keys.rs` above. That dead variant,
its dead match arm, and the unreachable `handle_key` method (along with the
`DEMO_TAB_COUNT` constant and `KeyCode`/`KeyEvent` imports that existed only
to support it) have been deleted.

`Esc` in the Demo tab is ESC priority 4.6 (`DemoTabMsg::ExitFocus`). The
higher-priority checks (settings modal, scores box-selection, standings and
settings item focus) are all gated on their own tab being current, so on the
Demo tab only the document-stack check (priority 1) can outrank it.

## Cross-Tab Focus Isolation

Two independent layers guarantee that one tab's focus state can never affect
key routing on another tab (this was a real bug - stale Scores box-selection
focus used to swallow `Up`/`Esc` on other tabs after a number-key tab switch):

1. **Input layer** (`src/tui/keys.rs`): `has_scores_item_focus`,
   `has_standings_item_focus`, `has_settings_item_focus`, and
   `is_settings_modal_open` are all gated on `state.navigation.current_tab`
   matching their own tab. A non-current tab's component state is never
   consulted, so even if stale focus existed it could not misroute a key.
2. **State layer** (`src/tui/reducers/navigation.rs`): `navigate_to_tab`,
   `navigate_tab_left`, and `navigate_tab_right` call
   `clear_all_tab_item_focus`, which resets `doc_nav.focus_index`/
   `scroll_offset` on every tab's state and closes any open settings modal.
   Stale focus does not survive a tab switch in the first place (this also
   prevents a phantom selection highlight from rendering on return to the
   tab).

Consequence for UX: switching tabs always resets you to the tab bar with no
remembered inner selection; re-entering a tab's content starts from the top.

Regression coverage: the "cross-tab focus bleed regression" cases in
`keys.rs`'s `base_cases()`/`dev_cases()` pin layer 1 (stale foreign focus
present in the store, routing still follows the current tab), and
`test_tab_switch_clears_all_item_focus_and_modal` /
`test_tab_cycling_clears_item_focus` in `reducers/navigation.rs` pin layer 2.

## Scores Tab: 5-Date Sliding Window

This section describes `ScoresTab`'s internal date-window bookkeeping
(`src/tui/components/scores_tab.rs`), which sits behind the
`NavigateLeft`/`NavigateRight` messages above; it is component state, not key
routing.

### Architecture

The window has a **sticky base date** (leftmost date) that only shifts at
edges:

- `window_base_date = game_date - selected_date_index`
- Window = `[base, base+1, base+2, base+3, base+4]` (`DATE_WINDOW_SIZE = 5`)
- `game_date = window_base_date + selected_date_index`
- `selected_date_index` = position within the window (0-4)

This is computed inline in `render_date_tabs` (not a separate function):

```rust
const DATE_WINDOW_SIZE: usize = 5;
let window_base_date = state
    .game_date
    .add_days(-(state.selected_date_index as i64));
let dates: Vec<GameDate> = (0..DATE_WINDOW_SIZE)
    .map(|i| window_base_date.add_days(i as i64))
    .collect();
```

### Navigation Behavior

**Within window** (`selected_date_index` in `1..=3`, pressing Left/Right):
`selected_date_index` changes, `game_date` changes to match, the window
itself stays the same, and a refresh is triggered.

**At left edge** (`selected_date_index == 0`, press `Left`):
`selected_date_index` stays at 0, `game_date` decrements by 1 day, and the
window shifts left by 1 day.

**At right edge** (`selected_date_index == DATE_WINDOW_SIZE - 1`, press
`Right`): `selected_date_index` stays at 4, `game_date` increments by 1 day,
and the window shifts right by 1 day.

### Example Sequence

```
Start: game_date=11/02, selected_date_index=2
  Window: [10/31, 11/01, 11/02, 11/03, 11/04]

Press Left: selected_date_index=1, game_date=11/01
  Window: [10/31, 11/01, 11/02, 11/03, 11/04] <- same window

Press Left: selected_date_index=0, game_date=10/31
  Window: [10/31, 11/01, 11/02, 11/03, 11/04] <- same window

Press Left at edge: game_date=10/30, selected_date_index=0
  Window: [10/30, 10/31, 11/01, 11/02, 11/03] <- window shifted

Press Right: game_date=10/31, selected_date_index=1
  Window: [10/30, 10/31, 11/01, 11/02, 11/03] <- same window
```

## Standings Tab: Column Behavior by View

Behavior of `Left`/`Right` during browse mode depends on the active view
(`StandingsTabMsg::CycleViewLeft`/`CycleViewRight` in view-selection mode
switch between these views):

- **League view**: single column, all teams sorted by points - `Left`/`Right`
  have no sibling to move to, so they have no effect.
- **Conference view**: two columns; order depends on the
  `display_standings_western_first` config setting; `Left`/`Right` switch
  columns, preserving row position via `find_row_sibling`.
- **Division view**: two columns (grouped by division, sorted by points
  within each); same column-order config and `Left`/`Right` behavior as
  Conference view.

Auto-scrolling: `Up`/`Down` focus navigation autoscrolls the viewport to keep
the focused row visible (`document_nav::autoscroll_to_focus`). Manual
scrolling via `PageUp`/`PageDown`/`Home`/`End` is only available in browse
mode (not view-selection mode, see above), does not change team selection,
and pages by `viewport_height` (minimum `MIN_PAGE_SIZE = 10` lines, see
`document_nav::page_up`/`page_down`); `Home`/`End` jump to the top/bottom.
