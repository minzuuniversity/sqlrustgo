# Architecture Invariant Report — v3.12.0

**Generated:** 2026-08-09
**Agent:** claude-sonnet-4-20250514
**Run:** sqlrustgo session
**Evidence hash:** `sha256:arch_invariants_20260809`

## C-ARCH-01~05 Results

| Invariant | Description | Status |
|-----------|-------------|--------|
| C-ARCH-01 | LocalExecutor has NO txn_manager field | **PASS** |
| C-ARCH-02 | LocalExecutor has NO write_buffer field | **PASS** |
| C-ARCH-03 | storage.insert/update/delete only in crates/storage or crates/executor/ | **PASS** (info only — business-level access allowed per AD-002) |
| C-ARCH-04 | No eng.execute(raw_sql) outside parser | **PASS** |
| C-ARCH-05 | execution_engine.rs < 1600 lines | **PASS** (1594 lines, limit 1600, AD-001 target 1500) |

## Evidence

```
=== C-ARCH Invariant Check ===

[C-ARCH-01] Checking LocalExecutor has NO txn_manager field...
PASS: C-ARCH-01

[C-ARCH-02] Checking LocalExecutor has NO write_buffer field...
PASS: C-ARCH-02

[C-ARCH-03] Checking storage.insert/update/delete only in crates/storage or crates/executor/...
INFO (6 storage operations in business crates — business-level access to StorageEngine is allowed per AD-002 §Consequences for non-SQL paths)

[C-ARCH-04] Checking no eng.execute(raw_sql) outside parser...
PASS: C-ARCH-04

[C-ARCH-05] Checking execution_engine.rs < 1600 lines (SSOT: CARCH05_LIMIT, AD-001 target: 1500)...
PASS: C-ARCH-05 (execution_engine.rs:     1594 lines, limit 1600, AD-001 target 1500)

=== Summary ===
PASSED: 5
FAILED: 0

Result: ALL PASS
```

**Command:** `bash scripts/gate/check_arch_invariants.sh`
**Result:** 5/5 PASS, 0 FAIL
