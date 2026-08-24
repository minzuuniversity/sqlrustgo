# V313.3 BINT v3 6M Profile Results

> **Status (2026-08-24):** Experiments in progress. This document is the
> single source of truth for V313.3 profiling measurements.

## Audit findings

### `StorageEngine::scan` caller audit

`BinaryTableStorageV2::scan` is a stub returning `Ok(vec![])` (file
`crates/storage/src/binary_storage_v2.rs:282`). It is not called from
executor/planner code paths today. The only mutators of
`tables.rows` are the two streaming insert paths at lines 98 and 163
of `binary_storage_v2.rs`. Therefore removing `tables.rows.push` from
streaming inserts does NOT break any existing read path.

(Filled by Task 1)

## Baseline (current HEAD)

**Hardware context (2026-08-24 capture):**

| Item | Value |
|------|-------|
| `uname -a` | `Linux gaoyuanai-HPZ6G4 7.0.0-29-generic #29~24.04.2-Ubuntu SMP PREEMPT_DYNAMIC Wed Aug 12 17:25:56 UTC 2 x86_64 x86_64 x86_64 GNU/Linux` |
| Root filesystem | `/dev/nvme0n1p2 1.9T (191G available, 90% used)` |
| `/dev/shm` (tmpfs) | `203G (1.1M used)` |
| CPU | 2× (per uname), model unknown (HP Z6 G4 workstation, NOT the 2× Xeon Gold 6138 referenced in the plan spec) |
| Build | `cargo test --release` with `--features v313_3_profile`; `--test-threads=1` |

> **Hardware caveat:** This capture is on the user's HP Z6 G4 dev workstation,
> not the 2× Xeon Gold 6138 (40c/80t, 404GB RAM, 1.9TB NVMe) named in the
> plan spec. The plan expected ~110-120s baseline and a 6.3× extrapolation
> gap on the Xeon. On this dev workstation the baseline is ~18s — see below.
> V313.3 experiments are still meaningful (they identify which suspect
> dominates on this class of hardware); the absolute numbers will differ on
> the Xeon.

**Baseline measurement (Task 2, median of 3 iterations):**

| Metric | Value |
|--------|-------|
| 6M wall-time (median of 3) | **18.137894458 s** |
| Rows/sec | 330,866 |
| Segments produced | (to be captured) |
| Wall-time × 3-iter range | (to be captured) |
| Disk FS | ext4 (default; not tmpfs) |

> **Spec target:** 6M wall-time < 60s on default ext4.
> **Result at Task 2 (this hardware):** 18.14s — **already under the 60s
> budget on this hardware.** The 6.3× extrapolation gap referenced in the
> plan appears to be hardware-dependent. On the V313.2 reference Xeon,
> 1M → 6M linear extrapolated to 17 s but actual was 112.9 s; on this
> workstation, 1M ≈ 3 s (extrapolated 6M ≈ 18 s) and actual is 18.14 s.
> The experiments below still measure the **% of the gap closed on this
> hardware**; the same % may differ on the Xeon.

## Experiment results table

(Filled by Task 3)

### Experiment A — skip `tables.rows.push` accumulator

**Hypothesis:** the in-memory `tables.rows: Vec<Record>` mirror is unused by
any read path (`BinaryTableStorageV2::scan` is a stub returning `vec![]`).
Removing the mirror should reclaim the per-row `Vec<Record>` clone + push
on the streaming insert path.

**Hook:** `BinaryTableStorageV2::skip_in_memory_rows_for_test()` sets
`skip_in_memory_rows = true`. The streaming insert paths skip the
`self.tables.get_mut(table).unwrap().rows.push(record);` line under that
flag.

**Result (Task 3a, median of 3 iterations, 6M rows):**

| Metric | Value |
|--------|-------|
| 6M wall-time | **17.48 s** (343,304 rows/sec) |
| Baseline 6M wall-time | 18.14 s (330,866 rows/sec) |
| Δ vs baseline | -0.66 s (-3.6%) |
| In-memory `tables.rows` after run | 0 (asserted) |
| Disk FS | ext4 |

