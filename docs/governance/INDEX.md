<!-- env:blocked:no-ci -->

# SQLRustGo 治理文档索引 (INDEX)

> **版本**: v3.8.0
> **最后更新**: 2026-06-05
> **维护者**: Hermes Agent
> **状态**: ACTIVE — 每个 v<X.Y.Z> GA 前需刷新

---

## 0. 如何使用本索引

| 你是... | 读什么 |
|--------|--------|
| **新加入项目的 AI Agent / 开发者** | 1.1 (AGENTS.md) → 1.2 (5-原则) → §2 治理资产 → §3 Gate 脚本 → §4 ADR |
| **版本 GA 前准备** | 4.1 (GA 治理示范) → 4.2 (回归审计模板) → §3.2 治理流程 |
| **Issue 创建/关闭/跟踪** | 4.3 (ISSUE_CLOSING_VERIFICATION) + 4.4 (Issue SLA Pattern) + 4.5 (ADR-010 跨版本债务) |
| **债务跟踪/审计** | 5.1 (ADR-011 状态机) + 5.2 (debt-registry.yaml) + 4.2 (LEGACY_AUDIT_CHECKLIST) |
| **Gate 脚本/门禁检查** | 6.1 (GA_SCRIPTS_SKILLS_REGISTRY) + 6.2 (GATE_CI_CD) + 6.3 (GATE_CONDITIONS) |
| **Pattern/ADR/Template 选择** | §4 (按需), §5 (ADR), §6 (Pattern), §7 (Template) |

---

## 1. 项目基础 (P0 必读)

| 文档 | 用途 | 严重度 |
|------|------|--------|
| `../AGENTS.md` | 强制规则 (worktree, 5 步流程, pre-commit 邮箱, 中文沟通) | P0 |
| `../.claude/CLAUDE.md` | Claude Code 专项指引 | P1 |
| `ENGINEERING_EVOLUTION_STANDARD.md` | CMM 4+ 轻量版标准 (5 原则) | P0 |
| `ISSUE_CLOSING_VERIFICATION.md` | Issue 关闭 5 步强制流程 | P0 |
| `DOC_CHECK_CORRECTION_RULES.md` | 文档修改 5 步流程 | P0 |
| `DOC_CHECK_CORRECTION_WORK_RECORD.md` | 5 步流程工作记录模板 | P0 |
| `ANTI_FABRICATION_POLICY.md` | Truthfulness 反伪造型 | P0 |
| `CONTRIBUTING.md` | 贡献流程 | P1 |
| `RELEASE_LIFECYCLE.md` | 版本生命周期 (Draft → Dev → Alpha → Beta → RC → GA) | P1 |
| `RELEASE_POLICY.md` | 发布策略 | P1 |
| `release_process.md` | 发布流程具体步骤 | P1 |
| `tasks/TASK_REGISTRY.md` | 任务注册表 | P2 |

---

## 2. 治理资产 (本次会话新增 — v3.8.0 GA 示范)

| 文档 | 类型 | 状态 | 用途 |
|------|------|------|------|
| **`GA_GOVERNANCE_DEMO_v3.8.0.md`** | 主报告 | ACTIVE | v3.8.0 GA 7h 治理示范 (14 PR + 12 Issue + 3 follow-up) |
| **`GA_SCRIPTS_SKILLS_REGISTRY.md`** | 工具注册表 | ACTIVE | 78 gate 脚本 (v3.9.0 audit 重统计) + 16 skills + 27 Gitea API 模板 + 18 rg/grep 模板 |
| **`REGRESSION_AUDIT_2026-06-05.md`** | 回归报告 | ACTIVE | 本次会话回归检查 (在 `docs/releases/v3.8.0/historical/`) |
| `docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md` | 审计报告 | ACTIVE | 13 项治理债务审计 (271 行) |

---

## 3. Gate 脚本与门禁

### 3.1 12 个 Active Gate 脚本 (v3.9.0 加 G1)

