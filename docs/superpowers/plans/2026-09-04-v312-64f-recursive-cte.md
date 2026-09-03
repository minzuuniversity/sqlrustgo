# V312-64f / Issue #4699 — Recursive CTE Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `WITH RECURSIVE` CTE execution so that issue #4699 (hierarchy/org-tree traversal queries) returns correct results and terminates properly.

**Architecture:** Replace the 4-line hardcoded rejection in `src/engine_cte.rs:28-32` with a two-table working/accumulated algorithm. Per recursive CTE `t`, materialize `t` (accumulated, visible to outer SELECT) + `t__work` (working set, visible to the recursive step after an AST-level table-ref rewrite). Iterate up to `MAX_RECURSION_DEPTH=1000`, replacing `t__work` each round. UNION ALL appends all; UNION dedupes against accumulated.

**Tech Stack:** Rust 2024, Cargo workspace (single crate `sqlrustgo` root + workspace), existing `StorageEngine` trait (no signature changes).

## Global Constraints

- Branch from `develop/v3.12.0` into `fix/v312-64f-recursive-cte`.
- No changes to `crates/parser/src/parser.rs` (AST already supports RECURSIVE + UNION ALL).
- No changes to `crates/storage/src/*` (StorageEngine primitives sufficient: create_table, insert, scan, drop_table; no truncate_table — use drop+recreate).
- No changes to `crates/sql-corpus/src/lib.rs` (reference impl stays).
- MAX_RECURSION_DEPTH = 1000 (const).
- MAX_RECURSION_ROWS = 1_000_000 (const).
- Convention: AST shapes are `SelectStatement.table: String` (parser.rs:725) + `JoinClause.table: String` (parser.rs:701) + `Expression::Subquery(Box<Statement>)` — flat String fields, not struct wrappers.
- Verification gates (from `v312-64d-4664-system-tables-closure.md` memory):
  ```bash
  cargo check --tests --all-features
  cargo test --all-features --no-run 2>&1 | grep repro_v312_64f
  cargo test --all-features --test repro_v312_64f
  ```
- Commit message prefix: `fix(v312-64f / #4699):`.

---

### Task 1: Branch setup + test scaffolding

**Files:**
- Modify: `Cargo.toml` (lines 304-307 area)
- Create: `tests/integration/sql/repro_v312_64f.rs`
- Branch: create `fix/v312-64f-recursive-cte` off `develop/v3.12.0`

**Interfaces:**
- Produces: registered test target `repro_v312_64f` discoverable by `cargo test --test repro_v312_64f`.

- [ ] **Step 1: Fetch latest develop and create branch**

```bash
cd /home/openclaw/workspace/dev/sqlrustgo
git fetch origin develop/v3.12.0
git checkout develop/v3.12.0
git pull --rebase origin develop/v3.12.0
git checkout -b fix/v312-64f-recursive-cte
```

- [ ] **Step 2: Create empty integration test file**

Create `tests/integration/sql/repro_v312_64f.rs` with this exact content:

```rust
//! V312-64f / Issue #4699 — Recursive CTE integration tests.
//!
//! Tests cover:
//! - Hierarchy traversal (issue body case)
//! - Count 1..N (terminating recursion)
//! - UNION (non-ALL) dedup
//! - Empty anchor
//! - Non-UNION body rejection
//! - Multiple CTEs mixed (recursive + non-recursive)
//! - Cleanup after recursive CTE
//! - MAX_RECURSION_ROWS exceeded
//! - Aggregation inside step
```

- [ ] **Step 3: Register test in root Cargo.toml**

In `Cargo.toml`, locate the existing `repro_v312_64d` block (around line 300-307) and add the following block immediately after it:

```toml
[[test]]
name = "repro_v312_64f"
path = "tests/integration/sql/repro_v312_64f.rs"
```

- [ ] **Step 4: Verify test target is registered**

Run: `cargo test --all-features --no-run 2>&1 | grep repro_v312_64f`
Expected: a line containing `repro_v312_64f` in the output (the binary target being compiled).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): scaffold repro_v312_64f integration test target"
```

---

### Task 2: CTE body decomposition helper

**Files:**
- Modify: `src/engine_cte.rs` (top of file, after `use` statements, before `materialize_cte_tables`)

**Interfaces:**
- Produces: `pub fn decompose_recursive_body(stmt: &Statement) -> SqlResult<(Box<Statement>, Box<Statement>, bool)>`
  - Returns `(anchor, step, union_all)` on success
  - Returns `Err` if body is not `Statement::Union { left, right, union_all }`
- Consumes: `sqlrustgo_parser::Statement`, `SqlError`

- [ ] **Step 1: Write failing unit test**

In `src/engine_cte.rs`, locate the `#[cfg(test)] mod tests` block. Add the following at the end of the existing tests (before the closing `}`):

```rust
    #[test]
    fn decompose_recursive_body_union_all_ok() {
        // V312-64f: anchor UNION ALL step
        use sqlrustgo_parser::Statement;
        let sql = "SELECT 1 AS n UNION ALL SELECT n + 1 FROM cte WHERE n < 3";
        let stmt = sqlrustgo_parser::parse(sql).unwrap();
        let body = match stmt {
            Statement::Union(u) => Box::new(Statement::Union(u)),
            _ => panic!("expected Statement::Union"),
        };
        let (anchor, step, union_all) =
            crate::engine_cte::decompose_recursive_body(&body).unwrap();
        assert!(union_all, "UNION ALL must set union_all=true");
        assert!(matches!(*anchor, Statement::Select(_)));
        assert!(matches!(*step, Statement::Select(_)));
    }

    #[test]
    fn decompose_recursive_body_union_distinct_ok() {
        use sqlrustgo_parser::Statement;
        let sql = "SELECT 1 AS n UNION SELECT n + 1 FROM cte WHERE n < 3";
        let stmt = sqlrustgo_parser::parse(sql).unwrap();
        let body = Box::new(stmt);
        let (_anchor, _step, union_all) =
            crate::engine_cte::decompose_recursive_body(&body).unwrap();
        assert!(!union_all, "UNION (no ALL) must set union_all=false");
    }

    #[test]
    fn decompose_recursive_body_non_union_rejected() {
        use sqlrustgo_parser::Statement;
        let body = Box::new(Statement::Select(
            sqlrustgo_parser::parse("SELECT 1").unwrap(),
        ));
        let r = crate::engine_cte::decompose_recursive_body(&body);
        assert!(r.is_err(), "non-UNION body must be rejected");
        assert!(r.unwrap_err().to_string().contains("UNION"));
    }
```

