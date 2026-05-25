# 分布式模块

> 分布式模块 (11,920 LOC) 提供数据分片、共识协议和分布式事务能力。

## 核心组件

### 1. Sharding (数据分片)

| 策略 | 说明 |
|------|------|
| Hash Sharding | 基于哈希键的均匀分布 |
| Range Sharding | 基于键范围的分布 |
| List Sharding | 基于预定义列表的分布 |

分片路由流程: SQL → Parser → Router → 目标分片执行 → 结果合并

### 2. Raft 共识 (v2.8.0)

Raft 协议实现用于集群元数据共识:


关键特性:
- Leader 选举与任期管理
- 日志复制与提交
- 成员变更 (Joint Consensus)

### 3. 2PC (两阶段提交)

分布式事务协调:


阶段:
1. **Prepare**: 协调者询问所有参与者是否可提交
2. **Commit**: 全部 Prepare 成功 → 全局提交；任一失败 → 全局回滚

### 4. 故障转移

- 自动 Leader 切换 (Raft 驱动)
- 读请求透明重试
- 写请求在 Leader 切换期间阻塞

## v2.8.0 新增特性

| 特性 | 状态 | 说明 |
|------|------|------|
| Load Balancing | 🟡 待合并 | LeastConnections 策略 (PR#45) |
| ReadWriteSplit | 🟡 待合并 | 读写分离路由 (PR#43, #50) |

## 相关链接

- [SQLRustGo Architecture](../wiki/SQLRustGo-Architecture) — 架构概览
- [Storage Engine](../wiki/Storage-Engine) — 存储引擎
- [v2.8.0 Overview](../wiki/v2.8.0-Overview) — 版本总览

---

*最后更新: 2026-05-02*