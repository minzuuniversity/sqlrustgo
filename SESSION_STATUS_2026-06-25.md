# Session Status — 2026-06-25

## What Was Done

### 1. Soak Test Infrastructure (Issue #3225)
- **Problem**: MariaDB occupies port 3306 on this machine; `start_sf01()` harness fails with EAGAIN/os error 35; binary starts with TLS by default (raw Python socket fails)
- **Solution**: Wrote `/tmp/soak.py` using `mysql` CLI with `--skip-ssl` flag
  - Server: `./target/release/sqlrustgo-mysql-server serve --port 3497`
  - 5m soak: **5532 queries, 0 failed, 100% success, QPS=18.4, p99=57ms**
- **Added**: `tests/soak_ladder_test.rs` (4x `#[ignore]` tests: 5m/10m/20m/30m) — committed as `43736b9e1`

### 2. Branch Cleanup
- Merged `origin/develop/v3.9.0` (6 commits including TPC-H wire migration #3271 done by another agent)
- `develop/v3.9.0` pushed and up-to-date with origin
- `fix/connect-with-caps` force-pushed with soak_ladder_test commit

### 3. Test Verification
- `tpch_wire_smoke`: 16/16 PASS ✅ (6x consecutive loop, no failures)
- `qps_benchmark_test`: 10/10 PASS ✅ (8.8s, 10k iterations each)
- `perf_eng_batched_insert_test`: 14/14 PASS ✅
- `oracle_g12_sysbench`: 15/15 PASS ✅
- `mysql_wire_protocol_test`: 26/26 PASS ✅
- `wire_deprecate_eof_test`: 16/16 PASS ✅
- Full `--all-features --tests` build: **CLEAN** ✅

## What Remains

### Issue #3225 (72h soak) — BLOCKED by Z6G4 network
- Z6G4 at 192.168.0.252 unreachable (SSH timeout 5s)
- Soak ladder script available at `/tmp/soak.py` and `tests/soak_ladder_test.rs`
- Local 5m PASS (see above); longer durations need Z6G4

### Issue #3312 (un-ignore long tests) — BLOCKED by #3225
- Needs 72h soak PASS report before un-ignoring `long_run_stability_72h_test`
- All other ignored long tests (`qps_benchmark_test`, `perf_eng_batched_insert_test`) already pass when un-ignored

### Issue #3271 (TPC-H wire migration) — RESOLVED
- Merged in 6 commits from `origin/develop/v3.9.0`:
  - `tpch_q9_audit` → wire protocol
  - `tpch_bug_regression_test` → wire protocol
  - `tpch_value_correctness_test` → wire protocol (COUNT fix 0→501)
  - `tpch_value_test_v2` → wire protocol (22 queries vs SQLite baseline)

## Key Findings
1. Binary defaults to TLS — use `mysql --skip-ssl` for plain connections
2. MariaDB on port 3306 must be killed before harness tests can use default port
3. EAGAIN/os error 35 is pre-existing bug #3307 (already documented)
4. All compilation blockers from prior sessions are resolved