NOTE: The exact parser entry point is `sqlrustgo_parser::parse(string) -> Result<Statement, String>`. If the crate's public API is different (e.g., `Parser::new(...)`), substitute the actual parse path. The plan engineer should run `cargo check --tests --all-features 2>&1 | grep "error\[E0433\]"` after writing this test to verify the import path; fix imports minimally until the test compiles.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --all-features --lib engine_cte::tests::decompose_recursive_body 2>&1 | tail -20`
Expected: compile error mentioning `decompose_recursive_body` not found OR test fails with "function not found".

- [ ] **Step 3: Implement `decompose_recursive_body`**

In `src/engine_cte.rs`, immediately after the `use` statements at the top, add:

```rust
/// V312-64f / Issue #4699: decompose a recursive CTE body into
/// (anchor, step, union_all). Per SQL:1999, a recursive CTE body MUST
/// be `SELECT ... UNION [ALL] SELECT ...` where the second SELECT may
/// reference the CTE itself. Returns an error for any other shape.
pub fn decompose_recursive_body(
    stmt: &sqlrustgo_parser::Statement,
) -> SqlResult<(Box<sqlrustgo_parser::Statement>, Box<sqlrustgo_parser::Statement>, bool)> {
    use sqlrustgo_parser::Statement;
    match stmt {
        Statement::Union(u) => Ok((u.left.clone(), u.right.clone(), u.union_all)),
        other => Err(SqlError::ExecutionError(format!(
            "Recursive CTE body must be UNION/UNION ALL of two SELECTs, got {:?}",
            std::mem::discriminant(other)
        ))),
    }
}
```

NOTE: If `UnionStatement` is a different type than `Statement::Union`, check `crates/parser/src/parser.rs:3868` (`current = if is_union { ... }`) for the actual struct name. Adjust the match arm accordingly.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --all-features --lib engine_cte::tests::decompose_recursive_body 2>&1 | tail -20`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine_cte.rs
git commit -m "fix(v312-64f / #4699): add decompose_recursive_body helper for UNION/UNION ALL split"
```

---

### Task 3: Column schema derivation helper

**Files:**
- Modify: `src/engine_cte.rs` (inside the existing `materialize_cte_tables`, extract lines 56-91 into a helper; add the helper before `materialize_cte_tables`)

**Interfaces:**
- Produces: `pub fn derive_cte_columns(cte: &CommonTableExpression, seed_rows: &[Vec<Value>], subquery_column_names: &[String]) -> Vec<ColumnDefinition>`
- Consumes: `CommonTableExpression` (parser type), seed rows, subquery column names

- [ ] **Step 1: Write failing unit test**

In the test module of `src/engine_cte.rs`, add:

```rust
    #[test]
    fn derive_cte_columns_explicit() {
        use sqlrustgo_parser::parser::CommonTableExpression;
        use sqlrustgo_storage::engine::ColumnDefinition;
        use sqlrustgo::Value;
        let cte = CommonTableExpression {
            name: "t".to_string(),
            columns: vec!["a".to_string(), "b".to_string()],
            subquery: Box::new(sqlrustgo_parser::parse("SELECT 1, 2").unwrap()),
        };
        let cols = crate::engine_cte::derive_cte_columns(
            &cte,
            &[vec![Value::Integer(1), Value::Integer(2)]],
            &["a".to_string(), "b".to_string()],
        );
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "a");
        assert_eq!(cols[1].name, "b");
    }

    #[test]
    fn derive_cte_columns_fallback_col_i() {
        use sqlrustgo_parser::parser::CommonTableExpression;
        use sqlrustgo::Value;
        let cte = CommonTableExpression {
            name: "t".to_string(),
            columns: vec![],
            subquery: Box::new(sqlrustgo_parser::parse("SELECT 1, 2").unwrap()),
        };
        let cols = crate::engine_cte::derive_cte_columns(
            &cte,
            &[vec![Value::Integer(1), Value::Integer(2)]],
            &[],
        );
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "col_0");
        assert_eq!(cols[1].name, "col_1");
    }
```

