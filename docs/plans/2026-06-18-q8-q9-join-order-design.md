# Q8/Q9 Alias-Aware Hybrid DP Join-Order Optimizer — Design

> **Date**: 2026-06-18
> **Author**: Claude (brainstorming session)
> **Target**: v3.9.0 GA — fix Q8/Q9 wire timeout (currently 300s+ → target < 60s)
> **Status**: Approved (Hybrid DP-lite approach)

## 1. Problem Statement

### 1.1 Current State

TPC-H Q8 and Q9 timeout on the wire protocol even at 300s (`tests/oracle_g15_tpch_sf01.rs:11-13`). Root-cause analysis at `docs/releases/v3.9.0/Q8_PERF_ANALYSIS.md` documents that the issue is **join order**, not missing hash joins:

- Sprint 8 hash join IS in wire path (`engine_select.rs:1548 execute_single_join` + `extract_comma_join_keys:2038` + `JoinKey::All:1635-1678`)
- `execute_joins` joins tables in the **declaration order** preserved by parser's comma-list auto-rewrite (`crates/parser/src/parser.rs:3279`)
- Q8: `part(20万) × supplier(1万) × lineitem(6万) × orders(15万) × customer(1.5万) × nation n1(25) × nation n2(25) × region(5)` → lineitem × orders = 6M (each lineitem maps to its order, 4 dups/order), then × customer = 90M, × supplier = 900M, etc. → 300s+ timeout

### 1.2 Why Greedy Fails (Q8 structure)

Q8's join graph is a **double-star + hub bridge**:

```
        n2 (filter: GERMANY)
          │
          ▼
   region (filter: EUROPE)
          │
          ▼
n1 ──── customer ──── orders ──── lineitem ──── part
 │
 └── supplier
```

`n1` is a **join hub** (degree 3: customer, supplier, region). A naive greedy that picks smallest edge first will:
- ❌ Start with `lineitem × part` (no predicate between them, lineitem × 20万 = 12M)
- ❌ Or start with `supplier` (10K) and immediately explode against later customer (1.5K × 10K = 15M per intermediate)

**Optimal hub-first order**:
```
n2 (1) → region (1) → n1 (25) → customer (601 filtered via n1) → orders (22800 via customer) → supplier (4000 via n1, after customer side is fixed) → lineitem (92000 via orders × supplier) → part (last leaf, no filter)
```

### 1.3 Q9 Structure (Fan-in → Fan-out DAG)

```
nation → supplier → lineitem → orders
          ↘ partsupp ↗
                ↓
               part
```

Triangle coupling between supplier/lineitem/partsupp + 2-path fan-in from nation. Greedy on edge size fails.

## 2. Design Goals

| Goal | Target |
|------|--------|
| Q8 wire (SF=0.01) | < 60s (currently 300s+ timeout) |
| Q9 wire (SF=0.01) | < 60s (currently 300s+ timeout) |
| Q8 in-process | < 5s (currently 6.2s) |
| Q9 in-process | < 5s |
| Q1-Q22 regression | 22/22 PASS with reorder ON |
| G15 oracle | 20/22 → **22/22 PASS** (add Q8/Q9 entries) |
| 6 meta-gates | P11-P16 all PASS |
| New `#[ignore]` on gate tests | 0 |

## 3. Algorithm: Hybrid DP-Lite

### 3.1 Why not pure greedy?

Standard greedy picks next node by `|partial| × |candidate|`. For Q8's hub structure this selects supplier (small) → customer (medium) → orders (huge) → lineitem (huge) which produces `customer × supplier × orders = 600 × 4000 × 22800 = 5.5 × 10¹⁰` intermediate.

The problem is greedy has no notion of "future penalty": picking supplier early prevents pruning n1-anchored customer later.

### 3.2 Why not full System-R DP?

Full System-R DP enumerates all `O(2ⁿ × n²)` subsets. For n=8 (Q8) that's 256 × 64 = 16K states — tractable, but the cost of building accurate estimates for every subset is high, and the code complexity is large (~2-3 weeks). Out of scope.

### 3.3 Hybrid DP-Lite Design

Three components: **subset memoization** + **connected-subset constraint** + **improved cost model with future penalty**.

