# SQLRustGo Governance Compliance Report

> **版本**: v3.9.0 (v3.8.0 baseline + 2026-07-01 re-evaluation)
> **评估日期**: 2026-06-04 (v3.8.0); 2026-07-01 (v3.9.0 re-evaluation, see §7)
> **维护人**: Hermes Agent
> **状态**: ACTIVE (v3.9.0 era, §1-§6 反映 v3.8.0 baseline; §7 是 v3.9.0 增量)
> **2026-07-01 修订者**: claude-macmini (governance audit 整改)

---

## 1. 治理规则执行状态

| 规则 | 检查脚本 | 执行频率 | 上次执行 | 状态 | 备注 |
|------|---------|---------|---------|------|------|
| B4 Format | `check_b4_format.sh` | 每次 PR | 2026-06-04 | ✅ | |
| Evidence Binding | `check_evidence_binding.sh` | 每次 PR | 2026-06-04 | ✅ | |
| Anti-Fabrication | `check_anti_fabrication.sh` | **每次 PR（新增）** | 2026-06-04 | ✅ | v3.8.0+ |
| Doc Completeness | `check_docs_consistency.sh` | 每次 PR | 2026-06-04 | ✅ | 已扩展 CHECK 6-8 |
| **Doc Completeness (Dedicated)** | `check_document_completeness.sh` | **每次 PR（新增）** | 2026-06-04 | ✅ | v3.8.0+ — Issue #2941 supplement, 独立检查 8 mandatory docs 存在性 + 大小 + 内部链接 |
| Coverage Gate | `check_coverage.sh` | 每次 PR | 2026-06-04 | ✅ | |
| Security Check | `check_security.sh` | 每次 PR | 2026-06-04 | ✅ | |
| Alpha Gate | `check_alpha_gate.sh` | 合并前 | 2026-06-04 | ✅ | |
| Beta Gate | `check_beta_gate.sh` | 合并前 | 2026-06-04 | ✅ | |
| RC/GA Gate | `check_rc_ga_gate.sh` | 发布前 | 2026-06-04 | ✅ | |
| Integration Gate | `check_integration_gate.sh` | 发布前 | 2026-06-04 | ✅ | |
| Anti-Fabrication Policy | ANTI_FABRICATION_POLICY.md | **门禁中（新增）** | 2026-06-04 | ✅ | v3.8.0+ |

---

## 2. 门禁有效性矩阵

| 门禁维度 | 检查项数 | 通过数 | 执行率 | 状态 |
|---------|---------|--------|--------|------|
| 代码质量 (A1-A5) | 10 | 10 | 100% | ✅ |
| Beta 功能覆盖 (B1-B4) | 11 | 11 | 100% | ✅ |
| RC/GA (D1-D5) | 5 | 5 | 100% | ✅ |
| Integration | 4 | 4 | 100% | ✅ |
| RECOVERY | 22 | 22 | 100% | ✅ |
| **文档治理 (CHECK 6-8)** | 3 | 3 | 100% | ✅ **新增** |

---

## 3. 文档完整性

| 强制文档 | 状态 | 检查 |
|---------|------|------|
| README.md | ✅ | check_docs_consistency.sh CHECK 6 |
| CHANGELOG.md | ✅ | check_docs_consistency.sh CHECK 2/6 |
| RELEASE_NOTES.md | ✅ | check_docs_consistency.sh CHECK 6 |
| MIGRATION_GUIDE.md | ✅ | check_docs_consistency.sh CHECK 6 |
| DEPLOYMENT_GUIDE.md | ✅ | check_docs_consistency.sh CHECK 6 |
| DEVELOPMENT_GUIDE.md | ✅ | check_docs_consistency.sh CHECK 6 |
| TEST_PLAN.md | ✅ | check_docs_consistency.sh CHECK 6 |
| TEST_MANUAL.md | ✅ | check_docs_consistency.sh CHECK 6 |
| EVALUATION_REPORT.md | ✅ | check_docs_consistency.sh CHECK 6 |
| FEATURE_MATRIX.md | ✅ | check_docs_consistency.sh CHECK 7 |
| SECURITY_POLICY.md | ✅ | check_docs_consistency.sh CHECK 8 |
| COMPATIBILITY_MATRIX.md | ✅ | check_docs_consistency.sh CHECK 6 |
| SUPPORT_MATRIX.md | ✅ | check_docs_consistency.sh CHECK 6 |

**完成率: 13/13 = 100%**

---

## 4. Anti-Fabrication 检查结果

| 检查项 | 状态 |
|--------|------|
| Gate 报告数字 vs 实际测试输出一致性 | ✅ PASS |
| Gate 报告 PASS 声明 vs 实际状态 | ✅ PASS |
| 文档代码块语法正确性 | ✅ PASS |
| Gate 报告 provenance 元数据 | ✅ PASS |

**结果: 0 errors, 0 warnings**

---

## 5. 债务治理

| 指标 | v3.8.0 (2026-06-04) | v3.9.0 (2026-07-01) | Δ |
|------|---------------------|---------------------|---|
| 债务总项 | 68 | 72 | +4 (GA-P0) |
| 已关闭 | 49 (72%) | 49 (68%) | 比例下降 (分母变大) |
| 部分关闭 | 8 (12%) | 8 (11%) | -1pp |
| 开放 | 11 (16%) | 13 (18%) | +2 (GA-P0 OPEN) |
| BLOCKED | (未计) | 2 (3%) | new metric |

