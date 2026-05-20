# v3.3.0 开发测试计划

> **版本**: v1.3
> **创建日期**: 2026-05-18
> **更新日期**: 2026-05-18
> **维护人**: hermes-agent
> **分支**: `develop/v3.3.0` (`c45fc660`)
> **起点**: `539f2957` (v3.2.0 GA release point)
> **Milestone**: v3.3.0-beta (id=14)
> **战略定位**: Industrial Trust Platform（工业级可信闭环平台）

---

## 一、版本目标与约束

### 1.1 核心目标

**v3.3.0 战略定位**：Industrial Trust Platform（工业级可信闭环平台）。

**版本演进**：
```
v3.2.x: 内核修复（覆盖率/性能/MySQL协议）
    ↓
v3.3.0: Trust Infrastructure（可信性工业闭环）
    ↓
v3.4.0: GMP Management Suite
    ↓
v3.5.0: AI Native GMP Platform
```

### 1.2 Trust Infrastructure 四层架构

```
┌─────────────────────────────────────────────────────────┐
│                    Trust Visualization                    │
│        (ComplianceDashboard, EvidenceChainViz)          │
├─────────────────────────────────────────────────────────┤
│                 Compliance Evidence Engine              │
│        (EvidencePackage, AuditManifest, HashProof)      │
├─────────────────────────────────────────────────────────┤
│                    Provenance Graph                      │
│         (Device→Operator→SOP→Batch→Deviation)        │
├─────────────────────────────────────────────────────────┤
│               Trust Infrastructure Kernel                 │
│  Performance Governance │ Crash Simulation │ WAL Formal │
└─────────────────────────────────────────────────────────┘
```

### 1.3 版本约束

| 约束 | 说明 |
|------|------|
| 必须从 GA 释放点出发 | `539f2957`（v3.2.0 GA tag） |
| 所有豁免必须关闭或续期 | EX-v320-xxx → v3.3.0 Alpha 前复审 |
| Coverage 以 L1 CRATES 为 SSOT | 统一测量标准，消除 85.81% vs 68.8% 矛盾 |
| 大内存测试在 Z6G4 执行 | 本机 Mac 禁止 cargo llvm-cov |

---

## 二、分支策略

```
develop/v3.3.0  ← 主开发分支
alpha/v3.3.0    ← Alpha 阶段（Trust Infrastructure 设计）
beta/v3.3.0     ← Beta 阶段（代码实现验证）
release/v3.3.0  ← 正式发布分支
main             ← GA 释放点（只接受 release/v3.3.0 PR）
```

### 分支同步规则

| 源 | 目标 | 方法 | 频率 |
|----|------|------|------|
| develop/v3.3.0 | alpha/v3.3.0 | PR merge | 按需 |
| alpha/v3.3.0 | beta/v3.3.0 | PR merge | Alpha Gate 通过后 |
| beta/v3.3.0 | release/v3.3.0 | PR merge | Beta Gate 通过后 |
| release/v3.3.0 | main | PR merge | GA Gate 通过后 |

---

## 三、Issue 状态总结

### 3.1 Trust Infrastructure P0/P1（已完成 8/8）

| # | Issue | 标题 | PR | 状态 | Crate |
|---|-------|------|-----|------|-------|
| #1235 | P0 | Performance Governance System | #1243 | ✅ | sqlrustgo-perf-baseline |
| #1236 | P0 | Crash Simulation Framework | #1244 | ✅ | sqlrustgo-crash-sim |
| #1237 | P0 | WAL Formal Verification | #1247 | ✅ | sqlrustgo-wal-verification |
| #1238 | P0 | Compliance-as-Code Engine | #1248 | ✅ | sqlrustgo-compliance-engine |
| #1239 | P0 | Evidence Engine | #1249 | ✅ | sqlrustgo-evidence-engine |
| #1240 | P0 | Provenance Knowledge Graph | #1250 | ✅ | sqlrustgo-provenance-graph |
| #1241 | P1 | Workflow V2 | #1252 | ✅ | sqlrustgo-workflow-v2 |
| #1242 | P1 | Trust Visualization | #1253 | ✅ | sqlrustgo-trust-viz |