NOTE: `CommonTableExpression` may be re-exported differently (e.g., from `sqlrustgo_parser::parser` directly). Adjust the import if needed. Run `cargo check --tests --all-features 2>&1 | grep "error\[E0433\]"` to find the correct path.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --all-features --lib engine_cte::tests::derive_cte_columns 2>&1 | tail -20`
Expected: compile error `cannot find function derive_cte_columns`.

- [ ] **Step 3: Extract and add helper**

In `src/engine_cte.rs`, replace the existing column-resolution block (lines 56-91 of `materialize_cte_tables`) with a call to a new helper, and add the helper above `materialize_cte_tables`. The helper:

```rust
/// V312-64f: derive CTE column definitions from explicit `name(col, ...)` form,
/// the subquery's SELECT-column aliases, or fallback `col_<i>` placeholders.
/// Extracted from `materialize_cte_tables` so recursive and non-recursive
/// paths share the same schema-resolution logic.
pub fn derive_cte_columns(
    cte: &sqlrustgo_parser::parser::CommonTableExpression,
    seed_rows: &[Vec<sqlrustgo::Value>],
    subquery_column_names: &[String],
) -> Vec<sqlrustgo_storage::engine::ColumnDefinition> {
    use sqlrustgo_storage::engine::ColumnDefinition;
    let column_count = if !cte.columns.is_empty() {
        cte.columns.len()
    } else if !seed_rows.is_empty() {
        seed_rows[0].len()
    } else {
        0
    };
    (0..column_count)
        .map(|i| {
            let name = if !cte.columns.is_empty() {
                cte.columns[i].clone()
            } else if i < subquery_column_names.len() && !subquery_column_names[i].is_empty() {
                subquery_column_names[i].clone()
            } else {
                format!("col_{}", i)
            };
            ColumnDefinition {
                name,
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }
        })
        .collect()
}
```

Then in `materialize_cte_tables`, replace lines 56-91 with:

```rust
        let subquery_column_names: Vec<String> = match cte.subquery.as_ref() {
            Statement::Select(s) => s
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect(),
            _ => Vec::new(),
        };
        let columns = derive_cte_columns(cte, &cte_rows, &subquery_column_names);
```

(Keep the rest of `materialize_cte_tables` unchanged.)

- [ ] **Step 4: Run all CTE tests to verify nothing broke**

Run: `cargo test --all-features --lib engine_cte::tests 2>&1 | tail -20`
Expected: All previous tests still PASS + 2 new tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine_cte.rs
git commit -m "fix(v312-64f / #4699): extract derive_cte_columns helper for shared schema logic"
```

---

### Task 4: AST rewriter — table + join + subquery substitution

**Files:**
- Modify: `src/engine_cte.rs` (add `rewrite_step_table_refs` and helper)

**Interfaces:**
- Produces: `pub fn rewrite_step_table_refs(stmt: &SelectStatement, from: &str, to: &str) -> SelectStatement`
  - Clones `stmt`, replacing any table reference equal to `from` with `to`
  - Walks `stmt.table`, every `JoinClause.table`, and every `Expression::Subquery` recursively

- [ ] **Step 1: Write failing unit tests**

In the test module, add:

```rust
    #[test]
    fn rewrite_step_top_level_from() {
        use sqlrustgo_parser::parser::SelectStatement;
        let sql = "SELECT id FROM tree WHERE id > 0";
        let stmt = match sqlrustgo_parser::parse(sql).unwrap() {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected Select"),
        };
        let rewritten = crate::engine_cte::rewrite_step_table_refs(&stmt, "tree", "tree__work");
        assert_eq!(rewritten.table, "tree__work");
    }

    #[test]
    fn rewrite_step_join_clause() {
        use sqlrustgo_parser::parser::SelectStatement;
        let sql = "SELECT e.id FROM emp e JOIN tree ON e.mgr_id = tree.id";
        let stmt = match sqlrustgo_parser::parse(sql).unwrap() {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected Select"),
        };
        let rewritten = crate::engine_cte::rewrite_step_table_refs(&stmt, "tree", "tree__work");
        assert_eq!(rewritten.joins.len(), 1);
        assert_eq!(rewritten.joins[0].table, "tree__work");
        assert_eq!(rewritten.table, "emp"); // unchanged
    }

    #[test]
    fn rewrite_step_subquery_in_where() {
        use sqlrustgo_parser::parser::SelectStatement;
        let sql = "SELECT id FROM outer_t WHERE id IN (SELECT id FROM tree)";
        let stmt = match sqlrustgo_parser::parse(sql).unwrap() {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected Select"),
        };
        let rewritten = crate::engine_cte::rewrite_step_table_refs(&stmt, "tree", "tree__work");
        // The inner subquery's SelectStatement should have table=tree__work
        let inner_table = match rewritten.where_clause.as_ref().unwrap() {
            sqlrustgo_parser::Expression::Subquery(inner_stmt) => {
                match inner_stmt.as_ref() {
                    sqlrustgo_parser::Statement::Select(s) => s.table.clone(),
                    _ => String::from("(not a select)"),
                }
            }
            _ => String::from("(no subquery)"),
        };
        assert_eq!(inner_table, "tree__work");
    }

    #[test]
    fn rewrite_step_preserves_unrelated_tables() {
        use sqlrustgo_parser::parser::SelectStatement;
        let sql = "SELECT id FROM other_t JOIN tree ON other_t.x = tree.x";
        let stmt = match sqlrustgo_parser::parse(sql).unwrap() {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected Select"),
        };
        let rewritten = crate::engine_cte::rewrite_step_table_refs(&stmt, "tree", "tree__work");
        assert_eq!(rewritten.table, "other_t");
        assert_eq!(rewritten.joins[0].table, "tree__work");
    }
```

NOTE: `Expression::Subquery` may be named differently or wrapped in `Box<>`. Inspect `crates/parser/src/parser.rs` around the `Expression` enum to find the exact variant. If the inner Statement is `Statement::Select(Box<SelectStatement>)`, unwrap appropriately.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --all-features --lib engine_cte::tests::rewrite_step 2>&1 | tail -20`
Expected: compile error `cannot find function rewrite_step_table_refs`.

- [ ] **Step 3: Implement rewriter**

Add to `src/engine_cte.rs`:

```rust
/// V312-64f: clone a SelectStatement, replacing every reference to table
/// `from` with `to`. Walks top-level FROM, every JOIN's table, and every
/// Expression::Subquery (recursive). Other tables are unchanged.
///
/// Required because the recursive step's `FROM t` must resolve to `t__work`
/// (the working set) rather than `t` (the accumulated result), so the step
/// only sees the current iteration's rows.
pub fn rewrite_step_table_refs(
    stmt: &sqlrustgo_parser::parser::SelectStatement,
    from: &str,
    to: &str,
) -> sqlrustgo_parser::parser::SelectStatement {
    use sqlrustgo_parser::{Expression, Statement};

    let mut cloned = stmt.clone();

    // Top-level FROM
    if cloned.table == from {
        cloned.table = to.to_string();
    }

    // JOINs
    for join in cloned.joins.iter_mut() {
        if join.table == from {
            join.table = to.to_string();
        }
    }

    // Subqueries in WHERE / HAVING / SELECT columns
    if let Some(where_expr) = cloned.where_clause.as_mut() {
        rewrite_expr(where_expr, from, to);
    }
    if let Some(having_expr) = cloned.having_clause.as_mut() {
        rewrite_expr(having_expr, from, to);
    }
    for col in cloned.columns.iter_mut() {
        rewrite_expr(&mut col.expression, from, to);
    }

    cloned
}

