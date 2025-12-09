Ask the user which NHL API endpoint or data type to explore (e.g., "player stats", "game details", "team roster", "schedule", "draft picks").

Then investigate:

**Step 1: Check the nhl-api crate**
```bash
# Search for relevant methods in the local nhl-api crate
grep -r "pub fn" ../nhl-api/src/ | grep -i {search_term}
```
- Show method signatures
- Display return types
- Note any required parameters

**Step 2: Find existing usage in codebase**
```bash
# Find how similar data is used
grep -r "{data_type}" src/tui/
```
- Show existing patterns
- Identify which reducers handle similar data
- Note state structure patterns

**Step 3: Check NHL API documentation**
Use WebSearch to find:
- Official NHL API endpoints for this data
- Response format and fields
- Rate limits or restrictions
- Whether the endpoint is already implemented in nhl-api crate

**Step 4: Show the data structure**
- Display relevant types from nhl_api crate
- Show key fields and their types
- Note any nested structures

**Step 5: Suggest integration approach**
Provide guidance on:
- Which reducer should handle this data (navigation/panels/data_loading/etc.)
- What state field to add (in AppState -> DataState or UiState)
- What effect pattern to use (Effect::Async with API call)
- Which component would display it
- Example of the data flow: User Action → Reducer → Effect → API Call → Action → State Update → Re-render

**Step 6: Show example usage pattern**
Point to similar existing code in the project that demonstrates the pattern.
