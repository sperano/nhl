Ask the user:
1. **File path and line range** (e.g., "src/tui/components/app.rs lines 45-60")
2. **Desired function name** (or say "suggest" to auto-generate)

Then perform extraction:

**Step 1: Read and analyze the code**
- Read the specified lines
- Understand what the code does
- Generate a descriptive name if user requested suggestion

**Step 2: Check for similar existing functions**
```bash
# Search for similar function names in the same file and related files
grep -n "fn.*{search_terms}" {file} {related_files}
```
- If similar function exists, ask: "Found similar function `{name}`. Use it instead, or create new one?"

**Step 3: Determine function signature**
- Identify parameters (variables used from outer scope)
- Determine return type (what the code produces)
- Decide visibility:
  - `pub` if used across modules
  - `pub(crate)` if used within crate
  - Private if only used in same file

**Step 4: Determine best location**
- **Same file, just above** - if only used in current function
- **Same file, in impl block** - if it's a method on the current type
- **Module helpers** (e.g., src/tui/helpers.rs) - if reusable utility
- **Dedicated module** - if part of a larger pattern

**Step 5: Create extracted function**
```rust
/// {Generated doc comment explaining what it does}
{visibility} fn {name}({params}) -> {return_type} {
    {extracted_code}
}
```

**Step 6: Replace original code**
```rust
let result = {function_name}({args});
```

**Step 7: Verify**
```bash
cargo test --lib {module} -- --nocapture
```

**Report:**
- Show before/after diff
- Confirm test results
- Show where function was placed
- ✅ "Extraction complete! All tests passing."
