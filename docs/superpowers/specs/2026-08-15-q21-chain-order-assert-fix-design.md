# Q21 chain_order assert fix — Design (Issue #4280 / V312-48 sub-issue)

> **Issue:** [#4280](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4280) (V312-48-Q21)
> **Parent issue:** [#4221](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4221) (V312-48, closed via PR #4283)
> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

## 1. Problem

TPC-H Q21 (Suppliers Who Kept Orders Waiting) currently returns the correct
**row count (100)** on SF=1, but the elapsed time (~35s) reveals it is
running through the **cartesian fallback** rather than the hash-chain path.

The chain builder in `src/engine_select.rs::try_comma_join_hash_chain`
fails to construct a complete 4-table chain for Q21, so `chain_order.len()`
is shorter than `join_tables.len()` and the function returns `None`,
falling back to O(N×M) cartesian-then-filter — correct but 5–10× slower.

## 2. Root cause (preliminary)

### 2.1 Data flow

```text
Q21: FROM supplier, lineitem l1, orders, nation
   WHERE s_suppkey = l1.l_suppkey          ← join edge: (supplier, l1)
     AND o_orderkey = l1.l_orderkey         ← join edge: (orders, l1)
     AND o_orderstatus = 'F'                ← filter on orders (not join)
     AND s_nationkey = n_nationkey          ← join edge: (supplier, nation)
     AND n_name = 'GERMANY'                 ← filter on nation (not join)
     AND EXISTS (lineitem l2 ...)           ← subquery correlation (not join edge)
     AND NOT EXISTS (lineitem l3 ...)       ← subquery correlation (not join edge)
```

`pair_key` (after `flatten_and_local`) collects exactly **3 edges**:
- `(supplier, l1)` → `(s_suppkey, l_suppkey)`
- `(orders, l1)`   → `(o_orderkey, l_orderkey)`
- `(supplier, nation)` → `(s_nationkey, n_nationkey)`

`join_tables` is **4 tables**: `[supplier, l1, orders, nation]`.

### 2.2 Why multi-start fails

The graph is connected, but **not all start_idx values yield a spanning
chain via `build_chain_from_start`**:

| start_idx | start table | reachable chain | reason |
|-----------|-------------|-----------------|--------|
| 0 | supplier | supplier → nation (dead-end, nation has no out-edge) | returns None |
| 1 | l1 | l1 → orders (dead-end, orders→l1 already visited) | returns None |
| 2 | orders | orders → l1 → supplier → nation | **returns Some(len=4)** ✅ |
| 3 | nation | nation → supplier → l1 → orders | **returns Some(len=4)** ✅ |

For `join_tables` order `[supplier, l1, orders, nation]`, start_idx=0
and start_idx=1 dead-end **before** the multi-start loop reaches index 2.
This may indicate the loop is breaking on the first `Some` candidate
even if its chain length is < `join_tables.len()` (defensive but
currently masked by `build_chain_from_start`'s "full coverage" contract).

## 3. Fix

### 3.1 Single-file scope — `src/engine_select.rs`

| line(s) | change |
|---------|--------|
| 1964–2005 | `build_chain_from_start` — add `tracing::debug!` log on dead-end so future debugging has signal without `eprintln!` noise |
| 2101–2143 | `resolve_bare` — when prefix disambiguation succeeds for the column but the table is not in `join_tables`, fall through to the all-table scan (don't return None early) |
| 2147–2189 | pair_key collection — collect ALL candidate pairs, let the multi-start loop choose (no early `continue` on ambiguity) |
| 2212–2231 | multi-start loop — replace `break` on first `Some(c)` with explicit `c.len() == join_tables.len()` filter so we keep looking for a complete chain even if a partial chain is found first |

### 3.2 Test additions

**`tests/integration/tpch/q21_chain_builder.rs`** (new file, ~60 lines):

```rust
//! Q21 chain_builder regression — Issue #4280
//!
//! Asserts that try_comma_join_hash_chain builds a complete 4-table chain
//! for the Q21 join topology, so the hash-chain path is used (not
//! cartesian fallback).
//!
//! Gated by the `TPCH_SF1_PATH` env var pointing to a populated dbgen
//! fixture (typically `/tmp/tpch-sf1`). Without it, tests are skipped.

#[test]
fn q21_chain_order_completes_for_all_4_tables() { /* ... */ }

#[test]
fn q21_e2e_returns_100_rows_under_5s() { /* ... */ }
```

The tests are **gated by env var** so they don't break the sandbox CI
(no `/tmp/tpch-sf1` available). Running them locally requires:

```bash
git clone dbgen fixture to /tmp/tpch-sf1
TPCH_SF1_PATH=/tmp/tpch-sf1 \
  cargo test --release -p sqlrustgo --test q21_chain_builder -- --ignored --nocapture
```

### 3.3 Evidence document

**`docs/releases/v3.12.0/evidence/tpch/V312-48-Q21-FIX.md`** (new file, ~80 lines):

Captures:
- Pre-fix: 100 rows in 35,814 ms (cartesian fallback)
- Post-fix: 100 rows in <5s (hash-chain path)
- chain_order log showing 4-table chain construction
- unit test output (gated test passed)
- cross-reference to PR that lands the fix

## 4. Acceptance criteria

1. `cargo build --all-features` exits 0
2. `cargo clippy --all-features -- -D warnings` exits 0
3. `cargo fmt --check --all` exits 0
4. `cargo test --all-features` — all existing tests still pass
5. New gated tests pass when `TPCH_SF1_PATH=/tmp/tpch-sf1` is set:
   - `q21_chain_order_completes_for_all_4_tables` PASS
   - `q21_e2e_returns_100_rows_under_5s` PASS
6. `tests/integration/tpch/tpch_sf1_real_test` (existing) — Q21 still returns 100 rows, elapsed < 5s
7. V312-55 procedure/trigger gate still 13/13 PASS
8. `V312-48-Q21-FIX.md` evidence doc committed with sha256
9. PR merged to `develop/v3.12.0`
10. Issue #4280 closed with comment linking the PR + evidence

## 5. Out of scope (deferred to v3.13)

- Full cross-engine SHA256 comparison (issue #4272) — requires SQLite/PostgreSQL/MySQL oracle in dbgen-fixture-populated environment
- TPC-H SF=10 chunked bulk-load verification (issue #4217) — already closed
- Other zero-row queries (#4273–#4279) — separate sub-projects (B, C, D)

## 6. Risks and mitigations

| Risk | Mitigation |
|------|-----------|
| Changing `multi-start` to filter on `c.len() == join_tables.len()` may regress other queries | Existing `tpch_sf1_real_test` covers Q1–Q22 — verifies no regression |
| `resolve_bare` fall-through change may resolve columns to wrong tables | `tpch_sf1_real_test` row-count assertions catch mis-resolution |
| Adding `tracing::debug!` requires `tracing` dependency check | Use existing `log`/`eprintln!` if `tracing` not in deps; otherwise add `tracing = "0.1"` |

## 7. Key files

| Path | Change |
|------|--------|
| `src/engine_select.rs` | modify `build_chain_from_start`, `resolve_bare`, `pair_key` collection, multi-start loop (~30 lines changed) |
| `tests/integration/tpch/q21_chain_builder.rs` | new file (~60 lines) |
| `docs/releases/v3.12.0/evidence/tpch/V312-48-Q21-FIX.md` | new evidence doc (~80 lines) |
| `README.md` | no change (V312-48 row already says `受控 / PARTIAL→DEFERRED`) |
| `MYSQL_COMPAT_STATUS.md` | no change (Q21 is not a MySQL compat feature) |

## 8. References

- Issue #4280: V312-48-Q21 chain_order.len()=3 != join_tables.len()=4
- Parent: #4221 (closed via PR #4283)
- Sub-issues analysis: `docs/releases/v3.12.0/evidence/tpch/V312-48-SUB-ISSUES-ANALYSIS.md` (sha256: 7f2f64dcb9a0c579e5cb50dee69e61ff5d69ad5a174b6f0fa5363e3dd5b9dba6)
- Parent evidence: `docs/releases/v3.12.0/evidence/tpch/V312-48-TPCH-SF1-CORRECTNESS.md` (sha256: 47121531f8a5b512130730d61e45535062a25756fb920d13451bba146cc580f3)
- Prior work: PR #4205 / #4207 / #4181 "multi-way join planner handles star/bridge topologies"