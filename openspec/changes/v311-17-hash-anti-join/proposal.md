## Why

V311-17 completes the **second half** of TPC-H EXISTS/NOT EXISTS optimization begun by V311-15 (Q4 Hash Semi Join). V311-15 fixed `EXISTS`; V311-17 fixes `NOT EXISTS` and `NOT IN` for TPC-H Q21, Q22 and similar workloads.

### Current State

- V311-15 Hash Semi Join handles `EXISTS` with O(log N + bucket_size) per outer row
- `NOT EXISTS` still uses pre-existing SubqueryIndex, but **does NOT invert semantics** correctly:
  - Currently: `pre_eval_exists_indexed` returns `Some(bool)` then caller inverts for NOT EXISTS
  - Bottleneck: for queries where MOST rows match (TPC-H Q21 with many lineitem), NOT EXISTS scans all qualifying rows before deciding "no match" = expensive
- TPC-H Q21 (Supplier Who Kept Orders Waiting): NOT EXISTS dominates; v3.10.0 baseline already 30s+ on SF=0.1

### What's Missing

**Hash Anti Join** is the natural counterpart to Hash Semi Join. Where Semi Join answers "does any inner row match?", Anti Join answers "does NO inner row match?". For NOT EXISTS with high-match workloads, Anti Join with **bloom filter + early termination** is dramatically faster.

### Real-World Impact

| Query | Operation | v3.10.0/v3.11.0 baseline | V311-17 target |
|-------|-----------|--------------------------|----------------|
| TPC-H Q21 | `NOT EXISTS` × 2 | ~30s (SF=0.1), ~5min (SF=1) | < 30s (SF=1) |
| TPC-H Q22 | `NOT EXISTS` × 1 | moderate | speedup ≥ 2x |
| General `WHERE NOT IN (SELECT ...)` | semi-naive | many seconds | sub-second |

## What Changes

### 1. New HashAntiJoin operator in `crates/executor/src/join/`

- **`HashAntiJoin`** struct with:
  - `build_side`: inner table (the SELECT target)
  - `build_keys`: columns to hash on
  - `probe_side`: outer rows (driver)
  - `bloom_filter`: optional pre-filter for build side  
  - `residual_predicate`: optional AND filter (e.g., `l3.l_suppkey <> l1.l_suppkey`)
  - `satisfied_outer_keys`: HashSet of outer rows that found a match (excluded from output)
- API: `pub fn execute(&self) -> Vec<Record>` returning outer rows whose inner key NOT found in match set

### 2. bloom filter fast-path

- For Q21 (most rows match), bloom filter false-positive rate is acceptable
- Anti Join short-circuits when bloom filter says "definitely no match" → O(1) decision
- Uses the same 128-byte (16x u64) filter as V311-15

### 3. NOT EXISTS / NOT IN routing in `pre_eval_correlated_exists`

- Add `Expression::NotExists` arm that uses HashAntiJoin path
- Reuse existing `SubqueryIndex` structure (added in V311-15)
- Inversion: instead of returning true on first match, return false on first mismatch
- For TPC-H Q21 (`NOT EXISTS (SELECT ... FROM lineitem l3 ...)`), Anti Join returns rows where ALL inner lookups miss

### 4. `crates/executor/src/join/hash_anti_join.rs` (new file)

- Pure-Volcano iterator model (matches `HashJoin` in `parallel_hash_join.rs`)
- API surface:
  - `pub struct HashAntiJoin { ... }`
  - `impl HashAntiJoin { pub fn new(...) -> Self; pub fn execute(&self) -> Result<Vec<Record>, String> }`
- Key invariant: each outer row appears in output **at most once**

### 5. VolcanoExecutor wiring

- `src/engine_select.rs`: replace per-outer-row full scan in NOT EXISTS arm with HashAntiJoin dispatch
- Fallback: when `build_subquery_index` returns None (complex WHERE), use existing naive path