fn rewrite_expr(expr: &mut sqlrustgo_parser::Expression, from: &str, to: &str) {
    use sqlrustgo_parser::{Expression, Statement};
    match expr {
        Expression::Subquery(inner_stmt) => {
            // Recurse: inner could be Select or Union
            match inner_stmt.as_mut() {
                Statement::Select(s) => {
                    *s = rewrite_step_table_refs(s, from, to);
                }
                Statement::Union(u) => {
                    // Both sides must be Selects (parser enforces this)
                    if let Statement::Select(s) = u.left.as_mut() {
                        *s = rewrite_step_table_refs(s, from, to);
                    }
                    if let Statement::Select(s) = u.right.as_mut() {
                        *s = rewrite_step_table_refs(s, from, to);
                    }
                }
                _ => {}
            }
        }
        // BinaryOp / UnaryOp / Function etc. may have nested subqueries.
        // For v312-64f, recurse into the most common shapes:
        Expression::BinaryOp { left, right, .. } => {
            rewrite_expr(left, from, to);
            rewrite_expr(right, from, to);
        }
        Expression::UnaryOp { expr: inner, .. } => {
            rewrite_expr(inner, from, to);
        }
        _ => {}
    }
}
```

NOTE: `SelectStatement` field names may differ (`joins` vs `join_clauses`, `where_clause` vs `where_expr`, `having_clause` vs `having`). Inspect the actual struct at `crates/parser/src/parser.rs:723`. Adjust field names as needed. `Expression::Subquery` may be `Expression::Subquery(Box<Statement>)` or have a different name. `Expression::BinaryOp` may be `Expression::BinaryOp { op, left, right }` or `Expression::Binary(Box<...>, BinaryOp, Box<...>)`. Adapt.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --all-features --lib engine_cte::tests::rewrite_step 2>&1 | tail -20`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/engine_cte.rs
git commit -m "fix(v312-64f / #4699): add AST rewriter for recursive step table refs"
```

---

### Task 5: Recursive CTE main algorithm (issue body test)

**Files:**
- Modify: `src/engine_cte.rs` (replace lines 28-32 with full recursive logic)
- Modify: `tests/integration/sql/repro_v312_64f.rs` (add first integration test)

**Interfaces:**
- Produces: `pub fn materialize_recursive_cte<S: StorageEngine + 'static>(engine: &mut ExecutionEngine<S>, cte: &CommonTableExpression) -> SqlResult<()>`

- [ ] **Step 1: Write the failing integration test (issue body case)**

Replace the contents of `tests/integration/sql/repro_v312_64f.rs` with:

```rust
//! V312-64f / Issue #4699 — Recursive CTE integration tests.
//!
//! Tests cover:
//! - Hierarchy traversal (issue body case)
//! - Count 1..N (terminating recursion)
//! - UNION (non-ALL) dedup
//! - Empty anchor
//! - Non-UNION body rejection
//! - Multiple CTEs mixed (recursive + non-recursive)
//! - Cleanup after recursive CTE
//! - MAX_RECURSION_ROWS exceeded
//! - Aggregation inside step

use sqlrustgo::{ExecutionEngine, MemoryExecutionEngine, SqlResult};
use sqlrustgo_executor::ExecutorResult;

fn fresh() -> ExecutionEngine<sqlrustgo::MemoryStorage> {
    sqlrustgo::MemoryExecutionEngine::new()
}

fn run(engine: &mut ExecutionEngine<sqlrustgo::MemoryStorage>, sql: &str) -> SqlResult<ExecutorResult> {
    engine.execute(sql)
}

#[test]
fn rec_cte_hierarchy_issue_body() {
    // V312-64f / Issue #4699: org hierarchy traversal.
    // Expected: 5 rows (CEO + 2 directs + 2 indirects).
    let mut e = fresh();
    run(&mut e, "CREATE TABLE emp(id INT, mgr_id INT, name TEXT)").unwrap();
    run(&mut e, "INSERT INTO emp VALUES (1, NULL, 'CEO'), (2, 1, 'VP'), (3, 1, 'CTO'), (4, 2, 'Dev1'), (5, 2, 'Dev2')").unwrap();

    let r = run(
        &mut e,
        "WITH RECURSIVE tree AS ( \
            SELECT id, name FROM emp WHERE mgr_id IS NULL \
            UNION ALL \
            SELECT e.id, e.name FROM emp e JOIN tree ON e.mgr_id = tree.id \
         ) SELECT id, name FROM tree ORDER BY id",
    ).unwrap();

    assert_eq!(r.rows.len(), 5, "expected 5 rows, got {}: {:?}", r.rows.len(), r.rows);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(1));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Text("CEO".into()));
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Integer(2));
    assert_eq!(r.rows[2][0], sqlrustgo::Value::Integer(3));
    assert_eq!(r.rows[3][0], sqlrustgo::Value::Integer(4));
    assert_eq!(r.rows[4][0], sqlrustgo::Value::Integer(5));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --all-features --test repro_v312_64f rec_cte_hierarchy_issue_body 2>&1 | tail -20`
