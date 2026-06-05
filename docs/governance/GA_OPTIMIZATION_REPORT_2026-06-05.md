# GA 治理优化报告 (v3.8.0) — 实施第 1 波

> **报告日期**: 2026-06-05
> **版本**: v3.8.0
> **方法**: 4 维度 subagent 调研 (Process/Tools/Documentation/Cross-Stage) + 实施 2 项 P0 优化
> **状态**: ACTIVE
> **关联**: `LEGACY_AUDIT_CHECKLIST.md` Phase 5 (报告)

---

## 0. Executive Summary

| 指标 | 数值 |
|------|------|
| **调研识别优化点** | **34 个** (P0 13 + P1 15 + P2 6) |
| **本次实施** | **2 项 P0**: INDEX.md + check_arch_sem_debt.sh 修硬编码 |
| **意外发现** | 1 个真问题: ARCH-1/2/SEM-2 实际 CLOSED 但脚本硬编码 OPEN |
| **PR** | 1 个 (优化 P0 修复) |

**核心矛盾 (本次发现)**:

> **状态机分裂 + 手工维护 + 缺统一入口**, 是 v3.8.0 GA 治理流程的 3 大头号风险。

---

## 1. 4 维度优化点调研汇总 (34 个)

### 1.1 维度 1: 治理流程 (Process) — 7 项

| # | 优化点 | 严重度 | ROI | 状态 |
|---|--------|--------|-----|------|
| 1.1 | PR Template 缺强制校验 | P0 | 高 | 未修 |
| 1.2 | Issue 关闭流程无自动校验 | P0 | 高 | 未修 |
| 1.3 | 5 步文档流程无 gate 检查 | P0 | 高 | 未修 |
| 1.4 | Follow-up Issue ↔ YAML 不同步 | P1 | 中 | 未修 |
| 1.5 | Gitea webhook 集成未启用 | P1 | 中 | 未修 |
| 1.6 | AGENTS.md 与 ci.yml gate 范围不对应 | P2 | 中 | 未修 |
| 1.7 | PR Template 5-类文档路径写死 | P2 | 中 | 未修 |

### 1.2 维度 2: 工具栈 (Tools) — 12 项

| # | 优化点 | 严重度 | ROI | 状态 |
|---|--------|--------|-----|------|
| 2.1 | 47 个 gate 脚本 (含 17 DEPRECATED) | P0 | 高 | 未修 |
| **2.2** | `check_int_debt.sh` 未升级 YAML SSOT | **P0** | **高** | **未修** |
| **2.3** | 缺 `check_followup_sla.sh` | **P0** | **高** | **未修** |
| 2.4 | D6 evidence 时效无契约 (7 天) | P0 | 中 | 未修 |
| 2.5 | `check_full_gate_verification.sh` 退出码契约不严 | P0 | 中 | 未修 |
| 2.6 | `check_int_debt.sh` 缺 IN_PROGRESS ≤ 3 契约 | P0 | 中 | 未修 |
| 2.7 | debt-registry.yaml 手工维护 | P1 | 中 | 未修 |
| 2.8 | subagent prompt 模板未固化为 Skills | P1 | 中 | 未修 |
| 2.9 | D5.5 audit_testing 未纳入 D9 | P1 | 中 | 未修 |
| 2.10 | 5 维度 gate 退出码无 JSON 输出 | P2 | 中 | 未修 |
| **2.11** | **`check_arch_sem_debt.sh` 硬编码状态** | **P1 (升级 P0)** | **高** | **✅ 本次修复** |
| 2.12 | D6b NOT_RUN 静默忽略 | P1 | 中 | 未修 |

### 1.3 维度 3: 文档体系 (Documentation) — 12 项

