# V313.3 BINT v3 6M Profile + Close 6.3× Perf Gap Design

> **Status:** Draft (post-brainstorming, awaiting user review)
> **Date:** 2026-08-23
> **Author:** V313.3 follow-up
> **Supersedes:** None
> **Implements:** V313 T8+ future-work item "Profile and close the 6.3× perf gap"
> **Skill context:** brainstormed via superpowers:brainstorming; spec; then writing-plans; then subagent-driven-development (or executing-plans) for execution

## Goal

In a single V313.3 PR:

1. **Quantify** the contribution of each hypothesized cost driver to the 6M TPC-H SF=1 lineitem load (1M → 6M extrapolation: ~17s; actual: 112.9s via `insert_streaming_iter` API; **6.3× gap**).
2. **Apply the data-driven fix** for the dominant contributor(s).
3. **Verify** the result: 6M wall-time < 60s on default ext4 + page cache conditions, 1M criterion bench stays < 5s, JSON-vs-BINT speedup stays ≥ 3.0×.

## Background

### Known measurements (V313 + V313.1 + V313.2)

| Workload | API | Wall-time | Rows/sec | Segments | Source |
|----------|-----|-----------|----------|----------|--------|
| 1M | `insert_streaming` (batch+flush) | 2.84s (criterion stable) | 352,113 | ~10 | T6.3 bench |
| 6M | `insert_streaming` (batch+flush) | 108.09s | 55,522 | 60 | T6.4 test |
| 6M | `insert_streaming_iter` (V313.2) | 112.92s | 53,147 | 15 | V313.2 follow-up |

- JSON baseline: ~16.7K rows/sec → 6M ~360s; **JSON-vs-BINT speedup ~3.4× at all sizes** (V313.1 measured).
- Iter API eliminated 4× rollovers (60 → 15) but wall-time unchanged (+4.5%, within noise). This is **strong evidence** that batch overhead is NOT the dominant cost; the per-row encode + disk write path dominates.

### Hypothesized cost drivers (now to be ranked via controlled-variable experiments)

| # | Suspect | Mechanism | Plausibility |
|---|---------|-----------|--------------|
| 1 | `tables.rows.push(record)` accumulator | In-memory `Vec<Record>` keeps every loaded row (~1.2 GB heap for 6M) | High — unused by read path (`scan()` returns `vec![]`); only cost, no benefit |
| 2 | Per-column `Vec<Option<Vec<u8>>>` allocation in hot loop | Allocates a fresh `Vec` per row, then per column, then `encode_value_to_bytes` allocates another `Vec` per column | High — ~96M `Vec` allocations for 6M (16 cols × 6M rows) |
| 3 | Per-row `encode_row` `Vec` allocation in `bin_segment::SegmentWriter::append` | Each `append` builds a fresh `Vec<u8>` for header + fixed fields + null bitmap + var fields | Medium — 6M allocations |
| 4 | Disk write throughput | 6M rows × ~150 bytes ≈ 900 MB; NVMe sequential write ≈ 1.5 GB/s → ~0.6s physical | Low (not actually the bottleneck) — but worth confirming via tmpfs |
| 5 | BufWriter capacity (1 MB) | `BufWriter::with_capacity(1 << 20)` flushes every ~1 MB; could increase to 16 MB or 64 MB | Low — but cheap to test |

The ranking produced by the A/B/C/D experiments below will drive the fix.

## Approach

### One V313.3 PR with the following structure

1. **Add a `v313_3_profile` Cargo feature flag** (default OFF, like `bin_storage_default`) that gates experimental hooks in `BinaryTableStorageV2`.
2. **Add 4 isolation experiments** as `#[ignore]d #[test]` functions in a new file `tests/integration/tpch/v313_3_profile_experiments.rs`. Each experiment changes **exactly one variable** and re-uses the same `lineitem_schema()` + `lineitem_row(i)` + `run_load_iter()` helper for cross-experiment comparability.
3. **Add a baseline measurement** that re-runs `tpch_sf1_6m_load_iter_test` for direct comparison with V313.2's 112.9s reference.
4. **Run all 4 experiments × ≥3 iterations each**, take the median, and rank suspects by contribution to total cost.
5. **Apply the data-driven fix** for the top suspect(s) (cumulative ≥ 50% of the gap). Fix is conservative: must preserve row count, segment count, root.bin schema, and read-path semantics.
6. **Re-measure 6M with the fix applied**; assert < 60s. Also verify 1M criterion bench < 5s and JSON-vs-BINT speedup ≥ 3.0×.
7. **Publish results** in `docs/releases/v3.13.0/perf/PROFILE_RESULTS.md` with the full ABCD table.
8. **Update `BIN_LOAD_PERF.md`** with the new "V313.3 follow-up — Profile + optimization" section.

