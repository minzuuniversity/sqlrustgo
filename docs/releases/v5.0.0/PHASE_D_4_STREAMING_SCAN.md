# Phase D.4 — Streaming scan iterator (DEFERRED)

## Background

Phase D analysis (`PHASE_D_READ_ONLY_TPS_ANALYSIS.md`) Bottleneck #4:

For non-PK queries, `scan()` returns `Vec<Record>` (the full table), then
`engine_select.rs` runs `Vec::retain(|row| eval_predicate(...))` to filter
to matching rows. This is O(N) clone of every row's `Vec<Value>` plus
each Text/Blob column's `String`.

Profile on full-scan path:
```
Vec::retain          3631 ticks (49% of execute_select CPU)
└── eval_predicate   697 ticks
    └── sql_compare  279 ticks (case-insensitive cmp dominant)
```

Estimated impact: 1.5-2× for full-scan queries.

## Why defer

Phase D.1 closed Bottleneck #1 by fixing PK column detection. After
D.1, **most queries that previously fell through to full scan now go
through the PK B+Tree path**:

- sysbench OLTP_RO: 14 SELECT variants — all use `WHERE id = ?`
- sysbench OLTP_RW: same
- Ad-hoc: depends on schema

For the bench schemas in this session (PK on `o_orderkey`/`id`,
random single-PK equality SELECTs), the PK lookup path is taken 100%
of the time. The full-scan path is only hit by:

- `SELECT * FROM t WHERE col <op> ?` where `col` is not the PK
- Multi-row PK range scans like `WHERE id BETWEEN ? AND ?` (these
  actually go through `scan_with_ahi` which already streams via B+Tree)

### Bench evidence

Multi-row PK range scan (Phase D.3 vs D.1):
- range=1:  2617 vs 2550 OPS (+2.6%)
- range=5:  2489 vs 2528 OPS (-1.5%)
- range=20: 2317 vs 2308 OPS (+0.4%)

These go through `Vec::retain` (full scan + filter). Even with multi-row
results where Phase D.3 (wire encode) should win more, the gains are
marginal. Phase D.4 (eliminate the clone) might give similar gains.

### Implementation cost

`scan()` signature change `Vec<Record> -> impl Iterator<Item = &Record>`
ripples through:

- 7+ StorageEngine impls (MemoryStorage, FileStorage, WalStorage,
  BinaryStorage, AppendOnlyStorage, BinaryStorageV2, ColumnarStorage,
  MvccStorage, VtuGuard, ScanCountingStorage)
- ~180 callsites that consume the Vec directly
- Mutex / lock semantics (the iterator borrows from `&self`)
- pk_lookup_rows, apply_rls_filter, projection logic — all assume Vec

Estimated: 1-2 weeks, high risk of subtle borrow-checker or
concurrency issues. Regression test suite (5 phase_c_1_race tests +
18 storage_integration tests + 47 file_storage_direct tests + 4
e2e_crash_recovery tests) is non-trivial to keep green.

## Alternatives if D.4 is wanted later

1. **`Vec::retain` allocation reduction**: use `Vec::retain` with a
   pre-sized Vec instead of `scan()` + new Vec. Saves 1 alloc per
   scan. ~30 lines, ~5% expected gain on full-scan path.

2. **`scan_with_filter` parallel**: split the table into partitions
   via `parallel_scan` (already exists), filter each partition in
   parallel on N threads, merge. Already partially implemented
   (see `crates/storage/src/file_storage.rs::parallel_scan`). Could
   give 2-4× on multi-core but adds overhead for small tables.

3. **Streaming iterator (D.4 original)**: most invasive but most
   flexible. Defer until #1 and #2 prove insufficient.

## Recommendation

**Skip Phase D.4 for now.** The current Phase D.3 baseline
(~4217 OPS PK lookup, +11.3% over D.1) is already a meaningful win
for the dominant OLTP workload. Phase D.4 would help only non-PK
queries, which are uncommon in OLTP benchmarks and have smaller
absolute time savings.

If a future workload is bottlenecked on non-PK queries (e.g., a BI
dashboard doing `WHERE date_col BETWEEN ? AND ?` against a wide
table), revisit D.4 with:
1. First try `Vec::retain` pre-sizing (~5% gain, ~30 lines).
2. If insufficient, try `parallel_scan` + per-partition filter.
3. Only as last resort, change `scan()` signature.

## What was attempted and reverted

- Phase D.2 attempt: inline B+Tree search in `scan_pk`. Result:
  -3% TPS regression. Doc: `PHASE_D_2_SCAN_PK_BORROW.md`.
