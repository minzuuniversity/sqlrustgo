# V312-64f / Issue #4699 — Recursive CTE Execution

**Date**: 2026-09-04
**Branch**: `fix/v312-64f-recursive-cte` (off `develop/v3.12.0`)
**Issue**: [#4699](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4699)
**Closes**: issue body bug; unblocks TPC-H Q11 (recursive), tree/graph traversal, org hierarchy queries.

## Problem

`WITH RECURSIVE tree AS (...)` is parsed and reaches the executor, but the executor
returns `Recursive CTE not yet supported` from `src/engine_cte.rs:28-32` — a single
hardcoded error in `materialize_cte_tables`. The bug is a **pure missing feature**;
all surrounding infrastructure (parser AST, storage materialization, cleanup) is
already wired up.

## Root Cause

`materialize_cte_tables` enforces a flat `for cte in &with_clause.ctes` loop that
materializes each CTE's body once. Recursive CTEs need **iterative re-materialization**
where each iteration's working set feeds the next iteration's body. The current
single-pass model cannot express this.

## Approach: Two-Table Working/Accumulated (PG/SQLite Standard)

For each recursive CTE `t` in `WITH RECURSIVE`, materialize **two** temp tables:

| Temp table | Visible to | Contains |
|-----------|-----------|----------|
| `t_acc` (accumulated) | outer SELECT, siblings in `WITH` | all rows ever produced |
| `t_work` (working set) | the recursive step's `FROM t` | current iteration's rows only |

The iteration loop replaces `t_work` contents each round (truncate + insert new_rows),
and inserts the new rows into `t_acc` as well. Termination: `t_work` becomes empty
after a step, or safety cap (`MAX_RECURSION_DEPTH = 1000`, `MAX_RECURSION_ROWS = 1_000_000`)
is hit.

This matches PostgreSQL and SQLite semantics exactly. The issue body's hierarchy
example terminates correctly on iteration 3 because the working set becomes empty.

### Why not the existing sql-corpus reference impl

`crates/sql-corpus/src/lib.rs:859-966::execute_recursive_cte` is a **single-accumulating-table**
implementation. On the issue body's hierarchy example it would produce thousands of
duplicate rows (the step's `JOIN tree` matches every accumulated row each iteration),
and terminate only by hitting `MAX_RECURSION_DEPTH=1000`. The two-table approach
terminates correctly because the working set is replaced each iteration.

## Algorithm

```rust
fn materialize_recursive_cte(
    engine: &mut ExecutionEngine,
    cte: &CommonTableExpression,
) -> SqlResult<()> {
    // 1. Decompose body: must be UNION/UNION ALL of two SELECTs (SQL:1999).
    let (anchor_stmt, step_stmt, union_all) = decompose_recursive_body(&cte.subquery)?;

    // 2. Execute anchor first → seed_rows. Derive column schema from seed.
    let seed_rows = engine.execute_select(anchor_stmt)?.rows;
    let columns = derive_cte_columns(cte, &seed_rows);

    // 3. Create both temp tables with same schema.
    create_temp_table(engine, &format!("{}_acc", cte.name), &columns)?;
    create_temp_table(engine, &format!("{}_work", cte.name), &columns)?;
    // Bind CTE name `t` to `t_acc` for outer query, `t_work` for step.
    // Implementation: re-alias `t_work` to `t` via CREATE VIEW, OR insert into
    // `t` and replace contents each iter. See Implementation section below.

    // 4. Initial: insert seed into both.
    if !seed_rows.is_empty() {
        insert_rows(engine, &format!("{}_acc", cte.name), seed_rows.clone())?;
        insert_rows(engine, &format!("{}_work", cte.name), seed_rows)?;
    }

    // 5. Iterate.
    let mut total: usize = seed_rows.len();
    for _depth in 0..MAX_RECURSION_DEPTH {
        // 5a. Run step (references `t`, which is bound to `t_work`).
        let step_rows = engine.execute_select(step_stmt)?.rows;
        if step_rows.is_empty() { break; }

        // 5b. UNION (not ALL): dedupe against accumulated.
        let new_rows = if union_all {
            step_rows
        } else {
            let existing = scan_table(engine, &format!("{}_acc", cte.name))?;
            dedupe(&step_rows, &existing)
        };
        if new_rows.is_empty() { break; }

        // 5c. Replace working set with new rows; append to accumulated.
        truncate_table(engine, &format!("{}_work", cte.name))?;
        insert_rows(engine, &format!("{}_work", cte.name), new_rows.clone())?;
        insert_rows(engine, &format!("{}_acc", cte.name), new_rows)?;
        total += new_rows.len();

        if total > MAX_RECURSION_ROWS {
            return Err(SqlError::ExecutionError(format!(
                "Recursive CTE {} exceeded {} row cap", cte.name, MAX_RECURSION_ROWS)));
        }
    }

    // 6. Rename t_acc → t_name for outer query visibility, OR register
    //    `t_name` as an alias for `t_acc` in engine's table resolution.
    //    See Implementation §Binding.

    Ok(())
}
```

### CTE name binding (the critical wiring detail)

Existing non-recursive `materialize_cte_tables` creates a temp table with the
**CTE's literal name** so `list_tables()` resolves references. For recursive, we
need **two tables** but the SQL only references one name (`t`). Solution:

Create `t_acc` and `t_work` as physical tables. Then:
- For the **outer SELECT** (after the recursive iteration): rename `t_acc` → `t`
  via `storage.rename_table`. The outer SELECT sees `t` (= accumulated).
- For the **recursive step**: we need `t` to resolve to `t_work`. We do NOT rename
  during iteration because the outer SELECT hasn't run yet. Instead, between
  iterations: `storage.rename_table("t", "t_acc")`, then for the next iter run,
  point the step's `FROM t` to `t_work`.

This is awkward. Cleaner alternative: **don't use literal names at all** — use
the existing `list_tables()` resolution which checks both table names AND any
alias mechanism. But the parser resolves `t` to a fixed identifier, so we have
to physically swap.

**Cleanest implementation**: between iterations, before running the step:
1. Rename current `t` → `t_acc` (move accumulated aside)
2. Rename `t_work` → `t` (working set becomes "the CTE")
3. After step: copy new rows from `t` back to a separate `t_acc` table
4. Reset for next iter: rename `t` → `t_work`, restore `t_acc`

Actually simpler: keep **three** names — `t`, `t_acc`, `t_work`:
- `t` always points to accumulated (visible to outer SELECT and non-recursive siblings)
- `t_work` always points to current working set (visible to recursive step)
- The step query has its `FROM t` rewritten at parse time? No — parser doesn't know.

OK the real solution: **between iterations, swap which physical table the name `t`
points to**, by creating a view:
- Create `t_acc` (accumulated) and `t_work` (working set) as real tables.
- Before step: `DROP VIEW IF EXISTS t; CREATE VIEW t AS SELECT * FROM t_work;`
- After step + append to t_acc: still keep `t` → `t_work` (next iter).
- AFTER loop completes: `DROP VIEW t; CREATE VIEW t AS SELECT * FROM t_acc;` (for outer SELECT).

Hmm, but views have their own complications (resolution, etc.). Let me think of another way:

**Simplest**: Don't bother with renaming. Create **only one** temp table `t` (the
accumulated one) AND one in-memory structure (or another temp table) for the
working set. The trick: rewrite the recursive step's `FROM t` references to
`FROM t_work` at execution time, by walking the AST and replacing identifiers.

This is cleaner because it doesn't require views or table renaming. The rewrite
happens once before the iteration loop starts.

**Refined algorithm (chosen)**:

```rust
// 1. Create two real temp tables.
let acc = format!("{}_acc", cte.name);
let work = format!("{}_work", cte.name);
create_temp_table(engine, &acc, &columns)?;
create_temp_table(engine, &work, &columns)?;

// 2. Rewrite step_stmt: every reference to cte.name (in FROM clauses, JOINs,
//    subqueries) becomes acc-or-work depending on context. For recursive step,
//    rewrite `cte.name` → `work`.
//    Helper: replace_table_identifiers(step_stmt, cte.name, &work)

// 3. Create the CTE name `t` as an alias for the accumulated table — so
//    outer SELECT sees it. Use a real table: rename `acc` → `t` at the end
//    of the loop, OR use a view.
//    Simpler: at start, create `t` table with same schema. After each iter's
//    new_rows are accumulated into `t_acc`, COPY t_acc → t (or just write to t
//    in the first place).
//
//    Even simpler: skip the `t_acc` indirection entirely. Use a single `t`
//    table for accumulated, and a single `t_work` table for working. The
//    step query references `t_work` (rewritten). The outer SELECT references
//    `t` (no rewrite needed). At each iter: append new rows to `t`; truncate
//    + insert into `t_work`.
```

**Final algorithm (no renaming, clean)**:

```rust
let t = cte.name.clone();           // accumulated — visible to outer SELECT
let t_work = format!("{}__work", cte.name);  // working set — visible to step

// 1. Execute anchor.
let seed = engine.execute_select(anchor_stmt)?.rows;

// 2. Build column schema (same as existing non-recursive code).
let columns = derive_cte_columns(cte, &seed);

// 3. Create temp table `t` (accumulated) + temp table `t_work` (working set).
create_temp_table(engine, &t, &columns)?;
create_temp_table(engine, &t_work, &columns)?;

// 4. Rewrite step_stmt: replace identifier `t` with `t_work` in FROM clauses
//    and JOIN clauses. AST visitor over SelectStatement / JoinClause / Subquery.
//    This is the critical bit.
let step_rewritten = rewrite_step_table_refs(step_stmt, &t, &t_work)?;

// 5. Insert seed into BOTH tables.
insert_rows(engine, &t, seed.clone())?;
insert_rows(engine, &t_work, seed)?;

// 6. Iterate.
let mut total = seed.len();
for _depth in 0..MAX_RECURSION_DEPTH {
    let step_rows = engine.execute_select(&step_rewritten)?.rows;
    if step_rows.is_empty() { break; }

    let new_rows = if union_all {
        step_rows
    } else {
        // UNION: dedupe against accumulated.
        let acc_existing = scan_table(engine, &t)?;
        dedupe_rows(&step_rows, &acc_existing)
    };
    if new_rows.is_empty() { break; }

    insert_rows(engine, &t, new_rows.clone())?;       // append to accumulated
    // StorageEngine has no truncate_table; drop + re-create with same schema.
    storage.drop_table(&t_work)?;
    storage.create_table(&temp_table_info_for(&t_work, &columns))?;
    insert_rows(engine, &t_work, new_rows)?;          // fill with new rows
    total += new_rows.len();

    if total > MAX_RECURSION_ROWS {
        return Err(SqlError::ExecutionError(format!(
            "Recursive CTE {} exceeded {} row cap", t, MAX_RECURSION_ROWS)));
    }
}

// 7. Drop t_work (only `t` remains, visible to outer SELECT).
storage.drop_table(&t_work);
Ok(())
```

**Critical bit — `rewrite_step_table_refs`**: walk the step's SelectStatement AST
and replace every table-name reference to `cte.name` with `t_work`. The parser
uses these AST shapes (verified in `crates/parser/src/parser.rs`):

- `SelectStatement.table: String` (line 725) — the main FROM clause's table name
  is a flat `String`, not a struct.
- `SelectStatement.joins: Vec<JoinClause>` (line 701) — each `JoinClause.table: String`.
- `Expression::Subquery(Box<Statement>)` — nested SELECTs inside WHERE/HAVING/SELECT.
  Recurse into the inner Statement, which may itself be `Select` or `Union`.

So the rewriter walks:
1. `step_stmt.table` — if `== cte.name`, set to `t_work`.
2. For each `JoinClause` in `step_stmt.joins` — if `clause.table == cte.name`, set to `t_work`.
3. Walk all expressions (`where_clause`, `having_clause`, every `SelectColumn.expression`).
   For each `Expression::Subquery(inner)`, recurse: if inner is `Statement::Select(s)`,
   call rewrite on `s`; if inner is `Statement::Union(u)`, rewrite both `u.left` and
   `u.right` if they're Selects.

The recursion depth is bounded by the query's nesting depth (typically ≤ 5).

Why a separate `t_work` table (instead of mutating the SELECT in place): the AST
type is immutable from the executor's perspective (parser owns it). We don't
clone the AST (expensive for large queries) — we either (a) build a shallow-cloned
SelectStatement with the rewrites applied, OR (b) use `serde_json::to_value` round-trip
+ mutate + `serde_json::from_value`. Path (a) is preferred — explicit field-by-field
cloning in the rewriter, no JSON dependency.

If this AST-rewrite approach is too invasive, fallback: do the rename dance
(between iterations swap `t` ↔ `t_acc` ↔ `t_work` via `storage.rename_table`).
The sql-corpus approach implicitly uses single-table + accumulate, which is wrong
for issue body semantics.

### Configuration

```rust
const MAX_RECURSION_DEPTH: usize = 1000;       // matches SQLite default
const MAX_RECURSION_ROWS:  usize = 1_000_000;  // safety cap from sql-corpus
```

Both as `const` in `src/engine_cte.rs`. Runtime configuration deferred (YAGNI for v3.12.0).

### Edge cases handled

| Case | Behavior |
|------|----------|
| Empty anchor (0 rows) | Both tables created empty; first step returns 0 rows → break; outer SELECT sees empty |
| Anchor only, no UNION body | Reject with `"Recursive CTE body must be UNION/UNION ALL"` (matches SQL standard) |
| `UNION ALL` with no termination | Hits `MAX_RECURSION_DEPTH=1000` or `MAX_RECURSION_ROWS=1_000_000`, returns error |
| Multiple CTEs in `WITH RECURSIVE`: `a AS (recursive), b AS (non-recursive referencing a)` | Materialize `a` first (recursive); `b` then references `a_acc` (since outer SELECT sees `a_acc` — need to expose `a` correctly). If `b` references `a` and `a` is recursive, `b` materializes against accumulated `a`. If `a` has 2 temp tables, `b` should see `a` (= `a_acc`) — fine via standard table resolution. |
| Recursive CTE in `WITH` (no RECURSIVE keyword, body has self-reference) | Parser would still set `with_clause.recursive = false` (no RECURSIVE token). Body executes as non-recursive — self-reference sees empty. **Out of scope**: warn/error on self-referencing non-recursive CTE. |
| Nested recursive CTEs (recursive CTE referencing another recursive CTE) | Each recursive CTE has its own `__work` table. Cross-references in step bodies resolve to the **accumulated** form. Standard semantics. Implemented naturally. |
| CTE in `WithDml` body | Out of scope for v3.12.0. The CTE must be materialized BEFORE the DML body. Recursive CTE in `WithDml` makes limited sense (e.g., `WITH RECURSIVE t AS (...) INSERT INTO target SELECT * FROM t`) — but tests will document current behavior. |

## Files to Modify

| File | Change |
|------|--------|
| `src/engine_cte.rs` | Replace lines 28-32 with full recursive CTE logic. Add helpers: `decompose_recursive_body`, `rewrite_step_table_refs`, `derive_cte_columns`. |
| `Cargo.toml` | Register `tests/integration/sql/repro_v312_64f.rs` (1 `[[test]]` block). |
| `tests/integration/sql/repro_v312_64f.rs` | NEW — 9 integration tests. |

## Files NOT Modified (Out of Scope)

| File | Why |
|------|-----|
| `crates/parser/src/parser.rs` | Already parses `RECURSIVE` keyword (line 3903) and `UNION ALL` (`Statement::Union`). No change needed. |
| `crates/parser/src/ast.rs` | AST types `WithClause { recursive }` and `CommonTableExpression { subquery: Statement }` already correct. |
| `crates/executor/src/stored_proc.rs` | Separate stored-procedure path uses `ctx.cte_tables` HashMap. Recursive CTEs inside stored procs not required for v3.12.0 (issue body test doesn't use stored procs). Follow-up if needed. |
| `crates/sql-corpus/src/lib.rs` | Reference impl stays — corpus tests rely on it. Don't change. |
| `crates/planner/src/` | CTEs bypass planner (direct parser→executor). Not in scope. |
| `crates/storage/src/*` | All storage backends already support `create_table`/`insert`/`scan`/`truncate`/`drop_table`. No signature change. |

## Tests

`tests/integration/sql/repro_v312_64f.rs` — 9 integration tests, all using
`ExecutionEngine::with_memory()` for hermetic runs.

1. **`rec_cte_hierarchy_issue_body`** — 5-row org hierarchy (exact reproduction from issue body). Verifies termination at iteration 3.
2. **`rec_cte_count_1_to_n`** — `WITH RECURSIVE cnt AS (SELECT 1 UNION ALL SELECT n+1 FROM cnt WHERE n<10) SELECT * FROM cnt` → 10 rows, terminates at WHERE.
3. **`rec_cte_union_dedup_implicit_termination`** — `UNION` (no ALL) with a query that produces duplicates. Implicit termination via dedup.
4. **`rec_cte_empty_anchor`** — Anchor returns 0 rows (e.g., `WHERE 1=0`). Step runs once with empty working set → 0 rows total.
5. **`rec_cte_rejected_when_no_union`** — `WITH RECURSIVE t AS (SELECT 1) SELECT * FROM t` → parse-time error or executor error (`Recursive CTE body must be UNION/UNION ALL`).
6. **`rec_cte_multiple_ctes_mixed`** — `WITH RECURSIVE tree AS (...), leaves AS (... references tree) SELECT * FROM leaves` — verifies non-recursive sibling can reference recursive.
7. **`rec_cte_cleanup_temp_tables`** — After recursive query completes, `SELECT name FROM sqlite_master WHERE name LIKE '%tree%'` shows no leftover temp tables.
8. **`rec_cte_max_depth_exceeded`** — `WITH RECURSIVE infinite AS (SELECT 1 UNION ALL SELECT n+1 FROM infinite) ...` should eventually error with `exceeded ... row cap` or hit `MAX_RECURSION_DEPTH`.
9. **`rec_cte_with_aggregation_in_step`** — Step contains `SUM(...)` / `COUNT(*)` over current iteration. Verify aggregation re-fires each iteration.

Each test follows the v312-64d pattern (temp dir, `MemoryStorage` or `SqliteMode::open`,
explicit row assertions). Run via:
```bash
cargo test --all-features --test repro_v312_64f
```

Also: register 1 existing test adjustment — `tests/integration/sql/cte_materialization_test.rs::test_cte_recursive_not_supported`
(line 117-131) currently asserts the recursive CTE FAILS. After this fix it should
either be deleted (if a positive recursive test subsumes it) or kept as a positive
test of the same shape.

## Risk & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `rewrite_step_table_refs` misses some identifier-bearing node | Medium | High — wrong rows or recursion panic | Test 1 (issue body) catches it; also test 9 (subquery in step) covers nested case. Add a debug-mode assertion that scanned tables include `t_work`. **Implementation**: explicit field-by-field walk of SelectStatement + Expression — no JSON round-trip; AST shapes verified in parser.rs:725 (`table: String`) and parser.rs:701 (`JoinClause.table: String`). |
| Column schema mismatch (anchor N cols, step M cols, no parser enforcement) | Low | Medium — runtime cast error | Tests verify common patterns. SQL standard requires UNION-compatible; if user violates, runtime error from executor. Document in test 5. |
| In-memory `Vec<Vec<Value>>` truncation not supported by all storage backends | Medium | Medium | **`StorageEngine` confirmed has no `truncate_table`** (only `drop_table`, `rename_table`, `create_table`, `insert`, `scan`). Use `drop_table(t_work) + create_table(same schema)` pattern between iterations. Slight overhead (re-create TableInfo per iter) but acceptable — table creation is cheap. |
| `execute_select` recursion depth on step query (step references step's results) | Low | Medium | Bounded by MAX_RECURSION_DEPTH=1000. Each step is one SELECT, not deep recursion. |
| Worsens single-statement execution time for non-recursive CTEs | None | None | Recursive check is `if with_clause.recursive`, only affects recursive path. |

## Out of Scope (Documented for Follow-up)

- Writable CTE (`WITH ... DELETE/UPDATE/INSERT`) — Issue #4692
- Materialized view (`CREATE MATERIALIZED VIEW`) — Issue #4692 (closed UPSERT is different)
- Recursive CTE inside stored procedures (separate `ctx.cte_tables` path)
- `MAX_RECURSION_DEPTH` runtime configurability (e.g., session variable)
- Recursive CTE in `WithDml` body (insert into target from recursive CTE)
- Cycle detection beyond UNION dedup (e.g., `CYCLE` clause in SQL standard)
- Search/cycle clauses (`SEARCH DEPTH FIRST BY ... SET ...`)

## Verification Checklist (before claiming complete)

Per V312-64d lesson (`v312-64d-4664-system-tables-closure.md` memory):

```bash
cargo check --tests --all-features                              # must PASS
cargo test --all-features --no-run 2>&1 | grep repro_v312_64f   # must appear
cargo test --all-features --test repro_v312_64f                  # 9/9 PASS
cargo test --all-features --test cte_materialization_test        # existing CTE tests still pass
cargo test --all-features --test e2e -- cte_e2e_test             # parser/executor integration
```

If any existing CTE test breaks because it asserted the old `Recursive CTE not yet supported`
error, fix it (test_cte_recursive_not_supported → positive test, or delete).
