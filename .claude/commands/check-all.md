Run a comprehensive quality check on the codebase.

```bash
echo "=== 1/3 Running Tests ==="
cargo test --lib 2>&1 | tail -20

echo -e "\n=== 2/3 Running Clippy ==="
cargo clippy --lib 2>&1 | grep -E "(warning:|error:)" | head -20

echo -e "\n=== 3/3 Checking Formatting ==="
cargo fmt -- --check 2>&1
```

If formatting check fails, automatically fix it:
```bash
cargo fmt
echo "✅ Code formatted automatically"
```

**Summary Report:**

```
╔════════════════════════════════╗
║     Quality Check Results      ║
╠════════════════════════════════╣
║ Tests:      [✅/❌] X/Y passed  ║
║ Clippy:     [✅/❌] X warnings  ║
║ Formatting: [✅] Auto-fixed    ║
╠════════════════════════════════╣
║ Overall:    [PASS/FAIL]        ║
╚════════════════════════════════╝
```

**Details:**
- If tests fail: Show which tests and offer to help debug
- If clippy has warnings: List the warnings and offer to fix critical ones
- If formatting was needed: Confirm it was auto-formatted

**Final action:**
- If overall PASS: "✅ All checks passed! Ready to commit."
- If overall FAIL: Offer specific next steps to fix issues
