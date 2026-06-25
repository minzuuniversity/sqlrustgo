# 当前版本状态

alpha/v3.8.0

## 阶段信息

- **阶段**: Alpha (功能开发阶段)
- **当前里程碑**: Execution Semantics Freeze → TransactionManager 集成
- **开始日期**: 2026-05-28
- **开发分支**: develop/v3.8.0
- **目标**: WAL + MVCC 事务 + TransactionManager → GA
- **协作 Issue**: [#2778](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/2778)

## 版本概述

v3.8.0 将 SQLRustGo 从**查询执行引擎**升级为**服务器级事务数据库**，具备明确的事务生命周期管理。

**核心目标**:
- TransactionManager 事务所有权
- WriteBuffer DML 暂存
- COMMIT/ROLLBACK 完整生命周期
- WAL + MVCC 支持

## v3.8.0 核心任务

### Phase 1 - TransactionManager 集成

| 功能 | 状态 | Issue |
|------|------|-------|
| TransactionManager 连接调度层 | ✅ | PR-820 |
| BEGIN/COMMIT/ROLLBACK 路由 | ✅ | PR-830F |
| LocalExecutor 保持无状态 | ✅ | PR-830F |

### Phase 2-3 - WriteBuffer + Commit Engine

| 功能 | 状态 | Issue |
|------|------|-------|
| DML 暂存 write_buffer | 🔄 | PR-840 |
| COMMIT 刷新到 StorageEngine | 🔄 | PR-840 |
| 无直接 DML → StorageEngine 路径 | 🔄 | PR-840 |

### Phase 4-5 - Rollback + Read Consistency

| 功能 | 状态 | Issue |
|------|------|-------|
| ROLLBACK 丢弃 write_buffer | ⏳ | PR-850 |
| 事务快照 (read-your-writes) | ⏳ | PR-860 |
| SSI 冲突检测 | ✅ | 已有 |

## v3.8.0 vs v3.7.0

| 方面 | v3.7.0 | v3.8.0 |
|------|---------|---------|
| 事务所有权 | 隐式 | 显式 (TransactionManager) |
| 写暂存 | 直接写 StorageEngine | 暂存 TransactionManager |
| BEGIN 处理 | N/A | txn_manager.begin() |
| COMMIT 处理 | N/A | txn_manager.commit() → flush |
| ROLLBACK 处理 | N/A | txn_manager.rollback() → discard |
| LocalExecutor | 无状态 | 无状态 (不变) |
| 读一致性 | StorageEngine 级别 | TransactionManager 快照 |
| SSI 冲突检测 | Yes | Yes (保留) |

## 开发时间线

| 版本 | 日期 | 目标 |
|------|------|------|
| v3.8.0-alpha | 2026-05-28 | Phase 1 完成 |
| v3.8.0-beta | 2026-06-07 | Phase 2-3 完成 |
| v3.8.0-rc | 2026-06-21 | Phase 4-5 完成 |
| v3.8.0-GA | 2026-06-28 | 正式发布 |

## 相关文档

- [v3.8.0 文档入口](docs/releases/v3.8.0/README.md)
- [v3.8.0 开发计划](docs/releases/v3.8.0/DEVELOPMENT_PLAN.md)
- [v3.8.0 路线图](docs/releases/v3.8.0/ROADMAP.md)
- [v3.8.0 版本计划](docs/releases/v3.8.0/VERSION_PLAN.md)

## 变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 2.0 | 2026-04-22 | 创建 v2.8.0 开发分支，基于 v2.7.0 GA |
| 3.0 | 2026-05-28 | 创建 v3.8.0 开发分支，基于 v3.7.0 GA |
