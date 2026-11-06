# 分支合并裁定表 · 刷新（2026-11-07，针对 develop `98a104a3f9`）

> 对 `BRANCH_MERGE_VERDICTS_2026-11-06.md` 里 34 条仍待裁定/待并的分支重跑三把尺子
> （补丁等价 `git cherry`、合并模拟 `git merge-tree --write-tree`、hunk 级新增行核对）。
> 基线是本轮 19 个 PR 合并之后的 develop（tree `8545937f71be`，8744 个文件）。
> 复算脚本口径见本文 §5。

## 1. 结论

| 分类 | 分支数 | 相对 11-06 表的变化 |
|---|---|---|
| 合并结果与 develop 逐文件相同（无需合并） | 10 | 新增 6 条（#5607/#5608/#5609/#5615/#5617/#5618 吸收） |
| 内容已在 develop 的其他形态里（不合并） | 9 | 新增 5 条 |
| 只剩注释级差异（不合并） | 4 | 全部同一行、且 develop 形态更好 |
| 实验/禁用步骤（按纪律不并） | 4 | 不变 |
| 需要专门批次（有真实增量） | 11 | 减 8 条 |
| **合计** | **34** | |

⇒ **11-06 表里 29 条「需要人工裁定」的分支，现在有 18 条已经不需要动作**：
5 条合并后 tree 与 develop 逐文件相同、5 条的实质内容早已以别的形态进入 develop
（cherry-pick / 改写）、4 条只剩一行注释差异、4 条是 `if: "false"` / `EXP-DISABLED` 的实验步骤。
**剩下 11 条需要批次，其中只有 4 条涉及产品代码语义**（其余是文档、工具、门禁清单变更）。

## 2. 已被吸收（合并结果 tree == develop tree ⇒ 不动不丢任何东西）

| 分支 | 依据 |
|---|---|
| `mcode-macmini/governance-kanban-expert-ladder` | 该线全部内容已在 develop（#5593 + #5608 + #5615）；11-06 表里它标的是「需专门批次」⇒ **现已闭合** |
| `feature/20261023-v410-mandatory-docs` | develop 侧是同名文档的更新版（11-06 已判「已被取代」，本次 tree 级确认） |
| `mcode-macmini/sqlrustgo-alpha-handoff` | #5609 |
| `feature/a03-callchain-audit-5225` | #5288 + #5609 |
| `test/5281-rowid-lifecycle` | #5283 + #5609 |
| `fix/omp-4938-incremental-backup-e2e` | #5587 / #5589 |
| `fix/ci-protoc-vendored` | #5609 |
| `fix/ci-instance-relative-url` | #5561（hunk 级缺失行 0） |
| `chore/v4.1.0-gitignore-cleanup-2026-10-04` | #5165 |
| `backup/pre-rebase-20261029/fix-5217-limit-comma-form` | #5240 |

## 3. 内容已在 develop 的其他形态里（hunk 级新增行 = 0 ⇒ 不合并）

| 分支 | 说明 |
|---|---|
| `fix/4708-block-comment-witness` | 新增 32 行，develop 里 0 行缺失（#5146 已补回） |
| `test/4708-block-comment-witness` | 同上 |
| `fix/ci-fmt-ptrarg-batch32` | 新增 11 行，0 缺失（#5512 / #5576 的 rustfmt 已覆盖） |
| `fix/ci-stage-config-fail-loud` | 新增 355 行，0 缺失（#5559 已把 16 个 G 关注点接进阶段门禁） |
| `mcode-macmini/rustfmt-4597` | 新增 99 行，0 缺失（rustfmt 已在 develop） |

## 4. 只剩注释级差异（不合并）

`backup-before-force-20261015T212430Z`、`fix/gate-paths-batch`、`temp-sync-250-latest`、
`temp-sync-250-merged` —— 四条分支 hunk 级缺失行都是 **1**，而且是**同一行注释**：

```
scripts/gate/check_rc_ga_gate.sh: # (Cargo.toml has a separate "ci_test" path=tests/ci/ci_test.rs entry;
```

