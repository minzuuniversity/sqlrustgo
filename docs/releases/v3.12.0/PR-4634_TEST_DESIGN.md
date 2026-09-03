# PR-4634 TEST DESIGN

> **PR**: PR-4634 `fix(v312-62 / #4610..#4623): issue batch — executor / parser / CLI bug fixes`
> **Created**: 2026-09-02
> **Source TEST_PLAN**: `PR-4634_TEST_PLAN.md`

---

## 1. 设计原则

- **三权分立**：本文件由实现者编写；TEST_REVIEW 由独立审核者编写；ACCEPTANCE 由门禁执行者编写
- **覆盖度优先**：每条 ISSUE 对应 ≥1 测试用例
- **独立性**：所有测试不依赖网络、文件系统（除 tempdir）、外部 DB
- **可重现**：单 crate 内单测 ≤5 分钟跑完
- **真实可执行**：`cargo test -p <crate> --test <name>` 100% PASS

---

## 2. 测试用例设计

### 2.1 #4610 — `parse_lit` Float precision

**被测函数**：`parse_lit(s: &str) -> Value`

| 用例 | 输入 | 期望输出 |
|---|---|---|
| 整数字面量 | `"55"` | `Value::Integer(55)` |
| 浮点字面量 | `"55.0"` | `Value::Float(55.0)` |
| 浮点字面量 | `"3.14"` | `Value::Float(3.14)` |
| 浮点字面量 | `"82.5"` | `Value::Float(82.5)` |
| 负数 | `"-3.14"` | `Value::Float(-3.14)` |
| 0 边界 | `"0.0"` | `Value::Float(0.0)` |
| Float 算术 | `eval("SELECT 20 + 35.0")` | `Value::Float(55.0)` |

**覆盖矩阵**：整数 / 浮点 / 负数 / 0 / 算术混合

### 2.2 #4611 — `LENGTH` codepoint 计数

**被测函数**：`eval_fn("LENGTH", &[Value::Text(s)])`

| 用例 | 输入 | 期望输出 |
|---|---|---|
| ASCII 字符串 | `"hello"` | `5` |
| 中文字符串 | `"电子技术"` | `4` |
| 混合字符串 | `"a电子b"` | `4` |
| 空字符串 | `""` | `0` |
| 4-byte emoji | `"👨‍👩‍👧"` | `1`（grapheme cluster, codepoint 视实现）|

**覆盖矩阵**：ASCII / 中日韩 / emoji / 空 / 混合

### 2.3 #4612 — `compare_values` BINARY collation

**被测函数**：`compare_values(Value::Text, Value::Text)`

