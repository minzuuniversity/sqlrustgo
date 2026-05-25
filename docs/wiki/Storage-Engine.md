# 存储引擎

> 存储引擎模块 (15,752 LOC) 负责数据持久化与检索，支持多种存储后端。

## 存储引擎类型

### 1. MemoryStorage
- 纯内存存储，重启数据丢失
- 适用: 测试、缓存场景
- 性能: 最高吞吐，零持久化开销

### 2. FileStorage
- 基于文件的持久化存储
- 支持 WAL (Write-Ahead Logging)
- 支持 Binlog 记录
- 适用: 单机生产环境

### 3. ColumnarStorage
- 列式存储格式
- 适用: 分析型查询、聚合场景
- 压缩比高，I/O 效率好

## 核心特性

### WAL (Write-Ahead Logging)

- 保证事务持久性 (Durability)
- WAL 文件滚动管理
- Checkpoint 机制控制恢复时间

### MVCC (Multi-Version Concurrency Control)
- 每个事务看到一致的快照
- 写操作不阻塞读操作
- 基于版本链实现行级并发控制

### Binlog
- 逻辑复制日志
- Statement / Row / Mixed 三种格式
- 用于主从复制和数据恢复

## 存储引擎对比

| 特性 | MemoryStorage | FileStorage | ColumnarStorage |
|------|-------------|-------------|-----------------|
| 持久化 | ❌ | ✅ | ✅ |
| 事务支持 | ✅ | ✅ | ⚠️ 部分 |
| MVCC | ✅ | ✅ | ❌ |
| WAL | ❌ | ✅ | ✅ |
| 列式压缩 | ❌ | ❌ | ✅ |
| 适用场景 | 测试/缓存 | OLTP | OLAP |

## 相关链接

- [SQLRustGo Architecture](../wiki/SQLRustGo-Architecture) — 架构概览
- [Distributed Module](../wiki/Distributed-Module) — 分布式模块
- [v2.8.0 Overview](../wiki/v2.8.0-Overview) — 版本总览

---

*最后更新: 2026-05-02*