If after fixing the top suspect 6M is **still** ≥ 60s, fix the next suspect in the same PR (V313.3 stays one PR; if a suspect cannot be addressed without layout change, escalate to V313.4).

### Why one PR, not several

The experiments + ranking + fix are sequential and data-driven; splitting into multiple PRs creates risk of merge races against `develop/v3.12.0` and makes the results doc incoherent. Per the V313 T8+ pattern (V313.1 + V313.2 were both follow-up PRs but each contained a complete measurable artifact), one PR with a complete profile report + optimization is the right unit.

## Architecture

### Feature flag — experimental hooks

The `tables.rows` accumulator and `BufWriter` capacity are `private` fields in `binary_storage_v2.rs` and `bin_segment.rs`. The experiments need minimal, **clearly-marked** hooks to flip these on/off. Naming convention `*_for_test` makes intent obvious to reviewers.

```rust
// crates/storage/src/binary_storage_v2.rs
#[cfg(feature = "v313_3_profile")]
impl BinaryTableStorageV2 {
    /// Skip pushing rows to the in-memory `tables.rows` accumulator.
    /// Test-only hook for V313.3 experiment A.
    pub fn skip_in_memory_rows_for_test(&mut self) { /* sets a flag */ }

    /// Override the BufWriter capacity used by SegmentWriter. Test-only.
    pub fn override_bufwriter_capacity_for_test(&mut self, cap: usize) { /* stores cap */ }
}
```

`bin_segment.rs` similarly exposes:

```rust
#[cfg(feature = "v313_3_profile")]
impl SegmentWriter {
    pub fn enable_in_place_encoding_for_test(&mut self) { /* sets a flag */ }
}
```

The hooks compile to nothing in the default build (`cargo build` without `--features v313_3_profile` produces the same binary as today). The feature is OFF by default per V313 convention (`bin_storage_default` pattern).

### New test file structure

```rust
// tests/integration/tpch/v313_3_profile_experiments.rs
//
// V313.3 controlled-variable experiments to identify the dominant cost driver
// in the 6M TPC-H SF=1 lineitem load.
//
// Run with:
//   cargo test --test v313_3_profile_experiments --features v313_3_profile -- --ignored --nocapture
//
// Each experiment changes EXACTLY ONE variable. All use the same
// lineitem_schema() + lineitem_row(i) + run_load_iter() helpers to ensure
// cross-experiment comparability.

#![cfg(feature = "v313_3_profile")]

const N_ROWS: usize = 6_001_215;  // SF=1 lineitem

fn lineitem_schema() -> Vec<ColumnDefinition> { /* T6.3/T6.4 schema */ }
fn lineitem_row(i: usize) -> Record { /* T6.3/T6.4 generator */ }

fn run_load_iter(storage: &mut BinaryTableStorageV2, n: usize) -> SqlResult<Duration> {
    let start = Instant::now();
    storage.insert_streaming_iter("lineitem", (0..n).map(lineitem_row))?;
    storage.flush()?;
    Ok(start.elapsed())
}

#[test]
#[ignore] // 6M × ~110s, slow
fn experiment_a_skip_rows_accumulator() {
    let dir = tempdir().unwrap();
    let mut storage = BinaryTableStorageV2::new(dir.path().to_path_buf()).unwrap();
    storage.create_table("lineitem", lineitem_schema()).unwrap();
    storage.skip_in_memory_rows_for_test();
    let median = median_of_n(3, || run_load_iter(&mut storage, N_ROWS).unwrap());
    println!("EXPERIMENT A (no tables.rows.push): 6M rows in {median:?}");
    // Do not assert budget — this is exploratory; result is captured for the report
}

#[test]
#[ignore]
fn experiment_b_in_place_encoding() { /* ... */ }

#[test]
#[ignore]
fn experiment_c_tmpfs_no_disk() { /* run on /dev/shm or similar */ }

#[test]
#[ignore]
fn experiment_d_64mb_bufwriter() { /* override cap to 1<<26 */ }

// 1k smoke tests for each experiment (NOT ignored, must always run in PR)
#[test] fn experiment_a_smoke() { /* 1k rows in <5s with flag set */ }
// ... etc
```