Expected: FAIL with `Recursive CTE not yet supported`.

- [ ] **Step 3: Replace the hardcoded rejection in `materialize_cte_tables`**

In `src/engine_cte.rs`, replace lines 28-32 (the `if with_clause.recursive { return Err(...) }` block) with:

```rust
    if with_clause.recursive {
        // V312-64f / Issue #4699: route to recursive CTE executor.
        let ctes = with_clause.ctes.clone();
        for cte in &ctes {
            materialize_recursive_cte(engine, cte)?;
        }
        return Ok(ctes.iter().map(|c| c.name.clone()).collect());
    }
```

(Add `use sqlrustgo_parser::parser::CommonTableExpression;` near the top of the file if not already imported.)

- [ ] **Step 4: Add the `materialize_recursive_cte` function**

In `src/engine_cte.rs`, add after `materialize_cte_tables`:

```rust
const MAX_RECURSION_DEPTH: usize = 1000;
const MAX_RECURSION_ROWS: usize = 1_000_000;

/// V312-64f / Issue #4699: materialize a single recursive CTE.
/// Creates two temp tables: `t` (accumulated, visible to outer SELECT)
/// and `t__work` (working set, visible to the recursive step after
/// AST-level table-ref rewrite). Iterates up to MAX_RECURSION_DEPTH
/// rounds; appends new rows to `t`, replaces `t__work` each round.
pub fn materialize_recursive_cte<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    cte: &sqlrustgo_parser::parser::CommonTableExpression,
) -> SqlResult<()> {
    use sqlrustgo_parser::Statement;
    use sqlrustgo_storage::engine::TableInfo;

    // 1. Decompose body into (anchor, step, union_all).
    let (anchor_stmt, step_stmt, union_all) =
        decompose_recursive_body(cte.subquery.as_ref())?;

    // 2. Execute anchor → seed_rows.
    let anchor_select = match anchor_stmt.as_ref() {
        Statement::Select(s) => s,
        _ => return Err(SqlError::ExecutionError(
            "Recursive CTE anchor must be SELECT".to_string(),
        )),
    };
    let seed_rows = engine.execute_select(anchor_select)?.rows;

    // 3. Derive column schema from seed (or anchor subquery aliases).
    let subquery_column_names: Vec<String> = match anchor_stmt.as_ref() {
        Statement::Select(s) => s
            .columns
            .iter()
            .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
            .collect(),
        _ => Vec::new(),
    };
    let columns = derive_cte_columns(cte, &seed_rows, &subquery_column_names);

    // 4. Create temp table `t` (accumulated) + temp table `t__work` (working set).
    let t = cte.name.clone();
    let t_work = format!("{}__work", cte.name);
    let table_info_for = |name: &str| TableInfo {
        name: name.to_string(),
        columns: columns.clone(),
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        compression: None,
        collations: std::collections::HashMap::new(),
        original_sql: String::new(),
    };
    {
        let mut storage = engine.storage.write();
        storage
            .create_table(&table_info_for(&t))
            .map_err(|e| SqlError::ExecutionError(format!("Create CTE table {}: {}", t, e)))?;
        storage
            .create_table(&table_info_for(&t_work))
            .map_err(|e| SqlError::ExecutionError(format!("Create CTE table {}: {}", t_work, e)))?;
        if !seed_rows.is_empty() {
            storage
                .insert(&t, seed_rows.clone())
                .map_err(|e| SqlError::ExecutionError(format!("Insert seed into {}: {}", t, e)))?;
            storage
                .insert(&t_work, seed_rows.clone())
                .map_err(|e| SqlError::ExecutionError(format!("Insert seed into {}: {}", t_work, e)))?;
        }
    }

    // 5. Rewrite step's references from `t` to `t__work`.
    let step_select = match step_stmt.as_ref() {
        Statement::Select(s) => s,
        _ => return Err(SqlError::ExecutionError(
            "Recursive CTE step must be SELECT".to_string(),
        )),
    };
    let step_rewritten = rewrite_step_table_refs(step_select, &t, &t_work);

    // 6. Iterate.
    let mut total: usize = seed_rows.len();
    for _depth in 0..MAX_RECURSION_DEPTH {
        let step_rows = engine.execute_select(&step_rewritten)?.rows;
        if step_rows.is_empty() {
            break;
        }

        // UNION (not ALL): dedupe against accumulated.
        let new_rows = if union_all {
            step_rows
        } else {
            let acc_existing = {
                let storage = engine.storage.read();
                storage
                    .scan(&t)
                    .map_err(|e| SqlError::ExecutionError(format!("Scan {}: {}", t, e)))?
            };
            let acc_existing_values: Vec<Vec<sqlrustgo::Value>> = acc_existing
                .into_iter()
                .map(|r| r.values)
                .collect();
            let mut new_rows = Vec::new();
            for r in &step_rows {
                if !acc_existing_values.contains(r) && !new_rows.contains(r) {
                    new_rows.push(r.clone());
                }
            }
            new_rows
        };
        if new_rows.is_empty() {
            break;
        }

        // Append to accumulated; replace working set.
        {
            let mut storage = engine.storage.write();
            storage
                .insert(&t, new_rows.clone())
                .map_err(|e| SqlError::ExecutionError(format!("Insert into {}: {}", t, e)))?;
            // StorageEngine has no truncate_table; drop + re-create.
            let _ = storage.drop_table(&t_work);
            storage
                .create_table(&table_info_for(&t_work))
                .map_err(|e| SqlError::ExecutionError(format!("Recreate {}: {}", t_work, e)))?;
            storage
                .insert(&t_work, new_rows.clone())
                .map_err(|e| SqlError::ExecutionError(format!("Insert into {}: {}", t_work, e)))?;
        }
        total += new_rows.len();

        if total > MAX_RECURSION_ROWS {
            // Cleanup before returning error.
            let mut storage = engine.storage.write();
            let _ = storage.drop_table(&t_work);
            return Err(SqlError::ExecutionError(format!(
                "Recursive CTE {} exceeded {} row cap",
                t, MAX_RECURSION_ROWS
            )));
        }
    }

    // 7. Drop t__work; `t` remains visible to outer SELECT.
    {
        let mut storage = engine.storage.write();
        let _ = storage.drop_table(&t_work);
    }
    Ok(())
}
```

