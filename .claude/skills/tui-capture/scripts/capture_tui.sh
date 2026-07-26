#!/bin/zsh
# Capture mock-mode TUI screens from a given nhl binary into text files.
# Usage: capture_tui.sh <binary> <outdir>
set -e
BIN="$1"
OUT="$2"
SES="nhlcap$$"
mkdir -p "$OUT"

tmux kill-session -t "$SES" 2>/dev/null || true
tmux new-session -d -s "$SES" -x 120 -y 30
tmux send-keys -t "$SES" "$BIN --mock" Enter
sleep 3

# Tab 1: scores
tmux capture-pane -t "$SES" -p > "$OUT/01_scores.txt"

# Enter first game -> boxscore document (team boxscore tables)
tmux send-keys -t "$SES" Enter
sleep 1
tmux capture-pane -t "$SES" -p > "$OUT/02_boxscore.txt"
tmux send-keys -t "$SES" Escape
sleep 1

# Tab 2: standings
tmux send-keys -t "$SES" 2
sleep 1
tmux capture-pane -t "$SES" -p > "$OUT/03_standings.txt"

# Scroll down a bit in standings
tmux send-keys -t "$SES" j j j
sleep 1
tmux capture-pane -t "$SES" -p > "$OUT/04_standings_scrolled.txt"

tmux send-keys -t "$SES" q
sleep 1
tmux kill-session -t "$SES" 2>/dev/null || true
echo "captured to $OUT"
