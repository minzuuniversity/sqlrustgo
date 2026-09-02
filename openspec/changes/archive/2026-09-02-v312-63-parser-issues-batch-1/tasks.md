# Tasks — v312-63 parser issues batch 1 (#4627/#4635/#4640/#4642)

## 1. TIMESTAMPDIFF unit keyword (#4627)

- [ ] 1.1 `crates/parser/src/lexer.rs`: 添加 `Token::TimestampDiff` 到 Token enum 与 keyword 映射（"TIMESTAMPDIFF" / "timestampdiff" 大小写不敏感）。
- [ ] 1.2 `crates/parser/src/parser.rs` `parse_primary_expression`: 在 DATE_ADD/DATE_SUB 分派后增加 TimestampDiff 分派；函数名为 `TIMESTAMPDIFF` 时第一个参数按 unit 关键字消费（接受 Identifier/StringLiteral/Keyword 之一），失败时报 SQLite 兼容错误信息。
- [ ] 1.3 `crates/executor/src/expr/mod.rs`: 复用既有 timestamp 表达式评估；当第一参为 unit 字符串时按 minute/hour/... 计算差值（秒级）。
- [ ] 1.4 测试 `crates/parser/src/parser.rs` `#[cfg(test)]`: `parse_timestampdiff_unit_keyword_recognized`、`parse_timestampdiff_with_column_refs_evaluates`。
- [ ] 1.5 集成测试 `tests/integration/sql/timestampdiff_test.rs`: 覆盖 `TIMESTAMPDIFF(MINUTE, ts1, ts2)` / `(HOUR, ...)` / `(DAY, ...)` 三个示例，断言数值正确性。

## 2. CASE val WHEN NULL (#4635)

- [ ] 2.1 `crates/parser/src/parser.rs` `parse_case_when_expression:8492`: 在 simple-CASE 分支（`base_expr.is_some()`）下，调用 `parse_expression()` 失败时回退一次：若 `self.current() == Some(Token::Null)` 则消费并返回 `Expression::Literal("NULL".to_string())`。
- [ ] 2.2 测试 `crates/parser/src/parser.rs` `#[cfg(test)]`: `parse_case_when_null_value_accepted`（解析 `CASE val WHEN NULL THEN 'n' END`）。
- [ ] 2.3 集成测试 `tests/integration/sql/case_test.rs`: 覆盖 `CASE val WHEN NULL THEN 'null' WHEN > 20 THEN 'big' ELSE 'small' END` 对 (10, NULL, 30) 的输出。

## 3. 短 INSERT (#4640)

- [ ] 3.1 `src/engine_dml.rs:107`: 当 `insert.columns.is_empty() && row.len() < expected_cols` 时，把 row 扩展到 `expected_cols`，缺失值填 `Value::Null`。
- [ ] 3.2 `src/engine_helpers.rs:139`: 同样的放宽处理 SELECT→INSERT 路径。
- [ ] 3.3 测试 `crates/executor/src/dml.rs` 或 `src/engine_dml.rs`: `insert_short_form_pads_null_columns`、`insert_short_form_excess_columns_still_errors`。
- [ ] 3.4 集成测试 `tests/integration/sql/insert_short_form_test.rs`: 复现 issue 例子（2 列表 + VALUES (10) → id=10, val=NULL）。

## 4. UPSERT (#4642)

- [ ] 4.1 `crates/parser/src/parser.rs` lexer: 添加 `Token::Conflict`（"CONFLICT"）。
- [ ] 4.2 `crates/parser/src/parser.rs` `parse_insert:6708`: 在 ON 分支中接受 `Token::Conflict` → 解析可选 `(col_list)` → `DO NOTHING` 或 `DO UPDATE SET ...`，产出 `OnConflictClause { target_cols, action: DoNothing | DoUpdate { assignments } }`。
- [ ] 4.3 `crates/parser/src/parser.rs`: `InsertStatement` 结构体新增 `on_conflict_clause: Option<OnConflictClause>`。
- [ ] 4.4 `src/engine_dml.rs`: 在 INSERT 执行前，若 `on_conflict_clause` 或 `on_duplicate_key_update` 存在：扫描主键/唯一索引（已有 `record_matches_unique_key`）；命中 → 应用 assignments 为 UPDATE；未命中 → 走 INSERT。`DO NOTHING` 在命中时跳过。
- [ ] 4.5 测试 `crates/parser/src/parser.rs`: `parse_on_conflict_do_update`、`parse_on_conflict_do_nothing`。
- [ ] 4.6 测试 `crates/executor/src/dml.rs` 或 `src/engine_dml.rs`: `on_duplicate_key_update_increments`、`on_conflict_do_update_sets_value`、`on_conflict_do_nothing_skips`。
- [ ] 4.7 集成测试 `tests/integration/sql/upsert_test.rs`: 复现 issue 两个例子。

## 5. Documentation & Verification

- [ ] 5.1 `CURRENT_VERSION.md` GA 阻塞项追加：#4627/#4635/#4640/#4642 已解决。
- [ ] 5.2 `cargo build --all-features` 干净构建。
- [ ] 5.3 `cargo test --all-features` 全部 PASS（含新测试 12-20 个）。
- [ ] 5.4 `cargo clippy --all-features -- -D warnings` 无 warning。
- [ ] 5.5 复现每个 issue 的 printf|cli 命令，确认输出。