NOTE: `engine.storage` may be a different access pattern (e.g., `engine.storage_mut()` method, or a separate field). Inspect `src/execution_engine.rs` for the actual `ExecutionEngine` storage field name. The plan engineer should verify `engine.storage.write()` works or substitute the correct accessor. Also: `Record::values` may be named differently; check `crates/storage/src/engine.rs` for the `Record` struct.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --all-features --test repro_v312_64f rec_cte_hierarchy_issue_body 2>&1 | tail -30`
Expected: PASS, 5 rows returned.

- [ ] **Step 6: Commit**

```bash
git add src/engine_cte.rs tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): implement recursive CTE two-table algorithm (issue body)"
```

---

### Task 6: Count 1..N + UNION ALL termination test

**Files:**
- Modify: `tests/integration/sql/repro_v312_64f.rs` (add 2 tests)

- [ ] **Step 1: Add tests**

Append to `tests/integration/sql/repro_v312_64f.rs`:

```rust
#[test]
fn rec_cte_count_1_to_n() {
    // Terminating recursion: WHERE n < 10 stops the loop at 10 rows.
    let mut e = fresh();
    let r = run(
        &mut e,
        "WITH RECURSIVE cnt AS ( \
            SELECT 1 AS n UNION ALL \
            SELECT n + 1 FROM cnt WHERE n < 10 \
         ) SELECT n FROM cnt ORDER BY n",
    ).unwrap();
    assert_eq!(r.rows.len(), 10, "expected 10 rows, got {}", r.rows.len());
    for i in 0..10 {
        assert_eq!(r.rows[i][0], sqlrustgo::Value::Integer((i + 1) as i64));
    }
}

#[test]
fn rec_cte_union_dedup_implicit_termination() {
    // UNION (not ALL): step produces duplicates; dedup against accumulated
    // causes termination when no new rows remain.
    let mut e = fresh();
    run(&mut e, "CREATE TABLE nums(n INT)").unwrap();
    run(&mut e, "INSERT INTO nums VALUES (1), (2), (3)").unwrap();

    // Step is `SELECT n FROM nums` — same as anchor; UNION dedupes.
    let r = run(
        &mut e,
        "WITH RECURSIVE all_n AS ( \
            SELECT n FROM nums \
            UNION \
            SELECT n FROM nums JOIN all_n ON nums.n = all_n.n \
         ) SELECT n FROM all_n ORDER BY n",
    ).unwrap();
    assert_eq!(r.rows.len(), 3, "expected 3 distinct values, got {}", r.rows.len());
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --all-features --test repro_v312_64f 2>&1 | tail -30`
Expected: All 3 tests PASS (hierarchy + count + union dedup).

- [ ] **Step 3: Commit**

```bash
git add tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): add count 1..N and UNION dedup integration tests"
```

---

### Task 7: Edge cases — empty anchor + non-UNION rejection

**Files:**
- Modify: `tests/integration/sql/repro_v312_64f.rs`

- [ ] **Step 1: Add tests**

Append:

```rust
#[test]
fn rec_cte_empty_anchor() {
    // Anchor returns 0 rows; step runs once with empty working set.
    let mut e = fresh();
    run(&mut e, "CREATE TABLE t(n INT)").unwrap();
    run(&mut e, "INSERT INTO t VALUES (1), (2), (3)").unwrap();

    let r = run(
        &mut e,
        "WITH RECURSIVE empty_cte AS ( \
            SELECT n FROM t WHERE 1=0 \
            UNION ALL \
            SELECT n + 10 FROM empty_cte WHERE n < 100 \
         ) SELECT n FROM empty_cte",
    ).unwrap();
    assert_eq!(r.rows.len(), 0, "expected 0 rows, got {}", r.rows.len());
}

