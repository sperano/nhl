---
name: tui-capture
description: Run the TUI headlessly in a tmux PTY with mock data, drive it with keystrokes, and capture screens as diffable text. Use to verify rendering changes (diff captures against a main-branch build), to debug TUI behavior without a terminal, or to grab text screenshots for docs.
---

# TUI capture (headless, diffable)

The Bash tool has no TTY, so the TUI can't run in it directly. tmux owns a
real PTY, so the app runs happily inside a detached tmux session and is
driven entirely through tmux's CLI.

## Capture a standard set of screens

```bash
cargo build --features development
.claude/skills/tui-capture/scripts/capture_tui.sh target/debug/nhl /tmp/cap
```

The script starts a 120x30 pane, runs `<binary> --mock` (deterministic
fixture data), then captures: scores tab, Enter-navigation view, standings
tab, and standings scrolled 3 rows — one `.txt` per screen.

## Regression-diff a rendering change

Capture from a baseline build and the changed build, then diff:

```bash
(cd /path/to/main-checkout && cargo build --features development)
.claude/skills/tui-capture/scripts/capture_tui.sh /path/to/main-checkout/target/debug/nhl /tmp/cap_main
.claude/skills/tui-capture/scripts/capture_tui.sh target/debug/nhl /tmp/cap_new
for f in /tmp/cap_main/*.txt; do
  diff <(grep -v "Updated .s ago" "$f") <(grep -v "Updated .s ago" "/tmp/cap_new/$(basename "$f")") \
    >/dev/null && echo "$(basename "$f"): identical" || echo "$(basename "$f"): DIFFERS"
done
```

The `Updated Ns ago` status-bar timer is the only expected difference —
always filter it. Any other diff is a rendering change to explain or fix.

## Custom drives

For screens the script doesn't cover, run tmux directly:

```bash
tmux new-session -d -s nhl -x 120 -y 30
tmux send-keys -t nhl "target/debug/nhl --mock" Enter && sleep 3
tmux send-keys -t nhl 2          # keys: 1-6 tabs, j/k scroll, Enter/Escape nav
sleep 1
tmux capture-pane -t nhl -p      # add -e to include ANSI styles
tmux send-keys -t nhl q && tmux kill-session -t nhl
```

Notes:
- `capture-pane -p` drops styling (glyphs only); `-e` keeps ANSI escapes for
  style-sensitive diffs, at the cost of noisier output.
- Sleeps are load-bearing: give the app ~3s to start and ~1s after each key.
- Works for any TTY-requiring program, not just this TUI.