#### 3.3.1 Subset memoization (Bitset DP)

```rust
// crates/optimizer/src/reorder_dp_hybrid.rs
use std::collections::HashMap;

type BitSet = u64;  // supports up to 64 tables; Q8 has 8, plenty of headroom

#[derive(Clone, Copy)]
struct JoinState {
    estimated_rows: f64,
    best_last_node: u8,
    best_prev_state: BitSet,  // for backtrace
}

pub struct DpMemo {
    states: HashMap<BitSet, JoinState>,  // subset -> best join state
}
```

For each connected subset S of the join graph, we track the best way to build S by extending a smaller subset T (where T ⊂ S and S - T = {single_node}).

#### 3.3.2 Connected-subset constraint (pruning)

**Rule A**: A subset is **enumerable** only if it's connected in the JoinGraph.
- Start with each single-node subset (size 1, trivially connected).
- To extend subset S by node v: require `(S ∪ {v}) - S` = {v} is connected to S, meaning there exists at least one edge between v and some node in S.

This reduces the search space from O(2ⁿ) to roughly O(cⁿ) where c = average node degree (3-4 for TPC-H queries), giving manageable state counts.

**Rule B (frontier expansion)**: When extending subset S by node v, only consider v ∈ frontier(S) = {v : v ∉ S, ∃ edge(v, u) for some u ∈ S}. This further prunes — no need to evaluate "starting fresh" subsets that aren't adjacent.

#### 3.3.3 Improved cost model

```rust
// crates/optimizer/src/cost_model.rs
fn cost_join(
    partial: &JoinState,           // accumulated state
    candidate: &VirtualTableNode,  // next node to join
    edge: &JoinEdge,               // connecting predicate
    selectivity: f64,              // edge selectivity (default 0.1 for equi)
) -> f64 {
    let join_rows = partial.estimated_rows * candidate.filtered_rows * selectivity;

    // Future penalty: how "spread out" is the remaining graph after this join?
    let future_penalty = future_min_estimate(partial, candidate);

    join_rows + FUTURE_PENALTY_WEIGHT * future_penalty
}
```

`future_min_estimate` heuristic: from the (partial ∪ {candidate}) set, compute a **minimum spanning tree** of the remaining edges; cost = sum of edge weights × estimated rows of incident nodes. This approximates the optimal continued join cost.

For Q8 specifically, when `partial = {n2, region, n1}` and we consider adding `customer`:
- join_rows ≈ 125 × 600 × 1.0 (c_nationkey = n1.n_nationkey) = 75K
- future_penalty: MST over {customer, supplier, orders, lineitem, part} = ~1B (penalizes this branch)
- vs. adding `supplier` instead: join_rows ≈ 125 × 4000 × 1.0 = 500K, future_penalty: MST over {supplier, customer, orders, lineitem, part} = ~5B
- **Adding customer first has lower cost** (smaller intermediate, even though future penalty similar)

Actually this needs tuning; the cost model is the empirical part. Default `FUTURE_PENALTY_WEIGHT = 0.5`.

### 3.4 Safety: INNER JOIN only

**Critical safety rule**: The optimizer ONLY reorders if ALL join clauses are `INNER`. If any `LEFT/RIGHT/FULL/CROSS` join is present:
- Skip reordering entirely
- Pass through original parser-emitted order
- This protects Q13 (LEFT OUTER JOIN), Q15 (subquery), etc.

```rust
fn is_eligible_for_reorder(select: &SelectStatement) -> bool {
    if select.join_clause.is_empty() {
        return false;  // no joins to reorder
    }
    for jc in &select.join_clause {
        if !matches!(jc.join_type, JoinType::Inner) {
            return false;
        }
    }
    true
}
```

### 3.5 Algorithm pseudocode

