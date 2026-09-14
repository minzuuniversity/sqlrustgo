# Phase D — Read-only TPS Improvement Analysis

**Date**: 2026-09-15
**Baseline**: ec6d96ac87 (Phase C.1 done + Phase C.2 outcome doc)
**Method**: pymysql persistent connection × 4 threads × 15s, schema 10000 rows × 5 cols (BIGINT PK + BIGINT + CHAR(1) + DECIMAL + DATE), random single-PK equality SELECT

## Headline numbers

| Schema | Bench | OPS | Path |
|---|---|---|---|
| PK=id, `WHERE id = ?` | pymysql 4t×15s | **3831** | PK B+Tree lookup (scan_pk) |
| PK=o_orderkey, `WHERE o_orderkey = ?` | pymysql 4t×15s | **1977** | Full scan + Vec::retain |
| Speedup from PK lookup | | **1.94×** | |

**Hardware**: macOS 26.6.2 / Apple Silicon (M-series), release build, default server_threads=16. Single M-series core equivalent under pymysql load (network-attached client saturates at ~4-8k qps).

## Bottleneck 1: PK column hard-coded to `"id"`

**Two layers**, both broken:

1. **`src/engine_select_pk.rs:18`** — `DEFAULT_PK_COLUMN = "id"`. The `try_extract_pk_eq` function only detects PK lookups when the WHERE column is literally `id`. Any other PK column name falls back to full table scan.

2. **`crates/storage/src/file_storage.rs:3321`** — `scan_with_index(table, "id", pk)`. Even after a successful PK lookup, this dispatches the index by hard-coded `"id"` — so a PK lookup on `o_orderkey` falls back to **full scan + linear find** (line 3328-3333), which is the worst of both worlds (allocates full Vec of rows, then iterates).

**Empirical proof**: Just renaming the table's PK column from `o_orderkey` to `id` (no code changes!) yielded **3831 OPS vs 1977 OPS** — a 1.94× speedup without touching a single line of Rust.

**Fix**: Both call sites should look up the table's actual `primary_key: true` column from `TableInfo` (already computed at line 3320: `info.columns.iter().find(|c| c.primary_key)`). The lookup result is currently discarded (`_pk_col`). Need to keep the name and use it in both `scan_with_index` and `try_extract_pk_eq`.

**Estimated effort**: 1-2 hours (small, isolated fix in 2 files).

## Bottleneck 2: PK lookup path overhead

`sample` of worker thread on PK lookup path shows:

```
execute_select                        4699 ticks
└── scan_pk                            2612 (55% of execute_select)
    └── Vec<&[Value]>::clone           2566 (98% of scan_pk)
        └── String::clone              1067
            └── xzm_xzone_malloc       323 (heap alloc cost)
└── Packet::read_from                 667  (input parse)
└── send_result_set_with_more         200+ (output serialise)
└── parse (SQL parser)                ~40  (mostly trivial)
```

**Where the cost comes from**:

- `scan_with_index` line 3360: `data.rows[row_id as usize].clone()` — clone entire Record (Vec<Value>)
- `scan_pk` line 3323: `rows.into_iter().next()` — produces an owned Record
- `engine_select.rs:1370`: `row.map(|r| vec![r])` — allocates outer Vec<Record> wrapper

For a 1-row match, that's **1 row clone + 1 outer Vec alloc per query**. With `Value::Text` columns, every clone is a heap-allocated `String::clone` (line 2923 of mysql-server `value_to_string` also clones).

For read-only traffic, the row is consumed exactly once and discarded. Borrowing would avoid all of it.

**Fix candidates** (in increasing scope):

1. **Make `scan_pk` return `Cow<'_, Record>`** — caller chooses borrow or own. Read-only paths take Borrow; mutation paths Clone. ~30 lines, 2-3 hours.
2. **Make scan return `impl Iterator<Item = &Record>`** — caller iterates without ever materializing Vec. Larger trait change (~7 impls × 180 callsites), 1-2 weeks.
3. **Replace `Record = Vec<Value>` with `Record = Box<[Value]>` or Arc<[Value]>** — cheaper clone (refcount bump vs heap alloc). Won't help if read path still allocates a fresh outer Vec per query.

Estimated speedup: 1.5-2× on the PK lookup path.

## Bottleneck 3: Result-set wire encoding

`send_result_set_with_more` (line 3132-3143 of mysql-server/src/lib.rs):

```rust
for r in rows.iter() {
    let mut p = Vec::new();             // ← alloc per row
    write_text_row(&mut p, r)?;         // calls value_to_string per cell
    Packet { length: p.len() as u32, sequence: seq, payload: p }.write_to(w)?;
    seq = seq.wrapping_add(1);
}
```

`value_to_string` (line 2917) allocates a new `String` per cell, then writes its `.as_bytes()`:

```rust
fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Boolean(b) => if *b { "1" } else { "0" }.into(),
        Value::Integer(i) => i.to_string(),    // String alloc
        Value::Float(f) => format!("{}", f),   // String alloc
        Value::Text(s) => s.clone(),           // String alloc (heap copy)
        ...
    }
}
```

**Three allocations per text cell**: `value_to_string` String, then outer `Vec::new()`, then `Packet::payload`. Plus `Packet::write_to` allocates a 4-byte header buffer. For a 5-column × 1-row result, that's **15+ allocations per query** on the wire side alone.

**Fix**: A `write_text_row_into<W: Write>(w: &mut W, row: &[Value])` that writes directly to the TCP stream (skipping the per-row `Vec` and the `Packet::payload`):

- `Value::Integer(i)` → `itoa::write(w, i)` (no String)
- `Value::Text(s)` → `w.write_all(s.as_bytes())` (no clone, no String)
- `Value::Float(f)` → ryu crate (no String)
- `Value::Null` → `w.write_u8(0xfb)`

Each row becomes a single ~200-byte TCP write. Eliminates ~10 allocs/query.

Estimated speedup: 1.3-1.8× on result-set encoding. Bigger impact for multi-row results (e.g., `SELECT * FROM t LIMIT 100`).

## Bottleneck 4: Full-scan path (currently 1977 OPS, 49% CPU in Vec::retain)

When PK lookup fails (non-id column, or no PK at all), the path is:

1. `storage.scan(table)` → clones full `data.rows` Vec (~10000 Record clones per query)
2. `Vec::retain(|row| eval_predicate(...))` → per-row predicate evaluation
3. Per-row: `eval_predicate_with_subq` → `eval_identifier` → `find_column_index` (memcmp), `to_uppercase` (case-insensitive cmp), and SQL `compare`.

**Sample shows** (full-scan path): `Vec::retain` consumes 3631/7475 = 49% of `execute_select` CPU. Of that, 49% is `eval_predicate_with_subq` and 22% is `sql_compare` (which in turn is dominated by `to_uppercase` — 279 ticks — for case-insensitive string comparison).

**Fix candidates**:

1. **Streaming scan iterator** (`scan() -> impl Iterator<Item = &Record>`) — eliminates the full-row Vec clone. Requires trait change, large refactor.
2. **Constant folding in WHERE** — `o_orderkey = ?` against a column the planner already knows is PK → use index. This is exactly what `try_extract_pk_eq` is supposed to do, but its hard-coded `"id"` defeats it (see Bottleneck 1).
3. **Adaptive hash index (AHI)** — already exists in code (`adaptive_hash_index.record_access`); promote hot lookups. May already be doing this in Phase B.

Estimated speedup after fixing PK column detection: most non-PK workloads become PK lookups automatically.

## Bottleneck 5: Phase C.2 deferred — server-layer lock contention

Already analyzed (see `PHASE_C_2_OUTCOME.md`). The `Arc<RwLock<S>>` outer lock serializes all read access via `storage_read() -> RwLockReadGuard`. Even with internal locking (`write_lock` from C.1), every reader must acquire and release the outer read lock. Removing this is non-trivial because:

- `StorageEngine` trait has 39 `&mut self` methods
- Rust aliasing rules forbid shared `&Arc<S>` while writing `&mut S`
- Would need UnsafeCell redesign or 7 impls × 180 callsites signature change

Estimated additional speedup if solved: 2-3× (this was Phase C's original target). **Deferred** — see prior doc for rationale.

## Recommended priority

1. **Fix PK column hard-coding** (Bottleneck 1) — 1-2 hours, 1.94× proven speedup
2. **Wire encoding optimization** (Bottleneck 3) — 2-3 days, 1.3-1.8× estimated
3. **PK lookup path borrow** (Bottleneck 2) — 2-3 hours, 1.5-2× estimated (compounds with #1)
4. **Streaming scan iterator** (Bottleneck 4) — 1 week, 1.5-2× for non-PK workloads
5. **Phase C.2 revisit** (Bottleneck 5) — 1-2 weeks, 2-3× but high risk

Combined realistic ceiling with #1-#3: 3000-5000 OPS for PK workloads, 2500-3500 for non-PK (full scan eliminated via #1).

## Appendix: bench tooling

`/tmp/sqlrustgo-pymysql-bench.py` — pymysql-based, persistent connection per thread, real per-query latency. Avoids the mysql CLI batch-write issue (CLI defers all queries until stdin closes, which makes timing misleading).

`/tmp/sqlrustgo-cli-bench.py` — mysql CLI per-query, accurate end-to-end but limited to ~70 qps due to fork+exec overhead. Useful for sanity check.

`/tmp/sqlrustgo-bench-v3.py` — streaming stdin to mysql CLI, but mysql CLI closes connection at EOF so timing still off.

`/tmp/seed.sql` and `/tmp/seed_pk.sql` — seed scripts for non-PK and PK-`id` schemas respectively.
