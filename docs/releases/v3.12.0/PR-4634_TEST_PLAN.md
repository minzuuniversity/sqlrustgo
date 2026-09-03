# PR-4634 TEST PLAN

> **PR**: PR-4634 `fix(v312-62 / #4610..#4623): issue batch — executor / parser / CLI bug fixes`
> **Created**: 2026-09-02
> **Source SPEC**: `PR-4634_SPEC.md`

---

## 1. 覆盖目标

| Issue | 修复面 | 覆盖目标 |
|---|---|---|
| #4610 | `parse_lit` Float precision | 单测 + TPC-H Q1 (Float SUM) |
| #4611 | `LENGTH`/`LEN` 字符计数 | 单测 + BustubX-EDU case 7 (中文 length) |
| #4612 | `compare_values` BINARY collation | 单测 + BustubX-EDU case 18 (`courseno = 'c05103   '`) |
| #4613 | `ROUND` 类型保留 | 单测 + TPC-H Q14 (`round(70 * 0.5, 2)`) |
| #4618 | `ROLLBACK TO <name>` SQLite 简写 | parser 单测 + 集成（CLI batch）|
| #4619 | 事务内错误自动 rollback | CLI batch 集成 + 集成测试（mock FileStorage）|
| #4620 | `ALTER TABLE` MODIFY/ADD CONSTRAINT 拒绝 | parser 单测 + 错误信息快照 |
| #4622 | 嵌套 CTE schema 传播 | parser + executor 集成（CTE chain）|
| #4623 | `GROUP_CONCAT` 不泄漏 sentinel | 单测 + BustubX-EDU case 24 |

---

## 2. 测试阶段

### 阶段 1 — 单元测试（unit）
位置：`crates/executor/src/expr/mod.rs` 附近的 #[cfg(test)] 模块，
`crates/parser/src/parser.rs` 附近的 #[cfg(test)] 模块。

| 测试名（建议） | 目标 |
|---|---|
| `test_parse_lit_preserves_float` | #4610 |
| `test_length_counts_chars` | #4611 |
| `test_compare_values_text_binary` | #4612 |
| `test_round_preserves_float_for_d_gt_0` | #4613 |
| `test_rollback_to_shorthand` | #4618 |
| `test_alter_table_modify_rejected` | #4620 |
| `test_alter_table_add_constraint_rejected` | #4620 |
| `test_group_concat_strips_sentinels` | #4623 |

### 阶段 2 — 集成测试（integration）
位置：`crates/executor/tests/` 或 `crates/sqlrustgo-cli/tests/`。

| 测试名（建议） | 目标 |
|---|---|
| `cte_nested_chain_returns_rows` | #4622 |
| `batch_stdin_tx_rollback_on_pk_violation` | #4619 |

### 阶段 3 — 回归测试套件（regression）

| 套件 | 检查项 |
|---|---|
| `cargo test --all-features` | 全部单测 + 集成测试 |
| `cargo test -p sqlrustgo-cli` | CLI 集成（含 batch 模式）|
| `cargo clippy --all-features -- -D warnings` | clippy 0 warning |
| TPC-H SF=1 | Q1 / Q14 数值正确性（重点检查 #4613 round, #4612 collation）|
| BustubX-EDU baseline | case 7 (length) / 18 (char) / 24 (group_concat) 全部通过 |

---

## 3. 验证前置条件

1. **网络可达**：执行 `cargo test` 需能下载 `web-sys`（本地环境受限时需先用 vendored deps）
2. **数据准备**：无需 fixture，重建内存存储即可
3. **CI 触发**：merge 后 CI 自动跑全部测试，无人工 trigger

---

## 4. 暂缓验收

| Issue | 暂缓原因 | 影响 |
|---|---|---|
| #4617 | 需要 optimizer 整体改造 | IndexScan 缺失，但 SeqScan 仍可用 |
| #4621 | 需要 AST 扩展 | COUNT(*) 走 SeqScan，无覆盖索引优化 |

---

## 5. 风险测试项

### 5.1 #4612 (BINARY collation)

- **风险**：原 RTRIM 行为由 PR #4492 为 TPC-H Q14 添加
- **回归验证**：TPC-H Q14 结果需与 SQLite / MySQL 8 / DuckDB 三方一致
- **回退方案**：若 TPC-H Q14 退化，可恢复 RTRIM，仅保留 BustubX-EDU baseline

### 5.2 #4619 (batch_stdin 事务原子性)

- **风险**：`engine.flush()` 在每条语句后被调用，可能绕过事务隔离
- **回归验证**：BEGIN + INSERT + INSERT(冲突) → 数据库无任何新行
- **回退方案**：若 FileStorage 不支持事务回退，#4619 仅在 in-memory 路径生效

### 5.3 #4622 (嵌套 CTE schema)

- **风险**：`cte_columns` 只在显式 `WITH a(id, val)` 命名时填充；
  `SELECT * FROM a` 投影未覆盖
- **回归验证**：BustubX-EDU case 系列 CTE 题目全部通过
- **回退方案**：若 SELECT * 投影 case 退化，fallback 到 `lookup_table_columns`

---

## 6. 测试环境

| 项 | 值 |
|---|---|
| Rust toolchain | 2024 edition（项目要求）|
| OS | Linux 6.17 |
| Storage | 默认 FileStorage（10_000 行 buffer）|
| CLI | sqlrustgo-cli batch mode |
| WITNESS | Hermes Agent / claude-z6g4 |