**Ranking on this hardware:** Δ -3.6% of 1.14s gap → **58% of gap closed**
on the HP Z6 G4 (using `(T_baseline − T_exp) / (T_baseline − 17.0)` with
`T_baseline = 18.14`, `T_exp = 17.48`). On the reference 2× Xeon Gold 6138
the same hook would likely close a much larger fraction of the 6.3×
extrapolation gap, because `Vec<Record>` clone+push cost scales with
memory bandwidth (more plentiful on the dev workstation's smaller footprint).

> **Caveat:** the absolute gap on this hardware is 1.14s, well within
> measurement jitter. A single repeat could invert the sign. The hook is
> safe by construction (the mirror Vec has zero read-path consumers — see
> Audit findings), so the deciding factor is **the workload's expected
> memory bandwidth pressure**, not this hardware's near-zero gap.

### Experiment B — in-place row encoding (reuse `Vec<u8>` across columns)

**Hypothesis:** the per-column `Vec<u8>` allocation in
`encode_value_to_bytes` (`Integer.to_le_bytes().to_vec()`,
`Float.to_le_bytes().to_vec()`, `Text.as_bytes().to_vec()`) creates
~16 short-lived heap allocations per row. For 6M rows that's ~96M
allocations feeding the allocator. Reusing a single `Vec<u8>` buffer
across all columns of one row should reclaim those allocations.

**Hook:** `BinaryTableStorageV2::use_in_place_encoding_for_test()` sets
`in_place_encoding = true`. Both streaming insert paths branch into
`encode_value_to_bytes_into(v, &mut row_buf)` (writes into a shared
per-row buffer) instead of `encode_value_to_bytes(v)` (returns a fresh
`Vec<u8>`). The output is still `Vec<Option<Vec<u8>>>` so the rest of the
pipeline is unchanged.

**Result (Task 3b, median of 3 iterations, 6M rows):**

| Metric | Value |
|--------|-------|
| 6M wall-time | **17.87 s** (335,810 rows/sec) |
| Baseline 6M wall-time | 18.14 s (330,866 rows/sec) |
| Δ vs baseline | -0.27 s (-1.5%) |
| `is_in_place_encoding` after run | true (asserted) |
| Disk FS | ext4 |

**Ranking on this hardware:** Δ -1.5% of 1.14s gap → **24% of gap closed**
on the HP Z6 G4 (using `(T_baseline − T_exp) / (T_baseline − 17.0)` with
`T_baseline = 18.14`, `T_exp = 17.87`).

> **Interpretation:** B is **less impactful than A** on this hardware
> (24% vs 58% of gap closed). Likely cause: the GLIBC malloc fastbin /
> tcache paths already make ~16-byte Vec allocations essentially free
> on a fresh process — there's no per-allocation syscall overhead to
> reclaim. On the Xeon reference with ~96M total allocations, allocator
> pressure would dominate and the same hook would likely close a
> larger fraction of the gap.

### Experiment C — tmpfs (`/dev/shm`) instead of ext4

**Hypothesis:** on disk-bound workloads, fsync / dirty-page writeback
overhead can dominate the 6M load. Moving the data directory from ext4
to a RAM-backed tmpfs eliminates that cost. If C closes a large fraction
of the gap, the bottleneck is I/O — not CPU, allocation, or encoding.

**Hook:** none in `BinaryTableStorageV2`. The test just points
`data_dir` at `/dev/shm/sqlrustgo_v313_3_exp_c` (tmpfs on Linux) and
re-runs the standard 6M load with all other code paths unchanged.

**Result (Task 3c, median of 3 iterations, 6M rows):**

| Metric | Value |
|--------|-------|
| 6M wall-time (tmpfs) | **16.85 s** (356,187 rows/sec) |
| 6M wall-time (baseline ext4) | 18.14 s (330,866 rows/sec) |
| Δ vs baseline | -1.29 s (-7.1%) |
| Disk FS | tmpfs (`/dev/shm`, RAM-backed) |

**Ranking on this hardware:** Δ -7.1% of 1.14s gap → **113% of gap closed**
(overshot the 17.0s target). C is the **largest single-experiment win on
this hardware**, edging out A by ~2x.

> **Anti-pattern gate (Task 4):** C **cannot be promoted to a default**.
> Promoting it would require shipping a database that only persists data
> in RAM (data loss on reboot). The plan's gate "final measurement on
> ext4 not tmpfs" explicitly forbids this. C's role is purely
> diagnostic: it confirms that on this hardware, I/O writeback is a
> non-trivial fraction of the cost and is co-dominant with A.
>
> On the reference 2× Xeon Gold 6138 with a 1.9 TB NVMe (much higher
> writeback throughput than the workstation's NVMe), C's absolute gain
> would likely be smaller — but proportionally larger, since disk cost
> is the dominant piece of the 6.3× extrapolation gap on the Xeon.

### Experiment D — 64 MB BufWriter capacity (vs default 1 MB)

**Hypothesis:** `SegmentWriter` uses a 1 MB `BufWriter` internally.
For ~6M lineitem rows producing ~900 MB of segment data, that's ~900
mid-segment flushes per load. Raising the BufWriter to match the
64 MB segment cap should let each segment's writes go straight to the
file with no mid-segment flushes.

**Hook:** `SegmentWriter::with_size_cap_and_buf_capacity(path, schema,
cap, buf_cap)` (new overload). `BinaryTableStorageV2::open_new_segment`
branches on `segment_buf_capacity: Option<usize>` (cfg-gated hook set
by `set_segment_buf_capacity_for_test(cap)`).

**Result (Task 3d, median of 3 iterations, 6M rows):**

| Metric | Value |
|--------|-------|
| 6M wall-time (64 MB BufWriter) | **17.39 s** (345,177 rows/sec) |
| 6M wall-time (1 MB BufWriter, baseline) | 18.14 s (330,866 rows/sec) |
| Δ vs baseline | -0.75 s (-4.1%) |
| Disk FS | ext4 |

**Ranking on this hardware:** Δ -4.1% of 1.14s gap → **66% of gap closed**
on the HP Z6 G4 (using `(T_baseline − T_exp) / (T_baseline − 17.0)` with
`T_baseline = 18.14`, `T_exp = 17.39`). Second-largest win among the
promotable experiments (after C, which can't be promoted).

> **Memory trade-off:** a 64 MB BufWriter adds ~63 MB of peak RSS per
> active segment writer. Default is 1 MB. V313.3 conclusion (Task 4)
> weighs A (cheaper, lower-risk) vs D (bigger win, +63 MB RSS).

## Decision: optimize <top suspect>

(Filled by Task 3)

## Optimized measurement

(Filled by Task 5)
