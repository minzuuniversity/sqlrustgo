# Phase C.2 revisit — 7-8× TPS target analysis

## Recap from PHASE_C_2_OUTCOME.md

Phase C.2 was deferred because:

1. **Cost model was wrong by 2 orders of magnitude**. Lock acquire
   is 0.012% of CPU at 2056 OPS, not the 7-8× hot path predicted.
2. **Actual hot path** is `save_table → serde_json::to_string_pretty`
   over O(rows) per INSERT — **only relevant for read+write workloads**,
   not pure read-only.
3. UnsafeCell redesign (1-2 weeks) would yield < 5% on read-only TPS.

The doc concluded: **the 7-8× target is not reachable via server-layer
lock removal**. Real gains require **binary storage** (skip JSON
serialisation) or **vectorised execution**.

## Phase D post-script: where we actually are

After Phase D.1+D.3 (committed) on top of Phase C.1:

| Phase                  | oltp_read_only 4t/10k OPS |
|------------------------|-------------------------:|
| Phase C baseline       |                    1977  |
| Phase D.1 (PK column)  |                    3788  |
| Phase D.3 (wire encode) |                    4217  |
| **Total improvement** |                  **+113%** |

We've already doubled the RO TPS without touching the lock layer.
**The 7-8× target (15000+ OPS) would need 4× more on top of current**.

## Where could another 4× come from?

### Phase C.2.1 — UnsafeCell redesign (defer)
- Risk: medium (unsafe impl Sync mis-write = silent UB)
- Cost: 1-2 weeks
- Expected gain: < 5% on RO TPS (the cost analysis is correct)
- **Conclusion**: not worth it for RO TPS target alone.

### Phase C.2.2 — Binary storage for FileStorage (NEW PATH)
- `crates/storage/src/binary_storage.rs` + `binary_storage_v2.rs` already exist.
- Replacing `save_table → serde_json::to_string_pretty` with binary
  format eliminates the per-INSERT O(rows) JSON serialisation cost.
- **BUT** for RO bench there's no INSERT, so this only helps OLTP_RW.
- For OLTP_RW with N inserts/sec, the binary storage would save
  N × O(rows) JSON serialisation + N × O(rows) fsync + O(rows) parsing
  on the next restart.

### Phase C.2.3 — Vectorised result-set serialisation (NEW PATH)
- The wire encode path (`send_result_set_with_more`) is already
  buffer+single-flush (Phase D.3).
- Remaining hot cost in execute_select is `Vec::retain` + `eval_predicate`
  on full-scan path (49% CPU).
- For PK-lookup path, the engine SELECT itself is ~50µs. To get to
  15000 OPS (667% of 4217), we'd need < 67µs per query.
- **Limit**: TCP RTT on localhost is ~50µs alone (one round trip per
  query). At 4 threads pymysql can issue ~20000 QPS theoretical max
  for ROUND-TRIP queries that fit in a single TCP packet.

### Phase C.2.4 — Reduce server-side scan overhead (MEDIUM EFFORT)
- Profile shows `execute_select` = 4222 ticks (after D.3). Where does
  this go?
  - `scan_pk`: ~600 ticks (with D.3 simplification)
  - `apply_rls_filter`: ?
  - `substitute_session_vars_in_select`: ~50 ticks (parser substitute)
  - `try_decorrelate`: ~30 ticks (decorrelator)
  - `try_extract_pk_eq_with_col`: ~30 ticks
  - `resolve_pk_column`: ~10 ticks
  - `get_table_info`: ~5 ticks
  - Project / WHERE / LIMIT / ORDER BY: ~? ticks
- **Combined pre-scan overhead**: ~700 ticks per query, ~17% of
  execute_select. Hard to remove without breaking semantics.

### Phase C.2.5 — Connection reuse + MySQL prepared statement protocol (NEW PATH)
- Current protocol: send COM_QUERY + read response per query. This
  is ~50µs round-trip overhead.
- **Binary protocol COM_STMT_PREPARE + COM_STMT_EXECUTE**: server
  caches the parse tree, can skip re-parsing. Saves ~10-20µs per
  query. Already partially implemented (`v400_stmt_prepare_*` tests).

## Realistic TPS ceiling with current architecture

| Phase                     | Predicted RO TPS | Cumulative |
|---------------------------|----------------:|-----------:|
| Phase D.3 (current)        |            4217 |          — |
| + Phase C.2.4 (pre-scan)  |            4500 |     +7%   |
| + Phase C.2.5 (prepared)  |            5500 |    +30%   |
| + Phase C.2.3 (vec wire)  |            6500 |    +18%   |
| **TOTAL Phase C.2 stack** |        **6500** |    **+54%** |
| + Phase C.2.1 (UnsafeCell) |         6700 |    +3%   |
| **Theoretical max (loopback)** |   **8000-10000** |  +90-140% |

