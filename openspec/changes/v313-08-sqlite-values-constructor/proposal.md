# V313-08: 修复 SQLite 兼容性 VALUES Constructor 解析错误

## Why

当前解析器在处理 VALUES Constructor 时存在 bug，导致部分 SQLite sqllogictest 失败：

- **表面错误**：`insert__test_insert_invalid.test`、`insert__test_insert.test`、`update__test_update.test` 报错 `"Parse error: Expected expression"` 或 `"unexpected token: con1"`
- **根因**：VALUES Constructor 的 row 解析逻辑在遇到特定 token 序列时提前终止，导致行内后续表达式未被解析
- **影响范围**：所有使用 VALUES Constructor 的 INSERT 语句

本变更修复 VALUES Constructor 的行解析逻辑，确保多行 VALUES 表达式（如 `VALUES (1, 2), (3, 4)`）能正确解析。

## What Changes

- **`crates/parser/src/parser.rs`**：修复 `parse_insert` 中 VALUES row 解析逻辑的提前终止 bug
- **`crates/parser/src/parser.rs.bak`**：备份文件删除（过时备份）
- **无新增 fixture 文件**：修复后现有 sqllogictest 应全部通过

## Capabilities

### 修复能力

- `sqlite-values-constructor`：修复 SQLite sqllogictest 中的 VALUES Constructor 解析问题
- 修复后 `insert__test_insert_invalid.test`、`insert__test_insert.test`、`update__test_update.test` 应返回 PASS

### 修改能力

- `parser-insert-values`：修正 INSERT VALUES 多行解析逻辑

## Impact

- **修改文件**：`crates/parser/src/parser.rs` 中的 `parse_insert` 方法
- **删除文件**：`crates/parser/src/parser.rs.bak`（过时备份）
- **风险**：VALUES 解析逻辑修改可能影响其他 INSERT 路径；需在全面回归测试后合并
- **无新增外部 crate 依赖**