### 3.2 内核修复项（已完成 4/4）

| # | Issue | 标题 | PR | 状态 |
|---|-------|------|-----|------|
| #1196/#1197 | P0 | executor 模块拆分 + 覆盖率 | #1232 | ✅ |
| #1201 | P0 | MySQL Protocol 握手修复 | #1234 | ✅ |
| #1229 | P1 | TPC-H SF=1 22/22 验证 | — | ✅ |
| #1222 | P1 | Coverage SSOT 规范 | #1231 | ✅ |

### 3.3 AI 辅助开发工具（已完成 4/4）

| # | Issue | 标题 | PR | 状态 |
|---|-------|------|-----|------|
| #1225 | P1 | AI Test Generator | #1230 | ✅ |
| #1226 | P2 | Intelligent Test Selection | #1230 | ✅ |
| #1227 | P2 | Bug Triage AI | #1230 | ✅ |
| #1228 | P3 | Flaky Test Detector | #1230 | ✅ |

### 3.4 遗留 Issue（未完成）

| # | Issue | 标题 | 说明 |
|---|-------|------|------|
| #1196/#1197 | P0 | executor 覆盖率 70.7% < 85% | **未达标**，需继续提升 |
| #1224 | P2 | 72h 稳定性测试 | Z6G4 执行 |
| — | — | Coverage llvm-cov 验证 | B5 未完成 |

---

## 四、Alpha/Beta/GA Gate 状态

### 4.1 Alpha Gate（✅ 通过）

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| A1 | Build | `cargo build --release --workspace` | 编译通过 | ✅ PASS |
| A2 | Test | `cargo test --lib` | 全部通过 | ✅ PASS |
| A3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ✅ PASS |
| A4 | Format | `cargo fmt --all -- --check` | 通过 | ✅ PASS |
| A8 | SSOT Spec | `docs/governance/gate_spec_v330.md` | 存在 | ✅ PASS |

**Alpha Gate 本地检查**: 5/5 PASS ✅

### 4.2 Beta Gate（进行中）

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B1 | Build --release | `cargo build --all-features --release` | 编译通过 | ✅ PASS (Z6G4) |
| B3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ✅ PASS (Z6G4) |
| B4 | Format | `cargo fmt --all -- --check` | 通过 | ✅ PASS (Z6G4) |
| B6 | MySQL Handshake | `check_mysql_handshake.sh` | 连接成功 | ❌ FAIL (脚本不存在) |
| B7 | TPC-H SF=0.1 | `bash scripts/gate/check_tpch.sh` | 22/22 | ✅ PASS (Z6G4) |
| B5 | Coverage ≥65% | `cargo llvm-cov` | L1 CRATES | ⏳ 未完成 |

### 4.3 Beta Gate 脚本

```bash
# scripts/gate/check_beta_v330.sh
source ~/.cargo/env
cd /home/openclaw/dev/yinglichina163/sqlrustgo
git checkout beta/v3.3.0
bash scripts/gate/check_beta_v330.sh
```

### 4.4 GA Gate（未开始）

| # | 检查项 | 标准 | 状态 |
|---|--------|------|------|
| G1 | Performance regression | <5% vs baseline | ⏳ |
| G2 | Crash recovery | 10,000 次注入无损坏 | ⏳ |
| G3 | TLA+ WAL model check | 无 violation | ⏳ |
| G4 | Compliance evidence | 第三方审计通过 | ⏳ |
| G5 | Provenance graph | GMP 血缘查询 | ⏳ |

---

## 五、版本交付物

### 5.1 新增 Crates（8个）

