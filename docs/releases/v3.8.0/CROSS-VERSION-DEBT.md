# Cross-Version Debt Tracking — Governance Gap Analysis
**Issue**: #2585  
**Author**: Hermes C  
**Date**: 2026-05-31  
**Branch**: `origin/docs/v380-cross-version-debt-tracking`  
**Status**: COMPLETED

---

## 1. 问题陈述

Issue #2585: "Cross-version debt tracking missing"

当前版本治理只跟踪当前版本的 debt，缺少跨版本累积债务的全局视图。导致：
- v1.x ~ v3.x 的集成缺失（INT-1~INT-4）跨 6 版本未被修复
- 每个版本门禁只检查当前版本，历史上累积的问题无人问津
- LEGACY_ISSUES.md 虽然存在，但只是 Issue 列表，没有系统性的跨版本追踪

---

## 2. 根因分析

### 2.1 Gate Spec 缺失跨版本检查

`docs/governance/GATE_CONDITIONS.md` 的 Alpha/Beta/RC/GA 门禁只定义当前版本检查项，没有要求检查历史遗留的集成缺失是否被修复。

### 2.2 Issue 没有 Version Span 标注

大部分 Issue 只标注当前目标版本，没有记录「从哪个版本开始存在」和「影响哪些版本」。例如 INT-1 (DML 不经过 WAL) 从 v1.2.0 存在，影响 v1.x ~ v3.6.0，但 Issue 只 retargeted 到 v3.8.0。

### 2.3 没有 Cross-Version Debt Review Gate

Alpha Gate 没有要求：「与上一个大版本相比，新增了多少跨版本遗留问题」。

---

## 3. 跨版本累积债务分类

### 3.1 Integration Debt（集成债务）

| ID | 问题 | 首次出现 | 影响版本 | 状态 |
|----|------|----------|----------|------|
| INT-1 | DML 不经过 WAL/TransactionManager | v1.2.0 | v1.x~v3.6.0 | ACTIVE |
| INT-2 | ParallelVolcanoExecutor 孤岛 | v2.6.0 | v2.x~v3.5.0 | ACTIVE |
| INT-3 | expr crate 功能孤岛 | v3.0.0 | v3.0~v3.6.0 | ACTIVE |
| INT-4 | mysql-server 未与主 server 集成 | v2.6.0 | v2.x~v3.6.0 | ACTIVE |

### 3.2 Architecture Debt（架构债务）

| ID | 问题 | 首次出现 | 影响版本 | 状态 |
|----|------|----------|----------|------|
| ARCH-1 | execution_engine.rs 膨胀 (4658→6829行) | v3.0.0 | v3.x | ACTIVE |
| ARCH-2 | 双执行路径 (ExecutionEngine + LocalExecutor) | v3.0.0 | v3.x | ACTIVE |
| ARCH-3 | StorageEngine trait 方法缺失 | v3.5.0 | v3.5~v3.x | ACTIVE |

### 3.3 Semantic Debt（语义债务）

| ID | 问题 | 首次出现 | 规则 |
|----|------|----------|------|
| SEM-1 | HashJoin NULL = NULL 匹配错误 | v3.0.0 | 禁止在 v3.8.0 修复 |
| SEM-2 | SQL 三值逻辑不一致 | v3.0.0 | 禁止在 v3.8.0 修复 |
| SEM-3 | ALTER TABLE 未实现 | v2.9.0 | roadmap 延期 |
| SEM-4 | Stored Procedure 未实现 | v2.9.0 | roadmap 延期 |

---

## 4. 跨版本 Debt Review Gate — 建议

### 4.1 在 GATE_CONDITIONS.md 中新增 Cross-Version Debt Gate

在 Alpha/Beta/RC/GA 门禁基础上，增加 **CVA (Cross-Version Analysis)** 门禁：

```
CV-Alpha: 每个版本门禁必须包含
  - [ ] 列出新增的跨版本问题（首次出现在本版本）
  - [ ] 确认历史跨版本问题没有被恶化
  - [ ] 对于 ACTIVE 跨版本问题，确认有修复计划

CV-Beta: Beta Gate 必须满足
  - [ ] INT-1~INT-4 全部 CLOSED 或有明确的 v3.9.0 修复计划
  - [ ] 没有新的跨版本集成缺失被引入

CV-RC: RC Gate 必须满足
  - [ ] 上一版本的跨版本债务没有被带到下一版本
```

### 4.2 Issue 模板扩展

每个 Issue 强制要求：
```
Version Span:
  - First Appeared: vX.Y.Z
  - Affects Versions: vX.Y.Z ~ vW.Z
  - Related Issues: #N (跨版本关联)
```

---

## 5. 实施建议

### 5.1 立即（v3.8.0 内）

- [x] 已在 LEGACY_ISSUES.md 中建立历史遗留问题列表
- [x] PR-831 Issue Closure Batch 关闭了 11 个 legacy issues
- [ ] 在 GATE_CONDITIONS.md 中新增 Cross-Version Debt 检查项

### 5.2 v3.9.0 开始

- [ ] 所有新 Issue 必须标注 `Version Span`（首次出现版本）
- [ ] Alpha Gate 必须检查新增跨版本债务
- [ ] 建立跨版本债务仪表盘（Neo4j → Dashboard）

---

## 6. 关联文档

- `docs/releases/v3.8.0/LEGACY_ISSUES.md` — 历史遗留问题清单
- `docs/governance/GATE_CONDITIONS.md` — 门禁条件（待更新加入 CV 检查）
- Issue #2585 — Cross-version debt tracking missing

---

## 7. Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-05-31 | Initial version | Hermes C |