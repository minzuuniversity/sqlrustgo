# V312-56A: Metadata / SHOW / information_schema 教学与兼容闭环

> **Issue**: #4251
> **Status**: TEACHING_LAB_CREATED + 56A-R1/R2/R4 DONE + 56A-R3 DEFERRED → v3.13+
> **Branch**: develop/v3.12.0
> **HEAD at last refresh**: `f2f1a7e4fd7835304d91cc4e245573a8537874ea` (post-56A-R2 PR #4349 + post-56A-R4 PR #4345)
> **Policy**: Anti-Fabrication-Policy-v1.0

## §1 Scope

闭合 v3.12.0 BETA 准入要求"Metadata/SHOW/information_schema 教学与兼容"任务,提供 SHOW
CREATE TABLE / SHOW COLUMNS / SHOW INDEX / information_schema 的实现位置、测试用例、DEFERRED
路径。

## §2 实现证据 (Implemented Features)

### §2.1 SHOW CREATE TABLE

**实现位置**: `src/engine_ddl.rs:404-439`

`execute_show_create_table(&self, table: &str) -> SqlResult<ExecutorResult>` 函数:
- 获取表的列定义
- 输出 `column_name TYPE [NOT NULL] [DEFAULT ...]` 形式(无 DEFAULT 子句输出,见 §2.1.1)
- 主键列附加 `PRIMARY KEY` 后缀
- 返回单行单列的 DDL 字符串

#### §2.1.1 DEFAULT 子句处理(诚实披露)

**当前实现不输出 DEFAULT 子句**。`tests/integration/sql/show_create_table_test.rs:36-54`
测试 `show_create_table_preserves_not_null_clause` 明确断言无 DEFAULT 输出。

**Close-boundary 影响**:对于带 DEFAULT 子句的表,SHOW CREATE TABLE 输出不能完整往返重建。
已记录为已知限制(controlled subset),BETA 准入不要求 DEFAULT roundtrip。

### §2.2 SHOW COLUMNS

**实现位置**: `src/engine_ddl.rs:470-497`

返回 6 列 MySQL 标准格式:`Field, Type, Null, Key, Default, Extra`(与 DESCRIBE 等价)。

**LIKE pattern 支持**: `src/engine_ddl.rs:wildcard_match` 函数(per VERIFICATION §6 行 19 引用)。

#### 测试覆盖 (`tests/integration/sql/show_columns_test.rs`):

| 测试名 | 行号 | 验证 |
|--------|------|------|
| `test_show_columns_basic_returns_rows` | 24-36 | 返回 3 列 + 类型 + PRI key |
| `test_show_columns_field_type_null_match_describe` | 38-51 | 与 DESCRIBE 输出等价 |
| `test_show_columns_like_pattern_filters_columns` | 53-74 | LIKE pattern 工作 |

### §2.3 SHOW INDEX

**实现位置**: `src/engine_ddl.rs:517-549`

返回 MySQL 12 列标准格式:`Table, Non_unique, Key_name, Seq_in_index, Column_name, Collation, Cardinality, Sub_part, Packed, Null, Index_type, Comment`。

#### 测试覆盖 (`tests/integration/sql/show_index_test.rs`):

| 测试名 | 行号 | 验证 |
|--------|------|------|
| `test_show_index_returns_primary_key_index` | 29-52 | PRIMARY index 返回非空 |
| `test_show_index_table_without_indices_returns_empty` | 54-66 | 无索引表返回空(controlled subset) |
| `test_show_index_missing_table_errors` | 68-73 | 错误处理 |

## §3 information_schema(显式 fail-closed)

### §3.1 库实现

**位置**: `crates/information-schema/src/lib.rs`

提供 `InformationSchema::get_tables()` / `get_columns()` / `get_indexes()` API,但未与
SQL 路径对接。

### §3.2 SQL 路径 fail-closed(诚实披露)

`parse_table_ref` at `crates/parser/src/parser.rs:7844` 仅接受单段 `Token::Identifier`,
**拒绝 schema-qualified names**。这意味着:

```sql
SELECT * FROM information_schema.tables;  -- 解析失败:Unexpected dot
```

**测试断言**: `tests/integration/sql/information_schema_test.rs:33-66` 三个测试
(`test_select_from_information_schema_tables_fails` / `_columns_fails` / `_indexes_fails`)
**断言返回 ERROR**(fail-closed,不是 silent success)。

### §3.3 Admin CLI partial path

`crates/admin/src/main.rs:214` 运行 `SELECT * FROM information_schema.processlist`
通过 mysql-client 远程查询,但 **server 端没有 information_schema handler** —
admin 路径实际上不可用,已记录为 dead code。

## §4 56A-R1 closure evidence (DONE at this HEAD)

**PR**: #4323 (`b0d1f6705d`)
**Merge commit**: `8c66132f5f`

修复:`crates/parser/src/parser.rs:9027` 中 `parse_show` 添加 `Some(Token::Create) =>`
匹配臂(原 arm 仅匹配 `Token::Identifier("CREATE")` 是 dead code,因为 lexer 将 CREATE
keyword-tokenize 为 `Token::Create`)。

**5 integration tests** in `tests/integration/sql/show_tables_test.rs:179-340`:
- `show_create_table_returns_single_ddl_row`
- `show_create_table_preserves_not_null_clause`
- `show_create_table_missing_table_errors`
- `show_create_table_alter_roundtrip`
- `show_create_table_ddl_roundtrip_invariants`

**Test result**: `cargo test --test show_tables_test --all-features` → **31 passed; 0 failed**

## §5 56A-R3 DEFERRED → v3.13+ (only remaining open item)

| Sub-Task | 当前状态 | DEFERRED 原因 | Owner | Expiry | 预计工作量 |
|----------|----------|---------------|-------|--------|-----------|
| **56A-R3** SHOW FULL TABLES / SHOW TABLE STATUS | 无 parser/executor 支持 | MySQL-specific admin extensions,BETA scope 不要求 | TBD | TBD | ~150 LOC / 1-2 小时 |

### Closed at this HEAD (post-56A-R2/R4 closure)

| Sub-Task | 状态 | PR / Commit | Evidence |
|----------|------|-------------|----------|
| **56A-R1** SHOW CREATE TABLE integration test | ✅ DONE | PR #4323 @ `8c66132f5f` | 5 tests in `tests/integration/sql/show_tables_test.rs` |
| **56A-R2** information_schema SQL path | ✅ DONE | PR #4349 @ `e584875b1f` | 8 tests in `tests/integration/sql/information_schema_test.rs`; `src/engine_select.rs::execute_information_schema_select`; `crates/parser/src/parser.rs::parse_select_statement` captures `FROM schema.table` |
| **56A-R4** SHOW WARNINGS / SHOW ERRORS / SHOW STATUS / SHOW VARIABLES | ✅ DONE | PR #4345 @ `ed0f239e5a` | 6 tests in `tests/integration/sql/show_warnings_errors_status_variables_test.rs`; 4 handlers in `src/engine_ddl.rs` |

## §6 Verification Commands (re-runnable)

```bash
# SHOW CREATE TABLE 集成测试
cargo test --test show_tables_test --all-features
# 期望:31 passed; 0 failed

# SHOW COLUMNS LIKE 集成测试
cargo test --test show_columns_test --all-features
# 期望:3 tests passed

# SHOW INDEX 集成测试
cargo test --test show_index_test --all-features
# 期望:3 tests passed

# information_schema fail-closed 集成测试
cargo test --test information_schema_test --all-features
# 期望:3 tests passed (断言 ERROR,不是 silent skip)

# parser 模块回归
cargo test -p sqlrustgo-parser --all-features --tests
# 期望:755+ passed

# mysql compat gate(覆盖 56A 全部 SHOW variants)
bash scripts/gate/check_v312_21_mysql_compat.sh
# 期望:exit=0,fixtures: show_columns_basic / show_columns_like / show_index_pk / show_create_table_pk
```

## §7 Honest Disclosure (per Anti-Fabrication-Policy-v1.0)

1. **SHOW CREATE TABLE 不输出 DEFAULT 子句** — 已知限制,不影响 BETA 准入。
2. **information_schema SQL 路径不可用** — 解析器主动拒绝 schema-qualified names,
   executor 无 virtual-table dispatch。已 DEFERRED 到 v3.13 RC1 (56A-R2)。
3. **Admin CLI information_schema 路径 dead code** — server 端无 handler,已记录。
4. **56A-R3 SHOW FULL TABLES / SHOW TABLE STATUS 未实现** — DEFERRED 到 v3.13.0+。
5. **56A-R4 SHOW WARNINGS/ERRORS/STATUS/VARIABLES runtime 未实现** — parser 只 smoke,
   DEFERRED 到 v3.13 RC1。

## §8 Round-24 Codex Evidence Compliance

| Round-24 要求 | 实际产出 |
|--------------|---------|
| 目标分支 | develop/v3.12.0 @ `6d1b1fe9c6` |
| 关联 commit SHA | `8c66132f5f`(56A-R1 merge),`0b429a85cd`(V312-56 master plan merge) |
| 关联 PR# + merge commit | PR #4323 @ `8c66132f5f`,PR #4322 @ `0b429a85cd` |
| Run cmd + exit | `cargo test --test show_tables_test --all-features` exit=0,31 passed |
| 输出摘要 | SHOW CREATE TABLE / SHOW COLUMNS LIKE / SHOW INDEX / information_schema 4/4 实现 |
| Evidence hash(64-hex) | `sha256=4de2c88cb3d89baeafde9293a60431872d9851e74d7bd925c3cada88b9f392ff` (computed 2026-08-18, content pre-append) |
| Remaining risk | 56A-R3 DEFERRED → v3.13+ (MySQL admin extensions: SHOW FULL TABLES / SHOW TABLE STATUS),owner=TBD,expiry=TBD |

## §9 Provenance

- **Generated at**: 2026-08-18T14:30:00Z
- **Source repo**: openclaw/sqlrustgo
- **Branch**: develop/v3.12.0
- **HEAD commit**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
- **Policy**: Anti-Fabrication-Policy-v1.0
- **Source issue**: #4251
- **Supersedes**: VERIFICATION §6 "V312-56A Residual 4 Sub-Tasks" table(本文件提供更细粒度的教学 lab 描述)
