# v3.8.0 全面现状分析与整改方案

> **报告版本**: v1.0
> **生成日期**: 2026-06-03
> **分支基准**: `develop/v3.8.0` (HEAD: `651433468`)
> **分析人**: Claude (claude-macmini)
> **报告类型**: 治理审计 + 整改规划
> **任务来源**: 用户要求"全面分析 3.8.0 的现有文档和 ISSUE，了解进展情况"

---

## 一、回答用户的两个核心问题

### 1.1 目前是否还是没有完整功能的服务器？

**答案：是，仍不是完整功能的服务器。** 当前是"**功能可工作、架构未统一**"的半成品状态。

| 维度 | 状态 | 证据 |
|------|------|------|
| **基本 SQL 执行** | ✅ 可工作 | A2_TEST: 286 passed; 0 failed（核心 6 crate） |
| **WAL 持久化** | ✅ 可工作 | PR-830A~E, PR-830F, PR-2758 (SPEC-003) 已合并 |
| **触发器持久化** | ✅ 新增 | `feat(storage): FileStorage trigger persistence` (c8ee597ba) |
| **MySQL 协议** | ✅ 可工作 | `crates/server` mysql-server crate |
| **架构统一** | ❌ 未达成 | F-06 TransactionalFacade NOT_DONE |
| **双路径归零** | ❌ 未达成 | `eng.execute(raw_sql)` 仍存在测试代码（Alpha Stage Review） |
| **Router 解耦** | ❌ 未达成 | F-07 PR-810 NOT_DONE |
| **DML 完整 WAL 截获** | 🟡 部分 | F-09 PR-840 存储层完成（PR-2761），完整截获未完成 |
| **MVCC 基础** | ❌ 未达成 | PR-890 推迟到 v3.8.0+1 |
| **会话级事务** | 🟡 部分 | BEGIN/COMMIT/ROLLBACK 路由完成，Session Binding 未完成 |
| **AD-001 架构冻结** | 🟡 接近违反 | ExecutionEngine 1562 行（上限 1500） |

**核心问题清单**：

1. **执行路径未统一** — `mysql-server`（crates/server/）和 `LocalExecutor`（crates/executor/）两条路径并存
2. **F-06~F-15 关键 PR 缺失** — 9 个 NOT_DONE 计划 PR，其中 5 个是 GA 阻塞
3. **架构冻结部分违反** — `eng.execute(raw_sql)` 在测试代码中保留
4. **AD-001 行数限制边缘** — 1562 行 / 1500 上限
5. **集成债务跨版本累积** — INT-1~INT-4 自 v1.2.0 起跨 6 版本未根治
6. **F-09 完整实现缺失** — UPDATE replay 架构已修（PR-2755），但 DML 完整截获未完成
7. **RECOVERY-007 仍 IGNORE** — DELETE replay 未实现
8. **L2/L3 测试未执行** — Multi-path 执行一致性 + ACID 验证未跑

### 1.2 这个任务（当前分析任务）是什么？

**这是 v3.8.0 整改协调任务**（meta-task），不是代码功能。

| 属性 | 内容 |
|------|------|
| **任务类型** | 治理审计 + 整改规划 + 任务发布 |
| **交付物** | 本报告 + 8 个 Gitea ISSUEs |
| **后续流程** | AI Agent 认领并执行 |
| **不涉及** | 代码修改、Gate 执行、PR 提交 |
| **属于** | 项目治理层（非功能实现层） |

---

## 二、v3.8.0 现状综合分析

### 2.1 文档体系（已基本完整）

