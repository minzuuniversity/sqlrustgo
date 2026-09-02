## Context

四个 open issue（#4627/#4635/#4640/#4642）共属 v312-62 批次后续修复（PR #4634 之外），它们各自的根因互不重叠但都属于"parser 接受/拒绝边界不准"模式：

- #4627 同 #4618（SAVEPOINT 关键字被当 Identifier）、#4620（ALTER MODIFY 不拒）、#4608（多行 SQL）的反面：parser 应当把关键字当关键字却不接受。
- #4635 同 #4627：parser 应当接受 NULL 当 case value 但报错。
- #4640 是 binder 而非 parser：SQLite 短 INSERT 在 `insert.columns.is_empty()` 时允许值数 < 列数。
- #4642 同 #4620 的反面（#4620 是拒而不执，#4642 是接受而不执）。

PR #4634 commit `5038d43d46` 已经把 #4610/#4611/#4612/#4613/#4618/#4619/#4620/#4622/#4623 这 9 个一次修了，但合并提交未使用 `closes #N` 语法，issue 仍显示 open。本批次继续后续 4 个。

## Goals / Non-Goals

**Goals:**
- 修复 #4627（TIMESTAMPDIFF unit 关键字识别）
- 修复 #4635（CASE WHEN NULL 简化形式）
- 修复 #4640（短 INSERT — SQLite 兼容）
- 修复 #4642（UPSERT — SQLite/PG ON CONFLICT + MySQL ON DUPLICATE KEY 真正执行）
- 每个 issue 至少 2 个集成测试覆盖 issue 复现命令

**Non-Goals:**
- 不修改 Lexer 主 grammar（仅追加关键字 token）
- 不修改存储格式
- 不引入新公共 API
- 不涉及其他 6 个 deferred issue（#4617/#4621/#4625/#4626/#4636/#4639）的架构改造
- 不做完整 PG UPSERT（不带 WHERE 子句的 conflict target 谓词支持暂缓）

## Decisions

1. **TIMESTAMPDIFF**: 复用 `DATE_ADD`/`DATE_SUB` 的关键字分派模式（`parser.rs:7411`），加 `Token::TimestampDiff` 入口；第一个参数走 `parse_primary_expression` 但若返回 `Identifier` 则转换为 `Literal("MINUTE")` 形式（保留向下兼容）。
2. **CASE WHEN NULL**: 在 `parse_case_when_expression:8492` 的 simple-CASE 分支（`base_expr.is_some()`）下，在 `parse_expression()` 失败后回退一次：直接消费 `Token::Null` 并返回 `Expression::Literal("NULL")`。这避免了改动 `parse_expression()` 的 NULL 处理（影响面过大）。
3. **短 INSERT**: 在 `engine_dml.rs:107` 与 `engine_helpers.rs:139` 的 column count 检查处放宽：`if columns.is_empty() && row.len() < expected_cols { pad }`，按目标表列定义顺序填 `Value::Null`。`row.len() > expected_cols` 仍然硬拒。
4. **UPSERT 执行**: 在 `engine_dml.rs` 主 INSERT 路径前增加 `on_duplicate_key_update` / `on_conflict_clause` 分支：扫描主键/唯一索引命中 → 改为 UPDATE；未命中 → 走原 INSERT。`ON CONFLICT (col) DO UPDATE SET ...` 的 conflict target 不带谓词（PG `WHERE` 子句暂缓）。

## Risks

- #4627 lexer 加 keyword 会影响 `TIMESTAMPDIFF` 作 identifier 的旧用法。Mitigation: 在 keyword token 不匹配时仍允许 `Identifier("TIMESTAMPDIFF")` 走老路径（兼容 `Identifier`/`StringLiteral` 两种入口）。
- #4640 短 INSERT 与 MySQL strict mode 冲突。Mitigation: 仅在 `insert.columns.is_empty()`（用户未指定列）时启用，指定列时维持严格匹配。
- #4642 ON CONFLICT 不带 WHERE 子句可能与完整 PG 语义不符。Mitigation: WHERE 子句后续 PR 加。