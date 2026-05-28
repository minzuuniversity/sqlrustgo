# v3.3.0 Release Notes

> **版本**: v3.3.0 GA
> **发布日期**: 2026-05-20
> **补充日期**: 2026-05-29（Retrospective）
> **分支**: `origin/develop/v3.3.0`
> **状态**: ✅ GA Released

---

## 版本概述

v3.3.0 是 SQLRustGo 的第二个主要 GA 版本，定位为 **Industrial Trust Platform（工业级可信闭环）**，首次将 Trust Infrastructure 纳入门禁体系。

---

## 核心变更

### Trust Infrastructure 纳入（新增）

- **数据溯源图**（Provenance Graph）：完整的数据血缘追踪
- **证据链引擎**（Evidence Engine）：不可篡改的证据记录
- **合规检查引擎**（Compliance Engine）：规则驱动的合规验证
- **WAL 写前验证**（WAL Verification）：预写日志完整性保证
- **信任可视化**（Trust Visualization）：溯源图可视化

### 文档门禁体系（新增）

- **Truthfulness 检查**：T1~T4 首次纳入门禁
  - T1: 项目描述与实际功能一致性
  - T2: README 与 CHANGELOG 对应性
  - T3: API 规范与 doc comments 完整性
  - T4: 错误码文档覆盖

### MySQL 协议增强

- `sqlrustgo-mysql-server` 支持微信小程序接入
- MySQL 5.7 wire protocol 完善

---

## 门禁通过记录

| 阶段 | 日期 | 状态 |
|------|------|------|
| Alpha | 2026-04-10 | ✅ PASS (12/12) |
| Beta | 2026-04-25 | ✅ PASS (12/12) |
| RC | 2026-05-15 | ✅ PASS (16/16) |
| GA | 2026-05-20 | ✅ PASS (16/16 + 豁免 G5) |

**豁免记录**: G5 Coverage 82.88% < 85%，批准豁免 EX-v330-001

---

## 组件版本

| 组件 | 版本 |
|------|------|
| sqlrustgo-core | 0.1.0 |
| sqlrustgo-mysql-server | 0.1.0 |
| sqlrustgo-gmp-api | 0.1.0 |
| sqlrustgo-gmp-retrieval | 0.1.0 |
| sqlrustgo-workflow-v2 | 0.1.0 |
| sqlrustgo-evidence-engine | 0.1.0 |
| sqlrustgo-compliance-engine | 0.1.0 |
| sqlrustgo-provenance-graph | 0.1.0 |
| sqlrustgo-trust-viz | 0.1.0 |
| sqlrustgo-perf-baseline | 0.1.0 |
| sqlrustgo-crash-sim | 0.1.0 |
| sqlrustgo-wal-verification | 0.1.0 |

---

## 升级说明

从 v3.2.0 升级无需数据库迁移。

---

## 已知问题

| 问题 ID | 描述 | 状态 |
|---------|------|------|
| EX-v330-001 | G5 Coverage 82.88% < 85% 目标 | 豁免批准 |

---

*发布于 2026-05-29（Retrospective）*