| 类别 | 文档 | 状态 |
|------|------|------|
| **版本计划** | `VERSION_PLAN.md` / `DEVELOPMENT_PLAN.md` / `ROADMAP.md` | ✅ 完整 |
| **门禁报告** | `RC_GA_GATE_REPORT.md` / `INTEGRATION_GATE_REPORT.md` / `beta/BETA_GATE_REPORT.md` | ✅ 完整 |
| **审计 Issue** | `ISSUE-2740~2743` | ✅ 完整 |
| **ADR** | `ADR-001~006` | ✅ 完整 |
| **阶段文档** | `alpha/`、`beta/`、`rc/`、`ga/` | ✅ 完整 |
| **特性清单** | `FEATURE_CHECKLIST.md` | ✅ 完整（含 DEFERRED 诚实记录） |
| **Cross-Version Debt** | `CROSS-VERSION-DEBT.md` | ✅ 框架已建，自动化缺失 |
| **DEFERRED PRs** | `DEFERRED_PRS.md` | ✅ 诚实记录 |
| **LEGACY_ISSUES** | `LEGACY_ISSUES.md` | ✅ 12 个可关闭 issue 已记录 |
| **CURRENT_VERSION.md** | 根目录 | ❌ 仍说 v2.8.0，未更新到 v3.8.0 |
| **ROADMAP.md** | 根目录 | ❌ 仍 v1.0（2026-03-06），未更新到 3.x |

### 2.2 近期活动（2026-05-25 ~ 2026-06-03）

**重大 PR**（按合并顺序）：

| PR | 主题 | 关联 Issue |
|----|------|-----------|
| #2754 | SPEC-001 B4 format truthfulness | - |
| #2756 | SPEC-002 PR-830F WAL lifecycle | F-16 |
| #2758 | SPEC-003 WAL replay encode updates | ISSUE-2740 |
| #2759 | suppress dead-code warnings | - |
| #2755 | PR-842 UPDATE replay 架构修复 | ISSUE-2741 |
| #2761 | F-09 UPDATE replay recovery (partial) | F-09 |
| **#2760** | **WAL 集成 + 契约测试套件** | **ISSUE-2743** |
| #2762 | SGL 阻塞性 Beta 门禁 | - |

**核心成果**：
- SGL-005 完整修复（5/5 PASS）
- WAL 持久化路径完整（FileStorage + WalStorage + RecoveryEngine）
- 触发器持久化（commit c8ee597ba）
- 3 个 SPEC 真化（format/lifecycle/replay）
- 19 个契约测试 PASS / 12 KNOWN_GAP

### 2.3 Gate 状态（RC 就绪）

| Gate | 状态 | 时间 |
|------|------|------|
| Alpha (A1-A6) | ✅ 10/10 PASS | 2026-05-31 |
| Beta (B1-B4 + B-F1~B-F7) | ✅ 11/11 PASS | 2026-05-31 |
| Integration (C-ARCH + SGL + WAL + Harness) | ✅ 4/4 PASS | 2026-06-01 |
| WAL Contract | ✅ 22/22 + 5/5 | 2026-06-01 |
| SGL-005 | ✅ 5/5 PASS | 2026-06-01 |
| RC/GA Unified | ✅ 5/5 PASS | 2026-06-01 |
| Clippy | ✅ 0 warnings | 2026-06-03 |
| Format | ✅ 0 diffs | 2026-06-03 |

**结论**：技术指标已通过，**但架构完整性未达成 v3.8.0 目标**（"Architecture Unification Release"）。

---

## 三、核心未解决问题（按优先级）

### 3.1 🔴 P0：阻塞 GA

#### A. ISSUE-2742 — WAL 架构澄清（4 个架构决策缺失）

| 问题 | 影响 | 阻塞 |
|------|------|------|
| ① DDL 是否走 WAL | 触发器持久化 / DDL 审计 | F-09 完整实现 |
| ② `is_wal_enabled()` 公开接口 | 应用层查询能力 | API 设计 |
| ③ DDL bypass 处理策略 | 架构冻结一致性 | AD-001 |
| ④ Readonly 模式行为 | 未来 v3.9 MVCC 基础 | 设计决策 |

**当前状态**：4 个问题均无 ADR 决策，F-09/F-10/F-11 等依赖项均 BLOCK。

