Generate a mock data screenshot of the TUI for documentation.

Ask the user:
1. **Which view?** (scores, standings-division, standings-conference, standings-league, settings)
2. **Terminal size?** (default: 80x24, or specify like "120x30")
3. **Any specific state?** (e.g., "with a game selected", "settings modal open")

**Step 1: Verify development feature is enabled**

```bash
grep 'features.*development' Cargo.toml
```

If not present, the mock mode won't work. Inform user.

**Step 2: Build with development feature**

```bash
cargo build --features development
```

**Step 3: Create VHS tape file**

Create `/tmp/screenshot_{view}.tape`:

For **scores** view:
```vhs
Output /tmp/nhl_scores.gif
Set Width {width}
Set Height {height}
Set FontSize 14
Set Theme "Builtin Dark"

Type "cargo run --features development -- --mock"
Enter
Sleep 2s
# Navigate if needed
{navigation_commands}
Sleep 1s
Screenshot /tmp/nhl_scores.png
Type "q"
```

For **standings** view:
```vhs
Output /tmp/nhl_standings.gif
Set Width {width}
Set Height {height}

Type "cargo run --features development -- --mock"
Enter
Sleep 2s
Type "2"  # Switch to standings tab
Sleep 500ms
{view_specific_navigation}
Sleep 1s
Screenshot /tmp/nhl_standings_{view}.png
Type "q"
```

For **settings** view:
```vhs
Output /tmp/nhl_settings.gif
Set Width {width}
Set Height {height}

Type "cargo run --features development -- --mock"
Enter
Sleep 2s
Type "6"  # Switch to settings tab (or appropriate number)
Sleep 500ms
Screenshot /tmp/nhl_settings.png
Type "q"
```

**Step 4: Run VHS**

```bash
vhs /tmp/screenshot_{view}.tape
```

**Step 5: Verify output**

```bash
ls -la /tmp/nhl_*.png
```

**Step 6: Report**

```
## Screenshot Generated

- View: {view}
- Size: {width}x{height}
- File: /tmp/nhl_{view}.png

### Preview Command
```bash
open /tmp/nhl_{view}.png  # macOS
xdg-open /tmp/nhl_{view}.png  # Linux
```

### To add to README
```markdown
![{View} Screenshot](docs/images/nhl_{view}.png)
```

### Move to docs
```bash
mkdir -p docs/images
cp /tmp/nhl_{view}.png docs/images/
```
```

**Alternative: Manual screenshot**

If VHS is not installed:
```bash
# Run in mock mode
cargo run --features development -- --mock

# Use your terminal's screenshot feature
# Or use a tool like scrot, gnome-screenshot, or macOS screenshot
```

**Mock data notes:**
- Mock mode returns deterministic fixture data
- Same data every time for consistent screenshots
- Includes all 32 teams, various game states
- No network calls made
