# v3.10.0 vs v3.9.0 Performance Comparison

**Status**: ⏳ PENDING — v3.9.0 binary not tested; v3.10.0 data partially available

**Last updated**: 2026-07-14

## v3.10.0 Test Results (actual)

### Platform
- MacBook Pro M2, macOS Darwin 25.5.0
- sqlrustgo-mysql-server v3.10.0 (`develop/v3.10.0`, commit `5c42917a91`)
- TLS fix verified: connection works with `--ssl=off`
- Storage: file-based with WAL

### TPC-H SF=0.1 (in-process, existing baseline)

v3.10.0 TPC-H SF=0.1 results in [`V310_BASELINE.md`](./V310_BASELINE.md):
- Total time: ~157.71s (22 queries, geometric mean ~5.16s/query)
- All 22 queries PASS
- Data: 600,572 lineitem rows (= 10% of official TPC-H SF=1)

### OLTP QPS

BLOCKED — see [`sysbench_v310.txt`](./sysbench_v310.txt):
- TLS: FIXED ✓
- Basic DML: WORKS ✓ (CREATE TABLE, INSERT, SELECT)
- REPEAT() function: WORKS ✓
- TRUNCATE/DROP/AUTO_INCREMENT: CRASH server ✗

Without working TRUNCATE/DROP, sysbench cannot run its prepare/cleanup cycle.

### Gap-lock P99

BLOCKED — see [`gap_lock_v310.txt`](./gap_lock_v310.txt):
- Requires TPC-C tables with AUTO_INCREMENT
- AUTO_INCREMENT crashes server
- Concurrent transactions needed for lock contention measurement

## v3.9.0 Comparison Steps

To complete the comparison:

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

# 5. Compare total times
grep "TOTAL" /tmp/v390_tpch.txt

# 6. Return to v3.10.0
git checkout develop/v3.10.0
```

## Current Verdict

| Metric | v3.10.0 | v3.9.0 | Delta |
|--------|----------|---------|-------|
| TPC-H SF=0.1 total | ~157.71s | TBD | pending |
| OLTP QPS | BLOCKED | TBD | TRUNCATE/DROP crash |
| Gap-lock P99 | BLOCKED | TBD | AUTO_INCREMENT crash |
| TLS protocol | FIXED ✓ | N/A | — |
| Basic DML | WORKS ✓ | TBD | — |

## Z440's 168h SOAK Test (250)

Z440's soak test (250) ran for 168 hours on v3.10.0 without crash.
This test uses an in-process runner that bypasses the wire protocol
crash bugs. The in-process path does not trigger TRUNCATE/DROP/AUTO_INCREMENT
issues because it uses the storage engine API directly.

## Required Fixes for Full Comparison

1. Fix TRUNCATE TABLE (implement or return graceful error)
2. Fix DROP TABLE cleanup (prevent panic)
3. Fix AUTO_INCREMENT column definition (prevent panic)

Once fixed, sysbench and gap-lock tests become possible.