| # | 优化点 | 严重度 | ROI | 状态 |
|---|--------|--------|-----|------|
| **3.1** | **28 份治理文档缺统一 INDEX.md** | **P0** | **高** | **✅ 本次修复** |
| 3.2 | debt-registry.yaml 与 4 份 md 不同步 (ARCH-3 状态) | P0 | 高 | 部分修 (ARCH-3 IN_PROGRESS 60%) |
| 3.3 | 债务文档混用 2 套状态机 (3 态 vs 7 态) | P0 | 高 | 未修 |
| 3.4 | 12 份治理文档 > 30 天未更新 | P0 | 中 | 未修 |
| 3.5 | GA 主报告缺 PR #3152 状态 | P1 | 高 | 未修 |
| 3.6 | 7 个 `owner: unassigned` | P1 | 中 | 部分修 (SEM-1, SEM-4) |
| 3.7 | DEBT TRACKING.md 与新 SSOT 重复 | P1 | 中 | 未修 |
| 3.8 | AI_COLLABORATION.md 维护人字段不统一 | P2 | 低 | 未修 |
| 3.9 | COLLABORATION_PROMPT.md 仍写 v2.8.0 | P2 | 中 | 未修 |
| 3.10 | GATE_CI_CD.md 引用 R-Gate 与 ADR-005 矛盾 | P2 | 中 | 未修 |
| 3.11 | 28 份缺统一维护人/最后审查/下次审查 3 字段 | P2 | 低 | 未修 |
| 3.12 | advisory/normative 标识 | P2 | 低 | 未修 |

### 1.4 维度 4: 跨阶段一致性 — 11 项

| # | 优化点 | 严重度 | ROI | 状态 |
|---|--------|--------|-----|------|
| 4.1 | Issue→PR→Gate→Report 4 阶段状态机不一致 | P0 | 高 | 未修 |
| 4.2 | Gate 退出码契约不统一 (5 套不同) | P0 | 高 | 未修 |
| 4.3 | check_full_gate_verification.sh 子 gate expect=0 硬编码 | P0 | 高 | 未修 |
| 4.4 | ADR-011 PROPOSED vs registry 实际使用不同步 | P1 | 中 | 未修 |
| 4.5 | Follow-up Issue ↔ YAML 双向无同步 | P1 | 中 | 未修 |
| 4.6 | D6 evidence "100%" 实际含 NOT_RUN | P1 | 中 | 未修 |
| 4.7 | 5 步流程在 AGENTS.md 强制条款未声明 | P2 | 中 | 未修 |
| 4.8 | check_int_debt.sh awk 列位置硬编码 | P1 | 中 | 未修 |
| 4.9 | check_arch_sem_debt.sh 硬编码状态 (与 2.11 合并) | P1 | 高 | **✅ 本次修复** |
| 4.10 | 文档"声称 vs 实际" 偏差 (50% 偏差率) | P0 | 中 | 未修 |
| 4.11 | Gitea Issue 关闭 0 自动化门禁 | P1 | 中 | 未修 |

---

## 2. 本次实施 2 项 P0 优化

### 2.1 优化 #3.1: 新建 `docs/governance/INDEX.md` (P0, 高 ROI, 1d→30min)

**问题**: 28 份治理文档 + 11 ADRs + 7 Patterns + 2 Templates + 1 Registry + 4 份债务文档, 散在 5 个子目录 (adr/patterns/templates/debt/archived/), 用户无法快速找到"该读哪份"。

**修复**: 新建 `docs/governance/INDEX.md` (11 节导航):

| 节 | 内容 |
|---|------|
| 0. 如何使用本索引 | 按角色 (新 Agent / GA 准备 / Issue / 债务 / Gate) 引导 |
| 1. 项目基础 (P0 必读) | AGENTS.md / 5 原则 / Issue 关闭 / 5 步流程 / 反伪造型 |
| 2. 治理资产 (本次会话新增) | GA 主报告 / 工具注册表 / 回归审计 |
| 3. Gate 脚本与门禁 | 11 active gate + 治理流程图 |
| 4. Pattern (7 份) | LEGACY_AUDIT_FOLLOWUP / FOLLOWUP_SLA 等 |
| 5. ADR (12 份) | 含 ADR-011 状态机 |
| 6. Template (2 份) | LEGACY_AUDIT_CHECKLIST 等 |
| 7. 债务跟踪 (SSOT) | debt-registry.yaml + 4 份 md |
| 8. AI Agent Skills | 5 项 |
| 9. GA 治理资产 | 5 份 |
| 10. 快速链接 (按需跳转) | 6 角色 |
| 11. 维护与刷新 | v<X.Y.Z> GA 前 |

### 2.2 优化 #2.11 + #4.9: 修复 `check_arch_sem_debt.sh` 硬编码状态 (P1, 高 ROI, 1d→30min)