### Why each experiment is the right granularity

- **Experiment A — disable `tables.rows.push`**: directly isolates the heap-accumulator cost. If wall-time drops significantly (e.g., to ~20s), A is the dominant cost and we remove `tables.rows.push` from the streaming insert path entirely (retaining it for the explicit `insert` path where callers expect it).
- **Experiment B — in-place encoding**: takes a `&mut Vec<u8>` buffer passed down from `BinaryTableStorageV2`, writes fixed-width columns directly into row-buffer positions, and only allocates per-row. If wall-time drops, B is significant.
- **Experiment C — tmpfs / disk-free path**: runs the same load with `data_dir` set to a path under `/dev/shm` (Linux tmpfs, RAM-backed; ~half of system RAM). This isolates disk write pressure by removing physical I/O while keeping all other code paths identical. Note: `MemoryStorage` is a separate in-memory backend (not used by `BinaryTableStorageV2`); tmpfs is the right tool here because it preserves file-creation + segment rollover + root.bin rewrite semantics. If wall-time drops significantly (>10%), disk is a meaningful contributor; expected outcome is a small drop because the current implementation already does bulk-buffered writes with no per-row fsync.
- **Experiment D — 64 MB BufWriter**: increases the BufWriter capacity from 1 MB to 64 MB. If wall-time drops, syscall overhead is significant.

### Decision rule after experiments

Sort suspects by **percentage contribution to the 6.3× gap** (not by raw seconds). The fix targets the top suspect(s) until the cumulative fix ≥ 50% of the gap is achieved. Per the brainstorming anti-pattern gates:

- ❌ Do not declare an experiment the "winner" because its wall-time is 5s lower; declare based on % of gap closed.
- ❌ Do not ship a fix that closes the gap only on tmpfs (ext4 default is the production condition).
- ❌ Do not ship a fix that breaks the 1M criterion bench or the JSON-vs-BINT ratio.

### Out of scope

- Rewriting BINT v3 (segment layout, encoding format, schema hash, row format).
- JSON baseline re-measurement (already measured V313.1: 16.7K rows/sec).
- Compactor concurrent-reader safety (separate V313 T8+ item).
- VARCHAR/TEXT compaction (separate V313 T8+ item).
- Per-row fsync (out of scope: the current design is bulk-buffered; per-row fsync would be a different performance regime).
- Cargo feature `bin_storage_default = true` promotion — that is a separate decision gated on the perf target.

## File-level changes

### New files

| File | Purpose |
|------|---------|
| `tests/integration/tpch/v313_3_profile_experiments.rs` | 4 isolation experiments + 4 smoke tests |
| `docs/releases/v3.13.0/perf/PROFILE_RESULTS.md` | Full ABCD table + decision rationale + final measurements |
| `docs/superpowers/plans/2026-08-23-v313-3-bint-6m-profile.md` | Implementation plan (written by writing-plans skill after spec approval) |

### Modified files

| File | Change |
|------|--------|
| `crates/storage/src/binary_storage_v2.rs` | Add 2 hooks under `#[cfg(feature = "v313_3_profile")]`; remove `tables.rows.push` from streaming path if Experiment A wins |
| `crates/storage/src/bin_segment.rs` | Add 1 hook under `#[cfg(feature = "v313_3_profile")]`; refactor `encode_row` to accept pre-allocated buffer if Experiment B wins |
| `crates/storage/Cargo.toml` | Add optional `v313_3_profile` feature |
| `Cargo.toml` | Add `[[test]]` entry for the new test file (tests are not auto-discovered) |
| `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md` | Add "V313.3 follow-up" section + update target table |

