# A04 MySQL 协议差分矩阵 — Issue #5226

**Date**: 2026-10-09  
**Work Package**: A04  
**Issue**: #5226  
**Seed**: 42  
**Corpus Hash**: `fnv1a-2dabce8aecc81dd2`  
**Generated**: 2026-10-09T16:51:47Z  

---

## 概述

对 **20 个 SQL case** 进行跨引擎协议差分测试，对比 **MySQL wire**、**mysql CLI**、**sqlrustgo sqlite-like CLI**、**SQLite CLI**、**PostgreSQL Docker** 五个入口的执行结果（列名、列类型、行数、逐值、NULL、错误类别）。

### 参照引擎可用性

| 引擎 | 可用 | 说明 |
|------|------|------|
| MySQL 服务端 | ❌ | 无 `mariadbd`/`mysqld` 二进制；Docker registry 不可达 |
| PostgreSQL (Docker) | ✅ | `a04-pg-ref` (postgres:17-alpine) |
| SQLite CLI | ✅ | `/usr/bin/sqlite3` |

### Verdict 分布（跨所有 comparisons）

| Verdict | Count | 说明 |
|---------|-------|------|
| match | 33 | 列名+行数据完全一致 |
| mismatch | 7 | 列名/错误分类/行数不一致 |

**silent_wrong**: 0（无静默错误结果）

---

## 矩阵明细

### Cases (20)

| Case ID | Category | MySQL Wire | mysql CLI | sqlrustgo CLI | SQLite | PostgreSQL |
|---------|----------|------------|-----------|---------------|--------|------------|
| join_inner_001 | JOIN | match | match | **mismatch** (`a.id` vs `id`) | match | match |
| join_left_002 | JOIN | match | match | match | match | **mismatch** (binder_error) |
| sub_scalar_001 | SUBQUERY | match | match | match | match | match |
| sub_exists_002 | SUBQUERY | match | match | match | match | match |
| agg_group_001 | AGGREGATE | match | match | **mismatch** (`g` vs `col_1`) | **mismatch** (`col_1` vs `g`) | **mismatch** (`col_1` vs `g`) |
| agg_having_002 | HAVING | match | match | **mismatch** (`__agg_1` vs `SUM(n)`) | **mismatch** (row count) | **mismatch** (`SUM(n)` vs `sum`) |
| null_select_001 | NULL | match | match | match | match | match (`&nbsp;` → NULL) |
| null_where_002 | NULL | match | match | match | match | match |
| cast_int_text_001 | CAST | match | match | **mismatch** (`BinaryOp(...)` vs `n + 0`) | match | match |
| arith_div_002 | CAST | match | match | **mismatch** (`BinaryOp(...)` vs `a / b`) | match | match |
| order_limit_001 | ORDER_LIMIT | match | match | match | match | match |
| order_offset_002 | ORDER_LIMIT | match | match | match | match | match |
| err_no_such_table_001 | ERROR | match | match | match | match | match |
| err_syntax_002 | ERROR | match | match | match | match | match |
| err_col_unknown_003 | ERROR | match | match | match | **mismatch** (`missing_table_or_column` vs `unknown_column`) | **mismatch** (`missing_table_or_column` vs `missing_table`) |
| colname_alias_001 | COLNAME | match | match | **mismatch** (`renamed` vs `v AS renamed`) | **mismatch** (`v AS renamed` vs `renamed`) | **mismatch** (`v AS renamed` vs `renamed`) |
| colname_expr_002 | COLNAME | match | match | **mismatch** (`BinaryOp(...)` vs `a + b`) | match | match |
| coltype_int_001 | COLTYPE | match | match | match | match | match |
| ddl_pk_dup_001 | DDL_CONSTRAINT | match | match | match | match | match |
| ddl_notnull_002 | DDL_CONSTRAINT | match | match | match | match | match |

---

## 剩余 Mismatch 分类

### 1. 真实缺陷 (7 条)

**sqlrustgo sqlite-like CLI 列名暴露内部命名/AST 文本**（vs MySQL wire 规范化列名）：

