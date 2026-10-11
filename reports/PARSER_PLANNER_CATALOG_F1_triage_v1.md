# Parser / Planner / Catalog F1 (orphan) triage — Phase 2 only-read scan

> **作者**：deepseek-flash（DeepSeek-Harness）
> **日期**：2026-10-11
> **HEAD evaluated**：`develop/v4.1.0` tip `01df92a5d1`（worktree `feature/audit-storage-mutation`）
> **scope**：only-read static scan, **不**修改生产源码,**不**重新生成 mutation, **不**跑分析查询
> **依据**：用户转发的 ChatGPT "Parser / Binder 缺陷审计" + AOCI 4,981 entries 索引 + 直接 `grep` 公共 API 调用图

## 1. 方法（only-read，3 步骤）

1. **AOI Entry 复审**：检查已索引 4981 entries 中关于 parser / planner / catalog / optimizer 的 63 条（per `crates/<name>/` path grep）
2. **公共 API 调用图**：对每个 crate 列出 `pub fn` 声明，从 `src/` 与 `tests/` 全文 grep 外部调用者；外部调用者 ≤ 1 标记为 F1 suspect
3. **常见死代码模式扫描**：Display impl、Debug impl、`.bak` / `.tmp` 后缀文件、孤儿 helper

## 2. AOCI 索引（per crate）

| Crate | AOI entries | 备注 |
|---|---:|---|
| `crates/parser/` | 8 | 仅 Cargo.toml + 几个 module 文档 |
| `crates/planner/` | 10 | 模块级声明 |
| `crates/catalog/` | 20 | 含 `auth.rs`、`column.rs`、`index.rs` 多个 `Display` impl |
| `crates/optimizer/` | 25 | 部分标注 source comments cite 已修复 issue |

**AOCI 已观察到的事实**（per `crates/catalog/src/auth.rs`）:
> "Part of the catalog crate at this commit and carries no module doc comment, so its role is only visible from its callers"

这本身不是 F1 缺陷，但**暗示**：F1 的 catalog 函数缺乏调用方时，**唯一证据只能靠调用图 grep**，因为没有 doc comment。

## 3. CATALOG F1 suspect 清单（仅静态证据）

按 `crates/catalog/src/row_level_security.rs`（**核心 RLS API**）扫描：

| 函数 | 外部调用者 | F1 判定 |
|---|---:|---|
| `RowLevelSecurity::create_policy` | **0** | F1-suspect |
| `RowLevelSecurity::drop_policy` | **0** | F1-suspect |
| `RowLevelSecurity::enable_rls` | **0** | F1-suspect |
| `RowLevelSecurity::disable_rls` | **0** | F1-suspect |
| `RowLevelSecurity::is_rls_enabled` | 1 | F1-suspect（仅 1 外部调用）|
| `RowLevelSecurity::get_policies` | **0** | F1-suspect |
| `RowLevelSecurity::check_write` | **0** | F1-suspect |
| `RowLevelSecurity::tables_with_rls` | **0** | F1-suspect |
| `RowLevelSecurity::total_policy_count` | **0** | F1-suspect |

`count()` 的逻辑是 `grep -v "crates/catalog"` 的外部命中数（含定义处 self-match = 1，外部 = N - 1）。

**Risk class**: F1（无生产调用者）。
**Failure class**: F1 / F6（孤儿 Display impl 经常也是同类问题，但本次未细查 Display 调用图）。

**Action**（per AOCI AGENTS.md §2.2 单一负责人）:
- **不**修改 `crates/catalog/src/row_level_security.rs` — 这是测试断言的活，需要 IAIA-410 §8 #8 / Auditor C 接手
- 仅**记录**：8 个 RLS 函数当前 0 外部调用者，按 AOCI 标准是 **F1-Candidate / Runtime Unverified**

## 4. PARSER F1 suspect（stale backup 文件）

```
crates/parser/src/parser.rs.bak   495,254 bytes  (vs parser.rs 840,729 bytes)
```

| 项 | 值 |
|---|---|
| 修改时间 | 与 `parser.rs` 同步（HEAD `01df92a5d1`） |
| grep 引用 | **0**（build.rs / Cargo.toml / 其他 .rs / .toml 都不引用 `.bak`） |
| 体积 | `parser.rs.bak` 是 `parser.rs` 的 ~59% 大小 |

