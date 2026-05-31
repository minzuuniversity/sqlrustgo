# PR-800 TEST_PLAN — COM_QUERY AST Routing

> **PR Number**: PR-800  
> **PR Title**: COM_QUERY AST Routing  
> **Version**: v3.8.0 Phase 0 Architecture Freeze  
> **Branch**: `develop/v3.8.0` (SHA: `745f24f1`)  
> **Auditor**: Hermes Agent  
> **Created**: 2026-05-30  
> **Status**: DRAFT — For Review  

---

## 1. 测试策略

### 1.1 三层验证体系

PR-800 使用三层验证体系，每层对应特定风险：

| Layer | 验证目标 | 门禁 | 测试类型 |
|-------|----------|------|----------|
| **L1** | Parser 正确生成 AST | SELECT/INSERT/UPDATE/DELETE 生成正确 AST | Unit Test |
| **L2** | Planner 正确生成 PhysicalPlan | AST → PhysicalPlan 映射正确 | Unit Test |
| **L3** | 端到端 SQL 执行结果一致 | mysql-server vs bench-cli 结果一致 | E2E Test |

### 1.2 PR-800 测试原则

1. **不测旧路径**: PR-800 删除 `eng.execute(raw_sql)`，不测它
2. **测新路径**: 所有 SQL 都经过 Parser → Planner → LocalExecutor
3. **向后兼容**: `eng.execute(sql)` API 调用实际路由到新路径（但不测旧 API）

---

## 2. L1 — Parser Unit Tests

### 2.1 测试目标

验证 Parser::parse() 为每种 SQL 类型生成正确的 AST。

### 2.2 测试用例

#### SELECT

```rust
#[test]
fn test_pr800_select_parsing() {
    let sql = "SELECT id, name FROM users WHERE age > 18";
    let ast = parser::parse(sql).unwrap();
    
    match ast {
        Statement::Select(select) => {
            assert_eq!(select.columns.len(), 2);
            assert_eq!(select.table_name, "users");
            assert!(select.where_clause.is_some());
        }
        _ => panic!("Expected SELECT statement"),
    }
}
```

#### INSERT

```rust
#[test]
fn test_pr800_insert_parsing() {
    let sql = "INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30)";
    let ast = parser::parse(sql).unwrap();
    
    match ast {
        Statement::Insert(insert) => {
            assert_eq!(insert.table_name, "users");
            assert_eq!(insert.columns.len(), 3);
            assert_eq!(insert.values.len(), 3);
        }
        _ => panic!("Expected INSERT statement"),
    }
}
```

#### UPDATE

```rust
#[test]
fn test_pr800_update_parsing() {
    let sql = "UPDATE users SET age = 31 WHERE id = 1";
    let ast = parser::parse(sql).unwrap();
    
    match ast {
        Statement::Update(update) => {
            assert_eq!(update.table_name, "users");
            assert!(update.where_clause.is_some());
        }
        _ => panic!("Expected UPDATE statement"),
    }
}
```

#### DELETE

```rust
#[test]
fn test_pr800_delete_parsing() {
    let sql = "DELETE FROM users WHERE id = 1";
    let ast = parser::parse(sql).unwrap();
    
    match ast {
        Statement::Delete(delete) => {
            assert_eq!(delete.table_name, "users");
            assert!(delete.where_clause.is_some());
        }
        _ => panic!("Expected DELETE statement"),
    }
}
```

#### CREATE/DROP TABLE

```rust
#[test]
fn test_pr800_ddl_parsing() {
    let create_sql = "CREATE TABLE t1 (id INT PRIMARY KEY, name VARCHAR(255))";
    let ast_create = parser::parse(create_sql).unwrap();
    assert!(matches!(ast_create, Statement::CreateTable(_)));
    
    let drop_sql = "DROP TABLE t1";
    let ast_drop = parser::parse(drop_sql).unwrap();
    assert!(matches!(ast_drop, Statement::DropTable(_)));
}
```

### 2.3 覆盖要求

| SQL 类型 | 最小测试数 | 当前状态 |
|---------|-----------|----------|
| SELECT | 10 | ✅ 已有基础 |
| INSERT | 5 | ✅ 已有基础 |
| UPDATE | 5 | ✅ 已有基础 |
| DELETE | 5 | ✅ 已有基础 |
| CREATE/DROP | 3 | ✅ 已有基础 |
| 其他 (TRANSACTION等) | 5 | ⚠️ 待补充 |

---

## 3. L2 — Planner Unit Tests

### 3.1 测试目标

验证 Planner::plan() 为每种 AST 类型生成正确的 PhysicalPlan。

### 3.2 测试用例

#### SELECT → PhysicalPlan

```rust
#[test]
fn test_pr800_select_planning() {
    let sql = "SELECT id, name FROM users WHERE age > 18";
    let ast = parser::parse(sql).unwrap();
    
    let plan = planner::plan(ast).unwrap();
    
    match plan {
        PhysicalPlan::SelectScan(scan) => {
            assert_eq!(scan.table_name, "users");
            assert!(scan.projection.len() >= 2);
            assert!(scan.filter.is_some());
        }
        _ => panic!("Expected SelectScan PhysicalPlan"),
    }
}
```

#### INSERT → PhysicalPlan

```rust
#[test]
fn test_pr800_insert_planning() {
    let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
    let ast = parser::parse(sql).unwrap();
    
    let plan = planner::plan(ast).unwrap();
    
    match plan {
        PhysicalPlan::Insert(insert) => {
            assert_eq!(insert.table_name, "users");
        }
        _ => panic!("Expected Insert PhysicalPlan"),
    }
}
```

#### UPDATE → PhysicalPlan

