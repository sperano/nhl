# Execute Plan

Execute the plan specified by the argument: $ARGUMENTS

## Instructions

1. Read the plan file from `.claude/work/reports/$ARGUMENTS` or the path provided
2. Parse all tasks, steps, and code changes described in the plan
3. Use TodoWrite to create a todo list from the plan's tasks
4. Execute each task in order, marking them complete as you go

## Critical Rules

- **NEVER give up** because a task seems tedious, large, or time-consuming
- **NEVER postpone** tasks to "future phases" or "later iterations"
- **NEVER simplify** the scope because it's taking too long
- **NEVER keep backward compatibility** code, shims, or re-exports unless the plan explicitly requires it
- **DELETE unused code** completely - no `_unused` prefixes, no "// removed" comments
- **Complete ALL tasks** in the plan before stopping
- If you encounter an error, fix it and continue - do not abandon the plan
- Run `cargo build` and `cargo test` after completing all changes to verify correctness

## Execution Flow

1. Read and understand the entire plan
2. Create todo list with all tasks
3. For each task:
   - Mark as in_progress
   - Implement the changes described
   - Mark as completed
4. After all tasks complete:
   - Run `cargo build`
   - Run `cargo test`
   - Fix any errors
5. Summarize what was accomplished