**The 7-8× target (15000+ OPS) requires**:
- Multiple server processes / sharding (Phase C.3 territory)
- Or a fundamentally different architecture (kernel-bypass, io_uring)

## What to do

Given:
- Phase C.2.4 (pre-scan overhead) = 1-2 days, modest gain, low risk
- Phase C.2.5 (prepared statement) = 1-2 weeks, significant gain, medium risk
- Phase C.2.1 (UnsafeCell) = 1-2 weeks, minimal gain, high risk
- Phase C.2.3 (vectorised wire) = 2-3 days, modest gain, low risk

**Recommended Phase C.2 roadmap**:

1. **Phase C.2.4 first** (1-2 days, +7%): trim pre-scan overhead.
2. **Phase C.2.5 second** (1-2 weeks, +30%): prepared statement
   binary protocol for sysbench-style OLTP.
3. **Phase C.2.3 third** (2-3 days, +18%): vectorised wire encode
   (`write_value_into` writing directly to TCP, no per-cell String).
4. **Skip Phase C.2.1** (UnsafeCell redesign) — not worth the risk for
   < 5% gain.

**Cumulative**: 4217 → ~6500-7000 OPS (~+50-65%), still below the
7-8× target but a meaningful +2-3× over current.

**The 7-8× target (15000+ OPS) requires sharding** (Phase C.3 / D.5),
which is a fundamentally different scaling strategy.

## Updated profile (Phase D.3 binary, single worker)

After Phase D.3, on a single busy worker thread executing PK lookups:

```
execute_select                  4174 ticks (down from 4699 with Phase D.1)
└── scan_pk                     2227 (53%) — still dominant
    └── Vec<&[Value]>::clone    2187 (98% of scan_pk)
        └── Vec<Value>::clone   1025 (Vec clone body)
        └── String::clone       782 (Text column clone)
└── execute_select pre-processing (47%) — substitute_session_vars,
    decorrelate, get_table_info, resolve_pk_column, projection setup
```

The remaining `scan_pk` cost is `Record = Vec<Value>` deep-clone inside
the storage engine. **Phase C.2 UnsafeCell redesign does not touch this
clone** — the clone happens whether the engine holds an `Arc<RwLock<S>>`
or an `Arc<S>`. The clone is required because the caller (`engine_select`)
consumes the result into an owned `ExecutorResult`.

To actually eliminate this clone would require changing
`ExecutorResult` to support borrowed rows, then propagating the borrow
through `apply_rls_filter`, projection, `send_result_set_with_more`, etc.
That's the same scope as Phase C.2.4 (streaming iterator) — 1-2 weeks
high risk, ~5-10% expected gain because `Vec<Record>::retain` already
shields the wire encode path from the per-row clone for PK lookups.

## Re-examining the Phase C premise

Original Phase C goal (`PHASE_C_SERVER_LOCK_REMOVAL.md`):

> oltp_read_only 4t/10k: 215 → 1500–1700 TPS (≈ 7–8×)

**Status against the original goal**:

- Phase C baseline: 2056 OPS (10× the doc's 215, because the doc was
  measured under different conditions and the codebase has changed).
- Phase D.1+D.3 today: 4217 OPS.
- Original 7-8× target: ~17000 OPS.
- **Gap to original target**: 17000 / 4217 = **4× more improvement needed**.

The original analysis incorrectly attributed the bottleneck to lock
contention. Real bottleneck (per Phase C.2 outcome) is:

1. **`save_table` JSON serialisation** — only relevant for write workloads.
2. **Per-row Vec<Record> clone** in PK lookup — relevant for RO too.
3. **Pre-scan overhead** — session var substitution, decorrelator,
   WHERE rewriter all run on every query.

Of these:
- (1) doesn't affect RO TPS.
- (2) is what Phase C.2.4 streaming scan would help, but bounded
  gain (5-10%).
- (3) is what C.2.4 "trim pre-scan" would help, also bounded (5-10%).

To reach 17000 OPS in a single process would require fundamental
changes (avoid cloning, kernel-bypass networking, async/await for
concurrent client handling). Multi-process sharding is the only
practical path.

## Decision

User wants to chase the 7-8× target. Phase C.2 cannot deliver it.

Recommend one of:

1. **Skip C.2 entirely** (recommended). Acknowledge that the 7-8× target
   was based on a flawed cost model. Current +113% improvement is the
   meaningful ROI. Move to other work.

2. **Implement C.2.4 only** (1-2 days, +7%): trim pre-scan overhead.
   Cheap and safe but small absolute gain.

3. **Implement C.2.5** (1-2 weeks, +30%): prepared statement binary
   protocol. The biggest single-process win, but only for clients
   that use COM_STMT_PREPARE/EXECUTE (sysbench does, ad-hoc clients
   don't).

4. **Restore `crates/distributed` shard router** (2-4 weeks, +400-700%):
   the only path to the original 7-8× target. Substantial work; brings
   back Raft/2PC/gRPC from archive.

This document to be updated as Phase C.2 work progresses.
