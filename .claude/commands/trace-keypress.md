Trace the complete path of a keypress through the system.

Ask the user:
1. **Which key?** (e.g., "Left arrow", "Enter", "j", "Escape")
2. **Current context?** (which tab, is content focused, any panels open)

**Step 1: Find the key handler**

```bash
# Search for the key code
grep -n "KeyCode::{Key}" src/tui/keys.rs
```

Show the matching handler and explain the dispatch logic.

**Step 2: Trace priority order**

Key handling follows this priority (from `keys.rs`):
1. Global keys (q, /, etc.)
2. ESC key (context-dependent)
3. Panel navigation (if panel open)
4. Number keys (1-6 for tabs)
5. Tab bar navigation (if not content focused)
6. Content-specific (delegated to tab handler)

Show which branch applies for the given context.

**Step 3: Identify the action**

```bash
# Find what action is returned
grep -A5 "KeyCode::{Key}" src/tui/keys.rs
```

Show: `KeyCode::{Key} => Some(Action::{ActionName})`

**Step 4: Find the reducer**

```bash
# Search for action handler
grep -rn "Action::{ActionName}" src/tui/reducers/
```

Show the reducer function and file.

**Step 5: Document state changes**

Read the reducer and list:
- Which state fields are modified
- What effect is returned
- Any side conditions (match guards, if statements)

**Step 6: Trace the effect**

If effect is not `Effect::None`:
- `Effect::Action(a)` → Show what action is dispatched next
- `Effect::Async(...)` → Show what API call is made and what action it returns
- `Effect::Batch(...)` → List all effects in the batch

**Step 7: Show the re-render path**

Identify which component's `view()` reads the modified state:
```bash
grep -rn "{state_field}" src/tui/components/
```

**Complete trace diagram:**

```
Key: {Key}
Context: Tab={Tab}, ContentFocused={bool}, PanelStack={panels}
     │
     ▼
keys.rs: key_to_action()
     │ matches: {branch}
     ▼
Action::{ActionName}
     │
     ▼
reducers/{file}.rs: reduce_{name}()
     │ state.{field} = {new_value}
     ▼
Effect::{EffectType}
     │
     ▼
{ComponentName}.view() re-renders
     │ reads state.{field}
     ▼
UI Update: {what changes visually}
```

**Common issues to check:**
- Key captured by higher-priority handler?
- Content focus blocking tab-level keys?
- Panel intercepting the key?
- Action handler returning `None` (unhandled)?