```rust
#[test]
fn test_pr800_update_planning() {
    let sql = "UPDATE users SET age = 31 WHERE id = 1";
    let ast = parser::parse(sql).unwrap();
    
    let plan = planner::plan(ast).unwrap();
    
    match plan {
        PhysicalPlan::Update(update) => {
            assert_eq!(update.table_name, "users");
        }
        _ => panic!("Expected Update PhysicalPlan"),
    }
}
```

### 3.3 覆盖要求

| AST → PhysicalPlan 映射 | 最小测试数 | 当前状态 |
|------------------------|-----------|----------|
| Select → SelectScan | 5 | ✅ 已有 |
| Insert → Insert | 3 | ✅ 已有 |
| Update → Update | 3 | ⚠️ 待补充 |
| Delete → Delete | 3 | ⚠️ 待补充 |

---

## 4. L3 — E2E Integration Tests

### 4.1 测试目标

验证端到端 SQL 执行结果在 PR-800 前后一致。

### 4.2 测试方法

```bash
# E2E 测试：mysql-server vs bench-cli 结果对比
sql_corpus --format=json --filter=basic_queries | while read sql; do
    result_server=$(mysql_server.execute "$sql")
    result_planner=$(planner_route.execute "$sql")
    diff <(echo "$result_server") <(echo "$result_planner") || echo "DIFF: $sql"
done
```

### 4.3 测试用例（28 E2E test files）

| Test File | SQL 类型 | 验证点 |
|-----------|----------|--------|
| `e2e_select_test.rs` | SELECT | 结果一致性 |
| `e2e_insert_test.rs` | INSERT | 数据持久化 |
| `e2e_update_test.rs` | UPDATE | 数据更新 |
| `e2e_delete_test.rs` | DELETE | 数据删除 |
| `e2e_transaction_test.rs` | BEGIN/COMMIT | 事务语义 |
| `e2e_join_test.rs` | JOIN | 多表查询 |
| `e2e_window_test.rs` | Window Functions | 窗口函数 |
| ... | ... | ... |

### 4.4 TPC-H SF=1 回归测试

```bash
# PR-800 后，TPC-H Q1-Q22 回归必须 < 5%
cargo test -p sqlrustgo-benchmark --test tpch_sf1
# 期望: 22/22 PASS
```

---

## 5. 特殊测试用例

### 5.1 空字符串处理

```rust
#[test]
fn test_pr800_empty_sql() {
    let result = parser::parse("");
    assert!(result.is_err());
}
```

### 5.2 语法错误处理

```rust
#[test]
fn test_pr800_syntax_error() {
    let sql = "SELEC * FORM users";  // 故意写错
    let result = parser::parse(sql);
    assert!(result.is_err());
}
```

### 5.3 多语句处理

```rust
#[test]
fn test_pr800_multi_statement() {
    let sql = "SELECT 1; SELECT 2";
    let result = parser::parse(sql);
    // 应该支持多语句
    assert!(result.is_ok());
}
```

### 5.4 参数化查询

```rust
#[test]
fn test_pr800_prepared_statement() {
    let sql = "INSERT INTO users (id, name) VALUES (?, ?)";
    let ast = parser::parse(sql).unwrap();
    // 应该正确解析 ?
    assert!(matches!(ast, Statement::Insert(_)));
}
```

---

## 6. 测试通过标准

### 6.1 PR-800 测试标准

| 测试类型 | 通过标准 | Gate ID |
|---------|----------|---------|
| L1 Parser Unit Tests | 100% AST node coverage | A2 |
| L2 Planner Unit Tests | 95% plan coverage | A2 |
| L3 E2E Tests | 28/28 files PASS | C2 |
| TPC-H SF=1 | 22/22 queries PASS | C3 |
| Backward Compatibility | 所有现有 tests 仍然 PASS | A2 |

### 6.2 不测试内容

以下内容不在 PR-800 测试范围内：

| 不测试 | 原因 | 后续 PR |
|--------|------|----------|
| ParallelVolcanoExecutor | PR-870 才接入 | PR-870 |
| WAL Integration | PR-830/840 才接入 | PR-830 |
| MVCC | PR-890 才实现 | PR-890 |
| VTU Predicate | PR-880 才统一 | PR-880 |

---

## 7. 测试执行计划

### 7.1 执行顺序

```
1. L1 Parser Unit Tests (本地 Mac)
   cargo test -p sqlrustgo-parser --lib

2. L2 Planner Unit Tests (本地 Mac)
   cargo test -p sqlrustgo-planner --lib

3. L3 E2E Tests (Z6G4 服务器)
   cargo test -p sqlrustgo-e2e --all-features

4. TPC-H SF=1 (Z6G4 服务器)
   cargo test -p sqlrustgo-benchmark --test tpch_sf1
```

### 7.2 回归基线

建立 PR-800 执行前的回归基线：

| 测试 | 当前 PASS 率 | PR-800 后 |
|------|-------------|-----------|
| sqlrustgo-parser --lib | 100% | 必须 ≥100% |
| sqlrustgo-planner --lib | 95% | 必须 ≥95% |
| sqlrustgo-executor --lib | 90% | 必须 ≥90% |
| mysql-server E2E | 28/28 | 必须 28/28 |

---

## 8. SSOT 引用

- `docs/releases/v3.8.0/PR-800_SPEC.md` — PR-800 规格说明
- `docs/releases/v3.8.0/PR-800_ACCEPTANCE.md` — PR-800 验收标准
- `docs/releases/v3.8.0/TEST_PLAN.md` — v3.8.0 测试计划
- `crates/sqlrustgo-parser/src/` — Parser 源代码
- `crates/sqlrustgo-planner/src/` — Planner 源代码