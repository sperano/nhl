Ask the user which reducer to document (e.g., "navigation", "panels", "scores", "standings", "data_loading").

Then generate comprehensive documentation:

**Step 1: Read and analyze the reducer**
- Read src/tui/reducers/{name}.rs
- Extract all actions handled
- Identify state changes made
- Document effects returned
- Note helper functions

**Step 2: Generate module-level doc comment**
```rust
//! {Reducer name} reducer - handles {brief description}
//!
//! This reducer is responsible for:
//! - {Responsibility 1}
//! - {Responsibility 2}
//!
//! ## Actions Handled
//!
//! - `Action::{Action1}` - {what it does}
//! - `Action::{Action2}` - {what it does}
//!
//! ## State Changes
//!
//! Modifies the following state fields:
//! - `state.{field}` - {how it changes}
//!
//! ## Effects
//!
//! May return:
//! - `Effect::None` - {when}
//! - `Effect::Action(...)` - {when}
//! - `Effect::Async(...)` - {when}
//!
//! ## Example
//!
//! ```rust
//! let state = AppState::default();
//! let action = Action::{Example};
//! let (new_state, effect) = reduce_{name}(&state, &action);
//! assert_eq!(new_state.{field}, {expected});
//! ```
```

**Step 3: Generate function-level doc comments**
For each public function, add:
```rust
/// {Brief description of what the function does}
///
/// # Arguments
///
/// * `state` - {description}
/// * `action` - {description}
///
/// # Returns
///
/// {What it returns and when}
///
/// # Example
///
/// ```
/// {example usage}
/// ```
```

**Step 4: Create markdown documentation file**
Generate docs/reducers/{name}.md:
```markdown
# {Name} Reducer

## Overview
{Description}

## State Flow Diagram

```
User Input → Action → Reducer → State Update → Effect
                ↓                      ↓
          {Action name}          {State change}
                                       ↓
                                  Re-render
```

## Action Sequences

### Example 1: {Scenario}
1. User {action} → `Action::{Name}`
2. Reducer updates `state.{field}` from {old} to {new}
3. Returns `Effect::{Type}`
4. UI re-renders showing {result}

### Example 2: {Another scenario}
...

## State Structure
Shows which fields this reducer modifies

## Effects Reference
Details each effect type and when it's used
```

**Step 5: Verify documentation**
```bash
cargo doc --no-deps --open
```
Check that docs render correctly.

**Report:**
- ✅ Added module-level docs
- ✅ Added {N} function doc comments
- ✅ Created docs/reducers/{name}.md
- 📖 Documentation ready to view
