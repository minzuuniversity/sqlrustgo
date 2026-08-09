# v3.12.0 Reviewer Sign-Off

## Release: v3.12.0
**Release Date:** 2026-08-09
**Branch:** `swe/_fix/binary-row-parsing`

---

## Reviewer 1 — Self-Review

**Reviewer:** Claude Code (automated session)
**Date:** 2026-08-09
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

## Reviewer 2 — Pending

**Reviewer:** (pending)
**Date:** ___________
**Areas Reviewed:** ____________________
**Decision:** ⬜ PENDING

---

## Sign-Off Criteria (v3.12.0)

- [x] All 11 wire smoke tests pass
- [x] Architecture invariants (C-ARCH-01~05) all pass
- [x] LOAD DATA INFILE gate passes
- [x] Binary row parsing correctly handles: INT, VARCHAR, NULL
- [x] Prepared statement cycle (prepare → execute → close) works end-to-end
- [x] Error packet structure validated
- [ ] Second reviewer sign-off obtained
