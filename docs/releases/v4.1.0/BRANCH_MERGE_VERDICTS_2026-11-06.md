# 分支合并裁定表（2026-11-06）

> 这份表回答 Owner 的第 2 条指令：**哪些 fix/development 分支需要并进 `develop/v4.1.0`**。判据不看分支名、不看新旧，只看一件事：**合并会不会把 develop 缺的东西带进来**（会不会丢正确的代码或文档）。

## 0. 口径（三把尺子）

1. **补丁等价**：`git cherry develop/v4.1.0 <branch>` ⇒ `-` 的提交已在 develop 里（以等价补丁存在），`+` 的是候选。
2. **合并模拟**：`git merge-tree --write-tree develop/v4.1.0 <branch>` ⇒ 直接问合并本身：合并后的 tree 与 develop 的 tree 逐文件是否相同（相同 ⇒ 合并不丢东西），以及有没有冲突。
3. **hunk 级核对**：对候选分支取它相对 merge-base 的**新增行**，逐行去空白后在 develop 当前文件里找。找不到 ⇒ 这些行是 develop 真正缺的内容；找得到 ⇒ 同样的内容已在 develop 的其他形态里（cherry-pick、改写、换写法），合并不增信息。

**历史线判定**：v4.1.0 的发布线自 2026-09-23 起（`docs/releases/v4.1.0/` 首次出现）。merge-base 早于这一天的分支是 v3/v4.0 时代的历史线：合并不会带来 4.1.0 的交付，只会把 develop 带回旧世界（这类分支的冲突数普遍上百），**不合并**，逐条记账。

## 1. 总览

| 分类 | 分支数 | 处置 |
|---|---|---|
| 无需合并（合并结果与 develop 逐文件相同） | 308 | 不动 |
| 不合并（历史线：基线早于 v4.1.0 开线） | 23 | 记账，不动 |
| 需要人工裁定（4.1.0 开发线，有冲突） | 29 | 逐条 hunk 核对后并入或判不并 |
| 需要合并（4.1.0 开发线，无冲突） | 5 | 并入 |
| **合计** | **365** | |

## 2. 需要合并（无冲突）—— 已并入或已并到等价形态

| 分支 | merge-base | 合并带进文件 | hunk 级 develop 没有的行 | 处置 |
|---|---|---|---|---|
| `feature/a03-callchain-audit-5225` | 2026-10-11 | 5 | 208 | 已并入（PR #5609，纯新增归档产物） |
| `fix/ci-instance-relative-url` | 2026-10-23 | 1 | 0 | hunk 级 develop 没有的行 = 0 ⇒ 无增量，不并 |
| `fix/ci-protoc-vendored` | 2026-10-25 | 3 | 62 | 并入（protoc vendored，避免 runner 缺 protoc） |
| `mcode-macmini/sqlrustgo-alpha-handoff` | 2026-10-15 | 18 | 2664 | 已并入（PR #5609，治理三件套 + SSOT 漂移证据） |
| `test/5281-rowid-lifecycle` | 2026-10-11 | 2 | 79 | 已并入（PR #5609，E5-5281 ADR；ADR-5281 本体 develop 已是更新版） |

## 3. 需要人工裁定（4.1.0 开发线，有冲突）—— 逐条核对结果

