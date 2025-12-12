Add a new color theme to the TUI.

Ask the user:
1. **Theme ID** (snake_case, e.g., "north_stars", "habs", "bruins")
2. **Display Name** (human-readable, e.g., "North Stars", "Habs", "Bruins")
3. **fg1 color** (highlight/brightest - RGB like "240,240,240" or hex like "#F0F0F0")
4. **fg2 color** (primary text color)
5. **fg3 color** (muted/accent color)

**Reference: Recent theme additions (commit 6729924)**

North Stars theme:
- fg1: 240,240,240 (white/highlight)
- fg2: 198,146,20 (gold)
- fg3: 0,122,51 (green)

Habs theme:
- fg1: 255,255,255 (white)
- fg2: 175,30,45 (red)
- fg3: 45,53,124 (blue)

Sabres theme:
- fg1: 255,255,255 (white)
- fg2: 255,184,28 (gold)
- fg3: 0,48,135 (blue)

**Step 1: Add THEME_ID constant to src/config.rs**

Find the block of `pub static THEME_ID_*` constants (around line 47-57) and add:
```rust
pub static THEME_ID_{NAME_UPPER}: &str = "{name_lower}";
```

**Step 2: Add Theme static to src/config.rs**

Find the last `pub static THEME_*: Theme = Theme {` block (before the `THEMES` map) and add:
```rust
pub static THEME_{NAME_UPPER}: Theme = Theme {
    name: "{Display Name}",
    fg1: Color::Rgb({r1}, {g1}, {b1}),
    fg2: Color::Rgb({r2}, {g2}, {b2}),
    fg3: Color::Rgb({r3}, {g3}, {b3}),
    fg2_dim: OnceLock::new(),
    fg3_dim: OnceLock::new(),
};
```

**Step 3: Add entry to THEMES map in src/config.rs**

Find the `pub static THEMES: phf::Map` block and add an entry:
```rust
    "{name_lower}" => &THEME_{NAME_UPPER},
```

**Step 4: Update src/tui/settings_helpers.rs imports**

Add `THEME_ID_{NAME_UPPER}` to the import list at the top:
```rust
use crate::config::{
    Config, THEMES, THEME_ID_BLUE, THEME_ID_CYAN, THEME_ID_GREEN, THEME_ID_HABS,
    THEME_ID_NORTH_STARS, THEME_ID_ORANGE, THEME_ID_PURPLE, THEME_ID_RED, THEME_ID_SABRES,
    THEME_ID_WHITE, THEME_ID_YELLOW, THEME_ID_{NAME_UPPER},
};
```

**Step 5: Add to THEME_IDS array in src/tui/settings_helpers.rs**

Find the `const THEME_IDS` array and add:
```rust
const THEME_IDS: &[&str] = &[
    // ... existing entries
    THEME_ID_{NAME_UPPER},
];
```

**Step 6: Verify**
```bash
cargo fmt
cargo check
cargo test --lib config -- --nocapture
cargo test --lib settings_helpers -- --nocapture
```

**Step 7: Test visually (optional)**
```bash
cargo run
# Go to Settings tab, change theme to verify it appears and works
```

**Report:**
- Theme ID: `{name_lower}`
- Display Name: `{Display Name}`
- Colors: fg1=rgb({r1},{g1},{b1}), fg2=rgb({r2},{g2},{b2}), fg3=rgb({r3},{g3},{b3})
- Files modified:
  - `src/config.rs` (THEME_ID constant, Theme static, THEMES map entry)
  - `src/tui/settings_helpers.rs` (import, THEME_IDS array)
