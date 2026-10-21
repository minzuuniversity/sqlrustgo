# JSON vs BINT v3 — Measured Load-Speed Comparison

> **Status (2026-08-23, V313.1 follow-up):** the measured speedup of BINT v3
> over the JSON `FileStorage` load path is **~3.4×**, not the "~333×" that the
> original plan estimated. The plan target was revised to "~3–5×".
>
> **This file is the standalone version of the measurement table that
> `docs/releases/v3.12.0/perf/BIN_LOAD_PERF.md` §"Speedup vs JSON — revised"
> carries.** It exists because that section's "Full report" pointer named a
> file that was never written — a broken link in an otherwise complete
> evidence chain (issue #5499). The numbers below are **transcribed from
> that section**; they are not a second, independent measurement.
>
> **What is *not* here:** per-iteration timings, variance, and the raw
> per-size run logs. Those were never recorded — the V313.1 follow-up kept
> the aggregate table and the `TPCH_COMPARE_ROWS` sweep, not the individual
> samples. Any future claim about variance needs a fresh measurement, not
> an inference from this table.

## Setup

- **Hardware:** 2x Intel Xeon Gold 6138 @ 2.00GHz (40 cores / 80 threads),
  404GB RAM, 1.9TB NVMe SSD (root partition)
- **Commit:** `759a062a2f` (HEAD of feature/v313-bin-storage)
- **Feature flag:** `bin_storage_default = true`
- **Schema:** the TPC-H `lineitem` shape (16 columns) — the same schema used by
  `tests/integration/tpch/tpch_sf1_6m_load_test.rs` and
  `benches/tpch_load_bench.rs`, so the comparison is apples-to-apples
- **Storage paths compared:**
  - **JSON** — `FileStorage` (row-oriented, one file per row)
  - **BINT v3** — `BinaryTableStorageV2` (binary, sequential writes)

## Measurement method

The comparison is the `tpch_json_vs_bint_compare` integration test. For each
size it:

1. builds the row batch **once** (excluded from the timing),
2. times `insert()` + `flush()` through `FileStorage`,
3. times the streaming insert + `flush()` loop through
   `BinaryTableStorageV2` (batch size 100 000, matching the real SF=1 load
   pattern),
4. reports `rows/sec` for each side and the ratio `JSON_wall / BINT_wall`.

The row-generation cost is excluded so that both sides see the same input.

## Measured results

| n_rows | JSON wall | JSON rows/sec | BINT wall | BINT rows/sec | Speedup |
|--------|-----------|---------------|-----------|---------------|---------|
| 50,000 | 2.97 s | 16,857 | 0.87 s | 57,373 | 3.40× |
| 200,000 | 11.99 s | 16,683 | 3.50 s | 57,082 | 3.42× |
| 500,000 | 30.46 s | 16,416 | 8.70 s | 57,458 | 3.50× |
| 1,000,000 | 59.92 s | 16,689 | 17.69 s | 56,529 | 3.39× |
| 6,001,215 (JSON extrapolated / BINT measured) | ~360 s | 16,700 | **110.62 s** | 54,252 | ~3.25× |

The speedup is **stable across sizes** (3.25×–3.50×), which is what makes it
usable as a *performance contract* rather than a one-off number.

## Reproduce

```bash
cargo test --test tpch_json_vs_bint_compare -- --nocapture
TPCH_COMPARE_ROWS=1000000 cargo test --test tpch_json_vs_bint_compare -- --nocapture
```

`TPCH_COMPARE_ROWS` sets the row count (default 50 000).

## How this number is used today — and where it stops

The `2×` threshold in that test is a **performance contract**: it is only
meaningful on the hardware the measurement was taken on. On other hardware the
ratio measures the **storage media**, not the code — the JSON path is a
write-amplification path (one file per row plus an fsync) that collapses on
slow storage, while the BINT path is a sequential binary write whose advantage
narrows on fast storage. Measured on a different machine, the same ratio came
out at 0.92×–1.08×.

Since iteration 5 batch 8 (issue #5481) the test therefore evaluates that
contract **only** when the running machine declares itself the baseline
machine via `TPCH_PERF_BASELINE_HW`, and reports it as
`PERF-CONTRACT NOT EVALUATED` otherwise. A hardware-independent degradation
floor (the BINT write path must not be more than 2× slower than the JSON write
path) is enforced on every machine.

See `docs/releases/v4.1.0/evidence/TEST_DISCRIMINATING_POWER_BATCH8_2026-10-21.md`
§1.1 and §2.1 for that change.

## Related

- `docs/releases/v3.12.0/perf/BIN_LOAD_PERF.md` — the full load-performance
  report (1M/6M criterion numbers, the extrapolation gap, the architectural
  batch+flush workaround)
- `docs/releases/v3.12.0/perf/PROFILE_RESULTS.md` — the V313.3 profiling run
- `docs/releases/v4.1.0/evidence/TEST_DISCRIMINATING_POWER_BATCH8_2026-10-21.md` —
  the iteration-5 batch-8 evidence document