它属于旧实现里「跳过重复占位目标 `ci_test`」的说明。develop 现在的形态是三步解析
（`scripts/gate/check_rc_ga_gate.sh:884-981`：路径 → 目标名 → 文件主干名映射），
已经把 `ci/ci_test` 这件事**按实现交代**而不是按注释交代 ⇒ 这行注释不带 develop 缺的信息 ⇒ 不合并。

## 5. 实验 / 禁用步骤（按纪律不并）

| 分支 | 缺失行 | 内容 |
|---|---|---|
| `ci/v4.1.0-v31213-single` | 33 | 全是 `if: "false"` 与 `EXP-DISABLED` 步骤（该线 8 条真实修复已随 #5607 并入） |
| `ci/v4.1.0-v31213-base-single` | 8 | 同上（`if: "false"` + 实验超时） |
| `exp/v31213-base-single` | 8 | 同上 |
| `sync/from-250-20261101/develop/v4.1.0` | 2 | ci.yml 的 timeout 形态（develop 已更新） |

## 6. 需要专门批次（真实增量，逐条给出去处）

| 分支 | 缺失行 | 增量的落点（文件 → 行数） | 批次判定 |
|---|---|---|---|
| `feature/session-context-5057` | 821 | `crates/storage/src/*.rs`：把 `SessionContext` 铺进 storage trait 每个方法 | **需属主裁定设计**：develop 走的是 #5282（删 `current_db`/`set_current_db`，改 per-connection），这是另一套 trait 形态；不是机械合并 |
| `ci/3847-d6b-no-reexec` | 406 | `scripts/gate/check_test_inventory.sh` 151、**`ci/3847-d6b-no-reexec:scripts/gate/check_d6_smoke.sh` 116（develop 完全没有这个文件）**、`tests/integration/sql/test_inventory_gate_test.rs` 110、`check_full_gate_verification.sh` 16、`ci.yml` 13 | **门禁清单变更**：`check_d6_smoke.sh` 是新增 `check_*` ⇒ 会把分母从 168 变成 169 ⇒ 按纪律**必须属主裁定**（P0-4 同源）。且 CI 执行面实测已死（#5597），无法用 CI 绿取证 |
| `fix/4597-chaos-t8d-awk-portability` | 450 | **`fix/4597-chaos-t8d-awk-portability:scripts/gate/selftest_anti_fabrication.sh` 256（develop 没有这个文件）**、`fix/4597-chaos-t8d-awk-portability:docs/releases/v4.1.0/evidence/E5_R9_R11_VERIFICATION_2026-10-25.md` 159、`src/execution_engine_tests.rs` 21、`parallel_main_path_test.rs` 14 | 文档 + 自检工具（`selftest_*` 不入分母）⇒ **可并**；证据文档另一条线（batch2）也带 ⇒ 以 batch2 为准，本批只带工具 |
| `fix/omp-engine-phase3-arc-mutex-tx` | 338 | `src/execution_engine_tests.rs` 184、**`fix/omp-engine-phase3-arc-mutex-tx:scripts/soak/v410_short_soak.sh` 75（develop 没有）**、**`fix/omp-engine-phase3-arc-mutex-tx:docs/releases/v4.1.0/PERF_4910_LOCK_PROFILE_NOTES.md` 47（develop 没有）**、引擎代码 32 | 引擎的 `Arc<Mutex<TxManager>>` 形态 develop 已有 ⇒ 代码不并；**soak 脚本 + #4910 锁画像文档必须并**（文档/工具不得丢） |
| `feat/4942-gate-manifest` | 102 | `scripts/gate/check_anti_fabrication.sh` 69、`crates/executor/src/expr/mod.rs` 24、`config/test-runner-manifest.toml` 9 | 改的是既有门禁脚本 ⇒ 需「改前漏什么 / 改后多报什么 + 反向变异」；executor 那 24 行需单独看 |
| `fix/5320-5327-view-public-rollback` | 54 | `src/execution_engine.rs` 52（indexed join 表选择）、`crates/storage/src/engine.rs` 1、oracle 测试 1 | 引擎语义批：需在当前引擎形态上重做并补回归（#5330 已并过一版，这次是它没带上的那 52 行） |
| `fix/5262-b2-binaries` | 55 | `crates/parser/src/parser.rs`（savepoint 名不得用裸字符串判定）、`scripts/gate/check_beta_v3.12.0.sh` 的 `DISABLED_TESTS_LIST` | **batch2 正在处理**（`omp-macmini/batch2-branch-batches`） |
| `fix/omp-ignore-registry-5098` | 77 | 全是 `docs/governance/debt/debt-registry.yaml` 的**更旧状态**（把 SEM-4 的合法关闭退回 OPEN）+ 覆盖率旧实现 | **不合并**（11-06 已判，本次复核维持） |
| `mcode-macmini/e5-r9-r11-verify` | 450 | 与 `fix/4597-chaos-t8d-awk-portability` 的重叠部分：`E5_R9_R11_VERIFICATION_2026-10-25.md` 159、`selftest_anti_fabrication.sh` 256、其余为同一批代码 | 与上一条**同一批处置**：证据文档以 batch2 那份为准，工具（`selftest_*`）并入一次即可 |
| `fix/ci-develop-baseline-red-6335` | 16 | `scripts/gate/check_rc_ga_gate.sh`：`if [ "$pct" -ge "$COVERAGE_MIN" ]` + #3844 三态判定的注释 | **已被 #5607 + 在途的 #5620 吸收**（同一判据改写成 `a5_coverage_decision`，阈值显式传参 + 无阈值 fail-closed）⇒ 口径改写后 hunk 文本仍不同，但内容已在 |
| `feat/5057-session-context` | 2 | `crates/mysql-server/src/lib.rs`：`*conn_db = storage.read().current_db();` | **不合并**：develop 已按 #5282 删掉 `current_db`/`set_current_db` 共享可变状态，这两行是把被废止的形态带回来 |

