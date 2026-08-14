# Design — Multi-way join planner chain builder

## Root cause

`try_comma_join_hash_chain` (`src/engine_select.rs:1812-`) builds two artefacts
from the WHERE clause:

1. `join_tables: Vec<(bare, alias)>` — the set of tables the query references.
2. `pair_key: HashMap<(min_alias, max_alias), (col_l, col_r)>` — equality
   predicates keyed alphabetically.

The chain builder picks the first non-visited table that has any `pair_key` edge
to the current tail. This greedy choice is **not** optimal: in a star schema
where the hub has multiple spokes, the first spoke picked might lead to a dead
end before the other spokes are visited.

Concrete example: TPC-H Q9 has 6 distinct tables (`n, p, s, ps, o, l`) joined
through `nation ↔ supplier ↔ lineitem ↔ orders ↔ part ↔ partsupp`. The greedy
walker can reach `n, s, l, o, p` (5 tables) but then cannot continue from `p`
to `ps` because no `pair_key` edge connects them via the current tail — and
the alternative edge from `s → ps` was never recorded since `s → l` was picked
first. Result: chain_order.len() = 5, join_tables.len() = 6.

## Fix

Replace the greedy loop with a DFS over the join graph. The join graph is
defined implicitly by `pair_key`: an edge exists between two aliases iff a
`pair_key` entry covers them.

```
DFS(start=base_alias, visited, chain_order):
  for each (bare, alias) in join_tables, in stable declaration order:
    if alias in visited: continue
    if pair_key has any edge between current_tail and alias:
      push (bare, alias) to chain_order, add alias to visited
      result = DFS(tail=alias, visited, chain_order)
      if chain_order.len() == join_tables.len(): return SUCCESS
      else: pop (backtrack) and remove alias from visited
  return FAIL
```

This guarantees that **if a Hamiltonian-style spanning chain exists over the
connected component containing `base_alias`, DFS will find one** — it tries
every permutation of choices for each step.

When the graph is disconnected (some table has no `pair_key` edge to any other
table), the cartesian fallback is the only option. We surface that case
explicitly: chain_order gets a partial chain + remaining tables appended in
declaration order, and we still return `None` so the caller falls back — but
the `eprintln!` DBG line now includes the reason ("disconnected: X, Y").

## Diff sketch

```rust
// src/engine_select.rs:1971-2022 — replace greedy loop with DFS
let mut chain_order: Vec<(String, String)> =
    vec![(base_bare.clone(), effective_base_alias.to_string())];
let mut visited: HashSet<String> =
    [effective_base_alias.to_string()].into_iter().collect();

fn try_extend(
    tail: &str,
    join_tables: &[(String, String)],
    pair_key: &HashMap<(String, String), (String, String)>,
    chain_order: &mut Vec<(String, String)>,
    visited: &mut HashSet<String>,
) -> bool {
    for (bare, alias) in join_tables {
        if visited.contains(alias) { continue; }
        let edge = pair_key.keys().any(|(a1, a2)| {
            (*a1 == tail && *a2 == *alias) || (*a2 == tail && *a1 == *alias)
        });
        if !edge { continue; }
        chain_order.push((bare.clone(), alias.clone()));
        visited.insert(alias.clone());
        if try_extend(alias, join_tables, pair_key, chain_order, visited) {
            return true;
        }
        chain_order.pop();
        visited.remove(alias);
    }
    chain_order.len() == join_tables.len()
}

// Initial call after seed
let _ = try_extend(
    &effective_base_alias,
    &join_tables,
    &pair_key,
    &mut chain_order,
    &mut visited,
);

if chain_order.len() != join_tables.len() {
    // Honest-gap: graph disconnected, fall back to cartesian
    let missing: Vec<&str> = join_tables.iter()
        .filter(|(_, a)| !visited.contains(a))
        .map(|(_, a)| a.as_str())
        .collect();
    eprintln!(
        "DBG chain_order disconnected: visited={}/{}, missing={:?}",
        chain_order.len(), join_tables.len(), missing
    );
    return None;
}
```

## Test plan

`tests/integration/planner_multi_way_join_test.rs` (new file):
- Construct 6 in-memory tables (`a, b, c, d, e, f`) with key columns.
- Issue `SELECT * FROM a JOIN b ON a.id=b.a_id JOIN c ON b.id=c.b_id JOIN d ON c.id=d.c_id JOIN e ON d.id=e.d_id JOIN f ON e.id=f.e_id`.
- Assert `chain_order.len() == 6` and the result row count matches a
  hand-computed reference.

`tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` (existing, currently
`#[ignore]`):
- Remove `#[ignore]` only after the regression test passes on the fixture.
- Document the closure in `docs/releases/v3.12.0/evidence/issue-4181/4181_closeout.md`.

## Files touched

- `src/engine_select.rs` — replace greedy loop, ~30 lines.
- `tests/integration/planner_multi_way_join_test.rs` — new, ~80 lines.
- `openspec/specs/multi-join-3-table-resolution/spec.md` — extend Requirement
  to N-table case.
- `docs/releases/v3.12.0/evidence/issue-4181/4181_closeout.md` — new evidence
  file.

## Non-goals

- Optimizing join order (cardinality-aware). The DFS finds ANY chain; a future
  pass can prune by row-count estimates.
- Disconnected-graph support beyond the cartesian fallback already in place.
- Cross-engine parity (MySQL/MariaDB) — out of scope; only SQLite oracle parity
  required per #4181.