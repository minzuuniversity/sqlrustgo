## Context

V311-15 Hash Semi Join fixed `EXISTS` for TPC-H Q4 (≥2.9x speedup). The **natural counterpart** `NOT EXISTS` was left unchanged: it still uses the `SubqueryIndex` structure added during V311-15's work, but the dispatch in `pre_evaluate_correlated_exists` performs a linear scan over `qualifying_rows` per outer row even though the inverted semantics (`match found ⇒ exclude this outer row`) makes the operation O(outer × inner) in the worst case.

Worse: for TPC-H Q21's `NOT EXISTS` nested twice (each with its own correlated `l3.l_suppkey <> l1.l_suppkey` filter), the per-row fanout is the dominant cost. The naive path is structurally identical to Semi Join but with inverted early-exit:

| Operator | Inner Match? | Output? |
|----------|--------------|---------|
| Semi Join | yes (early exit on first match) | outer row included |
| Anti Join | no  (must verify NO match) | outer row included |

Anti Join cannot early-exit on first match — it has to walk **all** candidate rows to confirm "no match". This makes the naive scan much worse than Semi Join's "first match short-circuits".

### Target Architecture

V311-15 already added `key_to_rows: HashMap<Value, Vec<Vec<Value>>>` to `SubqueryIndex` for O(1) bucket lookup. **This is exactly what Anti Join needs**:

```text
NOT EXISTS (SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND ...):
  Outer rowset: orders (N rows)
  Inner rowset: lineitem (M rows)

  build_subquery_index() = HashMap<Value, Vec<Vec<Value>>>     ← already in V311-15
  ┌──────────────────┐
  │ key = order_key  │
  │ bucket = [...]   │   ← existing structure, just reuse
  └──────────────────┘

  HashAntiJoin::execute():
    For each outer row r:
      1. Lookup r.orderkey in bloom_filter
         - If definitely NOT in set (no false negative for in-set): skip HashMap
         - Else (might be in set OR bloom positive): check key_to_rows[orderkey]
      2. For each inner row in bucket:
         - Run residual (e.g. l3.l_suppkey <> l1.l_suppkey)
         - If any match → this outer row is excluded
      3. If no match found after all checks → include outer row in output
```

The bottleneck removal: **Anti Join only walks inner rows when bloom filter OR key_to_rows has bucket entries**. For TPC-H Q21 with ~14 lineitem per orderkey, this turns naive 1.35T ops into ~14 × outer_rows = O(outer).

## Goals / Non-Goals

**Goals:**
- O(outer + inner) for NOT EXISTS (vs current O(outer × inner))
- Reuse SubqueryIndex + bloom filter from V311-15 (no parallel infra)
- TPC-H Q21 measurable speedup (≥3x at SF=0.1)
- No regression on `EXISTS` (V311-15 path) or `IN/NOT IN` (V311-23 path)

