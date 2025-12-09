Check if a file needs refactoring and identify specific opportunities.

Ask the user for a **file path** to analyze.

**Step 1: Gather metrics**

Read the file and calculate:
- Total lines of code
- Number of functions
- Longest function (line count)
- Maximum nesting depth
- Number of `match` arms > 5
- Number of `if/else` chains > 3

**Step 2: Check against thresholds**

| Metric | Good | Warning | Needs Refactor |
|--------|------|---------|----------------|
| File lines | <500 | 500-800 | >800 |
| Function lines | <50 | 50-100 | >100 |
| Nesting depth | ≤3 | 4 | ≥5 |
| Match arms | ≤7 | 8-10 | >10 |

**Step 3: Identify patterns**

Check for:
- [ ] **Repeated code blocks** (3+ similar sections)
- [ ] **Long parameter lists** (>5 params)
- [ ] **Deep nesting** (>3 levels of indent)
- [ ] **God functions** (>100 lines, multiple responsibilities)
- [ ] **Match explosion** (>10 arms in single match)
- [ ] **Clone chains** (multiple .clone() in sequence)
- [ ] **String building in loops** (inefficient allocation)

**Step 4: Generate report**

```
## Refactoring Analysis: {file}

### Metrics
| Metric | Value | Status |
|--------|-------|--------|
| Total lines | {n} | {status} |
| Functions | {n} | - |
| Longest function | {name} ({n} lines) | {status} |
| Max nesting | {n} | {status} |

### Issues Found

#### 1. {Issue type} (lines {X}-{Y})
**Severity**: {High/Medium/Low}
**Description**: {what's wrong}
**Suggested fix**: {how to fix}
```rust
// Before
{problematic code snippet}

// After
{refactored code snippet}
```

#### 2. {Next issue}
...

### Recommended Actions

Priority order:
1. {Most important refactor}
2. {Second priority}
3. {Third priority}

### Test Coverage Check
```bash
cargo tarpaulin --lib --output-dir /tmp -- {module_path}
```
Current coverage: {X}%
⚠️ Add tests before refactoring if coverage < 80%

### Quick Wins
- {Simple improvement 1}
- {Simple improvement 2}
```

**Step 5: Offer actions**

- "Apply fix #1?" - Apply the first suggested refactor
- "Show all fixes?" - Display all refactored code
- "Run tests?" - Verify tests pass before/after
- "Skip" - Don't make changes
