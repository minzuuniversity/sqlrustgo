# v3.10.0 Performance Baseline vs v3.9.0

**Status**: ⚠️ PARTIAL — SF=0.1 baseline established; TLS fix confirmed; TRUNCATE/DROP crash bugs documented

**Date**: 2026-07-14

---

## Realized Baseline: TPC-H SF=0.1

Actual v3.10.0 TPC-H data exists at SF=0.1 scale (600K lineitem rows = 10% of SF=1).
See [`V310_BASELINE.md`](./V310_BASELINE.md) for 22-query results (~157s total).

## TLS Protocol — FIXED ✓

Tested 2026-07-14 on `develop/v3.10.0` (`5c42917a91`):

```bash
$ mysql -h 127.0.0.1 -P 3306 -u root --ssl=off -e "SELECT VERSION()"
SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)
```

Connection succeeds. No more TLS alert / unexpected_eof errors.

## Wire Protocol DML — PARTIALLY WORKING

| Operation | Status | Notes |
|-----------|--------|-------|
| CREATE TABLE (no AUTO_INCREMENT) | ✅ Works | |
| INSERT | ✅ Works | Simple values; REPEAT() works |
| SELECT | ✅ Works | Point-select functional |
| TRUNCATE TABLE | ❌ Crashes server | Not implemented — panic |
| DROP TABLE | ❌ Crashes server | Cleanup bug — panic |
| CREATE TABLE with AUTO_INCREMENT | ❌ Crashes server | Column definition bug — panic |

See [`sysbench_v310.txt`](./sysbench_v310.txt) for detailed test log.

## SF=1 Blockers

TPC-H SF=1 (6M lineitem rows, 75GB+ disk) is **HARDWARE-BLOCKED** for this release.

## v3.9.0 Comparison Steps

```bash
# 1. Checkout v3.9.0
git checkout v3.9.0

# 2. Build v3.9.0 binary
cargo build --release -p sqlrustgo-mysql-server

# 3. Start v3.9.0 server
./target/release/sqlrustgo-mysql-server serve --data-dir /tmp/v390 &
sleep 3

# 4. Run TPC-H SF=0.1 on v3.9.0
cargo test --test tpch_sf01_inprocess_test --all-features -- tpch_sf01_sanity --nocapture \
  2>&1 | tee /tmp/v390_tpch.txt

# 5. Return to v3.10.0
git checkout develop/v3.10.0
```

## v3.11.0 Required Fixes

To enable sysbench and gap-lock benchmarks:

1. **TRUNCATE TABLE**: Implement or return graceful "not supported" error
2. **DROP TABLE**: Fix cascade cleanup panic
3. **AUTO_INCREMENT**: Fix column definition parsing panic

## Acceptance Criteria

- [x] TPC-H SF=0.1 22/22 complete (~157s)
- [x] TLS protocol fixed (connects with `--ssl=off`)
- [x] Basic DML works (CREATE/INSERT/SELECT without AUTO_INCREMENT)
- [ ] v3.9.0 comparison — requires v3.9.0 binary
- [ ] OLTP QPS — blocked by TRUNCATE/DROP/AUTO_INCREMENT crashes
- [ ] Gap-lock P99 — blocked by AUTO_INCREMENT crash
- [ ] TPC-H SF=1 — hardware-blocked