### Unmodified files (verified no change needed)

- `tests/integration/tpch/tpch_sf1_6m_load_iter_test.rs` — used as-is for the baseline reference run.
- `benches/tpch_load_bench.rs` — used as-is for 1M regression check.
- `tests/integration/tpch/tpch_json_vs_bint_compare.rs` — used as-is for JSON-vs-BINT regression check.

## Data flow (per-experiment)

```
┌────────────────────────────────────────────────────────────┐
│ run_load_iter(storage, n)                                   │
│   1. open BinaryTableStorageV2 with tempdir                 │
│   2. create_table("lineitem", lineitem_schema())            │
│   3. <experiment-specific hook: skip_in_memory_rows / …>    │
│   4. insert_streaming_iter("lineitem", 0..n | lineitem_row) │
│   5. flush()                                                │
│   6. close (drop storage)                                   │
└────────────────────────────────────────────────────────────┘
```

Each experiment repeats this with **one variable changed**. The baseline `tpch_sf1_6m_load_iter_test.rs` runs the same flow without any hooks.

## Testing strategy

### Test inventory

| Test | Run mode | Purpose |
|------|----------|---------|
| 4 × experiment tests (A/B/C/D @ 6M) | `#[ignore]`d; run with `-- --ignored --nocapture` | Capture median wall-time per suspect |
| 4 × experiment smoke tests (A/B/C/D @ 1k) | NOT ignored; always run in PR | Sanity-check each experiment path compiles + runs |
| 1 × baseline re-measure (6M via existing `tpch_sf1_6m_load_iter_test`) | `#[ignore]`d | Confirm V313.2 reference (112.9s) is reproducible on current HEAD |
| 1 × post-fix 6M gate (existing `tpch_sf1_6m_load_iter_test` with budget 60s) | `#[ignore]`d | Pass criterion: 6M < 60s |
| 1 × 1M criterion bench (existing `tpch_load_bench`) | NOT ignored | Regression check: stays < 5s |
| 1 × JSON-vs-BINT 1M (existing `tpch_json_vs_bint_compare`) | NOT ignored | Regression check: speedup ≥ 3.0× |

### Anti-pattern gates (close conditions)

The PR is **NOT done** if any of the following is true:

1. Any experiment was skipped (no ABCD table entry).
2. The decision rationale cites a 5-second wall-time delta as the reason for picking a suspect instead of a %-of-gap calculation.
3. The optimization is demonstrated only on tmpfs; the post-fix 6M run on default ext4 / page cache is missing.
4. The 1M criterion bench regressed (> 5s) OR the JSON-vs-BINT speedup regressed (< 3.0×).
5. The full `cargo test --lib -p sqlrustgo-storage` does not pass.
6. The `PROFILE_RESULTS.md` report is missing the ABCD table, the ranking rationale, the optimized measurement, or the regression checks.
7. The `tables.rows` in-memory accumulator is removed without auditing that no `StorageEngine` read path depends on it (specifically: `BinaryTableStorageV2::scan` currently returns `vec![]` — this must stay `vec![]` after the fix; it is a stub, not an implementation).

### Reproducibility protocol

For each experiment:
1. Run `cargo test --release --test v313_3_profile_experiments --features v313_3_profile -- --ignored --nocapture --test-threads=1` (serial, not parallel, to avoid disk contention).
2. Take median of ≥3 iterations.
3. Print `EXPERIMENT <name>: 6M rows in <T>s (segments=N)` per run; aggregate into the report.
4. Verify `tables.rows.len() == 0` after each Experiment A run (using a public `len_in_memory_rows_for_test` hook).