| 脚本 | 用途 | 入口 |
|------|------|------|
| `scripts/gate/check_rc_ga_gate.sh` | **D1-D5 主 gate** (Alpha/Beta/RC/GA 5 维) | `bash .../check_rc_ga_gate.sh ga` |
| `scripts/gate/check_full_gate_verification.sh` | **D9 全 8 维度综合验证** | `bash .../check_full_gate_verification.sh` |
| `scripts/gate/check_int_debt.sh` | **D7 INT-1~4 跨版本债务** (修 #3100) | `bash .../check_int_debt.sh` |
| `scripts/gate/check_arch_sem_debt.sh` | **D8 ARCH-1~3 + SEM-1~4** | `bash .../check_arch_sem_debt.sh` |
| `scripts/gate/check_cross_version_debt.sh` | **72 跨版本债务项 + Part 5 Code Reality Check** (修 #3106) | `bash .../check_cross_version_debt.sh` |
| `scripts/gate/check_arch2_no_bypass.sh` | **ARCH-2 storage.* bypass** (修 #3101) | `bash .../check_arch2_no_bypass.sh` |
| `scripts/gate/check_g1_tpch_baseline.sh` | **G1 TPC-H 22/22 不可退化** (v3.9.0 #3186 baseline hash) | `bash .../check_g1_tpch_baseline.sh` |
| `scripts/gate/audit_testing.sh` | **D5.5 TEST_PLAN ↔ Cargo.toml 16 个 [[test]] 对齐** | `bash .../audit_testing.sh v3.8.0 ga <out>` |
| `scripts/gate/check_docs_links.sh` | markdown 链接检查 | `bash .../check_docs_links.sh [--all]` |
| `scripts/gate/check_docs_consistency.sh` | 版本状态/链接/历史一致性 | `bash .../check_docs_consistency.sh` |
| `scripts/gate/check_docs.sh` | 文档完整性 | `bash .../check_docs.sh` |
| `scripts/gate/check_test_inventory.sh` | **D6b 62 测试文件 inventory** | `bash .../check_test_inventory.sh` |

**完整注册表 (78 脚本 + Active/Internal/Deprecated 分类)**: `GA_SCRIPTS_SKILLS_REGISTRY.md` §1 (v3.9.0 audit 重统计 2026-06-11)

### 3.2 治理流程图

```
[PR 创建]
  ↓ CI 触发 (ci.yml 13 个 gate)
  ↓ D1-D9 8 维门禁 (check_rc_ga_gate.sh ga)
  ↓ D7/D8 (check_int_debt.sh + check_arch_sem_debt.sh)
  ↓ D6b Test Inventory
  ↓ 文档治理 (check_docs_links + check_docs_consistency)
  ↓
[合并到 develop/v<X.Y.Z>]
  ↓ (每个 PR 单独 worktree + 5 步文档流程)
  ↓
[RC 阶段] → [GA 阶段]
  ↓ (LEGACY_AUDIT_CHECKLIST.md Phase 0-5)
  ↓
[GA 治理示范报告] → GA_GOVERNANCE_DEMO_v<X.Y.Z>.md
```

---

## 4. Pattern (工作流模式)

| Pattern | 触发 | 内容 |
|---------|------|------|
| **`PATTERN_LEGACY_AUDIT_FOLLOWUP.md`** | v<X.Y.Z> GA 前 1-2 周 | 4 阶段: 调研→跟踪→整改→验证 (本次会话 v3.8.0 示范) |
| **`PATTERN_FOLLOWUP_SLA.md`** | 创建 follow-up Issue | 4 强制字段 (owner/target_release/review_date/blocking) |
| `PATTERN_GATE_FALSE_POSITIVE.md` | Gate 报告声称 PASS 但实际 FAIL | 检测 + 解决 |
| `PATTERN_EVIDENCE_CHAIN.md` | Truthfulness 证据链 | 任何状态声明 |
| `PATTERN_LEGACY_RETIREMENT.md` | 旧代码/文档退役 | 清理过期资源 |
| `PATTERN_COVERAGE_DISPUTE.md` | 覆盖率争议处理 | 覆盖率异常 |
| `PATTERN_ARCHITECTURE_DEBT.md` | 架构债务处理 | 架构债务 |

---

## 5. ADR (架构决策记录)

| ADR | 标题 | 状态 |
|---|---|------|
| **ADR-001** | Truthfulness 框架 | ACCEPTED |
| **ADR-002** | Claim Registry | ACCEPTED |
| **ADR-003** | Decision Registry | ACCEPTED |
| **ADR-004** | Negative Evidence | ACCEPTED |
| **ADR-005** | Legacy Gate Retirement | ACCEPTED |
| **ADR-006** | TX-WAL Contract Deferral | ACCEPTED |
| **ADR-007** | WAL Architecture Clarification | ACCEPTED |
| **ADR-009** | G-01 Validation Chain Enforcement | ACCEPTED |
| ADR-010 | Cross-Version Debt Governance | ACCEPTED |
| ADR-010 | Ghost PR Resolution | ACCEPTED |
| **ADR-011** | Debt State Machine (7 状态) | **PROPOSED** (v3.9.0+ 升级) |
| ADR-011 | v3.8.0-1 TX-WAL Repair | PROPOSED |
| ADR-012 | SQLRustGo vs GMP Platform Scope | ACCEPTED |

---

## 6. Template (可复用模板)

| Template | 用途 |
|---------|------|
| **`LEGACY_AUDIT_CHECKLIST.md`** | 后续版本 (v3.9.0+, v4.0.0) 复用 checklist, 4 阶段 Phase 0-5 |
| `GOVERNANCE_COVERAGE_REPORT.md` | 治理覆盖报告 |
| `TEST_REVIEW_TEMPLATE.md` | 测试审查模板 |
| `DOC_CHECK_CORRECTION_WORK_RECORD.md` | 5 步文档流程工作记录 |
| `DOCUMENT_COMPLETENESS_CHECK.md` | 文档完整性检查 |
| `DOCUMENT_REVIEW_WORKFLOW.md` | 文档审查流程 |

---

## 7. 债务跟踪 (SSOT)

| 文件 | 用途 | 优先级 |
|------|------|--------|
| **`debt/debt-registry.yaml`** | 机器可读 SSOT (4 INT + 3 ARCH + 4 SEM + 10 F-XX + 5 F-XX + 4 Follow-up = 30 项) | P0 (单一真相来源) |
| `docs/releases/v3.8.0/debt/INT5_PLUS_DEBT_INVENTORY.md` | 68 旧债务 (v3.0.0 baseline) | P0 (Markdown 表) |
| `docs/releases/v3.8.0/debt/CROSS-VERSION-DEBT.md` | INT-1~4 状态 | P0 (Markdown 表) |
| `docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md` | INT 修复计划 (历史) | P1 (DEPRECATED 候选) |
| `docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | ARCH+SEM 修复计划 (历史) | P1 (DEPRECATED 候选) |
| `DEBT TRACKING.md` (顶层) | 6 项跨版本债务 | P1 (DEPRECATED 候选) |

**治理建议**: `debt-registry.yaml` 应作为 SSOT, 4 份 Markdown 债务文档用 yaml 生成 (避免漂移)。

---

## 8. AI Agent Skills (本次会话新增)

| Skill | 文件 | 用途 |
|-------|------|------|
| **subagent-driven 调研** | `GA_SCRIPTS_SKILLS_REGISTRY.md` §3 | 3 subagent 并行调研 节省 5h+ |
| **5 步文档流程** | `DOC_CHECK_CORRECTION_RULES.md` | 文档修改 1-5 步骤 |
| **Issue 关闭 PR 关联** | `ISSUE_CLOSING_VERIFICATION.md` | 关闭 Issue 必须有 PR 合并证据 |
| **Truthfulness 原则** | `ANTI_FABRICATION_POLICY.md` + `ADR-001` | 不编造数据 |
| **Debt Registry 同步** | `debt-registry.yaml` + `ADR-011` | SSOT 一致性 |

---

## 9. GA 治理资产 (本次会话新增)

| 资产 | 路径 | 状态 |
|------|------|------|
| **v3.8.0 GA 治理示范** | `GA_GOVERNANCE_DEMO_v3.8.0.md` | ACTIVE |
| **v3.8.0 回归审计** | `../docs/releases/v3.8.0/historical/REGRESSION_AUDIT_2026-06-05.md` | ACTIVE |
| **v3.8.0 13 项债务审计** | `../docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md` | ACTIVE |
| **7 状态债务机** | `adr/ADR-011-debt-state-machine.md` | PROPOSED (v3.9.0+ 升级) |
| **Debt Registry YAML** | `debt/debt-registry.yaml` | ACTIVE (SSOT) |

---

## 10. 快速链接 (按需跳转)

| 需求 | 链接 |
|------|------|
| 我是 AI Agent | `../AGENTS.md` + `DOC_CHECK_CORRECTION_RULES.md` + `ISSUE_CLOSING_VERIFICATION.md` |
| 我要 GA 治理 | `GA_GOVERNANCE_DEMO_v3.8.0.md` + `LEGACY_AUDIT_CHECKLIST.md` + `GA_SCRIPTS_SKILLS_REGISTRY.md` |
| 我要债务跟踪 | `debt/debt-registry.yaml` + `adr/ADR-011-debt-state-machine.md` + `patterns/PATTERN_FOLLOWUP_SLA.md` |
| 我要 Issue 跟踪 | `ISSUE_CLOSING_VERIFICATION.md` + `adr/ADR-010-cross-version-debt-governance.md` + `patterns/PATTERN_FOLLOWUP_SLA.md` |
| 我要 Gate 脚本 | `GA_SCRIPTS_SKILLS_REGISTRY.md` + `GATE_CI_CD.md` + `GATE_CONDITIONS.md` |
| 我要 ADR | `adr/` (12 份) |
| 我要 Pattern | `patterns/` (7 份) |
| 我要 Template | `templates/` (2 份) |

---

## 11. 维护与刷新

| 项目 | 值 |
|------|-----|
| 文档版本 | 1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | ACTIVE — 每 v<X.Y.Z> GA 前刷新 |
| 关联 | GA_GOVERNANCE_DEMO_v3.8.0.md, GA_SCRIPTS_SKILLS_REGISTRY.md |
| 已知问题 | 待修 34 个 P0 优化点 (见治理优化检查报告) |
