Ask the user which function to simplify (file path and function name, e.g., "src/tui/components/app.rs::render_scores_tab").

Then analyze and suggest improvements:

**Step 1: Check test coverage**
```bash
# Look for tests covering this function
grep -n "test.*{function_name}" {test_file}
```
- ✅ If well-tested: Proceed with suggestions
- ⚠️ If no tests found: "Warning: This function has no tests. Add tests before refactoring? [y/n]"

**Step 2: Read and analyze the function**
Calculate:
- **Line count**: {X} lines (target: <100)
- **Nesting depth**: {Y} levels (target: <4)
- **Cyclomatic complexity**: Estimate based on branches
- **Responsibilities**: What different things this function does

**Step 3: Identify simplification opportunities**

Check for:
- [ ] **Long function** (>100 lines) → Extract logical sections
- [ ] **Deep nesting** (>3 levels) → Use early returns or extract
- [ ] **Repeated patterns** → Extract common code
- [ ] **Complex conditionals** → Extract to named boolean functions
- [ ] **Push loops** → Convert to iterator chains
- [ ] **Multiple responsibilities** → Split into focused functions

**Step 4: Provide specific suggestions**

For each opportunity found, show:
```
🔧 Suggestion {N}: {Brief description}

Current code (lines X-Y):
{code snippet}

Proposed improvement:
{refactored code}

Benefits:
- Reduces line count by {N}
- Improves readability
- Easier to test

Would you like me to apply this change? [y/n]
```

**Step 5: Summary**
After showing all suggestions:
- Estimated line reduction: {current} → {projected}
- Complexity improvement: {assessment}
- Ask: "Apply all suggestions? [y/n/select]"

If user approves, apply changes and run tests.
