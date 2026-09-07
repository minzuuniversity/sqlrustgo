## Why

Issue #4809: `SELECT * FROM t INDEXED BY idx_name WHERE ...` 在
sqlrustgo 中 hint 被静默忽略 — parser 接受 INDEXED BY 子句但 executor
不应用 hint,继续使用全表扫描,plan 不变。

SQLite `INDEXED BY` 是一种 query planner override:强制使用指定索引,
不强制时如果索引不存在则报错。这是 diagnostic 工具(强制测试某个
index 是否 work)和 correctness 工具(防止 planner 在边缘 case 选错
plan)。

sqlrustgo 已经支持简单的 indexed scan(B+ tree index),所以 INDEXED BY
只需要把 executor 的 table scan 路径切换为 indexed lookup using the
given index。如果指定的 index 不存在,应该报错。

## What Changes

1. **AST 验证**: parser 已经解析 `TableReference { name: String,
   indexed_by: Option<String>, not_indexed: Option<()> }`(SQLite-style)。
     需要确认 executor 读这个字段。
2. **Executor 修改**: 在 `execute_select` 的 table resolution 路径:
     - 如果 `indexed_by` 是 `Some(idx_name)`:
       - 在 catalog 中查找索引 `idx_name`。
       - 如果索引不存在 → 报错。
       - 如果索引存在 → 把 table scan 路径替换为 indexed lookup
         using this index。
     - 如果 `not_indexed` 是 `Some(())`: 强制 sequential scan,不用
       index(even if planner 想要用)。
3. **错误处理**:
   - `INDEXED BY non_existent_index` → `SqlError::IndexNotFound`。
   - `INDEXED BY idx ON table_a` (table mismatch) → 报错。
4. **Compatibility**: PostgreSQL 没有 INDEXED BY,MySQL 用 `FORCE INDEX`
   / `IGNORE INDEX` (deferred)。SQLite 的 INDEXED BY 已经在
   `v312-90 #4625 PR #4788` 解析器层打通,executor 只需消费这个字段。

## Capabilities

### New Capabilities

- `executor-indexed-by-hint`: `INDEXED BY idx_name` MUST 强制 executor
  使用 `idx_name` 索引;若索引不存在,error 而非 silently fall through。

## Out of Scope

- `NOT INDEXED`: 同时实现 (SQLite 同一 token 的反义),与 INDEXED BY
  并列。
- `USE INDEX / FORCE INDEX` (MySQL-style): 推迟到 v3.14。
- `ANALYZE` 表收集 statistics 后优化 INDEXED BY 选择:不相关,INDEXED BY
  本来就是强制。
- 跨 schema INDEXED BY (`INDEXED BY db.idx`): 推迟到 v3.14。

## Verification

- 新测试 `tests/integration/sql/p3_indexed_by_4809_test.rs`:
  - `indexed_by_uses_named_index` — issue anchor:`SELECT * FROM t
    INDEXED BY idx_a WHERE key = ?` 走索引路径(通过 query log 或 row
    output 验证)。
  - `indexed_by_nonexistent_index_errors` — `INDEXED BY no_such_idx`
    报错。
  - `indexed_by_table_mismatch_errors` — `FROM t INDEXED BY idx_b`
    (idx_b 是其他表的索引)报错。
  - `indexed_by_with_composite_index` — 多列索引的等值查询。
  - `not_indexed_forces_sequential` — `FROM t NOT INDEXED` 即使有
    索引也用 seqscan(验证通过 instrumentation 或 row output 一致)。
  - `indexed_by_returns_same_rows` — INDEXED BY 与无 hint 返回相同
    结果(只是 plan 不同)。
  - `indexed_by_with_join` — `FROM t INDEXED BY idx JOIN ...` hint
    仅作用于 t。

Total: 7 tests。

- `cargo test --test p3_indexed_by_4809_test` 7/7 PASS。
- 回归: parser_e2e_test, indexed_by_* (v312-90 已有的相关测试)。

## Risks / Trade-offs

- **Index 存在性 check**: 必须在 executor 早期进行(在 scan loop 开始
  之前),否则会扫描部分数据然后报错,事务边界模糊。
- **Index 内容 check**: INDEXED BY 不要求索引覆盖查询的所有列;只要
  索引存在且至少一列匹配,就可以走索引路径(其余列做 row fetch)。
- **Plan instrumentation**: 当前 sqlrustgo 没有 EXPLAIN QUERY PLAN,
  所以无法直接验证 INDEXED BY 切换 plan。间接验证:在 indexing 和
  no-indexing 下,row output 一致,但执行时间/IO 模式不同(用 mock
  instrumented storage 验证)。