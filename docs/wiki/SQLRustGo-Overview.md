# SQLRustGo

> Rust 数据库引擎 — 目标是替代 MySQL 5.7 的生产级 SQL 数据库

## 概述

SQLRustGo 是一个基于 Rust 的 SQL 数据库引擎，支持 MySQL wire protocol，提供 OLTP 工作负载下的数据存储和查询能力。项目起点是 B-Tree + WAL 核心引擎，逐步扩展至分布式、安全、高性能方向。

## v2.8.0 关键指标

| 指标 | 数值 |
|------|------|
| 版本 | v2.8.0 (GA) — Production+Distributed+Secure |
| 代码规模 | 469 Rust 源文件, 166,002 行 |
| 项目成员 | 36 crates |
| 测试总数 | 1090 (258 unit + 685 distributed + 147 other) |
| 测试通过率 | 97.4% (33 ignored) |
| 安全测试 | 81 PASS (审计/防火墙/TLS/告警) |
| 分布式测试 | 685 PASS (分区/复制/故障转移/读写分离) |

## Phase 完成状态

- **Phase A**: MySQL 兼容性 — 100% (FULL OUTER JOIN, TRUNCATE/REPLACE, 窗口函数)
- **Phase B**: 分布式基础 — 100% (分区表, GTID 复制, 故障转移, 负载均衡, 读写分离)
- **Phase C**: 性能优化 — 60% (SIMD 3.2x ✅, Hash Join 并行化未集成 ⚠️, CBO 优化器 ✅)
- **Phase D**: 安全加固 — 50% (审计/告警 ✅, 列级权限 ⚠️, 数据加密 ❌)
- **Phase E**: 文档 — 100%

## 核心模块

- **parser**: SQL 词法/语法分析, 34 tests
- **planner**: 查询计划生成, 81 tests
- **optimizer**: CBO 查询优化
- **executor**: LocalExecutor 物理计划执行
- **storage**: B+Tree, BufferPool, WAL (8 entry types), MVCC
- **security**: audit (81 tests), firewall, TLS, encryption, alert
- **distributed**: partition(75), replication(79), failover(55), read_write(27)
- **server**: MySQL wire protocol server

## 相关页面

- [v2.8.0 Overview](../wiki/v2.8.0-Overview) — 版本功能总览
- [Storage 存储引擎](../wiki/Storage-存储引擎) — 存储架构详解
- [Security Module](../wiki/Security-Module) — 安全模块
- [Distributed Module](../wiki/Distributed-Module) — 分布式模块
- [MySQL 5.7 差距与 GMP 报告](../wiki/MySQL-5.7-差距与-GMP-报告) — 差距分析