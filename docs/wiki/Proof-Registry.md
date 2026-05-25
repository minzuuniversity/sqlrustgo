# Proof Registry System

证明注册表 (Proof Registry) 是 SQLRustGo 治理系统的执行机制，记录和追踪每个关键组件的形式化验证状态。

## 状态定义

| 状态 | 含义 |
|------|------|
| VERIFIED | 证明已通过，组件可信 |
| FAIL | 证明失败，存在被确认的问题 |
| UNVERIFIED | 尚未执行证明，状态未知 |
| BLOCKED | 被阻塞项阻挡，无法执行 |

## 当前证明状态 (2026-04-24)

| 证明 | 状态 | 说明 |
|------|------|------|
| executor_proof | VERIFIED | 执行器组件已验证 |
| mvcc_proof | FAIL | MVCC 组件存在确认问题 |
| wal_crash_proof | UNVERIFIED | WAL 崩溃恢复未验证 |
| benchmark_proof | UNVERIFIED | 基准测试未验证 |
| blockers | 4 remaining | 4 个阻塞项未解决 |

## Memory Sync 模式

触发条件: 新增测试 / 修复 bug / 新增 benchmark / 修改 contract
操作: 自动更新 `proof/*.json`, `versions/*/verification_report.json`, `blockers.md`, `memory_index.json`

## 相关页面

- [governance-system](../wiki/governance-system) — 证明系统是治理的执行层
- [sqlrustgo](../wiki/sqlrustgo) — 被证明的数据库引擎
- [baseline-verification](../wiki/baseline-verification) — 基线验证是证明的一种形式