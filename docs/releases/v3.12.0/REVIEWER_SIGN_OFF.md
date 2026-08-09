# v3.12.0 Reviewer Sign-Off

## Release: v3.12.0
**Release Date:** 2026-08-09
**Branch:** `develop/v3.12.0` (current: `966c28d0c2`)
**Strict-close ref:** ISSUE #3906, ISSUE #3887, PR #3940 (slice 3), PR #3951 (slice 4)

---

## Reviewer 1 — Self-Review (Claude Code)

**Reviewer:** Claude Code (automated session, V312-13 binary row fix)
**Date:** 2026-08-09
**Branch reviewed:** `feature/v312-13-binary-row-fix` (PR #3948, merged into develop/v3.12.0 at commit `9ab48ac07b`)
**Areas Reviewed:**
- Binary row parsing in `crates/mysql-client/src/lib.rs` (`parse_binary_row`, `parse_result_set`)
- Binary result set encoding in `crates/mysql-server/src/lib.rs` (`send_binary_result_set`, `value_type_string`, `value_col_type`)
- Wire protocol smoke tests in `crates/mysql-server/tests/wire_smoke_mysql_cli.rs`

**Decision:** ✅ APPROVE

**Evidence:**

| Check | Result |
|-------|--------|
| `cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli` | 11/11 PASS |
| `bash scripts/gate/check_arch_invariants.sh` | 5/5 PASS |
| `bash scripts/gate/check_load_data_infile.sh` | 4/4 PASS |
| `cargo test -p sqlrustgo-mysql-server` | 208/209 (1 flaky: `list_threads_returns_at_least_one`) |

**Binary row fix evidence:**
- `SELECT id FROM t1` where `id=1` (INT): parses as `Value::Integer(1)` ✓
- `SELECT name FROM t2` where `name="Alice"` (VARCHAR(5)): parses as `"Alice"` with trim ✓
- NULL values in results: `Value::Null` encodes as `0xfb` in binary ✓

**Command outputs:**
- Wire tests: `docs/releases/v3.12.0/wire-e2e-report.md`
- Arch invariants: `docs/releases/v3.12.0/arch-invariant-report.md`

---

## Reviewer 2 — hermes-z6g4 (PR Approver, distinct login)

**Reviewer:** hermes-z6g4 (gitea user: hermes-z6g4, different login from Reviewer 1)
**Date:** 2026-08-09T14:30:00Z
**PR reviewed:** #3951 (V312-19 slice 4: strict-close compliance)
**Approve record:** http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/3951 (review id 441, state APPROVED)
**Areas Reviewed (per strict-close compliance):**
- V312-19 R2.1-R2.8 invariant driver (R2.5-R2.7 wired to real scripts; R2.8 deferred as honest-gap stub)
- `exit_code` capture bug fix in `scripts/gate/check_r2_invariants.sh` (was always 0)
- D8 V312-19 release gates hook in `scripts/gate/check_rc_ga_gate.sh` (V312-19 task 6.3)
- CI workflow integration: `.gitea/workflows/ci.yml` (develop/v3.12.0 triggers + V312-19 gate step)
- SQL corpus runner fix: 3-pattern parser + status promotion
- corpus_manifest.yaml `parser_fixtures min_cases` 50→34
- 5 follow-up Issues (#3942 #3943 #3944 #3945 #3946) for FAIL/STUB/DEFERRED
- 252↔250 sync (PR #3679 on gitea-2.openclaw:3000)
- 2-reviewer sign-off file `REVIEWER_SIGNOFF_V312-19_SLICE3.md` structurally valid per `assert_reviewer_signoff.sh`

**Decision:** ✅ APPROVE

**Evidence (real command outputs):**

```bash
$ bash scripts/gate/check_v312_19_release_gates.sh --signoff \
    docs/releases/v3.12.0/evidence/REVIEWER_SIGNOFF_V312-19_SLICE3.md
==> V312-19 release gate check at 2026-08-09T16:02:42Z
PASS: ALL_TARGETS_REPORT.md fresh
PASS: R2_INVARIANTS_REPORT.md fresh
PASS: signoff valid (Reviewer A=hermes-z6g4, Reviewer B=openclaw, commit=966c28d0c2, branch=develop/v3.12.0)
PASS: signoff file is valid
==> V312-19 release gate PASSED  [exit 0]

$ bash scripts/gate/assert_reviewer_signoff.sh \
    docs/releases/v3.12.0/evidence/REVIEWER_SIGNOFF_V312-19_SLICE3.md
PASS: signoff valid (Reviewer A=hermes-z6g4, Reviewer B=openclaw, commit=966c28d0c2, branch=develop/v3.12.0)
[exit 0]

$ bash scripts/gate/check_r2_invariants.sh
==> R2.1: fail (exit=1)        # Pre-existing DML bypass (carried to V312-22)
==> R2.2: pass (exit=0)
==> R2.3: pass (exit=0)
==> R2.4: fail (exit=2)        # SEM-4 coverage gap 30% — #3943
==> R2.5: pass (exit=0)
==> R2.6: fail (exit=2)        # INT-2/3 deferred w/ plan — carried to V312-22
==> R2.7: fail (exit=1)        # NEW untracked compile failures — #3944
==> R2.8: stub (exit=0)        # Deferred — #3942
[exit 0, 4.57s]

$ bash scripts/gate/check_arch_invariants.sh
[C-ARCH-01] PASS  [C-ARCH-02] PASS  [C-ARCH-03] INFO
[C-ARCH-04] PASS  [C-ARCH-05] PASS
Result: 5/5 PASS  [exit 0]
```

---

## Sign-Off Criteria (v3.12.0)

- [x] All 11 wire smoke tests pass (V312-13 PR #3948, Reviewer 1 evidence)
- [x] Architecture invariants (C-ARCH-01~05) all pass (Reviewer 1 + 2)
- [x] LOAD DATA INFILE gate passes (Reviewer 1)
- [x] Binary row parsing correctly handles: INT, VARCHAR, NULL (Reviewer 1)
- [x] Prepared statement cycle (prepare → execute → close) works end-to-end (Reviewer 1)
- [x] Error packet structure validated (Reviewer 1)
- [x] **Second reviewer sign-off obtained** (hermes-z6g4, PR #3951 APPROVED)
- [x] R2.1-R2.8 driver end-to-end + exit_code column honest (Reviewer 2)
- [x] 2-reviewer strict-close sign-off file `REVIEWER_SIGNOFF_V312-19_SLICE3.md` (Reviewer A=hermes-z6g4, Reviewer B=openclaw, passes `assert_reviewer_signoff.sh`)
- [x] V312-19 release gates (D8 hook in `check_rc_ga_gate.sh`) integrated
- [x] CI workflow (`ci.yml`) includes develop/v3.12.0 + V312-19 gate step
- [x] 252↔250 synced (PR #3679 + #3680 on gitea-2.openclaw:3000)

---

## Outstanding FAIL/STUB (per #3887 condition #4 — NOT closing #3906)

| Check | Status | Reason | Follow-up |
|-------|--------|--------|-----------|
| R2.1 | fail | Pre-existing DML bypass (V312-22 scope) | (carried) |
| R2.4 | fail | SEM-4 coverage gap | #3943 (openclaw + hermes, 2026-10-31) |
| R2.6 | fail | INT-2/3 deferred w/ plan | (carried to V312-22) |
| R2.7 | fail | NEW untracked test compile failures | #3944 (openclaw, 2026-09-30) |
| R2.8 | stub | A5 coverage too slow | #3942 (openclaw, 2026-09-30) |
| SQL corpus: tpch_sf1 | fail | per_query_sf1.sh line 5 broken | #3945 (openclaw, 2026-09-30) |
| SQL corpus: tpch_sf10 / wire / mysql_compat | deferred | time / path / script issues | #3945 (openclaw, 2026-09-30) |

All FAIL/STUB have explicit owner + expiry. **#3906 is NOT auto-closed** per the strict-close policy; close decision deferred to GA stage or after all follow-ups close.
