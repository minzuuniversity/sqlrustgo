# v3.9.0 文档改正计划

> **创建日期**: 2026-06-01
> **执行人**: Hermes Agent
> **依据**: V390_DOCUMENT_INCONSISTENCY_ANALYSIS.md

---

## 一、问题清单

| # | 文件 | 问题 | 位置 | 依据 |
|---|------|------|------|------|
| 1 | ROADMAP.md | Phase 状态过时 | §1 | 显示 Phase 0 (启动)，实际已到 RC2 后期 |
| 2 | ROADMAP.md | GA 目标日期未标注风险 | §1 | 2026-09-23 未标注 at risk |
| 3 | ROADMAP.md | GA 治理报告路径不存在 | §7 | 引用 `docs/governance/GA_GOVERNANCE_DEMO_v3.9.0.md` 不存在 |
| 4 | V390_VERSION_PLAN.md | W0 日期错误 | §时间线 | W0 = 2026-07-01，实际应为 2026-06-05 |
| 5 | V390_VERSION_PLAN.md | 状态过时 | §头部 | Draft，实际应为 ACTIVE (RC2 后期) |
| 6 | V390_VERSION_PLAN.md | GA 目标日期未标注风险 | §时间线 | 未同步 RC3_PLAN 的延期风险 |
| 7 | CHANGELOG.md | 阶段表述不完整 | §头部 | RC2，应标注 form-only |
| 8 | alpha/ 目录 | 缺失 | — | 无 ALPHA1_RELEASE_NOTES.md |

---

## 二、修改原则说明

遵循 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 的最小修改原则：

**允许修改**：
- 状态标记错误（Phase 0 → Phase 6）
- 日期错误（W0 日期修正）
- 文档引用路径错误

**禁止修改**：
- 功能描述
- 架构设计内容
- commit 日志内容
- 任何实质性技术内容

---

## 三、具体修改操作

### 修改 1: ROADMAP.md - Phase 状态更新

**位置**: §1 版本概览
**oldString**: `Phase 0 (启动)`
**newString**: `Phase 6 收口 (form-only) → RC3 待启动`

---

### 修改 2: ROADMAP.md - GA 目标日期风险标注

**位置**: §1 版本概览
**oldString**: `GA 目标: 2026-09-23`
**newString**: `GA 目标: 2026-09-23 (at risk, 调整后 22-26 周)`

---

### 修改 3: ROADMAP.md - GA 治理报告路径修正

**位置**: §7 预期产物
**oldString**: `docs/governance/GA_GOVERNANCE_DEMO_v3.9.0.md`
**newString**: `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md`

---

### 修改 4: V390_VERSION_PLAN.md - W0 日期修正

**位置**: §时间线
**oldString**: `W0 (2026-07-01)`
**newString**: `W0 (2026-06-05)`

---

### 修改 5: V390_VERSION_PLAN.md - 状态更新

**位置**: §头部
**oldString**: `状态: Draft`
**newString**: `状态: ACTIVE (RC2 后期, form-only)`

---

### 修改 6: V390_VERSION_PLAN.md - GA 目标日期风险标注

**位置**: §时间线
**oldString**: `GA 目标: 2026-09-23`
**newString**: `GA 目标: 2026-09-23 (at risk, 参考 RC3_PLAN.md)`

---

### 修改 7: CHANGELOG.md - 阶段表述补充

**位置**: §头部
**oldString**: `当前阶段: RC2`
**newString**: `当前阶段: RC2 (form-only) → RC3 待启动`

---

### 新增 8: 创建 alpha/ALPHA1_RELEASE_NOTES.md

**操作**: 创建新文件
**内容**: Alpha1 发布说明（基于 V390_COMPREHENSIVE_ASSESSMENT.md §11.1 的描述）

---

## 四、预期结果

| # | 修改 | 预期结果 |
|---|------|----------|
| 1 | ROADMAP.md Phase 状态 | 与 V390_COMPREHENSIVE_ASSESSMENT.md 一致 |
| 2 | ROADMAP.md GA 日期风险 | 与 RC3_PLAN.md 一致 |
| 3 | ROADMAP.md 文档引用 | 引用存在的文件 |
| 4 | V390_VERSION_PLAN.md W0 日期 | 与 git log 一致 |
| 5 | V390_VERSION_PLAN.md 状态 | 反映实际进度 |
| 6 | V390_VERSION_PLAN.md GA 日期风险 | 与 RC3_PLAN.md 一致 |
| 7 | CHANGELOG.md 阶段表述 | 明确 form-only 状态 |
| 8 | alpha/ 目录 | 包含 ALPHA1_RELEASE_NOTES.md |

---

## 五、复核审查 Checklist

### 5.1 修改正确性

- [ ] 问题 1 已修复：ROADMAP.md Phase 状态正确
- [ ] 问题 2 已修复：ROADMAP.md GA 日期标注风险
- [ ] 问题 3 已修复：ROADMAP.md 文档引用路径正确
- [ ] 问题 4 已修复：V390_VERSION_PLAN.md W0 日期正确
- [ ] 问题 5 已修复：V390_VERSION_PLAN.md 状态正确
- [ ] 问题 6 已修复：V390_VERSION_PLAN.md GA 日期标注风险
- [ ] 问题 7 已修复：CHANGELOG.md 阶段表述完整
- [ ] 问题 8 已修复：alpha/ 目录存在

### 5.2 无过度修改

- [ ] commit 日志内容未被修改
- [ ] 功能描述未被修改
- [ ] 实质性技术内容未被修改

### 5.3 链接有效性

- [ ] ROADMAP.md 引用的 GA_GATE_REPORT.md 路径正确
- [ ] 所有新增文件已验证存在

### 5.4 git 状态

- [ ] git diff 无非预期修改
- [ ] 新增文件已 `git add`
- [ ] 修改文件已 `git add`

### 5.5 可撤销性

- [ ] 所有修改可通过 `git checkout -- <file>` 恢复
- [ ] 工作记录完整，可追溯每一步

---

*本计划遵循 DOC_CHECK_CORRECTION_RULES.md 的 5 步流程*