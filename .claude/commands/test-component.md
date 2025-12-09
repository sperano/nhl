Ask the user which component to test (e.g., scores_tab, standings_tab, app, table, tabbed_panel).

Once specified, run tests for the component and related code:

```bash
# Main component tests
cargo test --lib tui::components::{component_name} -- --nocapture

# If testing a tab component, also test its reducer
cargo test --lib tui::reducers::{component_base_name} -- --nocapture 2>/dev/null || true

# If testing a component with widgets, test related widgets
cargo test --lib tui::widgets -- --nocapture 2>/dev/null || true
```

**Report format:**

1. **Component**: {component_name}
2. **Tests run**: X passed, Y failed
3. **Execution time**: Xs
4. **Test results**:
   - ✅ Show passing tests with names
   - ❌ Show failing tests with full error output and context
5. **Related tests** (if any were run)

If any tests fail:
- Show the full error with line numbers
- Identify the assertion that failed
- Offer to help debug or fix the issue

If all tests pass:
- Show summary: "✅ All {count} tests passing for {component_name}"