详见: `docs/governance/debt/debt-registry.yaml` (SSOT, 机器可读)

---

## 6. 改进项

### 已完成

- [x] `check_anti_fabrication.sh` 创建并集成到 CI
- [x] `check_docs_consistency.sh` 扩展 CHECK 6-8（v3.8.0 强制文档）
- [x] 13 项强制文档全部补充完整

### 待完成 (v3.8.0 baseline, 2026-06-04)

- [ ] `check_anti_fabrication.sh` 集成到 `.gitea/workflows/ci.yml` postcheck
- [ ] 债务追踪增强（SpecFile/TestFile/PR 列）
- [ ] SPEC 文件头部增强（Debt ID/Status）

---

## 7. v3.9.0 增量评估 (2026-07-01)

本节记录 v3.9.0 时期相对 v3.8.0 baseline 的治理规则增量/变化。

### 7.1 新增 ADR (5 个)

| ADR | 标题 | 状态 | 影响 |
| --- | --- | --- | --- |
| ADR-006a | Meta-Governance Framework (P11-P15) | ACCEPTED | gate 自验证 + DRIFT 处理 + oracle 强制 |
| ADR-007a | 5-PR Truthfulness Recovery Sequence | ACCEPTED | Sprint 8 期间 truthfulness 修复流程 |
| ADR-008 | Test Claim Transparency (P16) | ACCEPTED | gate test 0-`#[ignore]` 政策 |
| ADR-011a | Cross-Version Debt State Machine | ACCEPTED | 7 状态生命周期 (取代 v3.8.0 4 状态) |
| ADR-014 | Multi-AI Coordination | ACCEPTED | 5 evidence fields, 冲突解决协议 |

### 7.2 新增治理资产

| 资产 | 路径 | 用途 |
| --- | --- | --- |
| 5-step doc check (7 步) | `DOC_CHECK_CORRECTION_RULES.md` | 改文档前必须实跑 gate 验证 |
| Issue 关闭 PR 关联强制 | `ISSUE_CLOSING_VERIFICATION.md` | 关闭 issue 必须有 PR 合并证据 |
| Truthfulness 强制清单 (P0) | `AGENTS.md` §"强制 governance 阅读清单" | 7 份必读 |
| 债务注册表 (机器可读) | `debt/debt-registry.yaml` | SSOT 取代 Markdown 散落 |
| ADR 重复编号 a/b 消歧 | `adr/INDEX.md` §Numbering notes | 4 对重复 ADR 显式标注 |

### 7.3 关键 gate 状态 (2026-07-01 本机)

| Gate | 状态 | 备注 |
| --- | --- | --- |
| `check_arch_invariants.sh` | ✅ 5/5 PASS | 含 C-ARCH-05 (execution_engine.rs 1471/1500) |
| `check_arch3_no_bypass.sh` | ✅ PASS | ARCH-3 VtuGuard 主路径 |
| `check_integration_gate.sh` | ✅ 4/4 PASS | SGL-001..005 + WAL lifecycle + C-ARCH-05 |
| `check_architecture_freeze.sh` | A7-3 PASS | ExecutionEngine 1471 < 1500 (AD-001 目标) |

### 7.4 待办 (新增, v3.9.0 增量)

- [ ] **P0**: ADR 重复编号正式消歧 (4 对 a/b 临时方案, v3.10 应正式重命名或合并)
- [ ] **P0**: 4 个 hardware-blocked GA-P0 项 (#3648, #3423, #3265, #3266) 已在 debt-registry.yaml 注册, 等待硬件调度
- [ ] **P1**: `GOVERNANCE_COMPLIANCE_REPORT.md` 此文档自身需要定期重跑 (上次完整评估 2026-06-04, §7 增量)
- [ ] **P1**: `debt-registry.yaml` 缺 v3.9.0 增量项 (INT-2/3 closure, ARCH-3 VtuGuard 完成 等) — 已通过 §7 + main INDEX addendum 补登记
- [ ] **P2**: 7 个 governance 文档 (CONTRIBUTING / RELEASE_LIFECYCLE / RELEASE_POLICY / DIRECTORY_POLICY) 仍是 v3.8.0 era, 应更新引用 ADR-006a/008/014

### 7.5 治理审计发现 (2026-07-01 claude-macmini 审计)

| 问题 | 严重度 | 状态 |
| --- | --- | --- |
| 4 对 ADR 编号冲突 (006/007/010/011) | P0 | ✅ a/b 消歧 (2026-07-01, 临时方案) |
| 主 governance INDEX 缺 ADR-008/013/014 | P0 | ✅ 已补 (2026-07-01) |
| main INDEX 自称 v3.8.0 但 ADR-007+ 是 v3.9.0 | P0 | ✅ 已更新 (2026-07-01) |
| debt-registry.yaml 缺 v3.9.0 GA-P0 4 项 | P0 | ✅ 已加 (2026-07-01) |
| 7 份 P0 governance 清单 AGENTS.md 与 main INDEX 引用脱节 | P1 | ⚠️ 部分 (ADR-008 等加入, 但 AGENTS.md 未重读) |
| 7 个 v3.8.0-era 治理文档 (CONTRIBUTING 等) 未更新 | P2 | ❌ 未修 |

---

*报告生成: `scripts/gate/check_docs_consistency.sh && scripts/gate/check_anti_fabrication.sh`*
*生成时间: 2026-06-04 (v3.8.0 baseline); 2026-07-01 (v3.9.0 §7 增量)*
