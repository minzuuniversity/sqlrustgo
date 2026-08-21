# V312-59-C RC8 — Crash Recovery + Upgrade/Downgrade Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[8]` — "Crash recovery and upgrade/downgrade reports exist"
**Verdict**: ✅ PASS
**Wrapper for**: `docs/releases/v3.12.0/crash-recovery-upgrade-verification-report.md`

---

## Source evidence

Primary report: `docs/releases/v3.12.0/crash-recovery-upgrade-verification-report.md`
- Source issue: #3901 (V312-14)
- Merge commit: `ac4c82b6f` (V312-14)
- Generated: 2026-08-09
- Anti-Fabrication-Policy-v1.0: applied

## Coverage scope

### Crash scenarios (7)

1. SIGKILL during INSERT
2. SIGKILL during COMMIT
3. SIGKILL during ROLLBACK
4. SIGKILL during LOAD DATA
5. Power loss (kill -9 + sync)
6. Disk full during INSERT
7. OOM during query

### Upgrade scenarios (4)

1. v3.11.0 → v3.12.0 (in-place binary swap)
2. v3.10.0 → v3.12.0 (skip-version upgrade)
3. v3.11.0 → v3.12.0 with WAL replay
4. v3.11.0 → v3.12.0 with partial schema migration

### Downgrade scenarios (4)

1. v3.12.0 → v3.11.0 (backward-compatible path)
2. v3.12.0 → v3.10.0 (multi-version downgrade)
3. v3.12.0 → v3.11.0 with WAL rollback
4. v3.12.0 → v3.11.0 with audit chain preservation

## Verification invariant

Every scenario reports:
- recovered_row_count == original_row_count (within ±0 for non-concurrent, ±5 for concurrent SKIP handling)
- WAL replay integrity preserved
- Audit hash chain continuity (genesis → tip)
- Embedding + graph relation parity (where applicable)

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via:
- `CRASH_RECOVERY_REQUIRED=true` → PASS (via `check_v312_14_crash_recovery.sh`)
- `UPGRADE_DOWNGRADE_REQUIRED=true` → PASS (via `check_p14_upgrade_test.sh` after path fix)

## RC8 verdict for V312-59-C composite gate

```
[8/11] RC8_CRASH_UPGRADE
  [EVIDENCE_FILE]      PASS
  [INTEGRATION_TEST]   N/A (7+4+4 scenarios in upstream V312-14)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[8]`.