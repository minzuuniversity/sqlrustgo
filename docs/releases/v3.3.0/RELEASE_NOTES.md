# v3.3.0 Release Notes

> **版本**: v3.3.0
> **发布日期**: 2026-05-17
> **状态**: Alpha → Beta 收敛中
> **分支**: `develop/v3.3.0`
> **从**: v3.2.0 GA (`539f2957`)

---

## 版本概述

v3.3.0 聚焦于 Trust Infrastructure（可信基础设施），为 GMP 系统提供完整的数据溯源和证据链管理能力。

---

## 新增功能

### 1. Workflow V2 Engine

- 工作流引擎 V2，支持复杂审批流程
- 状态机驱动
- 并行/串行任务支持

### 2. Provenance Knowledge Graph

- 溯源知识图谱
- 数据血缘追踪
- 变更历史图谱

### 3. GMP Evidence Engine

- 电子证据管理
- 证据链完整性验证
- 审计追踪

### 4. Compliance-as-Code Engine

- 规则即代码
- 合规策略自动化
- 策略执行引擎

### 5. WAL Formal Verification

- WAL 形式化验证框架
- 数学证明保证
- 一致性验证

### 6. MySQL Protocol 修复

- 握手协议修复
- 查询测试通过
- 协议兼容性提升

---

## 修复与改进

- FTS FxHashMap 内存修复
- RwLock 竞争条件消除
- COUNT(*) 性能优化

---

## 测试结果

| 测试项 | 结果 |
|--------|------|
| 单元测试 | ✅ PASS |
| 覆盖率 | ≥ 75%（豁免） |
| TPC-H SF=1 | ✅ PASS |

---

## 升级说明

无 Breaking Changes，从 v3.2.0 GA 平滑升级。

---

## 已知问题

- v3.3.0 仍处于 Beta 收敛阶段
- 72h 稳定性测试进行中