### 6. Tests

- **`tests/integration/sql/anti_join_main_path_test.rs`** — 6 tests:
  - NOT EXISTS returns outer rows where inner key NOT FOUND
  - NOT EXISTS with residual predicate
  - NOT IN equivalent correctness
  - Mixed EXISTS + NOT EXISTS in same query (TPC-H Q21)
  - Bloom filter effectiveness: confirm in metric output
  - 10K+ row scale benchmark (passes perf bar)

### 7. Documentation

- **`docs/releases/v3.11.0/perf/HASH_ANTI_JOIN_PERF.md`** — perf comparison HashAntiJoin vs naive loop
- Update `docs/governance/debt/debt-registry.yaml` if any debt items reference Q21 perf

## Capabilities

### New Capabilities

- `hash-anti-join`: New HashAntiJoin operator answering NOT EXISTS / NOT IN with O(outer + inner) complexity instead of O(outer × inner)
- `bloom-filter-anti-join-short-circuit`: When inner keys are dominated by a few values, bloom filter avoids full hash table lookup for most outer rows

## Impact

| File | Type | Lines |
|------|------|-------|
| `crates/executor/src/join/hash_anti_join.rs` | new | ~250 |
| `crates/executor/src/join/mod.rs` | modified | +5 |
| `crates/executor/src/lib.rs` | modified | +3 |
| `src/engine_select.rs` | modified | +40 (Anti Join arm in pre_eval) |
| `tests/integration/sql/anti_join_main_path_test.rs` | new | ~200 |
| `docs/releases/v3.11.0/perf/HASH_ANTI_JOIN_PERF.md` | new | ~120 |

### No Breaking Changes

- Existing Q4 / Q21 tests continue passing (V311-15's Hash Semi Join still used for EXISTS)
- Naive NOT EXISTS fallback retained for non-indexable predicates

## Acceptance Criteria

- **TPC-H Q21 @ SF=0.1**: < 5 seconds (baseline ~30s on v3.10.0)
- **`cargo test anti_join_main_path_test`**: 6/6 PASS
- **`cargo test q21_exists_hash_path_test`**: 2/2 PASS (no regression on Q21 EXISTS path)
- Performance: bloom filter effectiveness > 70% for typical TPC-H workloads
- **No regression**: existing 15+ ClusteredIndex / Q4 / Q21 tests still PASS

## Estimated Effort

| Step | Estimate | Complexity |
|------|----------|------------|
| HashAntiJoin operator + bloom filter | 24h | Medium |
| VolcanoExecutor integration | 8h | Low |
| NOT EXISTS routing in pre_eval | 4h | Low |
| Tests + benchmarks | 4h | Low |
| Documentation | 4h | Low |
| **Total** | **40h** | matches V311-17 plan |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Hash Anti Join semantics off-by-one (skip keys present multiple times) | High | Unit tests verify TPC-H Q21 row counts match Postgres truth |
| Bloom filter false-negative (correctness) | Low | Use well-tested 2-hash FNV-1a; bloom only used for "definitely no match" check (miss = false positive, ok); "definitely yes match" requires hash table |
| Concurrent NOT EXISTS performance regression on read-heavy | Medium | Benchmark with v3.10.0 baseline (existing test) |

## Scope

### IN SCOPE (V311-17)

- Single-table inner SELECT (no JOINs in inner)
- Single-column key (no multi-column PK)
- Optional residual predicate (e.g. `<>`, `>`, `<`)
- Bloom filter acceleration
- Pre-evaluated residual path (when residual has no outer refs)

### OUT OF SCOPE (deferred)

- Multi-column keys (multi_pk extension)
- JOIN inside inner SELECT (correlated decoration already handles)
- Persistent disk-backed index (v3.12)
- WAL integration (v3.12)

## Out-of-Scope Items (Design Constraint)

Stays consistent with V311-15's deferred items: page-based disk + WAL + MVCC.