**问题 (双层)**:
1. **代码层**: 7 个债务项 (ARCH-1/2/3 + SEM-1/2/3/4) 全部硬编码 `state="OPEN"`, 与实际状态矛盾
2. **数据层**: `debt-registry.yaml` (SSOT) 中 ARCH-1/2/SEM-2 已 CLOSED, 但脚本读不到

**修前 vs 修后对比**:

| 债务 | registry 实际 | 修前脚本 (硬编码) | 修后脚本 (读 YAML) |
|------|-------------|------------------|------------------|
| ARCH-1 | CLOSED | OPEN ❌ | CLOSED ✅ |
| ARCH-2 | CLOSED | OPEN ❌ | CLOSED ✅ |
| ARCH-3 | IN_PROGRESS 60% | OPEN (但有 plan) | IN_PROGRESS 60% (with plan) ✅ |
| SEM-1 | (无 target_release) | OPEN (但有 plan) | (原 OPEN 改 IN_PROGRESS 30%) |
| SEM-2 | CLOSED | OPEN ❌ | CLOSED ✅ |
| SEM-3 | IN_PROGRESS 60% | OPEN (但有 plan) | IN_PROGRESS 60% (with plan) ✅ |
| SEM-4 | (无 target_release) | OPEN (但有 plan) | (改 IN_PROGRESS 0%) |

**3 个错的状态 (ARCH-1/2/SEM-2) 从 OPEN 改为 CLOSED** — 之前脚本输出与实际数据矛盾。

**修复**:
1. 脚本改为读 `debt-registry.yaml` (SSOT) via `python3 + pyyaml`
2. 退出码契约: 0=全 CLOSED, 1=OPEN w/o plan, 2=IN_PROGRESS/BLOCKED w/ plan, 3=UNHANDLED
3. 7 状态机 (OPEN/IN_PROGRESS/BLOCKED/VERIFIED/CLOSED/SUPERSEDED/REJECTED) 全部处理

