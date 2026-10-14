# IAIA-410 SSOT 现状同步（2026-10-10）

> **文档性质**：SSOT 现状同步报告，不是审计结果或裁决。
> **作者**：deepseek-flash（DeepSeek-Harness）
> **日期**：2026-10-10
> **目标 commit**：`develop/v4.1.0` tip `89dcf74188`（已被覆盖至合入状态）
> **SSOT reference**：`docs/releases/v4.1.0/IAIA_410_SSOT.md`（合入 develop/v4.1.0）
> **Master Issue**：[#5235](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5235)

## 0. 范围声明

本报告**不修订 SSOT**，按 SSOT §0 "五类载体各管一层"原则将以下审计范围的**事实**同步到 Master：

- SSOT 协议要点（合入状态）
- A01–A12 工作包现状矩阵（per SSOT §2.2 状态机）
- 关键差距（SSOT 要求 vs 现状）
- 已知 P0 发现与缺陷 Issue 对照
- 角色/分配缺口（需用户/总控裁决）
- deepseek-flash 在 Phase 1 的交付与移交

**具体裁决、owner 指派、A01 重冻结执行均不属于 deepseek-flash scope。**

## 1. SSOT 协议要点（已合入）

- **§0 五类载体**：SSOT 文档 / Gitea Issue / Run manifest / Finding YAML / STAGE.yaml
- **§2.2 工作包状态机**：`not_started` → `running` → `evidence_ready` → `independently_verified`（可 `blocked`）
- **§4.2 / §12 关键规则**：A01–A10 在 A11 Independent Judge verdict 前一律保持 open——"PR 已合并"≠"已关闭"
- **§5.1 冻结运行协议**：`run_id = iaia410-<wp>-<UTC ts>-<short commit>`，manifest v1 必含：
  - `repository.*` 40 位 commit 块
  - `environment` 全字段（cpu/memory_bytes/filesystem/features/profile/reference_dbms/process_pollution_check）
  - `datasets`、逐命令 `stdout_sha256`
  - `SHA256SUMS`
- **§8 裁决**：5 类一票否决；100 分评分仅 `[SOFT GATE]`，Hard Gate 只看事实
- **§9 独立 Judge**：clean checkout + SHA256SUMS 校验 + 重跑全部 P0/否决类 finding，输出 `accepted/rejected/needs_more_evidence`

## 2. A01–A12 现状矩阵

| WP | Issue | Owner | 状态 | 说明 |
|---|---|---|---|---|
| A01 冻结基线 | #5223 | opencode+Mimo2.6Flash | evidence_ready（**需合规重冻结**） | PR #5254 已合（`803d8bb86e`）；旧 run `20261009-a01-001` 格式**不符合 SSOT §5.1 manifest v1**——缺 `repository.*` 块、environment 全字段、逐命令 `stdout_sha256`；`worktree_clean: false`；冻结点 `bf48818cd4` 早于 SSOT 合入 |
| A02 范围矩阵 | #5224 | OMP（已认领，零交付物） | running | 仅有 #5205 33 文件清单 + AOCI 4979 条索引作合成输入；无矩阵产物；DAG 上 A03–A10 的闸门未落地 |
| A03 静态路径 | #5225 | **无人认领** | not_started | R1（CLUSTERED 绕过 WAL）已具备静态证据，需立项 finding YAML |
| A04 差分矩阵 | #5226 | opencode+Mimo2.6Flash | evidence_ready（部分） | PR #5263 已合（`7823263464`）；20 SQL cases × 5 引擎；MySQL server 引擎 `not_comparable`；7 条 CLI 列名缺陷**未立 Issue** |
| A05 索引真实性 | #5227 | **无人认领** | not_started | 需多档规模 × warm-up × 重复 + EXPLAIN 证明 |
| A06 事务/WAL/恢复 | #5228 | **无人认领（关键路径）** | not_started（输入齐备） | **唯一 P0 否决项 R1 在此**；deepseek 输入：`crates/transaction/src/` 51 个 P0 SURVIVED 变异 |
| A07 变异 REVIEW | #5229 | 待 mcode/Auditor C 接手 | running（输入齐备） | PR #5264（counterexamples）+ PR #5265（reports）已合；前置缺失：`tests/baseline/v4.1_mutation_specs.json` 从未提交 → clean checkout 不可复现（治理事故 R2） |
| A08 禁用 binary 复核 | #5230 | opencode+Mimo2.6Flash | evidence_ready | PR #5259 已合（`eab189bbdc`）；89→77；REACTIVATED 12 / STILL_FAILING 22 → #5262 / COMPILE_FAIL 3 → #5261 / ENVIRONMENT 18 / WIRE_HARNESS 27 / PERF_ONLY 6 / TIMEOUT 1 |
| A09 跨版本 | #5231 | **无人认领** | blocked（依赖 A04–A06 稳定 corpus） | v3.12.0 / v4.0.0 / v4.1.0 三版本可比；正确性先于性能 |
| A10 生产边界 | #5232 | **无人认领** | not_started（3 项已知违规已定位） | ① parallelism 静默降级 ② 默认数据目录 `/tmp/sqlrustgo-data` ③ #5217 已关 |
| A11 独立 Judge | #5233 | **无人认领** | blocked（依赖 A01–A10 全 evidence_ready） | 必须独立于 A01/A04/A08 证据作者（opencode+Mimo2.6Flash）+ R1 发现者（codex） |
| A12 终审 | #5234 | **无人认领** | blocked（依赖 A11） | 评分仅 `[SOFT GATE]`；Hard Gate 只看事实 |
| Master | #5235 | codex | running | DAG 已对账（2026-10-10T09:33Z） |

## 3. 关键差距（SSOT 要求 vs 现状）

1. **A01 基线过期且格式不合规（首要差距）**。现有 run `20261009-a01-001`：run_id 格式（旧 vs §5.1 的 `iaia410-a01-<UTC ts>-<short>`）、manifest 形状（`schema_version: 1` vs `iaia410-manifest/v1`，缺 `repository.*` 40 位 commit 块、environment 全字段、datasets、逐命令 `stdout_sha256`）均不符合 SSOT；`worktree_clean: false`、冻结点 `bf48818cd4` 远早于当前 tip `91d0c6dd78`。**所有后续 A02–A12 证据都必须锚定一次合规重冻结**，否则 A11 "clean checkout 可重放"无从谈起。

2. **A02 缺失 = DAG 闸门**。A03–A10 的 in_scope/out_of_scope/deferred 裁决没有机器可读载体；OMP 已认领但零交付物。

3. **7/12 工作包无人认领**（A03/A05/A06/A09/A10 明确无人，A07 待接手确认，A11/A12 待定）。其中 **A06 是全局关键路径**：唯一的 P0 否决项 R1 在 A06 覆盖清单里。

4. **R1（CLUSTERED 绕过 WAL）只有静态证据**，finding 状态停留在 `suspected`。静态证据已充分：`src/engine_create.rs:397` 路由、`src/execution_engine.rs:182,374` 纯内存 `RwLock<HashMap>`、`src/engine_dml.rs:2171-2182` INSERT 零 WAL 行条目、全仓库无加载路径。但 SSOT §6/§9 要求**动态重放**（INSERT → kill -9 → 重启 → 核对数据）才能转 `confirmed`。命中一票否决第 2 类（COMMIT 成功后数据丢失）→ **R1 未关闭，A12 只能出"不得晋级"结论**。整改二选一：
   - 方案 A：CLUSTERED 行变更走库感知 WAL + 启动加载（复用 #5208 DNAM 编码）
   - 方案 B：显式声明非持久化引擎 + 文档 + CREATE 告警

5. **A07 前置治理事故未闭环**：27 条 mutation spec 文件从未提交 → clean checkout 不可复现；需从 #5015/#5012 证据重建 + 入版本控制 + 冻结 hash，然后逐条定性 11 条 REVIEW 债务和 92 SURVIVED（KILLED/EQUIVALENT/UNREACHABLE/UNRESOLVED 四选一，**禁止断言洗白**）。

6. **A08 尾巴**：#5262（22 个 STILL_FAILING 真缺陷）仍 open；ENVIRONMENT 18（TPC-H fixture）与 WIRE_HARNESS 27（共享 harness panic）有明确复验条件未触发。

7. **A04 两个缺口**：MySQL server 引擎 `not_comparable`（协议允许，但 A11 应复核可比性探测）；7 条 CLI 列名暴露内部命名/AST 的缺陷按 §10 应立独立缺陷 Issue（目前只写在矩阵报告里）。

8. **A11 独立性约束**：Judge 不能是 A01/A04/A08 证据作者（opencode+Mimo2.6Flash）或 R1 发现者（codex）——**需另派 agent**，clean checkout + SHA256SUMS 校验 + 重跑 P0/否决类。

9. **门禁现状（已验证）**：上次阻断 `cargo test --all-features`（`backup_restore_test.rs:520` WalEntry 缺 database_name，#5261）已由 #5278 修复——我刚在该测试 binary 上实跑 51/51 PASS（tip `91d0c6dd78`）；SSOT 分支推送禁令随之解除且分支已合入。**当前无已知门禁阻断**，但重冻结时应按 §5 跑全量门禁留证。

10. **工作区脏**：用户未提交 WIP（`crates/cli/src/main.rs`、`soak.rs`）+ AOCI 文件（`aoci.code.txt` 等）未提交。**重冻结必须在干净 commit 上做**（WIP 另行保留，绝不丢弃），`worktree_clean: true` 才能支撑 clean-checkout 重放。

## 4. 已知 P0 发现与缺陷 Issue 对照

| 发现 | 状态 | 载体 |
|---|---|---|
| **R1**：CLUSTERED 表完全绕过 WAL（INSERT 成功→kill -9→静默丢数据） | `suspected`（静态充分，待 A06 动态转 `confirmed`）→ 命中否决第 2 类，阻塞 A12 晋级结论 | 待 A06 立 finding YAML + 缺陷 Issue |
| `crates/transaction` 51 个 P0 SURVIVED 变异 | 输入就绪，待 A06 动态验证 / A07 分类 | PR #5264/#5265 已合；spec 文件重建是前置 |
| 22 个 STILL_FAILING B2 binary | `confirmed` 缺陷 | #5262（open） |
| WalEntry.database_name 编译债 | 已修复关闭 | #5261（closed，实测 51/51 PASS） |
| LIMIT offset,count 第二数静默丢弃 | 已修复关闭 | #5217（closed） |
| 7 条 CLI 列名暴露内部命名/AST | mismatch（非 silent），未立 Issue | A04 矩阵报告 §剩余 Mismatch |
| parallelism 静默降级 / 默认 /tmp 数据目录 | 未立 Issue | A10 整改分析已定位 |

## 5. 角色/分配缺口（需用户/总控裁决）

1. **A06 必须立即指派**——唯一否决项 R1 在此，且输入已齐（deepseek 51 P0 变异表 + crash 脚本）
2. A03/A05/A09/A10 无人认领；A07 的 mcode/Auditor C 接手需确认；A11 需指派独立 agent（排除 opencode+Mimo2.6Flash 与 codex）；A12 建议 Master 或新 agent
3. 外围 P0 仍未认领：#5113（CI fail-closed）、#5116（性能基线/CI）——W1 协调建议 OMP 优先
4. A02 是 A03–A10 的 DAG 闸门且零交付物，建议与 A06 并行最高优先级推进

**最短关键路径**：Step 1（A01 重冻结）→ Step 2（A02）→ Step 3（A03 登记 R1）→ Step 4 中 A06（R1 confirmed → 整改 → 回归）→ Step 6（A11 独立重放 R1）→ Step 8（A12）。其余工作包在 Step 4 内并行，但 A02 不落地则 A03–A10 的范围裁决无据可依。

## 6. deepseek-flash 在 Phase 1 的交付与移交

### 已合入 develop/v4.1.0

- **PR #5264** `test(transaction): 4 mutation-evidence counterexamples for cargo-mutants SURVIVED on sqlrustgo-transaction`
  - `crates/transaction/tests/audit_mutation_counters.rs`（4 active + 1 `#[ignore]`）
- **PR #5265** `reports: Phase 1 audit tooling plan + 92-SURVIVED evidence + 51-P0 risk triage`
  - `reports/sqlrustgo_audit_tooling.md`
  - `reports/SURVIVED_MUTANTS_EVIDENCE_v2.md`
  - `reports/M_REACHABILITY_EVIDENCE_v1.md`
  - `reports/SURVIVED_92_TRIAGE_v2.md`

### 关键数字（HEAD `89dcf74188`，现已并入 develop/v4.1.0）

| 指标 | 值 |
|---|---:|
| 总变异 | 293 |
| CAUGHT | 182 |
| SURVIVED | 92 |
| TIMEOUT | 4 |
| UNVIABLE | 15 |
| 有效杀伤率 | 66.42% |
| SURVIVED → P0（持久性/数据正确性） | **51** |
| SURVIVED → P1（并发/行为） | 30 |
| SURVIVED → P2（F1 orphan） | 10 |
| FIXED（已覆盖） | 1 |

### out-of-scope（按 ADR-014 §D-05 单一负责人）

- 88 个剩余 SURVIVED 的反例测试 → mcode / Auditor C
- `crates/transaction/src/lock.rs`、`gid.rs` 等生产源码修改 → 任何 owner
- A02 范围矩阵 → OMP（已认领）
- A01 重冻结 → 原 A01 执行者或 OMP
- A11 独立 Judge → 另派 agent（排除 opencode+Mimo2.6Flash、codex）
- A12 终审 → Master 或新 agent

## 7. 来源 / 运行 / 时间 / 冲突解决（ADR-014 §5.4）

- **source_agent**: deepseek-flash（DeepSeek-Harness）
- **source_run**: audit-tooling-phase1-20261009
- **timestamp**: 2026-10-09T22:30Z
- **evidence_hash**: PR #5264（counterexamples）+ PR #5265（reports）+ `target/audit-phase1/mutants-tx-rerun-FINAL/{caught,missed,timeout,unviable}.txt`（293/293 原始输出）
- **conflict_resolution**: 按 ADR-014 §D-05 单一负责人，本报告**不裁决**，仅同步事实。R1 否决项 / A06 owner / A11 独立 Judge 全部交总控（#5235 / codex）裁决。