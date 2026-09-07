## Why

Issue #4815: `CREATE TABLE t (id INT, age INT, CHECK (age >= 0))` 在
sqlrustgo 中约束静默失效 — parser 接受 CHECK 子句,但 INSERT 负数
时 executor 不报错,与 SQL 标准 (PostgreSQL/MySQL/SQLite) 行为不一致。

这是"假成功" pattern 的另一形态,违反数据完整性约束。常见后果:
- 应用层期待 DB 阻止无效数据(负年龄、超出范围的值),但实际写入
  了脏数据。
- ETL pipelines 依赖 CHECK 约束做数据校验,失效后污染下游分析。
- 多层防御(应用校验 + DB 校验)退化为单层,可靠性下降。

根因 (issue 描述): parser 把 `CHECK (expr)` 存到 `CreateTable.column_constraints`
或独立的 `table_constraints` 字段,但 executor 在 INSERT 时没遍历约束
并执行 evaluate;或者 evaluate 了但不报错。

## What Changes

1. **AST 验证**: `CHECK (expr)` 应该存储为 `TableConstraint::Check(Expr)`,
     或 `ColumnConstraint::Check(Expr)`。验证 parser 已经存到正确的
     variant。
2. **Catalog 持久化**: 在 `TableInfo` / catalog 中保存 CHECK expressions,
     与 columns 平级。
3. **Executor 校验**:
   - `execute_insert` 在写入每行前,**evaluate 每个 CHECK expr** (用
     incoming row 作为 evaluation context),任何一个 fail 立即返回错误。
   - `execute_update` 在写回每行前同样 evaluate。
4. **Constraint name support**: `CONSTRAINT name CHECK (expr)` 的 name
     可选保留(用于错误消息)。

## Capabilities

### New Capabilities

- `executor-check-constraint-validation`: `CREATE TABLE ... CHECK
  (expr)` MUST 在 INSERT/UPDATE 时 evaluate `expr`;若为 false,error
  而非 silently insert。

### Modified Capabilities

- `parser-create-table-constraints`: parser 接受 CHECK 已经正确,只需
  验证 storage。
- `storage-table-info`: 持久化 CHECK exprs 到 catalog metadata。

## Out of Scope

- **DOMAIN / custom types**: 推迟到 v3.14。
- **Deferrable / not deferrable constraints** (PostgreSQL): 推迟到
  v3.14。
- **CHECK 在 INSERT INTO ... SELECT 中 row-by-row evaluation**: 本 PR
  实现,但 row-by-row 错误 message 需要明确指出 conflict row id。
- **NOT VALID + VALIDATE CONSTRAINT**: 推迟到 v3.14 (PostgreSQL extension)。

## Verification

- 新测试 `tests/integration/sql/p3_check_constraint_4815_test.rs`:
  - `check_constraint_blocks_negative_age` — issue anchor:
    `INSERT INTO t (age=-1)` 必须失败。
  - `check_constraint_allows_valid_age` — `INSERT (age=10)` 成功。
  - `check_constraint_multiple_constraints` — 多个 CHECK 都 evaluate。
  - `check_constraint_named_constraint` — `CONSTRAINT age_check CHECK
    (age >= 0)`,错误消息含 name。
  - `check_constraint_with_expression` — `CHECK (price > cost)` 引用
    多列。
  - `check_constraint_blocks_update` — UPDATE 也 evaluate,不能把 age
    改成 -1。
  - `check_constraint_complex_expression` — `CHECK (status IN ('a','b','c'))`
    集合约束。
  - `check_constraint_with_subquery` — 推迟验证(本 PR out of scope)。

Total: 7 tests。

- `cargo test --test p3_check_constraint_4815_test` 7/7 PASS。
- 回归: parser_e2e_test, cte_materialization_test, reproduce_v313_99_4692_writable_cte。

## Risks / Trade-offs

- **Evaluation cost**: 每行 INSERT/UPDATE 都要 evaluate 所有 CHECK expr,
  对大表 bulk INSERT 是 O(rows × constraints) 开销。短期可接受。
- **Error message clarity**: 需要包含 (a) 哪个约束, (b) 哪一行,
  (c) 实际值是什么。SQL 标准要求 "CHECK constraint violated" + 可选
  constraint name。
- **NOT NULL vs CHECK**: NOT NULL 是独立 constraint,不要混入 CHECK 路径。
- **DEFAULT 值的 interaction**: DEFAULT 后再 evaluate CHECK,如果 DEFAULT
  违反 CHECK,这是 definition error (DBA 错误),可在 CREATE TABLE 时
  一次性检查。