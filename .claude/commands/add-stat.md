Ask the user:
1. **Which statistic to add?** (e.g., "Faceoff Win %", "Blocked Shots", "Plus/Minus", "Save %")
2. **Which table?** (skater stats, goalie stats, standings, team roster)
3. **Where to insert the column?** (after which existing column, or at the end)

Then add the statistic:

**Step 1: Identify the widget file**
- Skater stats: src/tui/components/skater_stats_table.rs
- Goalie stats: src/tui/components/goalie_stats_table.rs
- Standings: src/tui/components/standings_tab.rs
- Team roster: src/tui/components/team_detail_panel.rs

**Step 2: Verify data availability**
Check that the field exists in the NHL API type:
```bash
# Find the type definition
grep -r "pub struct.*Stats" ../nhl-api/src/
```
- Show the available fields
- Confirm the statistic field exists

**Step 3: Add column definition**
Insert at the specified position:
```rust
ColumnDef::new(
    "{Header}",           // Column header
    {width},              // Width in characters
    Alignment::{Left/Right/Center},
    |row: &{Type}| {
        CellValue::Text(format!("{}", row.{field}))
        // or CellValue::Number for numeric stats
    }
)
```

**Step 4: Update tests**
Find rendering tests and update expected output:
- Add the new column to assert_buffer expectations
- Ensure column alignment and width are correct

**Step 5: Verify**
```bash
cargo test --lib {module}::tests -- --nocapture
```

**Report:**
- ✅ Added "{stat}" column to {table}
- ✅ Positioned after "{previous_column}"
- ✅ All tests updated and passing
- Show example of updated table output
