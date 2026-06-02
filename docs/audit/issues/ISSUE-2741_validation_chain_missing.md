# Issue-2 (P0): Validation Chain Missing — Test Design Review Gate Absent

**Status**: `UNVERIFIED` — Requirement → Test Design → Assertion chain not established
**Type**: Governance Blocker (Highest Priority)
**Category**: Audit Finding — Validation System Problem
**Related Rule**: G-01 (proposed)

---

## Evidence Section

### E-1: RECOVERY-006 Exists But Does Not Prove Its Requirement

```
File:      tests/wal_integration_test.rs or crates/storage/tests/integration_wal.rs
Test:      RECOVERY-006 (or equivalent row-count test)
Assertion: COUNT(*) == 1
Requirement: "DELETE does not recover after crash"

Gap Analysis:
  Assertion:   row count = 1
  Requirement: data is absent (row returns None)

Conclusion: Assertion does not prove Requirement.
            This is an indirect assertion that validates WAL output,
            not an observable behavior of the database.

Evidence Command:
  grep -n "COUNT\|assert\|fn.*recovery" tests/wal_integration_test.rs
  (shows row-count based assertions, no data-value assertions)
```

### E-2: Integration WAL Tests Use Indirect Assertions

```
File:      crates/storage/tests/integration_wal.rs
Test:      test_wal_recovery_uncommitted_transaction
Assertion: commits == 1
Requirement: "uncommitted transaction data is discarded"

Gap Analysis:
  Assertion:   WAL output (commit count = 1)
  Requirement: database state (data absent)

Conclusion: Verifies WAL replay output, not database correctness.
            These are different things.

Evidence Command:
  grep -n "assert\|fn.*recover" crates/storage/tests/integration_wal.rs
  (shows internal state assertions, no SELECT-from-restart assertions)
```

### E-3: No Test Design Review Gate in GOVERNANCE.md

```
File:      docs/GOVERNANCE.md (or equivalent)
Search:    grep -i "test.*design\|requirement.*assertion\|observable.*behavior" docs/

Result:    No governance rule requiring Test Design Review before implementation

Observation:
  Current governance checks: "Does a test exist?"
  Missing governance check:   "Does the test prove the requirement?"
```

### E-4: 39 Tests Listed in MISSING_TESTS.md

```
File:      docs/governance/wal/MISSING_TESTS.md
Count:     39 tests listed as missing
Root Cause Hypothesis: Without Test Design Review gate,
                       tests are added without verifying they prove requirements.

Note: This is a symptom, not the root cause.
      Even 339 tests are meaningless without the Validation Chain.
```

---

## Impact

Current governance checks:

```
"Does a test exist?"  ✅ YES
"Is the assertion correct?"  ❌ NOT CHECKED
"Does it prove the requirement?"  ❌ NOT CHECKED
```

**This is the root cause of RECOVERY-006 and similar issues.**
If this Issue is fixed, many future problems will be caught before they propagate.

---

## G-01 Validation Chain (Proposed Rule)

Add to `GOVERNANCE.md`:

```
## G-01: Validation Chain

Any P0 Feature MUST have a documented Validation Chain:

| Item                  | Required | Description |
|-----------------------|----------|-------------|
| Requirement           | YES      | What must be proven |
| Observable Behavior   | YES      | What can be observed after the feature works |
| Test Design           | YES      | How to exercise the feature |
| Assertion             | YES      | What is checked (must directly prove Requirement) |
| Evidence              | YES      | cargo test output + FileStorage proof |

Missing any item → Status = UNVERIFIED
                 → Cannot close Feature Issue

Key Rule:
  Assertion must directly prove Requirement.
  COUNT(*) == 1 does NOT prove "row is deleted" if other rows exist.
  row == None proves "row is deleted" — this is a direct assertion.
```

---

## Resolution Path

```
1. Create G-01 rule in GOVERNANCE.md
2. Apply G-01 to existing P0 features (WAL, TX, Crash Recovery):
   - Write four-row table for each P0 feature
   - Identify indirect assertions (COUNT instead of value check)
   - Replace with direct assertions
3. Backfill four-row tables for all existing P0 tests
4. Require Evidence (cargo test output) to close each Issue
5. No "Fix" until Test Design Review validates the fix is correct
```

---

## Not Stating (Hypothesis)

- "Developers are incompetent"
- "The test framework is broken"
- Any blame assignment

**Root cause is a missing governance gate, not individual failure.**

---

## Why This Issue > #2740

| Aspect | Issue #2740 | Issue #2741 |
|--------|-------------|-------------|
| Scope | One feature (Crash Recovery) | Entire validation system |
| Fixes | One problem | Mechanism that prevents many problems |
| Effect | Correctness for v3.8.0 | Correctness for all future releases |
| Type | Release Blocker | Governance Blocker |

If only one Issue can be fixed before RC, choose #2741.
The Validation Chain establishment prevents the next 10 "Recovery Unverified" Issues.