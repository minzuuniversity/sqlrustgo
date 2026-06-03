# Design: Parser MERGE 语法实现

## Context

### 背景
SQLRustGo parser 完全不支持 `MERGE` 语句。`crates/executor/src/merge.rs` 中已有 `execute_merge` 函数定义但从未被调用（dead code），原因是 `local_executor.rs` 在遇到 `MERGE` 起始的 SQL 时直接返回 `MERGE via ExecutionEngine: wired but needs parser support` 错误。

### 当前状态
- `crates/parser/src/token.rs`: 关键字表无 `Merge`
- `crates/parser/src/lexer.rs`: 关键字识别表无 `"MERGE"`
- `crates/parser/src/parser.rs`: `Statement` enum 无 `Merge` 变体；`parse_statement` 无 MERGE 匹配臂
- `crates/parser/src/lib.rs`: 无 MERGE 相关 re-export

### 约束
- 仅修改 parser crate，不涉及 executor / planner
- AST 必须可被未来 G3/G4 PR 直接消费
- 保持 `Statement` enum 的现有结构（`derive(Debug, Clone, PartialEq)`）

## Goals / Non-Goals

### Goals
- ✅ Parser 能识别 `MERGE` 关键字
- ✅ Parser 能解析 SQL:2003 标准 MERGE 语句（INTO/USING/ON/WHEN MATCHED/NOT MATCHED）
- ✅ AST 包含完整 MERGE 信息（target/source/conditions/when clauses）
- ✅ 通过所有现有 parser 测试 + 新增 MERGE 测试
- ✅ 范围最小化（不实现 executor 调用）

### Non-Goals
- ❌ 不实现 `execute_merge` 调用（这是 G3 #2810）
- ❌ 不实现 LocalExecutor 重构（这是 G4 #2811）
- ❌ 不支持 `MERGE ... WHEN NOT MATCHED BY SOURCE`（扩展场景，留给后续）
- ❌ 不实现 MERGE 优化（planner 层不涉及）

## Decisions

### Decision 1: AST 结构采用嵌套枚举

```rust
pub struct MergeStatement {
    pub target_table: String,
    pub target_alias: Option<String>,
    pub source: MergeSource,
    pub source_alias: Option<String>,
    pub on_condition: Expression,
    pub when_clauses: Vec<MergeWhenClause>,
}

pub enum MergeSource {
    Table { name: String },
    Subquery(Box<SelectStatement>),
}

pub struct MergeWhenClause {
    pub is_matched: bool,
    pub additional_condition: Option<Expression>,
    pub action: MergeAction,
}

pub enum MergeAction {
    Update { set_clauses: Vec<(String, Expression)> },
    Insert { columns: Vec<String>, values: Vec<Expression> },
    Delete,
}
```

**理由**:
- 与 `SelectStatement` / `InsertStatement` 等现有 AST 风格一致
- `MergeSource` 枚举支持 table 和 subquery 两种 source 形式
- `MergeAction` 枚举统一 UPDATE/INSERT/DELETE 三种动作
- `is_matched: bool` 比 `enum {Matched, NotMatched, NotMatchedBySource}` 简洁（不支持 NOT MATCHED BY SOURCE）

**备选方案**:
- ❌ 扁平化（所有字段直接在 `MergeStatement`）：无法表达 subquery source
- ❌ 用 `Box<Statement>` 表示 action：要求 Statement 递归，对 MergeAction 类型化更差

### Decision 2: parse_merge 函数拆解为私有辅助函数

```rust
pub fn parse_merge(&mut self) -> Result<Statement, String> {
    self.expect(Token::Merge)?;
    self.expect(Token::Into)?;
    let target_table = self.parse_identifier()?;
    let target_alias = self.parse_optional_alias()?;
    self.expect(Token::Using)?;
    let source = self.parse_merge_source()?;
    let source_alias = self.parse_optional_alias()?;
    self.expect(Token::On)?;
    let on_condition = self.parse_expression()?;
    let mut when_clauses = Vec::new();
    while matches!(self.current(), Some(Token::When)) {
        when_clauses.push(self.parse_merge_when_clause()?);
    }
    if when_clauses.is_empty() {
        return Err("MERGE requires at least one WHEN clause".to_string());
    }
    Ok(Statement::Merge(MergeStatement { ... }))
}
```

**理由**:
- 主函数控制流程，辅助函数 (`parse_merge_source`, `parse_merge_when_clause`, `parse_merge_action`) 隔离细节
- 可读性 > 行数

**备选方案**:
- ❌ 单函数 200+ 行：可读性差

### Decision 3: 复用现有 `Expression::parse_*` 解析器

`on_condition` 和 `additional_condition` 复用 `parse_expression()`。`set_clauses` 的 RHS 也复用 `parse_expression()`。

**理由**:
- 已有完整的表达式解析器（含二元/一元/字面量/列引用）
- 不重复实现

**备选方案**:
- ❌ 简单表达式解析器（仅支持 `col = val`）：不灵活

### Decision 4: Token::Merge 与现有 Token 风格一致

```rust
// In token.rs Keyword section, near other DML keywords
Merge,
```

**理由**: 与 `Insert/Update/Delete` 同段，便于维护

## Risks / Trade-offs

| 风险 | 缓解 |
|------|------|
| AST 设计与 G3/G4 executor 期望不匹配 | G3 executor stub 已有 `MergeStatement` 字段假设；本设计对齐 |
| `parse_expression` 调用位置 | 已在 parser.rs 大量使用，是稳定接口 |
| 测试覆盖不足 | 新增 7 个测试用例覆盖各种 WHEN/alias 组合 |
| 错误信息不一致 | 使用 `format!("Expected {:?}, got {:?}", expected, actual)` 与现有错误风格一致 |

## Migration Plan

### Deploy
1. PR 合并到 `develop/v3.8.0`
2. CI 验证（cargo check/clippy/test）
3. 后续 G3/G4 PR 依赖本 AST 结构

### Rollback
- 单 commit revert
- AST 破坏性变更需下游 crate 同步更新（executor `merge.rs` 需调整）
- executor 已有 stub，本次 PR 不破坏现有 merge.rs

## Open Questions

- **Q1**: 是否在 parser 中添加 `MERGE` 语法验证（确保 WHEN MATCHED 后必有动作）？  
  建议：否，验证留给 executor 运行时（更友好的错误位置）

- **Q2**: `additional_condition` (即 `WHEN MATCHED AND <cond>`) 优先级如何？  
  现状：标准 SQL 语义，先 `WHEN MATCHED [AND cond]` 才执行 action
  本设计：`additional_condition: Option<Expression>` 在 G3 executor 中实现

- **Q3**: 是否支持 `DELETE` 在 `WHEN MATCHED` 中？  
  现状：标准 MERGE 支持，本设计 AST 包含 `MergeAction::Delete`
  parser 实现：留作 follow-up（本 PR 仅实现 UPDATE/INSERT 解析，DELETE 解析为占位错误）