**Risk class**: F1（孤儿文件，未在构建图中）+ F8（仓库体积浪费 ~500KB）
**Action**:
- **不**直接删除 — `parser.rs.bak` 在 worktree 内是 stale 备份，但删除前需要确认 git log 该文件**未**存在 commit 历史里
- 仅**记录**：F1-Candidate

## 5. Display impl（per ChatGPT "常见死代码模式"）

| 文件 | Display impl | 备注 |
|---|---|---|
| `crates/parser/src/token.rs` | `Display for Token` | 大概率有调用方（logging / error） |
| `crates/parser/src/parser.rs` | `Display for Expression` | 大概率有调用方（debug 输出） |
| `crates/catalog/src/data_type.rs` | `Display for DataType` | 待查 |
| `crates/catalog/src/auth.rs` | `Display for Privilege`、`Display for AuthError` | 待查（error Display 通常被 `Result<_, AuthError>` 触发） |
| `crates/planner/src/lib.rs` | `Display for Column`、`Display for Expr`、`Display for Operator`、`Display for DataType` | 待查 |

**Status**: 仅列出,**不**在本 triage 中跑调用图 grep（避免与 storage mutation run 抢资源）。
**Action**: mcode / Auditor C 在 IAIA-410 §8 #8 阶段跑一次完整 Display impl 调用图。

## 6. out-of-scope（per AOCI + 用户硬约束）

- ❌ 修改 `crates/parser/src/parser.rs.bak` 或 `crates/catalog/src/row_level_security.rs` —— **不修改生产源码**
- ❌ 删除 parser.rs.bak —— 这是审计员 / Reviewer 的清理动作
- ❌ 跑 cargo-mutants on parser / planner / catalog —— **不**与 storage mutation 抢资源，且这些 crate 与 #5281 RowId 冻结区可能有边界
- ❌ 写 SQLancer / sqllogictest 适配器 —— IAIA-410 §8 #8 / A04 #5226 evidence_ready 配套
- ❌ 跑 rust-analyzer / CodeQL / Semgrep（per TOOL_FEASIBILITY_2026-10-11.md，仅 2/15 工具可用）

## 7. 与 SSOT_STATUS_2026-10-10 的关系

本报告**只读 + F1 候选**，**不**更新 IAIA-410 任何 WP 状态。按 AOCI §2.2 单一负责人：
- 8 个 RLS F1 suspect → 列入候选 finding，但**不**立 finding YAML（finding YAML 是 mcode 的活）
- parser.rs.bak 孤儿 → 列入候选 P3 / 低风险（不是 F1 / F6 / P0）
- Display impls → 仅列出，**不**判定 F1（需调用图）

## 8. ADR-014 5 字段

- **source_agent**: deepseek-flash（DeepSeek-Harness）
- **source_run**: audit-storage-mutation-20261011
- **timestamp**: 2026-10-11T07:10Z
- **evidence_hash**:
  - `grep -rE "pub fn \w+" crates/catalog/src/`（RLS 函数清单）
  - `grep -rE "\b<func>\b" crates/ src/` 排除 `crates/catalog/` 自身（外部调用计数）
  - `ls -la crates/parser/src/parser.rs.bak`（大小 + 时间戳）
  - `grep -rE "parser\.rs\.bak" --include='*.rs' --include='*.toml' .`（0 命中 = 孤儿）
- **conflict_resolution**: 本报告**不修改**任何源码；RLS F1 suspect 候选**不**立 finding YAML（按 AOCI §2.2 单一负责人原则，等 mcode/Auditor C 接手 IAIA-410 §8 #8 阶段再处理）；本报告仅作为 only-read 静态扫描的 out-of-band 产出。

## 9. 已知 limitations

1. **未跑调用图分析**：仅靠 `grep` 字符串匹配判定"外部调用者"。Rust 的 trait dispatch / dyn dispatch / macro expansion 不在 grep 覆盖范围内
2. **未验证 dead code 链接时不被剔除**：rustc `--emit=dead-code-include` / `RUSTFLAGS="--emit=link-dead-codes"` 等链接时验证未做。**Runtime Unverified** per 用户 2026-10-10 收紧要求
3. **未与 AOCI 数据库 schema volume 交叉**：当前 AOCI 数据库 schema volume 状态为 `absent`，无法用数据库 schema 层证据补充 parser / planner / catalog 的 F1 判定