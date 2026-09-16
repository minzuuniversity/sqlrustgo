# Phase B single flush per result-set — wire encode batching

**Date**: 2026-09-15
**Branch**: `feat/v4.0.0-server-read-perf`
**Status**: Implemented and merged into develop/v4.0.0

## Background

Profile of `do_command_loop` after `PHASE_B_PK_COLUMN_HARDCODED_FIX.md`
landed (PK lookup path):

```
do_command_loop                       ~4749 ticks
└── execute_select                    ~4222 ticks
    └── scan_pk                       ~2612 ticks (53%, was the bottleneck)
        └── Vec<&[Value]>::clone    ~2566 ticks
            └── String::clone         ~782 ticks
    └── other (WHERE rewriter etc.)  ~1600 ticks
└── Packet::read_from                 ~667 ticks
└── send_result_set_with_more         ~200 ticks (looked small)
└── parse (SQL parser)                ~30 ticks
```

`sed_result_set_with_more` looked small in CPU ticks, but the
wall-clock impact was much bigger. Looking at `Packet::write_to`:

```rust
pub fn write_to<W: Write>(&self, w: &mut W) -> MySqlResult<()> {
    w.write_u24::<LittleEndian>(self.length)?;
    w.write_u8(self.sequence)?;
    w.write_all(&self.payload)?;
    w.flush()?;       // ← per-packet syscall!
    Ok(())
}
```

And `send_result_set_with_more` writes one packet per row:

```rust
for r in rows.iter() {
    let mut p = Vec::new();             // ← alloc per row
    write_text_row(&mut p, r)?;
    Packet { payload: p, ... }.write_to(w)?;  // ← flushes per row
}
```

For a single-row result (typical for sysbench OLTP point-SELECTs),
that's:
1. `Vec::new()` alloc for column count packet
2. `Vec::new()` alloc for each column-def packet (N times)
3. `Vec::new()` alloc for the row packet
4. `Vec::new()` alloc for EOF/OK packet
5. **`flush()` per packet** = (2 + N + 1 + 1) syscalls

For our 5-column PK lookup, that's **9 Vec allocs + 9 flush
syscalls per query**. Each flush is ~5-10µs even on localhost.

## Fix

Add a new `Packet::write_to_no_flush` method that writes header +
payload without flushing. Refactor `send_result_set_with_more` and
its helpers (`write_column_def`, `write_ok_packets`) to use the
no-flush variant, and call `w.flush()` once at the end.

This:
1. Eliminates per-packet flush (1 syscall instead of 2+N+1+1)
2. Eliminates `Vec::new()` per packet (the per-row `Vec` still
   exists for the payload buffer, but it's collected into the
   outer single buffer instead of being separately allocated and
   freed).

## Bench

Pymysql 4t/15s, 10k rows, PK=o_orderkey:

| Bench | Before (PK column fix) | After (single flush) | Speedup |
|---|---|---|---|
| Single-row PK lookup | 3788 OPS | **4217 OPS** | **+11.3%** |
| 1-row PK range | 2550 OPS | 2617 OPS | +2.6% |
| 5-row PK range | 2528 OPS | 2489 OPS | -1.5% |
| 20-row PK range | 2308 OPS | 2317 OPS | +0.4% |

Multi-row PK range scans barely move because the cost there is
dominated by full-scan + `Vec::retain` (not wire encode). The single-
row path is where the wins land.

## Validation

- 256 lib tests pass (1 pre-existing unrelated failure)
- 241 integration tests pass (wire_protocol, stmt_prepare, etc.)
- Pre-existing flakes unchanged

## What was tried and reverted (deferred)

A more invasive refactor that **inlines the B+Tree search into
`scan_pk` directly** (skipping the intermediate `Vec<Record>`
allocation in `scan_with_index`) was tried and reverted — it
produced a -3% regression because the lock guard + HashMap lookup
overhead exceeded the saved allocation cost. Doc retained as
`PHASE_B_SCAN_PK_INLINE_REVERTED.md` for future reference.

## Files affected

- `crates/mysql-server/src/lib.rs` (added
  `Packet::write_to_no_flush`, `write_column_def_no_flush`,
  `write_ok_packets_no_flush`; refactored `send_result_set_with_more`
  to use them + single end-of-function `flush()`)
- `docs/releases/v4.0.0/PHASE_B_SINGLE_FLUSH_WIRE_ENCODE.md` (this
  file)

## Effort

~2 hours: add helpers, refactor hot function, run tests + bench.

## References

- `PHASE_B_PK_COLUMN_HARDCODED_FIX.md` — prerequisite for accurate
  bench numbers (PK lookup path was always slower due to bug, fixed
  by D.1).
- `PHASE_B_INTERNAL_LOCKING_FOUNDATION.md` — prerequisite correctness
  fix (so the PK lookup path itself is race-safe to call from
  multiple threads).
- `PHASE_B_MINISTEP_READ.md` — earlier investigation that flagged
  the read-only TPS gap.
