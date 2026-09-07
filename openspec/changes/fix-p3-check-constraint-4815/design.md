## Context

Issue #4815 — `CREATE TABLE ... CHECK (expr)` 在 executor 层静默失效。
Parser 接受 SQL,AST 中 `TableConstraint::Check { name: Option<String>,
expr: Expr }` 或 `ColumnConstraint::Check { name: Option<String>,
expr: Expr }` 已经存储。

Bug 推测路径:
1. Parser 把 CHECK 存在 AST,但 `CreateTable` 序列化到 catalog metadata
   (FileStorage) 时丢失了 CHECK exprs。
2. 即使保存,在 INSERT/UPDATE 时,executor 没遍历 CHECK list。
3. 或者遍历了但 evaluate 出错被 silently swallowed。

根本修复:**executor 在 INSERT/UPDATE 的 row write path 中,必须对每行
evaluate 所有 CHECK expr,任何 false → 返回 ConstraintViolation 错误**。

## Approach

### A1. AST + Catalog 持久化

```rust
// crates/parser/src/ast.rs (已存在,只需验证)
pub enum TableConstraint {
    PrimaryKey { columns: Vec<String> },
    Unique { columns: Vec<String> },
    ForeignKey { ... },
    Check { name: Option<String>, expr: Expr },  // 已存在
}

// TableInfo (storage 层)
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub check_constraints: Vec<CheckConstraint>,  // NEW
}

pub struct CheckConstraint {
    pub name: Option<String>,
    pub expr: Expr,
}
```

### A2. INSERT 路径 evaluate

```rust
// src/executor/dml.rs execute_insert
for row in incoming_rows {
    for check in &table_info.check_constraints {
        let eval_ctx = EvalContext::for_row(&row, &table_info.columns);
        let result = evaluate_expression(&check.expr, &eval_ctx)?;
        if !result.to_bool() {
            return Err(SqlError::ConstraintViolation {
                constraint: check.name.clone().unwrap_or_else(|| "<unnamed>".to_string()),
                row: format!("{:?}", row),
            });
        }
    }
    storage.insert_row(table_name, &row)?;
}
```

### A3. UPDATE 路径 evaluate

```rust
// src/executor/dml.rs execute_update
for row in updated_rows {
    for check in &table_info.check_constraints {
        let eval_ctx = EvalContext::for_row(&row, &table_info.columns);
        let result = evaluate_expression(&check.expr, &eval_ctx)?;
        if !result.to_bool() {
            return Err(SqlError::ConstraintViolation {
                constraint: check.name.clone().unwrap_or_else(|| "<unnamed>".to_string()),
                row: format!("{:?}", row),
            });
        }
    }
    storage.update_row(table_name, &row.primary_key(), &row)?;
}
```

### A4. Error type

新增 `SqlError::ConstraintViolation { constraint: String, row: String }`。
错误消息格式:`CHECK constraint '<name>' violated: row <pk>=<val>`。

### A5. CREATE TABLE 时 default value CHECK

在 CREATE TABLE 完成后,如果有 DEFAULT 且 DEFAULT 违反 CHECK,记录 warning
(不阻塞 CREATE,因为 DDL 已经过 dba 显式执行)。这与 PostgreSQL 行为一致。

## Files Changed

| File | Lines | Purpose |
|------|-------|---------|
| `crates/parser/src/ast.rs` | +30 | 验证 CheckConstraint struct |
| `src/storage/metadata.rs` | +50 | TableInfo 加 check_constraints 字段 |
| `src/executor/dml.rs` | +80 | INSERT/UPDATE CHECK evaluate |
| `src/execution_engine.rs` | +30 | CREATE TABLE 路径把 CHECK exprs 写入 catalog |
| `src/error.rs` | +20 | 新增 ConstraintViolation variant |
| `tests/integration/sql/p3_check_constraint_4815_test.rs` | +180 | 7 tests |

Total: ~390 lines, ~6 files touched.

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_check_constraint_4815_test` | 7/7 PASS |
| `repro_v313_99_4692_writable_cte` | 6/6 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS |
| `cte_materialization_test` | 9/9 PASS |
| `cargo test --all-features --lib` | no regression |

## Non-Goals

- DOMAIN types
- Deferrable constraints
- NOT VALID / VALIDATE CONSTRAINT
- Cross-row CHECK (`CHECK (cnt <= (SELECT MAX(cnt) FROM t))`) — partial
  deferred

## Difficulty Tier

**🟡 MEDIUM** — 单点 evaluate logic + storage metadata 持久化,但需要
INSERT/UPDATE 都加 hook,涉及 transaction visibility + storage layer
schema 升级。