#### B. ISSUE-2740 — Crash Recovery 正确性未证实

| 子问题 | 现状 | 证据 |
|--------|------|------|
| E-1: RECOVERY-001/002/003 证据缺失 | 已有 cargo test 输出（22 PASS） | WAL Contract 报告 |
| E-2: `crash_recovery_test.rs` 用 MemoryStorage | 仍存在 | `tests/crash_recovery_test.rs:10-12` |
| E-3: 触发器持久化路径 E2E 证明 | T-001/002/003 已加 | commit e5422d7ec |
| E-4: 跨存储引擎一致性 | 未验证 | - |

**当前状态**：**部分缓解**——核心 WAL 路径有证据，但触发器持久化路径需要更多 E2E 证明。

### 3.2 🟡 P1：高优先级

| 问题 | 状态 | 影响 |
|------|------|------|
| **F-06 TransactionalFacade** | ❌ NOT_DONE + DEFERRED | 架构冻结无验证锚点 |
| **F-09 PR-840 完整 DML 截获** | 🟡 部分（PR-2761 修存储层） | INSERT/UPDATE 截获未完整 |
| **双路径残留** | 仍存在 | `eng.execute(raw_sql)` 在测试代码 |
| **ExecutionEngine 1562 行** | 接近 1500 上限 | AD-001 边缘 |
| **ISSUE-2741 验证链** | G-01 提议 + TEST_REVIEW_TEMPLATE 8 维度 30 项 | 无自动化门禁 |
| **RECOVERY-007 (DELETE replay)** | 仍 #[ignore] | F-09 完整后解除 |
| **L2/L3 测试未执行** | Alpha Stage Review 识别 | 缺失执行 |

### 3.3 🟢 P2：中优先级

| 问题 | 状态 | 说明 |
|------|------|------|
| **ISSUE-2743 12 个契约 gap** | DEFERRED 到 v3.8.0+1 | TX-Lifecycle 4 + WAL Recovery 8；ADR-006 |
| **F-07~F-15 9 个未实现 PR** | "幽灵 PR" | DEFERRED_PRS.md 诚实记录 |
| **Cross-Version Debt 自动化** | 框架已建 | INT-1~INT-4 跨 6 版本未修 |
| **CURRENT_VERSION.md / ROADMAP.md 滞后** | 文档 | 仍 v2.8.0 / v1.0 |

---

## 四、整改方案（按执行顺序）

### 🎯 第一阶段：解决 GA 阻塞（1~2 天）

#### 任务 #2744：ADR-007 WAL 架构澄清
- **优先级**: P0
- **关联**: ISSUE-2742
- **目标**: 决策 4 个 WAL 架构问题
- **完成标准**:
  - [ ] `docs/governance/adr/ADR-007-wal-architecture-clarification.md` 创建
  - [ ] 决策 ① DDL 是否走 WAL
  - [ ] 决策 ② `is_wal_enabled()` 公开接口
  - [ ] 决策 ③ DDL bypass 处理策略
  - [ ] 决策 ④ Readonly 模式行为
- **建议 AI Agent**: architect (架构角色)
- **预计 PR**: ~50 行 + ADR 文档

#### 任务 #2750：ISSUE-2740 Crash Recovery 实证
- **优先级**: P0
- **关联**: ISSUE-2740
- **目标**: 用 FileStorage + WalStorage 完整跑 RECOVERY-001~008
- **完成标准**:
  - [ ] `crash_recovery_test.rs` 不再使用 MemoryStorage
  - [ ] 触发器持久化 E2E 证明（CREATE TRIGGER + INSERT + crash + recover）
  - [ ] 证据文件 `docs/audit/wal_invariant_report_v380_2026-06-XX.md` 含 commit SHA + cargo test 输出
  - [ ] 更新 ISSUE-2740 状态为 `EMPIRICALLY_PROVEN`