#[test]
fn rec_cte_rejected_when_no_union() {
    // RECURSIVE keyword with non-UNION body must error.
    let mut e = fresh();
    let r = e.execute(
        "WITH RECURSIVE t AS (SELECT 1 AS n) SELECT * FROM t",
    );
    assert!(r.is_err(), "non-UNION recursive CTE must be rejected");
    let msg = r.unwrap_err().to_string();
    assert!(
        msg.contains("UNION") || msg.contains("Recursive"),
        "expected UNION/Recursive error, got: {}",
        msg
    );
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --all-features --test repro_v312_64f 2>&1 | tail -30`
Expected: 5 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): add empty anchor + non-UNION rejection tests"
```

---

### Task 8: Multiple CTEs mixed + cleanup

**Files:**
- Modify: `tests/integration/sql/repro_v312_64f.rs`

- [ ] **Step 1: Add tests**

Append:

```rust
#[test]
fn rec_cte_multiple_ctes_mixed() {
    // WITH RECURSIVE has 1 recursive + 1 non-recursive sibling.
    // Non-recursive sibling references recursive CTE in its body.
    let mut e = fresh();
    run(&mut e, "CREATE TABLE emp(id INT, mgr_id INT)").unwrap();
    run(&mut e, "INSERT INTO emp VALUES (1, NULL), (2, 1), (3, 1), (4, 2)").unwrap();

    let r = run(
        &mut e,
        "WITH RECURSIVE tree AS ( \
            SELECT id, mgr_id FROM emp WHERE mgr_id IS NULL \
            UNION ALL \
            SELECT e.id, e.mgr_id FROM emp e JOIN tree ON e.mgr_id = tree.id \
         ), \
         leaf_count AS ( \
            SELECT COUNT(*) AS cnt FROM emp WHERE id NOT IN (SELECT mgr_id FROM tree WHERE mgr_id IS NOT NULL) \
         ) \
         SELECT tree.id, leaf_count.cnt FROM tree, leaf_count ORDER BY tree.id",
    ).unwrap();
    assert_eq!(r.rows.len(), 4, "expected 4 rows, got {}", r.rows.len());
}

#[test]
fn rec_cte_cleanup_temp_tables() {
    // After recursive query, t__work should be dropped.
    let mut e = fresh();
    run(&mut e, "CREATE TABLE t(n INT)").unwrap();
    run(&mut e, "INSERT INTO t VALUES (1), (2)").unwrap();
    run(
        &mut e,
        "WITH RECURSIVE tree AS ( \
            SELECT n FROM t UNION ALL SELECT n + 1 FROM tree WHERE n < 3 \
         ) SELECT n FROM tree",
    ).unwrap();

    // Verify t__work does NOT exist as a visible table.
    // SELECT * FROM tree__work should fail (table not found).
    let r = e.execute("SELECT n FROM tree__work");
    assert!(r.is_err(), "tree__work should be dropped after query");
}
```

NOTE: The `leaf_count` test depends on NOT IN semantics working correctly. If `NOT IN` with NULL handling fails, this test may produce wrong rows. Verify or substitute with a simpler `WHERE` clause if needed.

- [ ] **Step 2: Run tests**

Run: `cargo test --all-features --test repro_v312_64f 2>&1 | tail -30`
Expected: 7 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): add multiple-CTE mixed and cleanup integration tests"
```

---

### Task 9: MAX_RECURSION_ROWS exceeded

**Files:**
- Modify: `tests/integration/sql/repro_v312_64f.rs`

- [ ] **Step 1: Add test**

Append:

```rust
#[test]
fn rec_cte_max_depth_exceeded() {
    // Recursive CTE with no termination condition; should hit MAX_RECURSION_ROWS
    // (1_000_000) or MAX_RECURSION_DEPTH (1000) cap.
    let mut e = fresh();
    let r = e.execute(
        "WITH RECURSIVE inf AS ( \
            SELECT 1 AS n UNION ALL SELECT n + 1 FROM inf \
         ) SELECT n FROM inf",
    );
    // Should error after hitting cap.
    assert!(r.is_err(), "non-terminating recursive CTE must error");
    let msg = r.unwrap_err().to_string();
    assert!(
        msg.contains("exceeded") || msg.contains("cap") || msg.contains("depth"),
        "expected cap-exceeded error, got: {}",
        msg
    );
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --all-features --test repro_v312_64f rec_cte_max_depth_exceeded 2>&1 | tail -30`
Expected: PASS. Note: this test may take a few seconds since it runs 1000 iterations producing ~1000 rows before hitting the depth cap.

- [ ] **Step 3: Commit**

```bash
git add tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): add MAX_RECURSION_ROWS exceeded test"
```

---

### Task 10: Aggregation inside step

**Files:**
- Modify: `tests/integration/sql/repro_v312_64f.rs`

- [ ] **Step 1: Add test**

Append:

```rust
#[test]
fn rec_cte_with_aggregation_in_step() {
    // Step body contains an aggregate (COUNT). Verify it re-fires each iteration.
    let mut e = fresh();
    run(&mut e, "CREATE TABLE nums(n INT)").unwrap();
    run(&mut e, "INSERT INTO nums VALUES (1), (2), (3), (4), (5)").unwrap();

    // Anchor: count of all nums. Step: each iteration, count rows in working set
    // (a different value each round since working set changes).
    // This is a contrived example; the main point is aggregate functions work
    // inside the recursive SELECT.
    let r = run(
        &mut e,
        "WITH RECURSIVE sizes AS ( \
            SELECT 1 AS iter, COUNT(*) AS cnt FROM nums \
            UNION ALL \
            SELECT iter + 1, cnt + 1 FROM sizes WHERE iter < 5 \
         ) SELECT iter, cnt FROM sizes ORDER BY iter",
    ).unwrap();
    assert_eq!(r.rows.len(), 6, "expected 6 rows, got {}", r.rows.len());
    // Iteration 0: cnt = 5 (full count). Each next: cnt = previous + 1.
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(5));
    assert_eq!(r.rows[5][1], sqlrustgo::Value::Integer(10));
}
```

NOTE: This test relies on aggregate functions being re-evaluable against the working set each iteration. The rewriter must preserve aggregate semantics. If this fails, the issue may be that aggregate arguments reference `sizes` columns (which resolve via the working set) — verify the working set has the same column schema as accumulated.

- [ ] **Step 2: Run test**

Run: `cargo test --all-features --test repro_v312_64f rec_cte_with_aggregation_in_step 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/integration/sql/repro_v312_64f.rs
git commit -m "fix(v312-64f / #4699): add aggregation-in-step integration test"
```

---

### Task 11: Update existing cte_materialization_test.rs

**Files:**
- Modify: `tests/integration/sql/cte_materialization_test.rs`

- [ ] **Step 1: Remove the old rejection assertion**

In `tests/integration/sql/cte_materialization_test.rs`, delete the `test_cte_recursive_not_supported` function (lines 117-131 per the explore agent's report).

- [ ] **Step 2: Run the modified test file**

Run: `cargo test --all-features --test cte_materialization_test 2>&1 | tail -30`
Expected: All remaining tests PASS. The recursive-rejection test no longer exists; the actual recursive CTE behavior is now tested in `repro_v312_64f`.

- [ ] **Step 3: Commit**

```bash
git add tests/integration/sql/cte_materialization_test.rs
git commit -m "fix(v312-64f / #4699): remove obsolete recursive-rejection test (now supported)"
```

---

### Task 12: Verification + push + open PR

**Files:** none modified

- [ ] **Step 1: Run the full verification gates**

```bash
cargo check --tests --all-features
cargo test --all-features --no-run 2>&1 | grep repro_v312_64f
cargo test --all-features --test repro_v312_64f
cargo test --all-features --test cte_materialization_test
cargo test --all-features --test e2e -- cte_e2e_test
cargo test --all-features 2>&1 | tail -30
```

Expected:
- `cargo check` exits 0.
- `grep repro_v312_64f` finds the test binary in output (registration works).
- `repro_v312_64f` shows 9/9 PASS.
- `cte_materialization_test` shows all remaining tests PASS.
- `e2e` CTE tests PASS.
- Full `cargo test` does not show any test failure caused by this fix.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-features --all-targets -- -D warnings 2>&1 | tail -30`
Expected: No warnings introduced by the fix. If pre-existing warnings exist, they should be unrelated to `engine_cte.rs` or `repro_v312_64f.rs`.

