# NHL CLI

A terminal-based NHL stats viewer written in Rust.

## Features

- **Interactive TUI mode**: Navigate scores, standings, and settings with keyboard controls
- **CLI commands**: Quick access to standings, schedules, boxscores, and live scores
- **Live updates**: Real-time game scores with period-by-period breakdowns
- **Date navigation**: Browse scores across different dates with a sliding window interface

## Quick Start

```bash
# Interactive mode (default)
cargo run

# Command-line mode
cargo run -- standings
cargo run -- scores
cargo run -- schedule
cargo run -- boxscore 2024020001
```

## Status

⚠️ **Under active development** - Features and APIs may change.

## Configuration

Optional config file: `~/.config/nhl/config.toml`

```toml
log_level = "info"
log_file = "/dev/null"
refresh_interval = 60
display_standings_western_first = false
time_format = "%H:%M:%S"

[display]
use_unicode = true
theme = "orange"
error_fg = "226,74,74"
```