| 用例 | 输入 | 期望输出 |
|---|---|---|
| 相同字符串 | `("c05103", "c05103")` | `0` |
| 不相同 | `("a", "b")` | `< 0` |
| 末尾空格差异 | `("c05103", "c05103   ")` | `!= 0` (#4612) |
| 前置空格差异 | `("c05103", "  c05103")` | `!= 0` |
| 大小写差异 | `("abc", "ABC")` | `!= 0` |

**覆盖矩阵**：equal / less / trailing space / leading space / case

### 2.4 #4613 — `ROUND` 类型保留

| 用例 | 输入 | 期望输出 |
|---|---|---|
| ROUND(整数, 2) | `eval("ROUND(55, 2)")` | `Integer(55)` |
| ROUND(浮点, 2) | `eval("ROUND(70 * 0.5, 2)")` | `Float(35.0)` |
| ROUND(浮点, 0) | `eval("ROUND(3.5, 0)")` | `Integer(4)` 或 `Float(4.0)` (per MySQL) |
| ROUND(负数, 1) | `eval("ROUND(-3.14, 1)")` | `Float(-3.1)` |

### 2.5 #4618 — `ROLLBACK TO <name>` 简写

**被测函数**：`parse(sql)`

| 用例 | 输入 | 期望输出 |
|---|---|---|
| SQLite 简写 | `"ROLLBACK TO sp1;"` | `Ok(SavepointStatement { name: "sp1", op: RollbackTo })` |
| MySQL 标准 | `"ROLLBACK TO SAVEPOINT sp1;"` | `Ok(SavepointStatement { name: "sp1", op: RollbackTo })` |
| 缺名字 | `"ROLLBACK TO;"` | `Err` |
| RELEASE 形式 | `"RELEASE SAVEPOINT sp1;"` | `Ok(... op: Release)` |

### 2.6 #4619 — 事务内错误自动 rollback

**被测路径**：`SqliteMode::dispatch_one()`

| 用例 | 输入脚本 | 期望结果 |
|---|---|---|
| 事务内 PK 冲突 | `BEGIN; INSERT 2,200; INSERT 1,999; COMMIT;` | 2 条语句后 ROLLBACK 自动触发，exit 1，DB 无新行 |
| 正常事务 | `BEGIN; INSERT 1,100; COMMIT;` | 事务正常结束，DB 有新行 |
| 无事务错误 | `INSERT bad_sql` | 立即报错，DB 无变化 |

### 2.7 #4620 — ALTER TABLE 拒绝

| 用例 | 输入 | 期望输出 |
|---|---|---|
| MODIFY | `"ALTER TABLE t MODIFY phone varchar(20);"` | `Err("ALTER TABLE ... MODIFY not supported ...")` |
| ADD CONSTRAINT UNIQUE | `"ALTER TABLE t ADD CONSTRAINT uq UNIQUE(phone);"` | `Err("ALTER TABLE ... ADD CONSTRAINT not supported ...")` |
| ADD COLUMN（合法）| `"ALTER TABLE t ADD COLUMN c INT;"` | `Ok(...)` |

### 2.8 #4622 — 嵌套 CTE schema

| 用例 | 输入 SQL | 期望结果 |
|---|---|---|
| 嵌套 CTE 显式列 | `WITH a(id, val) AS (...), b AS (SELECT * FROM a WHERE val < 25) SELECT * FROM b;` | 正常返回 |
| 嵌套 CTE `SELECT *` 投影 | `WITH a AS (SELECT id, val FROM t), b AS (SELECT * FROM a WHERE val < 25) SELECT * FROM b;` | 当前实现依赖 `lookup_table_columns`；val 列能解析 |
| 三层嵌套 | `WITH a AS (...), b AS (...), c AS (SELECT * FROM b) SELECT * FROM c;` | 正常返回 |

### 2.9 #4623 — `GROUP_CONCAT` 不泄漏 sentinel

**被测函数**：`group_concat(args)`

| 用例 | 输入 args | 期望输出 |
|---|---|---|
| 无 DISTINCT | `["__NO_DISTINCT__", 10, 20]` | `"10,20"` |
| DISTINCT | `["__DISTINCT__", 10, 10, 20]` | `"10,20"` |
| ORDER BY | `["__NO_DISTINCT__", 10, "__ORDER_BY__", Identifier, "__ASC__"]` | `"10"` |
| SEPARATOR | `["__NO_DISTINCT__", 10, 20, "__SEPARATOR__", "-"]` | `"10-20"`（注：当前实现 separator 写死为 ','，SEPARATOR 子句尚未实现）|

---

## 3. Mock 与 Fixture 策略

- **不依赖外部 DB**：所有测试用 in-memory 存储
- **不依赖网络**：所有 CLI 测试用 stdin pipe 或 tempfile
- **不依赖全局状态**：每个测试新建 `SqliteMode` / `ProcedureContext`
- **资源清理**：tempfile 用 `tempfile::TempDir` 自动 drop

---

## 4. 性能与稳定性

| 维度 | 要求 |
|---|---|
| 内存安全 | 8GB CI 限制内（无大数据集测试） |
| 无 O(n²) | 本批修复均不涉及大数据扫描 |
| 无锁泄漏 | 多线程测试不涉及 |
| 无 race condition | 不涉及并发执行 |

---

## 5. 失败诊断

每个断言失败时输出：
- 测试名（自解释）
- 输入 SQL / Value
- 期望 vs 实际 Value
- 文件:行号

示例：
```
test_parse_lit_preserves_float: expected Value::Float(55.0), got Value::Integer(55)
  at crates/executor/src/expr/mod.rs:934
```

---

## 6. 已知限制（本次 PR 内不修复）

1. **#4623 GROUP BY 聚合**：本次修复仅阻止 sentinel 泄漏，
   `GROUP_CONCAT` 仍以逐行方式求值（无聚合）。需 #4621 配套修复
2. **#4622 SELECT \* 投影**：嵌套 CTE 引用 `SELECT *` 投影时，
   当前实现依赖 storage 层推断 schema；若 storage 缺失则退化

---

## 7. 实施优先级

| 优先级 | 范围 |
|---|---|
| P0（必跑） | #4610, #4611, #4612, #4613, #4618, #4620, #4623 单测 |
| P1（必跑） | #4619, #4622 集成测试 |
| P2（回归）| TPC-H Q1/Q14 + BustubX-EDU baseline |
