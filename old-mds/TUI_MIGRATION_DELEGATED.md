# TUI Widget Migration - Delegated Plan

## Model Assignment Strategy

**Opus Tasks:**
- Architectural decisions
- Complex state management
- Performance-critical code
- Tasks requiring deep codebase understanding
- Integration of multiple complex systems

**Sonnet Tasks:**
- Well-defined widget implementations
- Utility functions with clear specs
- Test writing for existing code
- Code cleanup and refactoring
- Simple integrations with clear interfaces

---

## Task Delegation Map

### 🟦 Phase 1: Foundation (Mixed)

#### Step 1.1: Widget Infrastructure ✅ COMPLETED
- **Model:** Opus
- **Reason:** Foundational architecture, affects entire system
- **Status:** Already completed

#### Step 1.2: Buffer Utilities
- **Model:** Sonnet (can be parallelized)
- **Reason:** Well-defined utility functions, clear input/output
- **Subtasks for delegation:**
  - 1.2a: Text rendering utilities (Sonnet)
  - 1.2b: Border/box drawing utilities (Sonnet)
  - 1.2c: Table/grid utilities (Sonnet)
  - 1.2d: Test suite for utilities (Sonnet)

---

### 🟦 Phase 2: First Widget Proof of Concept

#### Step 2.1: Extract ScoringTable Widget
- **Model:** Sonnet with clear spec
- **Reason:** Extraction of existing logic with clear boundaries
- **Prerequisites:** Provide agent with:
  - Current `format_scoring_summary()` function
  - Widget trait definition
  - Example of expected output
  - Test requirements

#### Step 2.2: Integrate ScoringTable
- **Model:** Sonnet
- **Reason:** Simple integration, replace function call with widget
- **Prerequisites:** Step 2.1 completed

---

### 🟦 Phase 3: Score Table Components (Complex)

#### Step 3.1: ScoreTable Widget
- **Model:** Opus
- **Reason:** Complex state handling (live/final/scheduled), period management
- **Why Opus:** Needs to handle many edge cases and game states

#### Step 3.2: GameBox Widget
- **Model:** Sonnet with detailed spec
- **Reason:** Composition of existing widgets, fixed dimensions
- **Prerequisites:**
  - ScoreTable widget completed
  - Clear specification of 37×7 dimensions
  - Selection behavior spec

#### Step 3.3: GameGrid Widget
- **Model:** Opus
- **Reason:** Complex dynamic layout, column calculations, scrolling
- **Why Opus:** Critical performance and layout logic

#### Step 3.4: Replace Scores Tab
- **Model:** Sonnet
- **Reason:** Integration work with clear before/after
- **Prerequisites:** All score widgets completed

---

### 🟦 Phase 4: Standings Components

#### Step 4.1: TeamRow Widget
- **Model:** Sonnet (can be parallelized)
- **Reason:** Simple extraction, clear formatting rules
- **Can run parallel with:** Other simple widgets

#### Step 4.2: StandingsTable Widget
- **Model:** Opus or experienced Sonnet
- **Reason:** Multiple view modes, complex navigation
- **Compromise:** Could use Sonnet with very detailed spec

#### Step 4.3: Replace Standings Tab
- **Model:** Sonnet
- **Reason:** Integration work
- **Prerequisites:** StandingsTable completed

---

### 🟦 Phase 5: Polish

#### Step 5.1: Performance Optimization
- **Model:** Opus
- **Reason:** Requires profiling, analysis, architectural understanding

#### Step 5.2: Code Cleanup
- **Model:** Sonnet (can be parallelized)
- **Reason:** Mechanical cleanup work
- **Subtasks:**
  - Remove deprecated functions
  - Update documentation
  - Fix clippy warnings

---

## Parallel Execution Opportunities

### Wave 1 (After Step 1.1)
Can run **in parallel** with Sonnet agents:
- Step 1.2a: Text rendering utilities
- Step 1.2b: Border utilities
- Step 1.2c: Table utilities
- Step 4.1: TeamRow Widget (independent)

### Wave 2 (After buffer utilities)
Sequential but can delegate:
- Step 2.1: ScoringTable extraction (Sonnet)

### Wave 3 (After Phase 2)
Mixed parallel opportunity:
- Step 3.2: GameBox (Sonnet)
- Step 3.1: ScoreTable (Opus)

---

## Agent Instruction Templates

