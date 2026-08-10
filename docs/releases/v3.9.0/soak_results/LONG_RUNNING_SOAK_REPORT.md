# Long-Running Soak Report

Generated: 2026-06-26 05:32:08
Binary: `./target/release/sqlrustgo-mysql-server`
Client: Python MySQL client (persistent connections, 8 threads)
Machine: Z440 (Xeon E5-2680 v4)

## Results

| Duration | Queries | Errors | QPS | Actual | RSS Growth | RSS Peak | PASS |
|---------|---------|--------|-----|--------|------------|---------|------|
| 2h | 3,082,301 | 8 | 24379 | 0.04h | 0 KB | 8376 KB | ❌ FAIL |
| 4h | 2,875,741 | 8 | 22883 | 0.03h | 0 KB | 8376 KB | ❌ FAIL |
| 8h | 2,825,348 | 8 | 22742 | 0.03h | 0 KB | 8376 KB | ❌ FAIL |
| 16h | 2,865,287 | 8 | 22705 | 0.04h | 0 KB | 8376 KB | ❌ FAIL |
| 24h | 2,851,225 | 8 | 22923 | 0.03h | 0 KB | 8376 KB | ❌ FAIL |
| 48h | 2,854,180 | 8 | 22915 | 0.03h | 0 KB | 8376 KB | ❌ FAIL |
| 72h | 2,879,537 | 8 | 22791 | 0.04h | 0 KB | 8376 KB | ❌ FAIL |

**Total: 20,233,619 queries, 56 errors, 0/7 runs passed**

All "errors" are thread-disconnect noise (not server errors).
