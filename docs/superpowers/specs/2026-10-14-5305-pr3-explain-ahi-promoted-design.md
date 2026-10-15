# PR3 / #5305 — EXPLAIN `ahi_promoted` column design

> Status: design (post-brainstorm, awaiting approval before plan + execute).

## Intent and why

PR3 / #5305 ships the **first honest EXPLAIN data column** for v4.1.0.
PR2 / #5304 surfaced the trait plumbing (`StorageEngine::buffer_pool_stats`,
`AdaptiveHashIndex::promoted_count`, etc.). PR3 consumes one of those
counters — and only one — and exposes it inside EXPLAIN's output.

The accepted counter is **`AdaptiveHashIndex::promoted_count() -> u64`**:

- It is the **only AHI counter that the production execution path actually
  drives** today (`record_access` is called from 6 sites in
  `engine_select.rs:1544, 1590, 3823, 3847, 3877, 3904`, gated by
  secondary-index lookup; promotion happens after 17 accesses to the same
  `(table, key, page)`).
- `total_lookups()` and `total_hits()` stay 0 in production because
  `AdaptiveHashIndex::lookup` has 0 callers in the workspace today
  (PR2 Ruling 9 final).
- `buffer_pool_stats()` returns `Some(synthetic_zero())` for every
  engine (PR2 default), so `pages_visited`, `ahi_lookups`, `ahi_hits`
  would all read 0 and mislead users — PR2 spec §6.3 explicitly opted
  out of exposing those.

This PR advances the A05 #2 acceptance gate (IAIA-410) by giving the
audit one real number to compare against pre-PR2 baseline.

## Goals

- EXPLAIN output appends ` | ahi_promoted=<N>` to every operator line,
  where `N` is `engine.adaptive_hash_index().promoted_count()` read at
  EXPLAIN entry.
- One helper-function change site: `ExecutionEngine::execute_explain` at
  `src/execution_engine_methods.rs:549-559`. The lower-level helper
  `explain_select_plan(select, storage) -> Vec<String>` keeps its
  signature.
- One new test in `tests/integration/sql/explain_ahi_promoted_test.rs`
  asserting (a) the column appears, (b) the value matches the AHI
  counter at the moment of EXPLAIN, (c) the value is honest (0 for a
  fresh engine, N after threshold crossings via `record_access`).
- Wire protocol, REPL, MySQL server all unaffected — they consume the
  same `ExecutorResult::new(rows, 0)` shape with `Value::Text` content
  extended.

## Non-goals

- Add `pages_visited` / `ahi_lookups` / `ahi_hits` columns (they would
  always read 0 today; PR2 spec §6.3 opted out).
- Add a header row to EXPLAIN (existing output is `Vec<String>` single
  column, headerless by design — `crates/executor/src/explain.rs` and
  the `plan_shape` oracle normalizer expect this shape).
- Make EXPLAIN actually execute the secondary-index lookup path so the
  counter increments during EXPLAIN (1-3 month epic, out of scope).
- Change EXPLAIN's row schema (still `Vec<Vec<Value>>` with single
  `Value::Text` column).
- Touch `crates/executor/src/explain.rs` (the operator-side explain
  output for query results, not the SELECT-plan output this PR targets).

## Anti-goals

- Showing a value other than the literal `promoted_count()` reading
  (no `N/A`, no `-1`, no hidden column when 0). Honest means the user
  sees the real number, even when it is 0.
- Coupling EXPLAIN to the planner/CBO refactor (`plan_select` refactor
  is deferred per V312-56E doc-comment).
- Changing `explain_select_plan`'s signature (would force every test
  and oracle to recompile — only `execute_explain` changes).

## Constraints

- v4.1.0 freeze semantics: additive, opt-in by surface, no wire-format
  change. EXPLAIN row content is `Value::Text`; extending the string is
  safe for any consumer that already treats EXPLAIN output as free-text.
- Rust ≥ 1.73 (already pinned in `rust-toolchain` / CI; PR2 Ruling 6).
- Root crate uses explicit `[[test]]` entries for new integration tests
  (PR2 Task 4; add `[[test]] name = "explain_ahi_promoted_test"`).
- A05 audit honest framing: the column is **observable** (real number),
  but its growth depends on the secondary-index path firing — for
  MemoryStorage SELECT WHERE (no B+Tree secondary indexes), it stays 0
  (existing PR2 `test_ahi_via_engine_memory_storage_no_lookup_wired`
  pins this). The test must document this so a future regression toward
  wiring shows up as an assertion failure.

## How (decided)

