# Proposal: Parser 支持 MERGE 语法 (Issue #2809, G2)

## Why

`crates/parser/src/{token,lexer,parser}.rs` 完全不支持 `MERGE` 语句，导致任何 `MERGE INTO ... USING ... ON ...` SQL 都无法解析（返回 `Unexpected token: Merge` 或类似错误）。`PR-870` 的功能目标因此完全阻塞。`MergeExecutor` 与 `execute_merge` 已在 `crates/executor/src/merge.rs` 定义，但 parser 不支持导致死代码。

## What Changes

- `crates/parser/src/token.rs`: 新增 `Token::Merge` 变体
- `crates/parser/src/lexer.rs`: `next_token()` 关键字识别表添加 `"MERGE" => Token::Merge`
- `crates/parser/src/parser.rs`:
  - 新增 `pub struct MergeStatement`（含 target/source/when_clauses）
  - 新增 `pub enum MergeSource`（Table | Subquery）
  - 新增 `pub struct MergeWhenClause` + `pub enum MergeAction`（Update/Insert/Delete）
  - 新增 `Statement::Merge(MergeStatement)` 变体
  - 新增 `parse_merge()` 函数
  - `parse_statement()` 新增 `Some(Token::Merge) => self.parse_merge()` 匹配臂
- `crates/parser/src/lib.rs`: re-export 新增的 AST 类型

## Capabilities

### New Capabilities

- `parser-merge-statement`: parser 层支持 SQL:2003 MERGE 语句（INTO/USING/ON/WHEN MATCHED/NOT MATCHED）

### Modified Capabilities

（无现有 spec 涉及此变更）

## Impact

### 代码影响
- 仅 `crates/parser/` 内部 3 个文件 + `lib.rs` 4 行 re-export
- AST 新增类型约 80-100 行
- 解析函数约 100-150 行

### 依赖影响
- 无新增依赖

### 范围限定
- 本 PR **仅 parser 层**
- **不**实现 executor 端 `execute_merge` 调用（这是 G3 #2810）
- **不**重构 LocalExecutor（这是 G4 #2811）
- **不**修改 `crates/executor/src/merge.rs`（已有 `execute_merge` 函数）

### 后续路径
- 本 PR 后，`local_executor.rs` 仍返回 `MERGE not yet implemented` 错误
- G3 PR（独立）将把 `local_executor.rs` 的错误返回替换为 `execute_merge()` 调用
- G4 PR（独立）将 `LocalExecutor` 重构为 `Arc<Mutex<dyn ExecutionEngine>>`