### 6.1 develop 上确实缺失、只在分支上的文件（这就是「会丢东西」的具体清单）

| 文件 | 行数 | 只在 | develop 缺它的后果 |
|---|---|---|---|
| `fix/omp-engine-phase3-arc-mutex-tx:docs/releases/v4.1.0/PERF_4910_LOCK_PROFILE_NOTES.md` | 47 | `fix/omp-engine-phase3-arc-mutex-tx` | #4910 §3「BETA 门禁 #3 锁画像」的实测记录没有承载 ⇒ 该门禁的结论无处复核 |
| `fix/omp-engine-phase3-arc-mutex-tx:scripts/soak/v410_short_soak.sh` | 75 | 同上 | SOAK 的短程可复跑入口缺失 |
| `fix/4597-chaos-t8d-awk-portability:scripts/gate/selftest_anti_fabrication.sh` | 256 | `fix/4597-chaos-t8d-awk-portability` | AFP 门禁的自检（证明它能在造假时变红）缺失 |
| `ci/3847-d6b-no-reexec:scripts/gate/check_d6_smoke.sh` | 116 | `ci/3847-d6b-no-reexec` | D6 smoke 门禁缺失 ⇒ 但它会改分母 ⇒ 见 §6 的裁定要求 |

（表里用「分支名:路径」的写法，是因为这些路径**只存在于那条分支上**；写成分支限定形式，
`check_docs_pointer_paths.sh` 不会把它们当成仓库内已存在的指针，读的人也看不出歧义。）

## 7. 复核方式（可复跑）

```bash
# tree 级（34 条分支）
git merge-tree --write-tree gitea252/develop/v4.1.0 gitea252/<branch>   # 第一行是合并后 tree
# 逐文件 blob 比对 ⇒ 差异文件数；tree 与 develop 相同 ⇒ 无需合并
git ls-tree -r <tree>

# hunk 级（新增行是否已在 develop）
git diff -U0 $(git merge-base gitea252/develop/v4.1.0 gitea252/<branch>) gitea252/<branch>
```

注意 hunk 级口径的两处已知偏差，本文按实际情况修正：
- 只跳过 `//` 开头的注释行，未跳过 `#` ⇒ §4 那 4 条分支的「缺失 1 行」其实是注释；
- 按文件全文比对（不是按 hunk 上下文），因此**改写过的代码**会被记成缺失 ⇒
  §6 里 `fix/omp-engine-phase3-arc-mutex-tx` 的 184 行 `execution_engine_tests.rs` 属于这一类，
  已在批次判定里按代码/文档/工具拆开处理。