- **建议 AI Agent**: qa-lead / recovery-engineer
- **预计工作量**: 1 天

#### 任务 #2745：F-06 TransactionalFacade STUB
- **优先级**: P1
- **关联**: DEFERRED_PRS.md F-06
- **目标**: 创建最小 STUB + 完整 ADR 说明
- **完成标准**:
  - [ ] `crates/executor/src/transactional_facade.rs` 创建（~30 行 stub）
  - [ ] 接口定义（基于 PR-800F spec）
  - [ ] `pub use` 暴露给 server crate
  - [ ] ADR-008 创建说明 stub 状态
- **建议 AI Agent**: backend-engineer
- **预计工作量**: 半天

### 🎯 第二阶段：填补架构缺口（3~5 天）

#### 任务 #2746：F-09 PR-840 完整 DML 截获
- **优先级**: P1
- **关联**: PR-2761 后续, ISSUE-2740
- **目标**: 完整 DML（INSERT/UPDATE/DELETE）走 WAL+WriteBuffer
- **完成标准**:
  - [ ] 移除 RECOVERY-007 的 `#[ignore]`
  - [ ] INSERT/UPDATE/DELETE 全部经过 TransactionalFacade
  - [ ] 22/22 RECOVERY 测试 + 31/31 contract tests 全部 PASS
- **建议 AI Agent**: storage-engineer + recovery-engineer
- **预计工作量**: 3 天

#### 任务 #2747：G-01 验证链强制门禁
- **优先级**: P1
- **关联**: ISSUE-2741, TEST_REVIEW_TEMPLATE
- **目标**: 8 维度 30 项检查自动化
- **完成标准**:
  - [ ] `scripts/gate/check_validation_chain.sh` 创建
  - [ ] 接入 Beta/RC Gate
  - [ ] ISSUE-2741 状态更新为 `ENFORCED`
- **建议 AI Agent**: governance-engineer
- **预计工作量**: 2 天

#### 任务 #2748：Cross-Version Debt 自动化跟踪
- **优先级**: P1
- **关联**: CROSS-VERSION-DEBT.md
- **目标**: 跨版本债务累积检测
- **完成标准**:
  - [ ] `scripts/gate/check_cross_version_debt.sh` 创建
  - [ ] GA Gate 报告新增 "Cross-Version Debt Delta" 章节
  - [ ] INT-1~INT-4 状态机维护
- **建议 AI Agent**: governance-engineer
- **预计工作量**: 2 天

### 🎯 第三阶段：治理债务（5~7 天）

#### 任务 #2749：F-07~F-15 Ghost PR 决策
- **优先级**: P2
- **关联**: FEATURE_CHECKLIST.md, DEFERRED_PRS.md
- **目标**: 9 个未实现 PR 逐个决定（实现/正式 defer/取消）
- **完成标准**:
  - [ ] 每个 F-feature 单独 ADR 决策
  - [ ] DEFERRED_PRS.md 更新含重启时间表
  - [ ] 没有"幽灵 PR"
- **建议 AI Agent**: release-manager
- **预计工作量**: 3 天

#### 任务 #2751：ISSUE-2743 v3.8.0+1 契约 Gap 修复计划
- **优先级**: P2
- **关联**: ISSUE-2743, ADR-006
- **目标**: 12 个 gap 的修复时间表 + 设计决策
- **完成标准**:
  - [ ] `docs/releases/v3.8.0/POST_GA_PLAN.md` 创建
  - [ ] 12 gap 按 2 类（TX-Lifecycle 4 + WAL Recovery 8）分别给方案
  - [ ] 预计 v3.8.0+1 实现时间表
- **建议 AI Agent**: architect + qa-lead
- **预计工作量**: 2 天

### 🎯 第四阶段：文档与发布（并行）

