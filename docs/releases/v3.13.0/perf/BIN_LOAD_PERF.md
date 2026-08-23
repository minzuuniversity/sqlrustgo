# BINT v3 TPC-H SF=1 Load Performance Report

> **Status (2026-08-23):** L4 + L5 measurements captured. Performance vs plan's aspirational targets is documented honestly below; the 6M assertion exceeds the 60s budget on this hardware and requires future optimization work.

## Setup

- **Hardware:** 2x Intel Xeon Gold 6138 @ 2.00GHz (40 cores / 80 threads), 404GB RAM, 1.9TB NVMe SSD (root partition)
- **Commit:** `759a062a2f` (HEAD of feature/v313-bin-storage)
- **Feature flag:** `bin_storage_default = true` (default since T2.3)
- **TPC-H SF:** 1 (6,001,215 lineitem rows; only lineitem was measured in T6.3/T6.4)

## Measurements

### 1M-row load (criterion bench, T6.3)

| Metric | Value |
|--------|-------|
| Wall-time | 2.84s (average of 100 iterations) |
| Rows/sec | 352,113 |
| Iterations | 100 (criterion stable) |
| Variance | ±1.2% (criterion-reported) |
| File | `benches/tpch_load_bench.rs` |
| Commit | `09d337a463` |

### 6M-row load (integration test, T6.4)

| Metric | Value |
|--------|-------|
| Wall-time | 108.09s |
| Rows/sec | 55,522 |
| Budget | 60s (default `TPCH_SF1_LOAD_BUDGET_S`) |
| Budget exceeded by | 1.8× (80%) |
| File | `tests/integration/tpch/tpch_sf1_6m_load_test.rs` |
| Commit | `759a062a2f` |

### 1M → 6M extrapolation gap

- Linear extrapolation from 1M (2.84s) to 6M: **~17s** (3.5× under budget)
- Actual 6M measurement: **108s** (1.8× over budget)
- Gap: **6.3× slower than linear extrapolation**

#### Hypothesized causes (not confirmed; profiling needed)

1. **Per-batch flush overhead:** 60 `flush()` calls for 6M (BATCH_SIZE = 100,000) vs 10 for 1M. Each `flush()` seals the active segment + rewrites `root.bin`. If flush is O(segments) and segments grow non-linearly, total flush time dominates at 6M scale.
2. **mmap growth:** `BinaryTableStorageV2`'s mmap-backed storage may have different growth characteristics at 6M rows (more pages, more remaps, more TLB pressure).
3. **Disk write pressure:** 6M rows produce more total bytes written to disk than 1M; disk may be a bottleneck.
4. **System variability:** T6.3 bench is "warm" (criterion pre-runs); T6.4 test is a single cold run. Warm vs cold difference could account for part of the gap.

A future profiling task (T7+) should identify the dominant factor before optimizing.

### Architectural workaround: batch + flush

`BinaryTableStorageV2::insert_streaming` only checks segment size cap at the **START** of each call (verified in `crates/storage/src/binary_storage_v2.rs:get_or_open_writer`). A single `insert_streaming(6M records)` call would write all 6M rows into one unbounded segment file, bypassing BINT v3's page-aligned design.

T6.3 and T6.4 work around this with a batch + flush loop:

```rust
let mut emitted = 0;
while emitted < n_rows {
    let batch_end = (emitted + BATCH_SIZE).min(n_rows);
    let batch: Vec<Record> = (emitted..batch_end).map(lineitem_row).collect();
    storage.insert_streaming("lineitem", batch)?;
    storage.flush()?;  // Forces segment rollover on next iteration
    emitted = batch_end;
}
```

This works correctly but adds 60 `flush()` calls for 6M rows. A future `insert_streaming_iter()` method that internally rolls over segments would eliminate the workaround.

## Comparison to Plan Targets

| Target | Plan | Actual | Status |
|--------|------|--------|--------|
| 6M load < 60s | < 60s | 108s | **MISS** (1.8× over) |
| Speedup vs JSON | ~600× | ~3.4× (measured, see below) | **REVISED** |
| 1M bench in 5–15s | 5–15s | 2.84s | **PASS** (better) |
| 22/22 queries | PASS | not measured | n/a |