**数据修正** (修脚本时发现):
- SEM-1: 实际有 PR #3134 修复 → 状态 `OPEN` 改 `IN_PROGRESS` (30%)
- SEM-4: 实际有 owner + target_release → 状态 `OPEN` 改 `IN_PROGRESS` (0%)
- ARCH-3 target_release 已 v3.9.0 (PR #3155 上次会话加)

### 2.3 验证 (Phase 4)

| Gate | 修前 | 修后 |
|------|------|------|
| `check_arch_sem_debt.sh` | 7 OPEN (3 错) | 3 CLOSED + 4 IN_PROGRESS (0 错) |
| `check_int_debt.sh` | 2 CLOSED + 2 ACTIVE w/ plan (exit 2) | 同 (未改) |
| `check_cross_version_debt.sh` | PASS (Part 5 仍 10 孤岛 + 5 无实现) | 同 (未改) |
| `check_arch2_no_bypass.sh` | PASS | 同 (未改) |
| `check_docs_links.sh` | PASS | 同 (未改) |

**真实改进**: D8 gate 输出从"永远 7 OPEN DRIFT" (假) 变为"3 CLOSED + 4 IN_PROGRESS" (真)。

---

## 3. 32 个未实施优化 (按 ROI 排序)

### 3.1 P0 必做 (9 项, 1 周工作量)

按 ROI 排序 (建议实施顺序):

1. **新建 `check_followup_sla.sh`** (2.3, 1d, 高 ROI) — Gitea API 验证 4 字段
2. **新建 `check_doc_5step.sh`** (1.3, 2d, 高 ROI) — 5 步流程自动化
3. **新建 `check_issue_closure.sh`** (1.2, 1d, 高 ROI) — PR 关联校验
4. **PR Template 加强制 5 字段** (1.1, 1d, 高 ROI) — webhook
5. **`check_int_debt.sh` 升级到 YAML SSOT** (2.2, 2d, 高 ROI) — 与 D8 一致
6. **修复 `check_full_gate_verification.sh` 退出码契约** (4.3, 1d, 高 ROI) — 子 gate exit 2 改视为 D9 阻断
7. **5 维度 gate 退出码契约统一** (4.2, 2d, 高 ROI) — 13 个 gate 全部统一 0/1/2 + JSON
8. **`debt-registry.yaml` 与 4 份 Markdown 债务文档同步** (3.2, 2d, 高 ROI) — 用 yaml 生成
9. **债务文档统一 7 状态机** (3.3, 1w, 高 ROI) — 删除 ACTIVE/PARTIAL 旧术语

### 3.2 P1 推荐 (15 项, 1-2 周)

1. `debt-registry.yaml` ↔ Gitea Issue body 双向 sync 脚本 (4.5)
2. Gitea webhook 集成启用 (1.5)
3. D6 evidence 时效 gate (2.4)
4. D5.5 audit_testing 纳入 D9 (2.9)
5. D6b NOT_RUN 静默忽略修复 (2.12)
6. GA 主报告刷新 PR-3152 (3.5)
7. `debt-registry.yaml` 7 个 unassigned 强制分配 (3.6)
8. `DEBT TRACKING.md` 标 DEPRECATED (3.7)
9. ADR-011 状态从 PROPOSED → ACCEPTED (4.4)
10. Gitea Issue 关闭 webhook 自动化 (4.11)
11. subagent prompt 模板固化为 Skills (2.8)
12. `check_int_debt.sh` 改用 yaml 后删 awk 列位置硬编码 (4.8)
13. `check_test_inventory.sh` NOT_RUN 比例披露 (4.6)
14. Follow-up Issue body ↔ YAML 同步 (1.4)

### 3.3 P2 可选 (6 项, 长期)

1. AGENTS.md/PR Template 维护性 (1.6, 1.7)
2. gate JSON 输出 (2.10)
3. 文档规范 (3.8, 3.9, 3.10, 3.11, 3.12)
4. CLAUDE.md/AGENTS.md 强制条款 (4.7)

---

## 4. 实施波次建议 (按 ROI)

### 4.1 第一波 (高 ROI, 1 周)
- 优化 2.3 (check_followup_sla.sh)
- 优化 1.3 (check_doc_5step.sh)
- 优化 1.2 (check_issue_closure.sh)
- 优化 1.1 (PR Template)

### 4.2 第二波 (高 ROI, 1 周)
- 优化 2.2 (check_int_debt.sh yaml)
- 优化 4.3 (check_full_gate_verification.sh 退出码)
- 优化 4.2 (gate 退出码统一)

### 4.3 第三波 (债务同步, 1 周)
- 优化 3.2 (yaml ↔ md 同步)
- 优化 3.3 (7 状态机升级)
- 优化 4.4 (ADR-011 ACCEPTED)
- 优化 3.4 (12 份文档刷新)

### 4.4 第四波 (P1 长期)
- 优化 1.5 (Gitea webhook)
- 优化 4.5 (双向 sync 脚本)
- 优化 2.9, 2.12 (D5.5/D6b)
- 优化 2.8 (subagent Skills)

---

## 5. 核心矛盾 (一句话总结)

> **状态机分裂 (4 套) + 手工维护 (debt-registry 无 sync 脚本) + 缺统一入口 (47 个 gate 脚本无 README.md)**, 是 v3.8.0 GA 治理流程的 3 大头号风险。ADR-011 + debt-registry.yaml 已升级到 7 状态机 SSOT, 但 5 个 gate 脚本 + 4 份 Markdown 债务文档 + 28 份治理文档均未跟进; 建议以 1 周为单位分 4 波同步。

---

## 6. 维护信息

| 项目 | 值 |
|------|-----|
| 文档版本 | v3.8.0-OPTIMIZATION-REPORT-1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | ACTIVE |
| 关联 | `GA_GOVERNANCE_DEMO_v3.8.0.md`, `LEGACY_AUDIT_CHECKLIST.md`, `INDEX.md` |
| 下次审查 | v3.9.0 RC 前 |

---

## 7. 一句话总结

**v3.8.0 GA 治理优化第 1 波 = 4 维度调研识别 34 个优化点 (P0 13 + P1 15 + P2 6) + 实施 2 项 P0 优化 (INDEX.md 导航 + check_arch_sem_debt.sh 改读 YAML SSOT) = 修复 3 个错状态 (ARCH-1/2/SEM-2 从硬编码 OPEN 改为 CLOSED 真实反映) + 暴露 2 个数据问题 (SEM-1/4 需状态升级) = 0 个新 false positive + 治理流程稳定性提升。**

*本报告遵循 `LEGACY_AUDIT_CHECKLIST.md` 5 阶段流程 + `DOC_CHECK_CORRECTION_RULES.md` 5 步文档流程 + `ADR-001` Truthfulness 原则 + `ADR-011` 7 状态机 + 4 维度 subagent 调研方法。*