### For Sonnet Agents - Buffer Utilities (1.2a-c)

```markdown
## Task: Implement [Buffer Utility Name]

You are implementing buffer-based rendering utilities for a TUI widget system.

**Context:**
- We have a `RenderableWidget` trait in `src/tui/widgets/mod.rs`
- Testing utilities exist in `src/tui/widgets/testing.rs`
- You're creating utilities in `src/tui/widgets/buffer_utils.rs`

**Your specific task:** [1.2a/b/c]

**Requirements:**
1. All functions must work directly with `ratatui::buffer::Buffer`
2. Support both ASCII and Unicode box characters via `DisplayConfig`
3. Include comprehensive tests
4. Follow existing patterns from testing.rs

**Deliverables:**
- Implementation in buffer_utils.rs
- Unit tests with 90%+ coverage
- Documentation for each function
```

### For Sonnet Agents - Widget Implementation

```markdown
## Task: Implement [Widget Name] Widget

**Current State:**
[Paste current string-based implementation]

**Target State:**
Create a widget implementing `RenderableWidget` trait

**Requirements:**
1. Extract logic from existing function
2. Preserve exact visual output
3. Support all existing features
4. Add comprehensive tests

**Test Requirements:**
- Snapshot tests comparing old vs new
- Edge case tests
- 90% coverage minimum

**Files to create/modify:**
- src/tui/widgets/[widget_name].rs
- Update src/tui/widgets/mod.rs exports
```

---

## Execution Strategy

### Phase 1: Immediate Parallel Tasks (Sonnet)
Launch 4 Sonnet agents in parallel for:
1. Text rendering utilities
2. Border/box utilities
3. Table/grid utilities
4. TeamRow widget (independent)

### Phase 2: Sequential Foundation (Mixed)
1. Review and integrate utilities (Opus)
2. ScoringTable extraction (Sonnet)
3. Integration testing (Opus)

### Phase 3: Complex Widgets (Opus-led)
1. ScoreTable widget (Opus)
2. GameBox widget (Sonnet with Opus review)
3. GameGrid widget (Opus)
4. Integration (Sonnet)

### Phase 4: Standings (Mixed)
1. StandingsTable (Opus or detailed Sonnet)
2. Integration (Sonnet)

### Phase 5: Optimization (Opus)
1. Performance profiling and optimization (Opus)
2. Cleanup tasks (Parallel Sonnet agents)

---

## Success Metrics for Delegation

### For Sonnet Tasks
- Clear, self-contained requirements
- Existing code to reference
- Well-defined input/output
- Test criteria specified upfront

### For Opus Tasks
- Architectural decisions required
- Complex state management
- Performance-critical sections
- Multiple system interactions

---

## Risk Mitigation

### For Delegated Tasks
1. **Clear specifications:** Provide complete context and examples
2. **Test-first approach:** Define expected behavior upfront
3. **Incremental review:** Check work after each subtask
4. **Rollback ready:** Keep old implementation until validated

### Quality Gates
- Each delegated task must pass:
  - Unit tests (90% coverage)
  - Integration tests
  - Manual testing checklist
  - Code review

---

## Recommended Execution Order

### Week 1
**Day 1-2:**
- Launch Wave 1 parallel tasks (4 Sonnet agents)
- Opus reviews and integrates

**Day 3-4:**
- ScoringTable extraction (Sonnet)
- Begin ScoreTable widget (Opus)

**Day 5:**
- Integration and testing

### Week 2
**Day 1-2:**
- GameBox (Sonnet) + GameGrid (Opus) in parallel
- Integration work

**Day 3-4:**
- Standings components
- Performance profiling

**Day 5:**
- Cleanup and polish

---

## Agent Communication Protocol

### When delegating to Sonnet:
1. Provide this context:
   - Specific task from this plan
   - Current implementation (if extracting)
   - Widget trait and test utilities
   - Expected output format
   - Test requirements

2. Request deliverables:
   - Implementation file
   - Test file
   - Usage example
   - Coverage report

### When using Opus:
1. Provide full context:
   - Entire migration plan
   - Current architecture
   - Performance requirements
   - Integration points

2. Decision points:
   - Architecture choices
   - Performance trade-offs
   - API design
   - Integration strategy

---

**Recommendation:** Start with Wave 1 parallel Sonnet tasks to maximize efficiency while Opus handles architectural decisions and complex widgets.