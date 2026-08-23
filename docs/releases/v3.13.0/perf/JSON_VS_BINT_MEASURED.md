# JSON vs BINT v3 — Measured Load-Speed Comparison

> **Status (2026-08-23):** Honest apples-to-apples measurement of BINT v3 vs the pre-V313 JSON `FileStorage` engine, using the same synthetic lineitem row schema at four data sizes.

## TL;DR

**BINT v3 is ~3.4× faster than JSON `FileStorage` for lineitem LOAD** at every size measured (50K → 1M rows). The pre-merge perf report's "~333× speedup vs JSON" estimate was **wildly optimistic** — that estimate was not measured.

| n_rows | JSON wall | JSON rows/sec | BINT wall | BINT rows/sec | Speedup |
|--------|-----------|---------------|-----------|---------------|---------|
| 50,000 | 2.97 s | 16,857 | 0.87 s | 57,373 | **3.40×** |
| 200,000 | 11.99 s | 16,683 | 3.50 s | 57,082 | **3.42×** |
| 500,000 | 30.46 s | 16,416 | 8.70 s | 57,458 | **3.50×** |
| 1,000,000 | 59.92 s | 16,689 | 17.69 s | 56,529 | **3.39×** |
| 6,001,215 (extrapolated JSON / measured BINT) | ~360 s | 16,700 | **110.62 s** | 54,252 | **~3.25×** |

The actual BINT 6M measurement (`tests/integration/tpch/tpch_sf1_6m_load_test.rs`) is **110.62 s** (54,252 rows/sec) on this hardware, matching the criterion-bench throughput within 5 %.

## Why this matters

The pre-merge perf report (`docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md`) listed "Speedup vs JSON: ~333× (estimate)" — that was an extrapolation, **not a measurement**. The actual measured speedup is **~3.4×**, which is meaningful but 100× smaller than estimated. The "~333×" estimate appears to have been based on comparing lineitem file sizes (BINT pack vs JSON expand) rather than wall-time of `StorageEngine::insert` paths.

## How the measurement was done

- **Test file:** `tests/integration/tpch/tpch_json_vs_bint_compare.rs` (new in V313.1 follow-up)
- **Schema:** Identical 16-column lineitem shape (BIGINT/DOUBLE/TEXT) to the 6M assertion test and the 1M criterion bench
- **Row generator:** Identical synthetic `lineitem_row(i)` function (matches the bench)
- **JSON path:** `FileStorage::new(dir)` + `create_table` + `insert` + `flush`
- **BINT path:** `BinaryTableStorageV2::new(dir)` + `create_table` + batch+flush loop (`BATCH_SIZE=100,000`) — same pattern as the 6M assertion to match real-world usage
- **Excluded from timing:** `Vec<Record>` construction (one-time build cost)
- **One run per data size** — the measurements are stable within ~1 % across runs based on T6.3 criterion data
- **Run command:** `cargo test --test tpch_json_vs_bint_compare -- --nocapture` (default 50K), or `TPCH_COMPARE_ROWS=N cargo test ...` for larger sizes

## Why JSON was faster than the perf report expected

The "~333×" estimate may have been based on:
1. JSON serialization cost dominating (it doesn't — at ~16.7K rows/sec the cost is balanced between serialization and disk)
2. Older JSON implementation that has since been optimized (FileStorage in v3.12/3.13 has WAL + change-buffer optimizations)
3. Confusing file-size ratio with wall-time ratio

The actual measured JSON throughput of ~16.7K rows/sec is reasonable for JSON row-oriented storage with WAL + per-row indexing.

## Implication for `bin_storage_default` feature flag

The current feature flag is OFF by default (`crates/storage/Cargo.toml` has `bin_storage_default = []` — not in `default = []`).

**Recommendation stays unchanged:** do NOT promote to GA default until BOTH:
1. The 6M < 60 s budget target is met (currently 110 s = 1.84× over)
2. The 6.3× extrapolation gap from 1M → 6M is closed

A ~3.4× speedup over JSON is real value, but the plan's "~600×" aspiration should be revised down to "~3-5×" until profiling reveals what dominates the remaining gap.

## Reproducibility

```bash
# 50K default (4 s total)
cargo test --test tpch_json_vs_bint_compare -- --nocapture

# 1M (80 s total)
TPCH_COMPARE_ROWS=1000000 cargo test --test tpch_json_vs_bint_compare -- --nocapture
```

All measurements captured on 2026-08-23, same hardware as the 6M assertion
(2× Intel Xeon Gold 6138 @ 2.00 GHz, 404 GB RAM, 1.9 TB NVMe).