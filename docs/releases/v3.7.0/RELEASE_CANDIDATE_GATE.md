# v3.7.0 Release Candidate Gate — GA Superseded

> **状态**: 本文档已被 GA 门禁取代
> **原因**: v3.7.0 已于 2026-05-30 正式 GA
> **替代文档**: [RELEASE_GATE_CHECKLIST.md](RELEASE_GATE_CHECKLIST.md)

---

## 历史记录

此文档记录了 v3.7.0 RC1 阶段（commit `3e647254`）的冻结检查清单。

### RC1 阶段定义（已冻结）

> **v3.7.0 RC1 = SQLRustGo Core SQL Execution Engine**

**RC1 包含**：
- MySQL Wire Protocol Server
- SQL Parser → AST
- LocalExecutor execution pipeline
- StorageEngine CRUD operations
- DDL/DML support
- Prepared Statement framework

**RC1 排除**：
- Transaction consistency（RC1 阶段 BEGIN/COMMIT 为 stub）
- MVCC / isolation levels
- SHOW / metadata completeness

### RC1 → GA 变更

| Issue | RC1 状态 | GA 状态 |
|-------|----------|---------|
| P0-1: Transaction state | ❌ NOT PERSISTENT | ✅ FIXED (session-level) |
| P0-2: Auth bypass | ❌ SKIP_AUTH=true | ✅ FIXED (SKIP_AUTH=false) |
| GA Score | 41/100 | ✅ 65/100 |

**GA 重新定义版本定性**：

> **v3.7.0 GA = Stable SQL Execution Engine with Session-level Transaction Semantics**

参见 [RELEASE_SUMMARY.md](RELEASE_SUMMARY.md) 和 [RELEASE_NOTES.md](RELEASE_NOTES.md) 了解 GA 完整定义。

---

## v3.8.0 RC Gate

参见 [../v3.8.0/GA_GATE_CHECKLIST.md](../v3.8.0/GA_GATE_CHECKLIST.md)