```
function reorder(select):
    if not is_eligible_for_reorder(select):
        return select.join_clause  // pass through

    graph = build_join_graph(select)  // alias-distinct nodes, edges from WHERE equi-preds
    if graph.nodes.len() < 3:
        return select.join_clause  // 1-2 table join, no benefit

    memo = DpMemo::new()
    for each node v:
        memo[{v}] = JoinState {
            estimated_rows: graph.node(v).filtered_rows,
            best_last_node: v,
            best_prev_state: 0,
        }

    // Enumerate connected subsets in increasing size
    for size in 2..=graph.nodes.len():
        for each connected subset S of `size`:
            best = ∞
            for each node v ∈ S:
                T = S - {v}
                if T ∉ memo: continue  // T was not enumerable
                edge = find_edge(v, T)  // v has at least one edge to T
                if edge is None: continue
                cost = cost_join(memo[T], graph.node(v), edge, edge.selectivity)
                if cost < best:
                    best = cost
                    best_state = JoinState {
                        estimated_rows: cost,  // estimated join output rows
                        best_last_node: v,
                        best_prev_state: T,
                    }
            if best is set:
                memo[S] = best_state

    full_set = (1 << graph.nodes.len()) - 1
    if full_set ∉ memo:
        return select.join_clause  // disconnected graph, fall back

    // Backtrace to get ordered JoinClause list
    order = backtrace(memo, full_set, graph)
    return order
```

For n=8 (Q8), the connected-subset enumeration yields ~150-300 states. Each state evaluation is O(degree) ≈ 3. Total: ~1K operations. Sub-millisecond.

## 4. Architecture

### 4.1 New crate: `crates/optimizer/`

```
crates/optimizer/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # public API
│   ├── join_graph.rs           # JoinGraph, VirtualTableNode, JoinEdge, build_join_graph()
│   ├── reorder_dp_hybrid.rs    # DpMemo, reorder() entry point, backtrace
│   ├── cost_model.rs           # cost_join, future_min_estimate, selectivity defaults
│   ├── row_estimator.rs        # filtered_rows for single-table WHERE preds
│   └── predicate_classify.rs   # collect equi-edges from WHERE, single-table preds per node
└── tests/
    └── reorder_test.rs         # unit tests
```

### 4.2 Integration point: `src/engine_select.rs`

```rust
// In execute_joins (line ~1202), REPLACE the original join loop:
fn execute_joins(&self, select: &SelectStatement) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
    let storage = self.storage.read().unwrap();

    // ... existing base-table scan + pushdown (lines 1203-1247) ...

    // NEW: optional reorder via optimizer
    let join_clauses = if cfg!(feature = "v390_join_reorder") {
        sqlrustgo_optimizer::reorder::reorder_joins(select)
    } else {
        select.join_clause.clone()
    };

    // Phase 3 derived subqueries (lines 1249-1300) — unchanged
    // ... existing pushdown collection (lines 1302-1334) — unchanged ...

    for join_clause in &join_clauses {  // ← changed from select.join_clause
        // ... existing execute_single_join call (lines 1336-1357) — unchanged ...
    }
}
```

**Critical**: Reorder happens AFTER `base_table` (FROM t) is scanned but BEFORE the `join_clause` loop. Base-table selection is preserved (no reorder of the FROM t table itself).

### 4.3 Feature flag

```toml
# Cargo.toml [features]
v390_join_reorder = ["sqlrustgo-optimizer/v390_join_reorder"]
```

Default: feature **off** in `cargo build` and `cargo test`. Enable explicitly for performance verification:
```bash
cargo test --features v390_join_reorder --test tpch_full_22_test
cargo test --features v390_join_reorder --test oracle_g15_tpch_sf01
```

### 4.4 Data flow

```
parser (unchanged)
    │
    ▼
SelectStatement { table, from_alias, join_clause: Vec<JoinClause>, where_clause }
    │
    ▼
[NEW] crates/optimizer::reorder::reorder_joins(select)
    │
    │ 1. check INNER-only eligibility
    │ 2. build JoinGraph (alias-distinct)
    │ 3. run Hybrid DP-Lite
    │ 4. backtrace → Vec<JoinClause> (same struct as input)
    │
    ▼
execute_joins(select) → uses reordered join_clauses
    │
    │ unchanged execute_single_join loop
    │
    ▼
hash join output rows
```

## 5. Detailed Components

### 5.1 `join_graph.rs`