**Non-Goals:**
- Multi-column keys (deferred to V311-17-multi follow-up)
- Inner-side JOINs (already handled by V311-15's `try_inner_join_aware` rule)
- Persistent disk-backed index (v3.12)

## Decisions

### Decision 1: HashAntiJoin as Volcano iterator (matches `HashJoin`)

**Choice**: Implement `HashAntiJoin` as Volcano pull-iterator. Build phase constructs the key→rows map on first `next()`. Probe phase lazily returns outer rows that pass NOT EXISTS.

**Alternative considered**: Materialize all outer rows, then filter in one pass. Rejected — doesn't fit Volcano's pull model used throughout the codebase.

**Tradeoff**: ~250 LoC, matches existing `parallel_hash_join.rs` style.

### Decision 2: Reuse V311-15's `SubqueryIndex` (no new structure)

**Choice**: AntiJoin consumes the existing `SubqueryIndex`. No new fields needed; only new dispatch logic in `pre_evaluate_correlated_exists`.

**Alternative considered**: Add `key_to_residual_match: HashMap<Value, bool>` flag per key. Rejected — the residual logic already correctly handles per-bucket re-evaluation.

### Decision 3: Bloom filter for short-circuit "definitely not match"

**Choice**: For each outer row, first check bloom filter against the (already-built) inner key set. If bloom says "definitely no", Anti Join emits the row immediately (early termination).

**Why this is safe**: bloom filter has **no false negatives** for items in the set. A "not in bloom" ⇒ definitely not in key_to_rows ⇒ definitely NOT EXISTS ⇒ emit outer row. **A "in bloom" requires HashMap confirmation** — but in that case bloom was just a hint, the real check is the same.

For TPC-H Q21 typical data: many lineitem rows per orderkey → most outer rows look up keys that DO exist → bloom is unhelpful for them. But for "rare-key" outer rows (orders without any matching lineitem), bloom short-circuits in O(1).

### Decision 4: First-Match Termination disabled in Anti Join (correctness)

**Choice**: In Semi Join, the inner loop exits on first match. In Anti Join, **no early termination**: keep checking residual_predicate across ALL bucket rows because residual may have outer refs (TPC-H Q21: `l3.l_suppkey <> l1.l_suppkey`).

**Why**: For Semi Join, finding ANY match means EXISTS is true → emit. For Anti Join, finding ONE match means NOT EXISTS is false → exclude. But there might be a different residual_match_result for a different bucket entry.

If residual has no outer refs (pure-static predicate), then **one match = all match**, so early-termination is OK. We can detect this case and short-circuit.

### Decision 5: Keep pre-existing naive path as fallback

**Choice**: When `build_subquery_index` returns None (complex WHERE: joins/ORs/multiple keys), fall through to existing naive `execute_select + .is_empty()`. No behavior regression.

**Tradeoff**: Some complex NOT EXISTS queries won't see Anti Join speedup. Acceptable for V311-17 — they were slow before; they're still slow but not slower.

## Implementation Plan

### Phase 1: `crates/executor/src/join/hash_anti_join.rs` (NEW ~250 LoC)

```rust
pub struct HashAntiJoin {
    pub build_keys: Vec<usize>,           // columns of inner row to hash
    pub probe_keys: Vec<usize>,           // columns of outer row to hash
    pub build_side: Vec<Record>,          // pre-collected inner rows (passed in)
    pub probe_side: Box<dyn Iterator<...>>,// pulled from child
    pub residual: Option<Expression>,     // post-key filter
}

impl HashAntiJoin {
    pub fn build(&mut self) { /* build bloom + key map */ }
    pub fn next(&mut self) -> Option<Record> { /* Anti Join outer rows */ }
}
```

Internal:
- `bloom: [u64; 16]` (128 bytes from V311-15)
- `key_index: HashMap<Value, Vec<Record>>` (re-uses V311-15 semantics)
- `match_found: HashSet<Value>` (probe keys that found a match → excluded)

### Phase 2: NOT EXISTS routing in `pre_evaluate_correlated_exists`

Replace the `Expression::NotExists` arm:
```rust
Expression::NotExists(subq) => {
    let substituted = substitute_outer_refs_in_select(subq, ...);
    
    // V311-17: try Anti Join path
    let any_matched = if let Some(index) = subquery_indexes.get(*cursor) {
        // Call pre_eval_not_exists_indexed (NEW)
        self.pre_eval_not_exists_indexed(...)
    } else {
        // Fallback: pre-existing naive path
        !self.pre_eval_exists_subquery_fast(&substituted, ...)...
    };
    
    *cursor += 1;
    Expression::Literal(if any_matched { "false" } else { "true" }.to_string())
}
```

New `pre_eval_not_exists_indexed`:
```rust
fn pre_eval_not_exists_indexed(&self, ..., index: &SubqueryIndex) -> Option<bool> {
    let lit = find_top_level_equality_literal(where_expr, index.col_idx)?;
    
    // Optimization 1: bloom filter says definitely NOT in set → emit outer row
    if !bloom_might_contain(&index.bloom, &lit) {
        return Some(true);  // NOT EXISTS is true (no match)
    }
    
    let bucket = index.key_to_rows.get(&lit)?;
    
    // Optimization 2: pure-static residual (no outer refs)
    if !residual_has_outer_ref(&index.residual) {
        // Build-time filter already removed non-matching rows → bucket empty = NOT EXISTS
        return Some(bucket.is_empty());
    }
    
    // Slow path: residual has outer refs (TPC-H Q21 case)
    // Walk bucket, looking for ANY residual match
    for inner in bucket {
        let substituted = substitute_outer_refs_in_expr(&index.residual, ...);
        if eval_predicate(&substituted, inner, ...) {
            return Some(false);  // Found a match → NOT EXISTS is false → exclude
        }
    }
    Some(true)  // No match found → NOT EXISTS is true → include
}
```

### Phase 3: Anti Join as standalone operator (alternative path)

For callers that don't use SubqueryIndex, expose `HashAntiJoin::execute()` as standalone API. Used by ad-hoc optimizations and tests, but core path goes through `pre_eval_not_exists_indexed`.

### Phase 4: Tests (`tests/integration/sql/anti_join_main_path_test.rs`)

6 tests:
1. `not_exists_returns_rows_with_no_inner_match` — basic correctness
2. `not_exists_with_residual_predicate` — TPC-H Q21 shape
3. `not_in_equivalent_correctness` — `NOT IN` vs `NOT EXISTS` semantics
4. `mixed_exists_not_exists_in_q21` — both arms coexist
5. `bloom_filter_short_circuit` — verify metric shows short-circuit hits
6. `large_scale_10k_under_5_seconds` — perf bar test

### Phase 5: Docs (`docs/releases/v3.11.0/perf/HASH_ANTI_JOIN_PERF.md`)

Document:
- Algorithm overview with diagram
- Benchmark results (Q21 SF=0.1, SF=1)
- Bloom filter effectiveness analysis
- Migration notes for callers using naive NOT EXISTS

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Anti Join semantics: skip-keys logic off-by-one | High | Unit tests verify exact row counts from `q21_exists_hash_path_test.rs` continue passing |
| Bloom filter false negatives | Low | Bloom is only used for "skip HashMap" path; false positive still does HashMap; no false negatives |
| Build phase memory: large bucket still copied | Medium | Bloom & HashMap are owned by SubqueryIndex; only shallow references in HashAntiJoin |
| Anti Join with `not_in_equivalent` differs from `not exists` | Medium | Use `pre_evaluate_non_correlated_in_subquery` (V311-23) for `NOT IN` — separate path |

## Verification Strategy

- Functional: 6 unit + integration tests
- Performance: perf bar in test #6 + `docs/.../HASH_ANTI_JOIN_PERF.md`
- Regression: existing 16+ F-23 tests still PASS