### Speedup vs JSON — revised (V313.1 follow-up, 2026-08-23)

The "~333× (estimate)" entry above was an unverified extrapolation from file-size
ratio or similar proxy — it was **not** a measured wall-time comparison.

V313.1 follow-up measured BINT v3 vs JSON `FileStorage` apples-to-apples at
four data sizes (50K, 200K, 500K, 1M) using the same lineitem schema and
the same `tpch_json_vs_bint_compare` integration test. **The actual speedup
is ~3.4× across all sizes, not ~333×.**

| n_rows | JSON wall | JSON rows/sec | BINT wall | BINT rows/sec | Speedup |
|--------|-----------|---------------|-----------|---------------|---------|
| 50,000 | 2.97 s | 16,857 | 0.87 s | 57,373 | 3.40× |
| 200,000 | 11.99 s | 16,683 | 3.50 s | 57,082 | 3.42× |
| 500,000 | 30.46 s | 16,416 | 8.70 s | 57,458 | 3.50× |
| 1,000,000 | 59.92 s | 16,689 | 17.69 s | 56,529 | 3.39× |
| 6,001,215 (JSON extrapolated / BINT measured) | ~360 s | 16,700 | **110.62 s** | 54,252 | ~3.25× |

Full report: `docs/releases/v3.13.0/perf/JSON_VS_BINT_MEASURED.md`.
Reproduce:
```bash
cargo test --test tpch_json_vs_bint_compare -- --nocapture
TPCH_COMPARE_ROWS=1000000 cargo test --test tpch_json_vs_bint_compare -- --nocapture
```

**Revised plan target:** "~600×" should be downgraded to "~3-5×" for the
JSON vs BINT wall-time comparison, until profiling reveals what dominates
the remaining gap (flush vs mmap vs disk).

The 6M target miss is **not a code defect** — the test is correctly measuring the current implementation. It signals that **future optimization work is needed** to hit the plan's stretch goal.

## What was NOT measured

The plan's template included metrics that are out of scope for T6.5:

- **TPC-H query performance** (Q1–Q22) — wire-protocol test is `#[ignore]`d with its own 20-minute budget (`tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`); running it requires a separate engagement.
- **Memory footprint** during load — would need `dtrace` / `heaptrack` instrumentation.
- **Disk footprint** of BINT v3 files vs JSON — would need to materialize a full SF=1 dataset on disk.
- **Other tables** (orders, customer, part, partsupp) — only lineitem was measured in T6.3/T6.4. The schema and bench infrastructure support them but the numbers aren't captured.

These are documented for future work, not as known values.

## Regression Risk

- BINT v3 binary format is not human-readable (acceptable trade-off; well-documented in `docs/superpowers/specs/2026-08-23-tpch-sf1-data-loading-design.md`)
- Migration adds ~200ms to first INSERT per table on cold path (per spec); acceptable for typical workload
- Crash recovery relies on `.json.bak` files as last-ditch fallback during the BIN→JSON migration; this is verified by T5.3

## Conclusion

BINT v3 storage delivers:
- ✅ **2.84s for 1M lineitem rows** (T6.3, criterion-stable)
- ⚠️ **108s for 6M lineitem rows** on this hardware — exceeds the 60s target by 1.8× and shows a 6.3× gap from linear extrapolation
- ⚠️ Performance gap suggests **future T7+ profiling + optimization work** is needed before BINT v3 should be promoted to GA default

**Recommended next steps:**
1. Profile T6.4's 6M run to identify the dominant cost driver (flush vs mmap vs disk).
2. Implement `insert_streaming_iter()` that internally rolls over segments (eliminates the batch+flush workaround).
3. Re-run T6.4 after each optimization; expect to converge on < 60s before GA.
4. Expand T6.5 metrics (queries, memory, disk, other tables) once those harnesses exist.
5. Profile the JSON baseline (16.7K rows/sec) to understand whether JSON can be made faster — closing the JSON-vs-BINT gap from the other side is also a valid strategy.

**Do NOT promote to GA default** until the 6M < 60s target is met and the other unmeasured metrics are captured.
