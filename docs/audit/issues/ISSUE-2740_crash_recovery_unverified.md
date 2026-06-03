# Issue-1 (P0): Crash Recovery Correctness Unverified

**Status**: `UNKNOWN` — Not Proven
**Type**: Release Blocker (RB-1)
**Category**: Audit Finding — Correctness Not Proven

---

## Evidence Section

### E-1: CRASH_RECOVERY_001/002/003 — No Test Output on Record

```
File:      docs/audit/wal_invariant_report.md
Claim:     "CRASH_RECOVERY_001/002/002 通过"
Evidence:  No cargo test output attached
           No Commit SHA linked to test run
           No log file reference

Status:    UNVERIFIED — Claim without evidence
```

### E-2: MemoryStorage Used in Crash Recovery Tests

```
File:      tests/crash_recovery_test.rs
Command:   grep -n "MemoryExecutionEngine\|MemoryStorage" tests/crash_recovery_test.rs
Output:
    10:  fn create_engine() -> MemoryExecutionEngine {
    11:      ExecutionEngine::with_memory()
    12:  }

Command:   grep -n "fn create_engine" tests/crash_recovery_test.rs | head -1
Output:    Returns MemoryExecutionEngine

Observation:
  MemoryStorage lifecycle = Process Exit → Data Gone
  Crash Recovery requires   = Process Exit → Disk → Restart → Data Exists
  Conclusion: Test cannot verify persistence requirement
```

### E-3: No FileStorage E2E Recovery Test

```
Command:   find . -name "*.rs" -exec grep -l "FileStorage.*restart\|FileStorage.*crash\|e2e.*recovery" {} \; 2>/dev/null
Output:    (empty — no results)

Command:   grep -r "fn.*recovery.*test\|#\[test\]" tests/ crates/storage/tests/ 2>/dev/null | grep -i crash
Output:    crash_recovery_test.rs lines 1-119
           All use MemoryExecutionEngine
```

---

## Impact

```
Crash Recovery Correctness Status = UNKNOWN

Not Proven:
- INSERT survives restart
- UPDATE survives restart
- DELETE survives restart
- COMMIT durability
```

---

## Required Evidence for Closure (RB-1 Gate)

| Requirement | Observable Behavior | Test Design | Assertion | Evidence |
|-------------|---------------------|--------------|-----------|----------|
| INSERT survives recovery | Restart后SELECT返回该行 | INSERT→COMMIT→Crash→Restart→SELECT | `value == inserted_value` | `cargo test e2e_recovery_insert` PASS + FileStorage |
| UPDATE survives recovery | Restart后SELECT返回更新值 | UPDATE→COMMIT→Crash→Restart→SELECT | `value == updated_value` | `cargo test e2e_recovery_update` PASS + FileStorage |
| DELETE survives recovery | Restart后SELECT不返回该行 | DELETE→COMMIT→Crash→Restart→SELECT | `row == None` | `cargo test e2e_recovery_delete` PASS + FileStorage |

**Until all three rows show Evidence: PASS, Status = UNVERIFIED.**

---

## Not Stating (Hypothesis — Not Yet Proven)

- "WAL has bug"
- "Recovery is broken"
- Any root cause claim

---

## Resolution Path

```
1. Create FileStorage-based E2E crash recovery test
2. Test pattern: BEGIN → DML → COMMIT → STOP → RESTART → SELECT → Assert
3. Attach cargo test output as Evidence
4. Verify UPDATE replay is real code (not stub)
```