- [ ] **Step 3: Run format check**

Run: `cargo fmt --check --all 2>&1 | tail -10`
Expected: exit 0 (or only warnings unrelated to our files).

- [ ] **Step 4: Push branch**

```bash
git push -u origin fix/v312-64f-recursive-cte
```

- [ ] **Step 5: Open PR via tea**

```bash
tea pr create \
    --base develop/v3.12.0 \
    --head fix/v312-64f-recursive-cte \
    --title "fix(v312-64f / #4699): recursive CTE execution (two-table working/accumulated)" \
    --description "$(cat <<'EOF'
Closes #4699.

## Problem

`WITH RECURSIVE tree AS (anchor UNION ALL step) SELECT * FROM tree` accepted by
parser but executor returned `Recursive CTE not yet supported`.

## Fix

Two-table working/accumulated algorithm (PG/SQLite standard) in `src/engine_cte.rs`:

- `t` (accumulated): visible to outer SELECT.
- `t__work` (working set): visible to recursive step (after AST-level table-ref
  rewrite in `rewrite_step_table_refs`).
- Iterate up to MAX_RECURSION_DEPTH=1000; append new rows to `t`, replace
  `t__work` each round.
- UNION ALL appends all; UNION dedupes against accumulated.
- Safety cap MAX_RECURSION_ROWS=1_000_000.

## Files

- `src/engine_cte.rs`: Replaced 4-line rejection with full recursive logic.
  Added helpers: `decompose_recursive_body`, `derive_cte_columns`,
  `rewrite_step_table_refs`, `materialize_recursive_cte`. Extracted
  column-schema derivation from existing non-recursive path.
- `tests/integration/sql/repro_v312_64f.rs`: 9 integration tests.
- `tests/integration/sql/cte_materialization_test.rs`: Removed obsolete
  `test_cte_recursive_not_supported` (now subsumed).
- `Cargo.toml`: Registered repro_v312_64f test target.

## Out of Scope

- Writable CTE (#4692).
- MATERIALIZED VIEW.
- Recursive CTE in stored procedures (separate `ctx.cte_tables` path).
- Runtime-configurable MAX_RECURSION_DEPTH.

Spec: docs/superpowers/specs/2026-09-04-v312-64f-recursive-cte-design.md
Plan: docs/superpowers/plans/2026-09-04-v312-64f-recursive-cte.md
EOF
)" \
    --labels bug,v3.12.0,v312-64f
```

- [ ] **Step 6: Capture PR URL from output**

Save the PR URL output by `tea` for the final report.

- [ ] **Step 7: Final commit (if any review-fix commits were needed)**

If the PR review requires fixes, commit them as separate fix commits and force-push:
```bash
git push --force-with-lease
```

---

## Self-Review

**Spec coverage:**
- Algorithm (two-table, PG/SQLite standard): Task 5
- CTE body decomposition: Task 2
- Column schema derivation: Task 3
- AST rewriter (table + join + subquery): Task 4
- Hierarchy test (issue body): Task 5
- Count 1..N test: Task 6
- UNION dedup test: Task 6
- Empty anchor test: Task 7
- Non-UNION rejection test: Task 7
- Multiple CTEs mixed: Task 8
- Cleanup: Task 8
- MAX_RECURSION_ROWS exceeded: Task 9
- Aggregation in step: Task 10
- Existing test update: Task 11
- Verification + PR: Task 12

**Placeholder scan:** No "TBD" / "TODO" / "implement later" markers in critical steps. The "NOTE:" blocks are intentional adaptation guidance for the plan engineer (since the exact parser AST types are not 100% verified at plan time).

**Type consistency:**
- `decompose_recursive_body` returns `(Box<Statement>, Box<Statement>, bool)` — defined in Task 2, consumed in Task 5.
- `rewrite_step_table_refs` takes `&SelectStatement, &str, &str` — defined in Task 4, consumed in Task 5.
- `derive_cte_columns` takes `&CommonTableExpression, &[Vec<Value>], &[String]` — defined in Task 3, consumed in Task 5 (and called by `materialize_cte_tables` after extraction).
- `materialize_recursive_cte` returns `SqlResult<()>` — defined in Task 5, called from `materialize_cte_tables` Task 5.

**Adaptation points** (where the plan engineer may need to adjust based on actual code):
1. The exact `sqlrustgo_parser::parse(string)` entry point (or `Parser::new`).
2. The exact `Expression::Subquery` variant name.
3. The exact `SelectStatement` field names (`joins`, `where_clause`, `having_clause`, `columns`).
4. The exact `engine.storage` accessor (`engine.storage.write()` vs `engine.storage_mut()`).
5. The exact `Record::values` field name.
6. The exact `UnionStatement` left/right field types (`Box<Statement>` assumed).

Each NOTE block calls this out explicitly so the plan engineer can adapt without re-deriving.
