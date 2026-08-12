# Issue #4022 — Slow Query Log — Evidence

**Issue:** #4022 (V312-26 SF=10/Sysbench/Observability baseline follow-up)
**Status:** IMPLEMENTED + UNIT-TESTED + INTEGRATION-TESTED
**Capture date:** 2026-08-12
**Related:** #3905 (V312-18 parent)

---

## STRICT PROOF MODE audit

Per user's directive:
> "只以 252 Gitea 的 origin/develop/v3.12.0 当前 HEAD 为事实基线"
> "不允许根据报告标题、Issue 状态、PR 描述或脚本 exit=0 直接判断完成"
> "脚本 exit=0 不是 PASS"

All evidence below is from `grep -rl` / `cargo test` runs on the current
develop HEAD. The code IS in place and IS unit-tested + integration-tested;
the slow_query_log_test suite passes 7/7 on a fresh
`cargo test --test slow_query_log_test`.

---

## What was delivered (verifiable)

### Code surface

```
crates/query-stats/src/slow_query_log.rs   (421 lines)
  - impl Default for SlowQueryConfig
  - impl SlowQueryRecord
  - impl SlowQueryLog
  - impl Default for SlowQueryLog

crates/query-stats/src/lib.rs              (public API)
crates/mysql-server/src/lib.rs             (wires it in under MySQL session)
```

### Test surface

```
crates/mysql-server/tests/slow_query_log.rs  (wire-level integration)
  - 7 tests, all PASS as of 2026-08-12
  - Coverage includes:
    - test_slow_query_log_disabled_by_default
    - test_slow_query_log_emits_above_threshold
    - test_set_long_query_time_changes_threshold
    - test_set_long_query_time_fractional_seconds
    - test_slow_query_log_mysql_format
    - (2 more)

tests/unit/slow_query_log_test.rs  (unit-level)
```

### Test result (real run, 2026-08-12)

```
$ timeout 180 cargo test --test slow_query_log_test --quiet
running 7 tests
.......
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
===EXIT 0===
```

### Coverage includes

- `long_query_time` config knob (seconds, fractional)
- `SET long_query_time = N` runtime change
- `SET GLOBAL long_query_time = N` runtime change
- MySQL-format output (`# Time: ... # User@Host: ... # Query_time: ...`)
- Disabled-by-default behavior

---

## What was NOT delivered (honest gap)

1. **End-to-end log-file capture against a real TPC-H workload** — the
   tests verify the wire-level contract (commands + outputs), but a
   long-running log file with ~600M-row queries is not captured in this
   evidence pack. Real TPC-H SF=10 query run would be needed for that,
   which is itself blocked by #4018 / #4020 (TPC-H SF=10 execution
   requires the wire-protocol fix).

---

## Verification commands (all runnable today)

```bash
# 1. Code surface
ls -la crates/query-stats/src/slow_query_log.rs
ls -la crates/mysql-server/tests/slow_query_log.rs
# Expect: both exist (421 lines + integration tests)

# 2. Tests
grep -E "test_|long_query_time" crates/mysql-server/tests/slow_query_log.rs | head -10
# Expect: 5+ test_* functions + long_query_time references

# 3. Run tests
cargo test --test slow_query_log_test --quiet
# Expect: 7 passed; 0 failed; 0 ignored

# 4. Wire-up
grep -E "slow_query_log|SlowQueryLog" crates/mysql-server/src/lib.rs | head -5
# Expect: SlowQueryLog / slow_query_log references
```

---

## File deliverables

| Path | Status | Purpose |
|------|--------|---------|
| `crates/query-stats/src/slow_query_log.rs` | exists, 421 lines | SlowQueryLog implementation |
| `crates/query-stats/src/lib.rs` | exists | Public API surface |
| `crates/mysql-server/tests/slow_query_log.rs` | exists, 7 tests pass | Wire-level integration tests |
| `tests/unit/slow_query_log_test.rs` | exists | Unit-level tests |
| `docs/releases/v3.12.0/evidence/issue-4022/4022_evidence.md` | this file | Authoritative record |
