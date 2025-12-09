Update the README.md to reflect the latest codebase state.

**Step 1: Identify new features**
```bash
# Check git log for recent additions
git log --since="1 month ago" --oneline src/

# Scan for new commands
ls -1 src/commands/*.rs

# Check for new TUI features
ls -1 src/tui/components/*.rs
```

**Step 2: Extract current capabilities**

Scan the codebase for:
- **CLI Commands**: Parse `src/commands/*.rs` for subcommands
- **TUI Features**: Check `src/tui/types.rs` for Tab enum variants
- **Configuration**: Read `src/config.rs` for Config struct fields
- **Dependencies**: Check Cargo.toml for key dependencies

**Step 3: Update README sections**

Update the following sections with current information:

**Features**
- List all CLI commands
- List all TUI tabs and features
- Highlight key capabilities

**Installation**
- Ensure build instructions are current
- Verify dependency requirements

**Usage**
- Show examples of each CLI command with actual output
- Show TUI navigation examples

**Configuration**
- Document all config.toml options
- Show example config file

**Screenshots**
Generate fresh screenshots using `vhs`:
1. Create .vhs tape files for key features:
   - TUI main view
   - Standings display
   - Scores display
   - Settings panel
2. Run: `vhs {tape_file}.tape`
3. Update README with new screenshot paths

**Step 4: Validate**
```bash
# Check for broken links
# Verify code blocks compile
```

**Step 5: Preview**
Show the updated README sections and ask for approval before saving.

**Report:**
- ✅ Updated {N} sections
- ✅ Generated {N} new screenshots
- 📝 README ready for review