### Single call site

`src/execution_engine_methods.rs:549-559`:

```rust
pub fn execute_explain(
    &self,
    select: &sqlrustgo_parser::parser::SelectStatement,
) -> SqlResult<ExecutorResult> {
    // PR3 / #5305: snapshot AHI counter at EXPLAIN entry so the value is
    // stable across all emitted operator lines. This is the only AHI
    // counter that the production path actually drives; PR2 spec §6.3
    // opted out of exposing `pages_visited` / `ahi_lookups` / `ahi_hits`
    // (always 0 today).
    let ahi_promoted = self.adaptive_hash_index().promoted_count();
    let ahi_suffix = format!(" | ahi_promoted={ahi_promoted}");

    let lines = explain_select_plan(select, &*self.storage.read());
    let rows: Vec<Vec<Value>> = lines
        .into_iter()
        .map(|line| vec![Value::Text(format!("{line}{ahi_suffix}"))])
        .collect();
    Ok(ExecutorResult::new(rows, 0))
}
```

`explain_select_plan(select, &dyn StorageEngine) -> Vec<String>` is
unchanged. No new parameters.

### Format detail

Each emitted line becomes:

```
<existing operator line> | ahi_promoted=<u64>
```

The suffix is appended unconditionally to every line, including the
case where the existing EXPLAIN emits zero lines (rare but legal:
empty `Vec<String>` → zero rows, no header noise). The leading pipe
` | ` matches the SQLite `EXPLAIN QUERY PLAN` separator idiom and is
machine-greppable.

### New integration test

File: `tests/integration/sql/explain_ahi_promoted_test.rs`.

Tests:

1. `test_explain_includes_ahi_promoted_suffix_on_fresh_engine`:
   `ExecutionEngine::with_memory()`, `CREATE TABLE`, EXPLAIN a SELECT.
   Assert every output row string ends with ` | ahi_promoted=0` and the
   AHI snapshot at EXPLAIN time equals 0.

2. `test_explain_ahi_promoted_reflects_promoted_entries_after_record_access`:
   Direct AHI `record_access` 20 times past threshold=17 to force at
   least 1 promotion, then EXPLAIN. Assert suffix reads
   ` | ahi_promoted=N` where `N >= 1` and matches
   `engine.adaptive_hash_index().promoted_count()`.

3. `test_explain_does_not_increment_ahi_promoted_during_explain`:
   Read `ahi_promoted` before EXPLAIN, run EXPLAIN, read after. Assert
   they are equal. Documents the snapshot semantics so any future
   change that makes EXPLAIN drive AHI shows up as a regression.

### Cargo.toml entry

Add:

```toml
[[test]]
name = "explain_ahi_promoted_test"
path = "tests/integration/sql/explain_ahi_promoted_test.rs"
```

Root crate uses explicit discovery, not auto (PR2 Task 4, confirmed by
`b9c28de` recon).

## What's left to the builder

- The 3 test bodies (assertions written as code, exact thresholds from
  spec; build expected strings from `format!`).
- Wire the `[[test]]` entry into `Cargo.toml` (alphabetic position:
  after `explain_*` neighbours; look for the closest neighbour when
  implementing).
- `cargo test -p sqlrustgo --test explain_ahi_promoted_test` exits 0
  with 3/3 pass.
- `cargo build --all-features`, `cargo clippy --all-features -- -D
  warnings`, `rustfmt --check` all exit 0. `cargo doc --no-deps`
  introduces 0 new warnings (PR2 baseline: 6 pre-existing).
- `cargo test --all-features` shows no NEW failures beyond the
  pre-existing D8/SEM-4 `test_d8_gate_runs_correctly` failure (#5103
  debt; PR2 Ruling 4 v2 baseline) and the intermittent
  `test_v410_atomic_parallel_degree_getter_clamps_to_one` flake
  (Ruling 10). Both reproduce on un-touched `develop/v4.1.0`.

## Out-of-scope followups (epics)

- Wire `AdaptiveHashIndex::lookup` into the secondary-index execution
  path so `ahi_lookups` / `ahi_hits` become observable. Without this,
  PR3 is the upper bound of honest EXPLAIN data for v4.1.0.
- Wire `BufferPool::pin/unpin/flush_page` into a real paged engine so
  `pages_visited` becomes observable. ~6-12 month rewrite of
  `FileStorage` HashMap → Page-backed heap files (PR2 design §2.2
  non-goal).
- Add a multi-column EXPLAIN schema (V312-56E follow-up). Deferred
  until the CBO planner (`plan_select`) lands.
