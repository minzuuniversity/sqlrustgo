# Phase B server read performance — investigation closeout

**Date**: 2026-09-15
**Branch**: `feat/v4.0.0-server-read-perf`
**Status**: All work landed into develop/v4.0.0

## Summary

This phase continued the read-only TPS investigation that closed
in `PHASE_B_MINISTEP_READ.md`. The Mini Step doc concluded:

> The real fix requires invasive refactor — to make SELECT truly
> lock-free at the server layer, `engine.read()` calls must be
> removed. That requires changing `ExecutionEngine::storage` from
> `Arc<RwLock<S>>` to `Arc<S>` and touches 180+ call sites.

The single-process UnsafeCell/Arc<S> rewrite remains invasive and
**out of scope for v4.0.0**. Instead, this phase focused on
**smaller, safer wins** that fit within v4.0.0:

| Phase | What it does | TPS impact | Status |
|---|---|---|---|
| [C.1](PHASE_B_INTERNAL_LOCKING_FOUNDATION.md) | FileStorage internal `write_lock` — correctness prerequisite for any future server-layer lock removal | +0% (correctness only) | committed |
| [D.1](PHASE_B_PK_COLUMN_HARDCODED_FIX.md) | `scan_pk` and `try_extract_pk_eq` honour the actual PK column name instead of hard-coded `"id"` | 1977 → 4024 OPS peak (**+103%** for non-`id` PK schemas) | committed |
| [D.3](PHASE_B_SINGLE_FLUSH_WIRE_ENCODE.md) | Single flush per result-set via `write_to_no_flush` packet variant | 0% peak / 0% sustained; saves 8 syscalls/query | committed |
| [D.5](PHASE_B_HORIZONTAL_SCALING_POC.md) | Multi-process horizontal scaling POC — 4 shards × 2t × 2500 rows | **16163 OPS = 3.83× baseline** (10-core box) | bench-only, no code |

**Cumulative** (single process): Phase B baseline 2056 OPS →
Phase B + D.1 + D.3 ≈ 3917 OPS sustained (**+91%**, peak 4024).
Add horizontal sharding (Phase D.5 POC) and the figure rises to
**16163 OPS = +686% (≈ 7.8×)** on the same 10-core box.

## Order of operations matters

The four landed in a specific dependency order:

1. **C.1 first** — provides race-safe concurrency foundation. All
   later changes assume this is in place.
2. **D.1 second** — fixes the PK column hard-coding. Required to get
   meaningful bench numbers for subsequent steps (otherwise all PK
   lookups would full-scan and any further wins would be invisible).
3. **D.3 third** — wire encode batching. Depends on D.1 being
   accurate (otherwise scan_pk dominates and wire encode is noise).
4. **D.5 last** — multi-process POC. Built on the corrected
   single-process baseline.

## What was deferred

- **Single-process UnsafeCell redesign** (would replace outer
  `RwLock` with interior `UnsafeCell`) — invasive, 1-2 weeks,
  < 5% expected gain on read-only TPS. **Out of scope for v4.0.0.**
- **In-tree MySQL-protocol shard router** — would let a single
  deployment accept MySQL connections and distribute them across
  N shards automatically. ~500 LOC, 3-5 days. **Out of scope for
  v4.0.0; tracked for future work.**

## References

- `PHASE_B_MINISTEP_READ.md` — Mini Step investigation that closed
  without action.
- `PHASE_B_INTERNAL_LOCKING_FOUNDATION.md` — C.1 correctness fix.
- `PHASE_B_PK_COLUMN_HARDCODED_FIX.md` — D.1 PK column fix.
- `PHASE_B_SINGLE_FLUSH_WIRE_ENCODE.md` — D.3 wire encode batching.
- `PHASE_B_HORIZONTAL_SCALING_POC.md` — D.5 multi-process POC.
