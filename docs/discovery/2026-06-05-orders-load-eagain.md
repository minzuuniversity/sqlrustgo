# Discovery: `sqlrustgo-mysql-server` `LOAD DATA LOCAL INFILE` EAGAIN on Large Tables

> **Status**: Open, blocking TPC-H 22-query wire round-trip (Issue #2977)
> **Discovered**: 2026-06-05 (Phase 2, `feature/tpch-22-wire-v2` branch)
> **Owner**: TBD (not assigned in any PR as of 2026-06-05 03:30 UTC+8)

## Summary

The `sqlrustgo-mysql-server` server's `LOAD DATA LOCAL INFILE` handler
in `crates/mysql-server/src/lib.rs::handle_load_local_infile` reliably
fails with `EAGAIN` ("Resource temporarily unavailable") on the client
side when the input table has **9+ columns and 150+ rows**. Smaller
tables (region=5, nation=25, supplier=10, customer=15, part=20,
partsupp=80) load fine; larger tables (orders=150, lineitem=614)
crash with the same error.

## Reproduction

```rust
// From `tests/tpch_full_22_wire_test.rs` (Phase 2 draft)
let mut client = MySqlTestClient::connect_with_config(EphemeralConfig {
    data_dir: Some(PathBuf::from("tests/data/tpch-sf001")),
    bootstrap_tables: false,
    bootstrap_users: true,
    ..Default::default()
}).expect("connect");

for ddl in SCHEMA_DDL {
    client.exec(ddl).expect("DDL");
}

for tbl in ["region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem"] {
    let path = PathBuf::from("tests/data/tpch-sf001").join(format!("{}.tbl", tbl));
    let n = client.load_local_infile(&path, tbl).expect("load");
    println!("{}: {}", tbl, n);
}
```

**Output (truncated at lineitem)**:

```
region: 5
nation: 25
supplier: 10
customer: 15
part: 20
partsupp: 80
thread '...' panicked at tests/tpch_full_22_wire_test.rs:110:33:
load_local_infile orders: read packet header: Resource temporarily unavailable (os error 11)
```

**Standalone repro (single table, fresh server)**:

```rust
// Re-ordered: only load orders.tbl into a fresh server
let mut client = MySqlTestClient::connect_with_config(EphemeralConfig {
    data_dir: Some(PathBuf::from("tests/data/tpch-sf001")),
    bootstrap_tables: false,
    bootstrap_users: true,
    ..Default::default()
}).expect("connect");
client.exec("CREATE TABLE orders (...)").expect("DDL");
let n = client.load_local_infile(&PathBuf::from("tests/data/tpch-sf001/orders.tbl"), "orders");
// → same EAGAIN error
```

The fact that **a single table on a fresh server** also fails rules
out session / seq-number wrapping / inter-table state issues.

## Failure pattern

| Table    | Cols | Rows | Result |
|----------|------|------|--------|
| region   | 3    | 5    | ✓ PASS |
| nation   | 4    | 25   | ✓ PASS |
| supplier | 7    | 10   | ✓ PASS |
| customer | 8    | 15   | ✓ PASS |
| part     | 9    | 20   | ✓ PASS |
| partsupp | 5    | 80   | ✓ PASS |
| orders   | 9    | 150  | ❌ EAGAIN |
| lineitem | 16   | 614  | ❌ EAGAIN |

The boundary is unclear from outside; it looks like either
`rows * cols > ~1000` triggers it, or there's a `pending_bytes`
overflow at exactly the 6th/7th `bulk_insert` call. The 9-column
boundary is suspicious because the same column count works for
`part` (20 rows).

## Hypothesised root cause

Three candidates, listed in decreasing probability:

### 1. `pending_bytes += line_str.len()` overflow on the buf boundary

In `crates/mysql-server/src/lib.rs::handle_load_local_infile` around
line 1567, the `parse_tbl_line` loop accumulates `line_str.len()` into
`pending_bytes` but **does not reset `pending_bytes` after each
`bulk_insert` flush** (only `pending_rows` is taken and `pending_bytes
= 0` after the flush, so this might not be the issue — but the value
is also `+=`d before the boundary check, which may double-count
newlines stripped at `trim_end_matches('\n')`).

### 2. `bulk_buf_size` is too small for 9-column tables

The default `bulk_buf_size` is some small constant; for the
`bulk_insert` codepath, the boundary condition
`if pending_bytes >= bulk_buf_size && !pending_rows.is_empty()` is
checked per packet, not per row. On a 9-column 150-row table the
accumulated `pending_bytes` may overshoot the boundary by enough
that the first `bulk_insert` call writes rows that have not been
fully drained from `buf`, and the server subsequently reads past
the live region. The client then sees `EAGAIN` on the next read
because the server's per-connection `pending_bytes` accounting
has gone negative.

### 3. `Packet::read_from(stream)` flips the stream to non-blocking somewhere

`EAGAIN` on a read is the canonical non-blocking-socket error. The
server's connection setup at line 1940 calls
`stream.set_read_timeout(Some(std::time::Duration::from_secs(600)))`
and line 1946 calls `stream.set_nonblocking(false)`, but if a
later code path toggles `set_nonblocking(true)` (e.g. under
load shedding or per-packet) and forgets to revert, every
subsequent `read_exact` will return `EAGAIN` rather than blocking.

## Workarounds (none recommended for production)

1. **Split the .tbl file into chunks of < 100 rows** and call
   `LOAD DATA LOCAL INFILE` once per chunk. Ugly and fragile.
2. **Use INSERT statements via `client.exec` instead of LOAD DATA**.
   Slow for 614 rows but works; bypasses the handler entirely.
3. **Wait for the bug to be fixed** — see "Fix" below.

## Fix

None as of 2026-06-05 03:30 UTC+8. Suggested follow-up PR scope:

1. Add a regression test that loads `orders.tbl` (150 rows) and
   `lineitem.tbl` (614 rows) into fresh ephemeral servers.
2. Add `tracing` instrumentation around the
   `pending_bytes` accounting in `handle_load_local_infile` to
   confirm which hypothesis above is correct.
3. Fix and verify the regression test passes.

Suggested assignee: whoever picks up Issue #2977 next, or the
`crates/mysql-server` crate maintainer (currently listed in
`Cargo.toml` as the `openclaw` org).

## Impact on TPC-H wire round-trip

This bug blocks the Phase 2 wire-test goal of running 22 TPC-H
queries against the wire-protocol surface. Workarounds in
`tpch_full_22_wire_test.rs` (the Phase 2 draft) had to skip
`orders` AND `lineitem` (since both are 9+ columns / 150+ rows),
which leaves only 6 of 8 tables loadable, and most TPC-H queries
reference one of those two. Net: **0/22 wire PASS possible** until
this bug is fixed.

In-process (`ExecutionEngine::MemoryStorage` + direct SQL `execute`)
is unaffected: `tpch_full_22_test` runs 22/22 PASS in ~8 minutes
on SF=0.01 data, and `tpch_bug_regression_test` 6/6 PASS on
SF=0.001.

## References

- Issue: #2977 (TPC-H 22-query wire round-trip)
- PR #3086 — Phase 0 (SQLite-only expected row counts)
- PR #3089 — Phase 1a (regression markers, 4 ignored)
- PR #3093 — Phase 1b (fixture loader fix, 6/6 PASS)
- PR #3095 — Phase 3 (Macmini, parser + executor fixes, 18/22 in-process)
- `tests/load_local_infile_test.rs` — existing 5/5 LOAD DATA tests
  (covers small tables only, 5–80 rows)
- `docs/plans/2026-06-05-tpch-22-wire-three-way.md` — Phase 2 plan