| Crate | 版本 | 关键功能 |
|-------|------|----------|
| `sqlrustgo-perf-baseline` | 0.1.0 | BaselineDb + 9指标 + 10%回归阈值 |
| `sqlrustgo-crash-sim` | 0.1.0 | 11崩溃点 + RecoveryVerifier + CrashTestRunner |
| `sqlrustgo-wal-verification` | 0.1.0 | 8个WAL属性验证 + 5个不变量 + WALVerifier |
| `sqlrustgo-compliance-engine` | 0.1.0 | ComplianceRule + RuleEvaluator + GMP规则库 |
| `sqlrustgo-evidence-engine` | 0.1.0 | EvidencePackage + SHA-256哈希链 + EvidenceGenerator |
| `sqlrustgo-provenance-graph` | 0.1.0 | 14节点类型 + 9边类型 + DFS遍历 |
| `sqlrustgo-workflow-v2` | 0.1.0 | 状态机 + 7实例状态 + 6步骤状态 |
| `sqlrustgo-trust-viz` | 0.1.0 | ComplianceDashboard + EvidenceChainViz + TrustMetrics |

### 5.2 OO 文档（8个）

| 文档 | 路径 |
|------|------|
| Performance Governance | `oo/Trust-Infrastructure/PERFORMANCE_GOVERNANCE.md` |
| Crash Simulation | `oo/Trust-Infrastructure/CRASH_SIMULATION_FRAMEWORK.md` |
| WAL Formal Verification | `oo/Trust-Infrastructure/WAL_FORMAL_VERIFICATION.md` |
| Compliance-as-Code Engine | `oo/Compliance/COMPLIANCE_AS_CODE_ENGINE.md` |
| Evidence Engine | `oo/Compliance/EVIDENCE_ENGINE.md` |
| Provenance Knowledge Graph | `oo/Compliance/PROVENANCE_KNOWLEDGE_GRAPH.md` |
| Workflow V2 | `oo/GMP-Management/WORKFLOW_V2.md` |
| Trust Visualization | `oo/GMP-Management/TRUST_VISUALIZATION.md` |

---

## 六、未完成工作（Beta → GA）

### 6.1 高优先级

| # | 任务 | 说明 | 估计工时 |
|---|------|------|----------|
| B5 | Coverage llvm-cov 验证 | executor ≥85%，其他 ≥65% | 2h (Z6G4) |
| B6 | MySQL Handshake 验证 | `check_mysql_handshake.sh` 创建并通过 | 1h |
| #1224 | 72h 稳定性测试 | Z6G4 执行 | 72h |

### 6.2 中优先级

| # | 任务 | 说明 | 估计工时 |
|---|------|------|----------|
| #1196 | executor 覆盖率提升 | 70.7% → 85% | 2周 |
| GA-G2 | Crash Simulation 集成测试 | 1000次注入无损坏 | 1周 |

### 6.3 v3.3.0 可选增强（不阻塞 GA）

| # | 任务 | 说明 |
|---|------|------|
| — | Evidence Engine CLI 导出 | `audit-chain-export` 工具 |
| — | Trust Visualization 基础版 | 审计链可视化原型 |

---

## 七、豁免状态

| ID | 豁免项 | 复审条件 | 状态 |
|----|--------|----------|------|
| EX-v320-001 | executor 覆盖率 70.7% | ≥85% | ⚠️ 未关闭，需续期 |
| EX-v320-002 | MySQL Protocol 握手 | 连接成功 | ✅ PR #1234 已修复 |
| EX-v320-003 | TPC-H SF=1 数据缺失 | 22/22 通过 | ✅ 已通过 |
| EX-v320-004 | Sysbench 服务器环境 | 72h 无崩溃 | ⚠️ 未测试 |

---

## 八、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-05-18 | 初始版本 |
| 1.1 | 2026-05-18 | 新增 AI 辅助开发规划 |
| 1.2 | 2026-05-18 | 战略重构：Industrial Trust Platform |
| 1.3 | 2026-05-18 | Alpha/Beta 实际结果更新，8/8 Trust Infrastructure 实现完成 |

---

*文档版本: v1.3*
*更新: hermes-agent*
*日期: 2026-05-18*