#### 任务 #2752：文档同步（CURRENT_VERSION.md / ROADMAP.md）
- **优先级**: P2
- **目标**: 更新到 v3.8.0 实际状态
- **完成标准**:
  - [ ] `CURRENT_VERSION.md` 状态改为 v3.8.0-Alpha
  - [ ] `ROADMAP.md` 添加 3.x 章节
  - [ ] 链接验证
- **建议 AI Agent**: doc-writer

---

## 五、推荐执行顺序

```
[Day 1] Task #2744 (ADR-007) ── 决策先行
         ↓
[Day 1-2] Task #2750 (ISSUE-2740 实证) ── 并行
[Day 1] Task #2745 (F-06 STUB) ── 并行
         ↓
[Day 3-5] Task #2746 (F-09 完整 DML) ── 依赖 ADR-007
[Day 3-4] Task #2747 (G-01 强制) ── 并行
[Day 4-5] Task #2748 (Cross-Version Debt) ── 并行
         ↓
[Day 6-8] Task #2749 (Ghost PR 决策)
[Day 7-8] Task #2751 (v3.8.0+1 计划)
         ↓
[Day 9+] Task #2752 (文档同步) + RC Gate
```

**总预计工作量**: 8~10 天

---

## 六、AI Agent 认领协议

请查阅: [`docs/audit/AI_AGENT_TASK_CLAIM_PROTOCOL.md`](./AI_AGENT_TASK_CLAIM_PROTOCOL.md)

**认领流程**:
1. AI Agent 找到匹配的 Task #2744~#2752
2. 在对应 Gitea Issue 添加 `<!-- claim: agent-role -->` 注释
3. 创建对应工作分支：`feature/task-2744-wal-architecture-adr`
4. 完成后关闭 Issue（必须有 PR 合并）

---

## 七、引用文档清单

| 文档 | 路径 |
|------|------|
| v3.8.0 Version Plan | `docs/releases/v3.8.0/VERSION_PLAN.md` |
| v3.8.0 Development Plan | `docs/releases/v3.8.0/DEVELOPMENT_PLAN.md` |
| v3.8.0 Roadmap | `docs/releases/v3.8.0/ROADMAP.md` |
| v3.8.0 Feature Checklist | `docs/releases/v3.8.0/FEATURE_CHECKLIST.md` |
| v3.8.0 RC/GA Gate Report | `docs/releases/v3.8.0/RC_GA_GATE_REPORT.md` |
| v3.8.0 Integration Gate Report | `docs/releases/v3.8.0/INTEGRATION_GATE_REPORT.md` |
| SGL-005 Audit | `docs/releases/v3.8.0/SGL-005_STORAGEBYPASS_AUDIT.md` |
| DEFERRED PRs | `docs/releases/v3.8.0/DEFERRED_PRS.md` |
| Cross-Version Debt | `docs/releases/v3.8.0/CROSS-VERSION-DEBT.md` |
| LEGACY Issues | `docs/releases/v3.8.0/LEGACY_ISSUES.md` |
| Issue 2740 (Crash Recovery) | `docs/audit/issues/ISSUE-2740_crash_recovery_unverified.md` |
| Issue 2741 (Validation Chain) | `docs/audit/issues/ISSUE-2741_validation_chain_missing.md` |
| Issue 2742 (WAL Architecture) | `docs/audit/issues/ISSUE-2742_wal_architecture_clarification.md` |
| Issue 2743 (TX+WAL Contract Gaps) | `docs/audit/issues/ISSUE-2743_tx_wal_contract_gaps.md` |
| ADR-006 (Deferral) | `docs/governance/adr/ADR-006-tx-wal-contract-deferral.md` |
| Doc Correction Rules | `docs/governance/DOC_CHECK_CORRECTION_RULES.md` |
| Gate Evidence | `artifacts/gate/v3.8.0/` |

---

## 八、变更历史

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| v1.0 | 2026-06-03 | Claude (claude-macmini) | 初始版本：v3.8.0 全面现状分析 + 整改方案 + 9 个 Task 发布 |