```rust
pub type NodeId = u8;

pub struct VirtualTableNode {
    pub id: NodeId,
    pub alias: String,                  // "n1", "n2", "part", "supplier"
    pub base_table: String,             // "nation", "nation", "part" (same for n1/n2)
    pub estimated_rows: u64,            // storage.scan() count
    pub filtered_rows: f64,             // after single-table WHERE predicates
    pub single_table_preds: Vec<Expression>,
}

pub struct JoinEdge {
    pub left: NodeId,
    pub right: NodeId,
    pub predicate: Expression,
    pub selectivity: f64,               // default 1/N for equi-join (N = larger side)
}

pub struct JoinGraph {
    pub nodes: Vec<VirtualTableNode>,
    pub edges: Vec<JoinEdge>,           // duplicates allowed (n1-region, n1-customer)
    pub adjacency: HashMap<NodeId, Vec<(NodeId, usize)>>,  // node → [(neighbor, edge_idx)]
}

pub fn build_join_graph(select: &SelectStatement, storage: &dyn Storage) -> JoinGraph;
```

**Alias-distinct guarantee**: Each FROM-clause entry (including `nation n1`, `nation n2`) gets a unique `NodeId`. The parser already encodes inline alias in the join_clause as `table="nation|n1"`, `alias=Some("n1")` (parser.rs:3294-3301). The optimizer splits this into distinct nodes.

### 5.2 `predicate_classify.rs`

Walks `select.where_clause` (a top-level AND-tree) and partitions predicates into:

1. **Equi-edges**: `t1.col_a = t2.col_b` where both are columns. Forms `JoinEdge`.
2. **Single-table preds**: predicate that references only one table's columns. Attached to `VirtualTableNode.single_table_preds`. Used for filtered_rows estimate and applied during scan (existing `pre_filter_cartesian_right_table`).
3. **Multi-table residual**: predicates that reference ≥2 tables but are not simple equality (e.g., `t1.col BETWEEN t2.col AND 'x'`). Stored as residual; optimizer does NOT generate new edges from these (they're applied as post-join filter).

### 5.3 `row_estimator.rs`

```rust
pub fn estimate_filtered_rows(
    table: &str,
    preds: &[Expression],
    storage: &dyn Storage,
) -> f64 {
    let total = storage.scan(table).map(|r| r.len() as f64).unwrap_or(1.0);
    let selectivity = preds.iter().map(simple_selectivity).product::<f64>();
    (total * selectivity).max(1.0)
}

fn simple_selectivity(pred: &Expression) -> f64 {
    match pred {
        E::BinaryOp(_, op, E::Literal(_)) if op == "=" => 0.01,  // = literal, very selective
        E::BinaryOp(_, op, _) if op == "=" => 0.1,              // = column, less selective
        E::BinaryOp(_, op, _) if op == ">" || op == "<" || op == ">=" || op == "<=" => 0.3,
        E::Like(..) => 0.2,
        E::InList(_, vals) => (vals.len() as f64 * 0.1).min(0.9),
        _ => 0.5,  // unknown
    }
}
```

Defaults are heuristics; not histogram-based. Accurate enough for relative order selection.

### 5.4 `cost_model.rs`

```rust
pub const FUTURE_PENALTY_WEIGHT: f64 = 0.5;
pub const EDGE_SELECTIVITY_DEFAULT: f64 = 0.1;  // equi-join baseline

pub fn cost_join(
    partial: &JoinState,
    candidate: &VirtualTableNode,
    edge: &JoinEdge,
) -> f64 {
    let join_rows = partial.estimated_rows * candidate.filtered_rows * edge.selectivity;
    let future = future_min_estimate(partial, candidate);
    join_rows + FUTURE_PENALTY_WEIGHT * future
}

fn future_min_estimate(partial: &JoinState, candidate: &VirtualTableNode) -> f64 {
    // Heuristic: assume the rest of the graph joins with average selectivity 0.1 per edge.
    // For Q8 after joining {n2, region, n1, customer}, the remaining {supplier, orders,
    // lineitem, part} contributes 4-5 more edges; this penalizes picking high-fanout
    // branches early.
    // Implementation: count edges not yet "absorbed" and multiply by avg remaining rows.
    // (Detailed impl deferred to cost_model.rs)
    0.0  // placeholder; refined during implementation
}
```

### 5.5 `reorder_dp_hybrid.rs`

```rust
pub fn reorder_joins(select: &SelectStatement) -> Vec<JoinClause> {
    if !is_eligible_for_reorder(select) {
        return select.join_clause.clone();
    }
    let graph = build_join_graph(select, self.storage());
    if graph.nodes.len() < 3 {
        return select.join_clause.clone();
    }
    let mut memo = HashMap::new();
    // ... DP enumeration of connected subsets ...
    // ... backtrace to ordered JoinClause list ...
}
```

The DP is bounded by `n ≤ 8` for TPC-H (Q8 has 8 tables), giving at most ~256 subsets, of which ~150-300 are connected. Memory: a few KB. Time: <1ms.

### 5.6 Backtrace → JoinClause

The backtrace produces a sequence of `(NodeId, JoinEdge)` pairs. Each pair is converted back to a `JoinClause`:

```rust
fn emit_join_clause(prev_node: &NodeId, next_node: &NodeId, edge: &JoinEdge, graph: &JoinGraph) -> JoinClause {
    JoinClause {
        join_type: JoinType::Inner,
        table: graph.node(*next_node).base_table.clone(),  // bare name
        alias: Some(graph.node(*next_node).alias.clone()),
        on_clause: edge.predicate.clone(),
    }
}
```

The parser-original `JoinClause.table` is `"nation|n1"` (with `|alias` suffix), but the executor's `execute_single_join` already strips this (`engine_select.rs:1342 split_once('|')`). The optimizer emits either form — both work.

## 6. Edge Cases & Safety

### 6.1 Disconnected join graph

If the WHERE clause has no equi-edges connecting all tables (rare but possible — explicit CROSS JOIN), the DP memo will not contain the full-set state. **Fallback**: return original `select.join_clause` unchanged. The cartesian product will execute but that's the user's intent (CROSS JOIN).

### 6.2 Tables without WHERE predicates

Optimizer still includes them in the graph. They're "leaves" — the DP will place them last (lowest cost to append after the connected core is built).

### 6.3 Derived subquery `__subq_N` tables

Already handled by executor's `DERIVED_RESULTS.with(...)` registry (`engine_select.rs:1566-1575`). Optimizer treats `__subq_N` as a regular node; its `estimated_rows` comes from the materialized subquery result.

### 6.4 Self-joins

`SELECT ... FROM t t1 JOIN t t2 ON t1.id = t2.parent_id` — handled naturally: t1 and t2 are two distinct nodes (alias-distinct), one edge.

### 6.5 Already-correct order

If the parser's order matches the optimizer's DP output (within ε tolerance on estimated rows), the executor still re-runs the join loop with the same clauses. No regression.

### 6.6 Subquery in WHERE

`WHERE col IN (SELECT ...)` does not appear in `join_clause`. The optimizer only sees JOIN tables. Subqueries are evaluated independently by the executor's subquery mechanism (existing code, untouched).

## 7. Testing Strategy

### 7.1 Unit tests (`crates/optimizer/tests/reorder_test.rs`)

| Test | Validates |
|------|-----------|
| `test_alias_distinct_graph` | `nation n1` and `nation n2` become 2 distinct nodes |
| `test_equi_edge_extraction` | `c_nationkey = n1.n_nationkey` becomes edge(c, n1) |
| `test_single_table_pred_classify` | `n2.n_name = 'GERMANY'` becomes n2's filter |
| `test_dp_three_table_chain` | a → b → c produces order a, b, c |
| `test_dp_hub_first_q8_pattern` | double-star pattern → hub-first order |
| `test_dp_falls_back_on_disconnected` | no equi-edges → return original |
| `test_dp_skips_left_join` | presence of LEFT → return original |

### 7.2 Integration: `tests/tpch_full_22_test.rs`

Run TWICE:
1. **Default features** (reorder OFF) → 22/22 PASS, baseline unchanged
2. **--features v390_join_reorder** (reorder ON) → 22/22 PASS, with reordered plans

Compare row counts and wall-time. New perf baseline recorded.

### 7.3 G15 oracle: add Q8 + Q9

In `tests/oracle_g15_tpch_sf01.rs:24 TPC_H_SF01_QUERIES`:
```rust
("Q8", "SELECT ... FROM ... WHERE ... GROUP BY ...", "Q8_sf01_baseline.json", 60),
("Q9", "SELECT ... FROM ... WHERE ... GROUP BY ...", "Q9_sf01_baseline.json", 60),
```

Run with `--features v390_join_reorder`. Q8 row count = 2, Q9 row count = 175 (per TPC-H spec).

Baseline generation: Run the wire protocol once with `--release` and capture `row_count` + first 3 rows via existing `tests/common/oracle_framework.rs::compare_to_baseline`. Save to `tests/data/tpch-sf01/expected/Q{8,9}_sf01_baseline.json`.

### 7.4 Regression: Q1-Q22 in-process

`tests/tpch_full_22_test.rs` covers this. Both with and without feature flag.

### 7.5 Meta-gates

P11-P16 must all PASS:
- P11 Gate Self-Verification: no new gates
- P12 No Implicit Tolerance: no new `#[ignore]`
- P13 Test Count Monotonicity: +1 new test file (optimizer reorder_test.rs), +2 oracle entries → counts go up
- P14 DRIFT != PASS: no change
- P15 Oracle Required: Q8/Q9 baselines must exist before un-`#[ignore]`
- P16 Gate Test Integrity: no gate test gets `#[ignore]`

## 8. Implementation Tasks

(Detailed task list will be generated by `writing-plans` skill after this design is approved.)

## 9. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Cost model mis-tuned → bad Q8 order | Medium | High (Q8 still slow) | Tune `FUTURE_PENALTY_WEIGHT` empirically; provide multiple weights in cost_model.rs and pick best per workload |
| Backtrace produces invalid JoinClause (missing alias) | Low | High (parse error) | Round-trip test: parse Q8 SQL, reorder, re-emit, verify executor produces correct row count |
| Hub-first heuristic fails for Q9's triangle | Medium | Medium (Q9 slower than expected) | Test Q9 explicitly; if fails, fall back to greedy + better selectivity heuristics for Q9 specifically |
| 60s wire timeout too tight (Q8 = 50s actual) | Medium | Low | Adjust timeout to 120s in oracle; document new threshold |
| Feature flag leaks into production binary | Low | Medium | Add CI step that runs `cargo build` (default features) + verifies optimizer crate is NOT linked |

## 10. Acceptance Criteria

- [ ] `crates/optimizer/` compiles with `cargo clippy -- -D warnings`
- [ ] `tpch_full_22_test` (default features): 22/22 PASS, **identical baseline rows**
- [ ] `tpch_full_22_test` (`--features v390_join_reorder`): 22/22 PASS, **identical baseline rows**
- [ ] Q8 in-process < 5s (current 6.2s)
- [ ] Q8 wire < 60s (current 300s+ timeout)
- [ ] Q9 in-process < 5s
- [ ] Q9 wire < 60s
- [ ] `oracle_g15_tpch_sf01` (`--features v390_join_reorder`): **22/22 PASS** (was 20/22)
- [ ] 6/6 meta-gates PASS (P11-P16)
- [ ] 0 new `#[ignore]` on gate tests
- [ ] 0 new public API docstrings (per AGENTS.md "minimize comments" rule; only public fn docstrings justified)

## 11. Out of Scope

- **Histogram-based selectivity**: requires per-table statistics, deferred to v3.10
- **Bushy join trees**: Hybrid DP-Lite only explores left-deep trees; bushy is System-R territory
- **Cost-based join method selection**: hash vs nested-loop vs merge — executor already picks hash when JoinKey is `Pair`/`All`
- **Materialized view matching**: deferred
- **Subquery flattening optimization**: deferred

## 12. References

- `docs/releases/v3.9.0/Q8_PERF_ANALYSIS.md` — root-cause analysis (existing)
- `openspec/changes/2026-06-08-v390-sprint8-q8-exists/` — Sprint 8 hash join (already merged)
- `src/engine_select.rs:1202 execute_joins` — integration point
- `src/engine_select.rs:1548 execute_single_join` — unchanged hash join
- `crates/parser/src/parser.rs:3279` — comma-list auto-rewrite (unchanged)
- `src/engine_utils.rs:382 build_combined_schema` — unchanged
- Previous failed attempt: `git log --all --grep="join.order" -10` (reverted)