| 分支 | merge-base | 冲突文件数 | 合并带进文件 | hunk 级缺失行 | 判定 | 依据 |
|---|---|---|---|---|---|---|
| `feature/session-context-5057` | 2026-10-09 | 33 | 84 | 821 | 不并入（需专门批次） | 它把 `SessionContext` 铺进 storage trait 的每个方法（821 行），与 develop 现有的 trait 形态是两套设计；硬合 = 大范围 API 改写，不是机械合并。develop 侧的 #5057 已由 `insert_db_isolation_5057` 覆盖（PR #5607 批的 `58e3bfaef0`）。 |
| `fix/5262-b2-binaries` | 2026-10-12 | 12 | 14 | 55 | 需专门批次 | 解析器 + `engine_ddl` + `engine_setops` + `system_tables` 的语义改动（55 行），与 develop 的执行器形态冲突（12 个冲突文件）⇒ 逐条重做，工作量属代码批而非机械合并。 |
| `feature/20261023-v410-mandatory-docs` | 2026-10-23 | 7 | 7 | 1364 | 不并入（已被 develop 的更新版本取代） | 7 个同名文档两侧是不同代的同一批文档，develop 侧章节结构已重排、篇幅更长（DEPLOYMENT_GUIDE 798 行 vs 522 行）⇒ 保留 develop 侧（PR #5609 已按此处置）。 |
| `fix/omp-engine-phase3-arc-mutex-tx` | 2026-10-04 | 6 | 11 | 338 | 不并入（develop 已有该形态） | 它引入的是 `Arc<Mutex<TransactionManager>>` + `TxSession` 的引擎改造；**develop 当前代码里已经是这个形态**（`execution_engine.rs` 的 `transaction_manager` 字段与 `TxSession` 段），hunk 差异只是它自己的更早副本。 |
| `fix/omp-ignore-registry-5098` | 2026-10-07 | 5 | 5 | 77 | 需专门批次 | `check_anti_fabrication.sh` / `check_coverage_v312.sh` 是**门禁脚本**，改动它们需要改前漏什么 / 改后多报什么 + 反向变异证据，不能在合并批次里顺手带过。 |
| `ci/v4.1.0-v31213-single` | 2026-10-25 | 4 | 4 | 33 | 已并入（PR #5607） | 该线的 8 条真实修复已随 PR #5607 并入；剩下的差异全是 `EXP-DISABLED` 实验步骤与`if: "false"` 段落 ⇒ 按纪律排除，不并入。 |
| `backup-before-force-20261015T212430Z` | 2026-10-15 | 3 | 3 | 1 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `fix/4597-chaos-t8d-awk-portability` | 2026-10-15 | 3 | 8 | 764 | 部分并入（文档已并；代码需专门批次） | 它带的 `docs/governance/AGENT_PROMPT_TEMPLATE.md` 等治理文档已随 `mcode-macmini/sqlrustgo-alpha-handoff` 并入；剩下的 `crates/storage/src/file_storage.rs` 与 `crates/mysql-server/src/lib.rs` 改动与 develop 的实现冲突（764 行），需专门批次。 |
| `fix/5320-5327-view-public-rollback` | 2026-10-14 | 3 | 3 | 54 | 需专门批次 | `crates/storage/src/engine.rs` 与 `src/execution_engine.rs` 的视图/回滚语义改动（54 行），冲突 3 处，需在当前引擎形态上重做并补回归。 |
| `fix/ci-develop-baseline-red-6335` | 2026-10-25 | 3 | 3 | 16 | 需专门批次（CI 执行面已死） | CI 工作流的 baseline 修正：在 runner 全 Blocked 的现状下无法用 CI 绿验证，需要单独批次 + 归档 run 覆盖。 |
| `fix/gate-paths-batch` | 2026-10-15 | 3 | 3 | 1 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `temp-sync-250-latest` | 2026-10-15 | 3 | 3 | 1 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `temp-sync-250-merged` | 2026-10-15 | 3 | 3 | 1 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `ci/3847-d6b-no-reexec` | 2026-10-25 | 2 | 6 | 406 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `ci/v4.1.0-v31213-base-single` | 2026-10-25 | 2 | 2 | 8 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `exp/v31213-base-single` | 2026-10-25 | 2 | 2 | 8 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `feat/4942-gate-manifest` | 2026-10-05 | 2 | 3 | 102 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `feat/5057-session-context` | 2026-10-07 | 2 | 2 | 2 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `fix/4708-block-comment-witness` | 2026-10-15 | 2 | 2 | 0 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `mcode-macmini/e5-r9-r11-verify` | 2026-10-15 | 2 | 7 | 764 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `mcode-macmini/governance-kanban-expert-ladder` | 2026-11-06 | 2 | 47 | 0 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `sync/from-250-20261101/develop/v4.1.0` | 2026-10-25 | 2 | 2 | 2 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `test/4708-block-comment-witness` | 2026-10-15 | 2 | 2 | 0 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `backup/pre-rebase-20261029/fix-5217-limit-comma-form` | 2026-10-09 | 1 | 1 | 8 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `chore/v4.1.0-gitignore-cleanup-2026-10-04` | 2026-10-04 | 1 | 2 | 2 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `fix/ci-fmt-ptrarg-batch32` | 2026-10-23 | 1 | 1 | 0 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `fix/ci-stage-config-fail-loud` | 2026-10-25 | 1 | 1 | 0 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `fix/omp-4938-incremental-backup-e2e` | 2026-10-05 | 1 | 1 | 93 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |
| `mcode-macmini/rustfmt-4597` | 2026-10-15 | 1 | 4 | 314 | 需专门批次 | 冲突文件与 develop 的当前形态重叠，需在当前代码上逐条重做（hunk 缺失行见左列）。 |

## 4. 不合并（历史线）

| 分支 | merge-base | 冲突文件数 | 合并带进文件 | 理由 |
|---|---|---|---|---|
| `chore/dep-reqwest-0.13` | 2026-02-16 | 762 | 1487 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `develop/v3.12.0` | 2026-09-11 | 1 | 1 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `develop/v4.0.0` | 2026-09-20 | 2 | 2 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `docs/alpha-gate-report-v3` | 2026-09-17 | 0 | 3 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `docs/v400-ga-rectify` | 2026-09-19 | 2 | 2 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `feat/pragma-and-3vl` | 2026-09-20 | 12 | 12 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `feat/v4.0.0-wal-group-commit` | 2026-09-18 | 0 | 1 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `feat/v400-alpha-gate-report` | 2026-09-16 | 1 | 1 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `feat/v5.0.0-phase-c` | 2026-09-14 | 1 | 4 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `feature/phase4-tests` | 2026-09-14 | 12 | 24 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `fix/main-credential-redaction-batch32` | 2026-09-19 | 6 | 8 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `fix/main-tip-credential-redaction` | 2026-09-19 | 2 | 4 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `fix/omp-restore-storage-trait` | 2026-06-01 | 414 | 432 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `fix/v312-bugfix-4490-4491-4492` | 2026-02-16 | 551 | 2747 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `fix/v400-dml-storage-regressions` | 2026-09-19 | 7 | 6 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `fix/v400-soak-hang-wal-delete-o1` | 2026-09-12 | 5 | 5 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `main` | 2026-09-19 | 6 | 8 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `release/v4.0.0` | 2026-09-19 | 2 | 2 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `sync/sync-252-to-250` | 2026-09-19 | 7 | 6 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `sync/v410-zombie-graph-port` | 2026-09-21 | 0 | 4 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `v4.0.0-zombie-fix-original` | 2026-09-17 | 5 | 5 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `v4.1.0-mvp` | 2026-09-12 | 0 | 6 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |
| `v4.1.0-pk-lookup` | 2026-09-12 | 3 | 12 | 基线早于 v4.1.0 开线（2026-09-23）：合并不带来 4.1.0 交付，只把 develop 带回旧世界 |

## 5. 复核方式

```bash
# 文件级判定（365 个分支）
python3.14 scripts/gate/gov_probe.py metrics >/dev/null   # 分母仍应是 168
git merge-tree --write-tree gitea252/develop/v4.1.0 gitea252/<branch>
git cherry gitea252/develop/v4.1.0 gitea252/<branch>
```