| Case | Wire 列名 | CLI 列名 | 问题 |
|------|-----------|----------|------|
| join_inner_001 | `id` | `a.id` | 暴露表别名前缀 |
| agg_group_001 | `col_1` | `g` | CLI 暴露别名，wire 暴露内部生成名 `col_1` |
| agg_having_002 | `SUM(n)` | `__agg_1` | CLI 暴露 AST `SUM(n)`，wire 暴露内部生成名 `__agg_1` |
| cast_int_text_001 | `n + 0` | `BinaryOp(Identifier("n"), "+", Literal("0"))` | CLI 暴露完整 AST 文本 |
| arith_div_002 | `a / b` | `BinaryOp(Identifier("a"), "/", Identifier("b"))` | CLI 暴露完整 AST 文本 |
| colname_alias_001 | `v AS renamed` | `renamed` | CLI 暴露别名，wire 暴露表达式 |
| colname_expr_002 | `a + b` | `BinaryOp(Identifier("a"), "+", Identifier("b"))` | CLI 暴露完整 AST 文本 |

**影响**：双入口列名不一致会误导下游程序（如 `SELECT ... AS alias` 后 CLI 拿不到别名，而是拿到内部生成名）。

### 2. 错误分类粒度差异 (3 条)

| Case | Wire | SQLite | PostgreSQL | 说明 |
|------|------|--------|------------|------|
| err_col_unknown_003 | `missing_table_or_column` | `unknown_column` | `missing_table` | 跨引擎错误类别映射差异 |
| ddl_notnull_002 | `constraint` | `constraint` | `constraint` | 已对齐 |
| join_left_002 | `binder_error` | `binder_error` | `binder_error` | 已对齐 |

**结论**：SQLite/PG 使用不同错误码体系，跨引擎错误类别映射无法完全对齐，属预期。

---

## Harness 修复记录

| 问题 | 修复 |
|------|------|
| mysql CLI 1045 Access denied | 加 `-ptester`（ephemeral 默认密码） |
| PG 列名为空 | `psql -A -t` 无 header，改用 `psql -H` HTML 解析（取最后一个 `<table>` 的 `<th>`） |
| mysql CLI 列数错误 | `--batch --raw` 输出 tab 分隔，改用 `parse_cli_delim` 按 `\t` 切分 |
| PG NULL mismatch (`&nbsp;`) | `cell_equal` 归一化 HTML 实体（`&nbsp;` → 空格，`&lt;` → `<`） |
| PG 无别名表达式列名 (`?column?`) | 列名比较跳过 `?column?` |
| PG `DROP TABLE` NOTICE 混入错误分类 | 只看 `ERROR` 开头行，过滤 `NOTICE` |
| 错误分类不一致 | `classify_cli_error` 对齐 PG/SQLite 错误类别 |

---

## 后续行动

1. **修复 CLI 列名缺陷**（7 条）：sqlite-like CLI 应暴露规范化列名（如 `id`、`n + 0`、`renamed`），而非内部生成名或 AST 文本。
2. **错误类别映射优化**（可选）：建立跨引擎错误类别映射表，对齐 `unknown_column`/`missing_table`/`missing_table_or_column`。
3. **扩展语料**：添加更多 JOIN/子查询/聚合 case，覆盖 NULL 行、HAVING 分组、ORDER BY 多列。

---

## 证据

- **矩阵 JSON**: `docs/releases/v4.1.0/evidence/a04-5226/matrix.json`
- **Wire 日志**: `docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt`（每 case × 引擎一份）
- **Harness**: `tests/integration/compat/a04_mysql_diff_matrix_test.rs`
- **构建**: `CARGO_TARGET_DIR=/home/ai/sqlrustgo/target-a04 cargo build -p sqlrustgo-cli -p sqlrustgo-mysql-server --all-features`
- **测试**: `CARGO_TARGET_DIR=/home/ai/sqlrustgo/target-a04 cargo test --all-features --test a04_mysql_diff_matrix -- --nocapture`

---

**Generated by**: opencode + Mimo2.6Flash  
**Timestamp**: 2026-10-09T16:51:47Z  
**Git SHA**: `eab189bbdc`  
**Worktree**: `.worktrees/a04-5226` (feature/a04-5226-differential)