## Risks & mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Hook functions in `BinaryTableStorageV2` get called from production code by mistake | High | `#[cfg(feature = "v313_3_profile")]` makes them zero-cost absent the feature; PR review must grep for any caller in `src/` (not just `tests/`) |
| Refactoring `encode_row` to take `&mut Vec<u8>` changes row size | High | Existing reader unit tests in `bin_segment` (e.g., `test_segment_writer_respects_size_cap`) must pass; the on-disk layout (header + fixed + bitmap + var) does not change |
| Increasing BufWriter capacity from 1 MB to 64 MB causes OOM with N tables open concurrently | Medium | Lineitem test uses 1 table; if fix is general, add a `BufWriter::with_capacity` upper bound; measure RSS via `/proc/self/status` before/after |
| Tmpfs `/dev/shm` size insufficient for 6M (~1.5 GB) | Medium | `df /dev/shm` check before Experiment C; skip C if size < 2 GB; partial-fallback: run 1M subset |
| Removing `tables.rows.push` breaks a future read-path implementation that hasn't been written yet | Low | Audit `StorageEngine::scan` callers — currently `BinaryTableStorageV2::scan` returns `vec![]` (stub). Document in code comment: "streaming insert intentionally does NOT mirror rows to the in-memory Vec; read paths must scan segments". If a caller in the executor / planner currently routes to `BinaryTableStorageV2::scan` expecting non-empty results, that caller would break — must audit before merging the fix |
| Experiment noise masks real signal | Medium | ≥3 iterations median; cross-check by re-running baseline |
| Local dev machine differs from CI / production | Low | Document hardware in report; ship a `scripts/v313_3_reproduce.sh` that records `uname -a`, CPU model, `df -h` |
| The fix requires schema / format change (e.g., column ordering, footer layout) | Scope escalation | Per the brainstorming anti-pattern gate "if a suspect cannot be addressed without layout change, escalate to V313.4"; not V313.3 scope |

## Definition of Done (V313.3 PR)

- [ ] All 4 experiments ran with ≥3 iterations each; ABCD table complete (anti-pattern gate #1).
- [ ] Baseline 6M re-measured on current HEAD; value within ±10% of V313.2 reference (112.9s).
- [ ] Suspects ranked by % of gap (NOT raw seconds delta); fix applied to top suspect(s) until cumulative ≥ 50% (anti-pattern gate #2).
- [ ] Post-fix 6M wall-time < 60s on **default ext4** (NOT tmpfs) (anti-pattern gate #3).
- [ ] 1M criterion bench ≤ 5s (regression check, anti-pattern gate #4).
- [ ] JSON-vs-BINT speedup at 1M ≥ 3.0× (regression check, anti-pattern gate #4).
- [ ] `cargo test --lib -p sqlrustgo-storage` passes; no regressions in any test path that touches `BinaryTableStorageV2` or `SegmentWriter` (anti-pattern gate #5).
- [ ] `StorageEngine::scan` audit complete: documented that `BinaryTableStorageV2::scan` is a stub and no caller relies on `tables.rows` to be populated by streaming inserts (anti-pattern gate #7).
- [ ] `docs/releases/v3.13.0/perf/PROFILE_RESULTS.md` published with full table + decision rationale.
- [ ] `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md` updated with V313.3 section + new target-table row.
- [ ] `bin_storage_default` feature stays OFF (no perf change warrants promotion to GA default).
- [ ] Anti-pattern gate audit checklist included in PR description.
- [ ] PR template sections (per `gitea-merge-workflow-gotchas.md`): Summary / 变更内容 / 关联 / 不做的事 / Verification / Anti-Pattern / Changes.

## Open questions

None at design time. If during execution the data shows a top suspect that requires a layout change (e.g., segment format), the spec is escalated to V313.4 and V313.3 ships with only the layout-preserving fixes.

## References

- `docs/superpowers/specs/2026-08-23-tpch-sf1-data-loading-design.md` — original V313 design
- `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md` — V313 + V313.1 + V313.2 measurements
- `docs/releases/v3.13.0/perf/JSON_VS_BINT_MEASURED.md` — V313.1 baseline
- `tests/integration/tpch/tpch_sf1_6m_load_iter_test.rs` — V313.2 6M gate
- `benches/tpch_load_bench.rs` — T6.3 1M criterion bench
- `crates/storage/src/binary_storage_v2.rs` — current implementation
- `crates/storage/src/bin_segment.rs` — SegmentWriter + encode_row
- Memory: [[v313-bint-storage-completion]] — V313/V313.1/V313.2 completion log
- Memory: [[gitea-merge-workflow-gotchas]] — PR push/create/merge workflow
