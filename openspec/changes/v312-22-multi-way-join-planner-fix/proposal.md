## Why

Issue #4181 — TPC-H Q7/Q8/Q9 SF=1 queries return 0 rows because the multi-way join
planner's greedy chain builder in `try_comma_join_hash_chain` gets stuck before
visiting every table. The current implementation picks the **first** unvisited
table with a `pair_key` edge to the current tail and never backtracks:

```rust
// src/engine_select.rs:1983-2013
while visited.len() < join_tables.len() {
    let tail_alias = chain_order.last().map(|(_, a)| a.clone()).unwrap_or_default();
    let mut found: Option<(String, String)> = None;
    for (bare, alias) in &join_tables {
        if visited.contains(alias) { continue; }
        let matches = pair_key.keys().any(|(a1, a2)| {
            (*a1 == tail_alias && *a2 == *alias) ||
            (*a2 == tail_alias && *a1 == *alias)
        });
        if matches {
            found = Some((bare.clone(), alias.clone()));
            break; // <-- greedy: take first match, no backtrack
        }
    }
    match found {
        Some((next_bare, alias)) => {
            chain_order.push((next_bare, alias.clone()));
            visited.insert(alias);
        }
        None => break, // <-- silent partial chain, returns None below
    }
}

if chain_order.len() != join_tables.len() {
    eprintln!("DBG chain_order.len()={} != join_tables.len()={}", ...);
    return None;
}
```

Observed SF=1 failures (from `docs/releases/v3.12.0/perf/SF1_BASELINE_REPORT.md` and
`docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`):

| Query | join_tables | chain_order.len() | gap |
|-------|-------------|-------------------|-----|
| Q7    | 6           | 5                 | missing 1 table |
| Q8    | 7           | 5                 | missing 2 tables |
| Q9    | 7           | 6                 | missing 1 table |
| Q21   | 4           | 3                 | missing 1 table |

When `try_comma_join_hash_chain` returns `None`, the caller falls back to the
cartesian-product path which then either OOMs or executes without filter
pushdown, producing 0 rows for the required-non-empty queries (#3650 P0-2).

## What Changes

- Replace greedy chain-order builder in `src/engine_select.rs::try_comma_join_hash_chain`
  with a DFS that explores the join graph from the base alias and constructs any
  Hamiltonian-style spanning chain through all `join_tables`.
- When the join graph is connected, the DFS is guaranteed to find a chain that
  covers every table. When the graph is disconnected (e.g., a table without any
  WHERE equality), fall back to appending the unreachable tables in any order so
  the cartesian fallback still has well-defined inputs.
- Add `tests/integration/planner_multi_way_join_test.rs` regression test that
  builds a 6-table join in memory and asserts the chain covers every table
  (chain_order.len() == join_tables.len()).
- Verify with the existing SF=1 fixture that Q7 and Q9 now return non-zero rows
  matching the SQLite oracle row counts.

## Capabilities

### New Capabilities

- `multi-join-n-table-resolution`: extends `multi-join-3-table-resolution`
  to handle 4-7 table joins (TPC-H Q7/Q8/Q9/Q21) without silent `None` returns.

### Modified Capabilities

- `multi-join-3-table-resolution`: replace greedy single-table picker with
  graph-based DFS so 3-table joins continue to work but larger joins no longer
  fail.

## Acceptance Criteria (per issue #4181 close boundary, expiry 2026-09-15)

- Q7-Q9 on SF=1 return matching row count vs SQLite oracle.
- Q10-Q22 on SF=1 at least produce a non-error result (no panic, no infinite loop).
- New regression test `tests/integration/planner_multi_way_join_test.rs` passes
  on the merged `develop/v3.12.0` branch.
- Fix commit reachable from `develop/v3.12.0`.
- `evidence_hash` + log path written in the issue #4181 comment.