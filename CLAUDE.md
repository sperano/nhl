# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

NHL CLI tool written in Rust with two modes:
- **CLI Mode**: `cargo run -- standings`, `cargo run -- scores`, etc.
- **TUI Mode**: `cargo run` (no subcommand) launches interactive terminal UI

## Quick Reference

```bash
cargo build                    # Build
cargo test                     # Run tests
cargo run                      # TUI mode
cargo run -- standings         # CLI: standings
cargo run -- scores            # CLI: scores
cargo run -- schedule          # CLI: schedule
cargo run -- boxscore <id>     # CLI: boxscore

# Mock mode (for development/screenshots)
cargo run --features development -- --mock
```

## Architecture Summary

React/Redux-inspired TUI with unidirectional data flow:
- **Components** own UI state, render via `view()` method
- **Global state** holds API data, current tab, document stack
- **Actions** dispatched from key events, processed by reducers
- **Effects** handle async operations (API calls)

See `docs/architecture.md` for detailed architecture documentation.

## Key Directories

```
src/tui/
├── components/     # React-like components (ScoresTab, StandingsTab, etc.)
├── document/       # Document system for scrollable content
├── reducers/       # Action handlers
├── widgets/        # Low-level rendering primitives
├── action.rs       # Action enum
├── state.rs        # AppState structure
├── keys.rs         # Key event → Action mapping
└── runtime.rs      # Orchestrates components, effects, rendering
```

## Specialized Agent Commands

Use these slash commands for domain-specific help:
- `/api-integrate` - Adding new NHL API endpoints
- `/fixture-build` - Creating test fixtures
- `/navigation-debug` - Debugging keyboard navigation
- `/test-write` - Writing unit tests
- `/tui-architect` - Architecture decisions

## Requirements

### Code Style
- Use `anyhow::Result` for error handling
- No unsafe code
- Functions under 100 lines when possible
- Use imports, not full paths (`crate::foo::bar`)
- Only comment non-obvious code
- Be unicode-aware (no byte-length assumptions)

### Testing
- 90% minimum coverage for new code
- Always use `assert_buffer` for rendering tests
- Add regression tests after fixing bugs
- Use `tui::testing` utilities

### Architecture Rules
- Components own UI state; global state holds shared data
- Never dispatch actions from render loop
- Use `DocumentNavState` for document-based components
- Messages are the component API (not global state modifications)

## Documentation

Detailed documentation in `docs/`:
- `docs/architecture.md` - Full TUI architecture
- `docs/component-patterns.md` - Component creation patterns
- `docs/navigation.md` - Navigation behavior specs
- `docs/document-system.md` - Document system details

## Work Files

Plan and state files go in `.claude/work/` directory.
- when i ask to write a report, write it in .claude/work/reports
- when i ask to write a report, plan or any reference md file, add date and time in filename
- update test_config_to_toml when adding new config attributes
- never do any git add or commit
- when building, always use --feature development