# v1.5.0 变更日志 (CHANGELOG.md)

> **版本**: v1.5.0
> **发布日期**: 2026-03-18
> **代号**: Storage & Expression Optimization
> **类型**: 事后补充（Retrospective）
> **生成日期**: 2026-05-29

---

## 格式说明

本文档使用 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/) 格式。

### 版本类型

- **Added**: 新功能
- **Changed**: 已有功能的变化
- **Deprecated**: 已废弃的功能
- **Removed**: 已移除的功能
- **Fixed**: 问题修复
- **Security**: 安全相关

---

## [已发布] v1.5.0

> **发布日期**: 2026-03-18
> **代号**: Storage & Expression Optimization
> **发布类型**: GA (正式发布)

### Added

#### 存储引擎 (P0)
- **功能**: 页式存储 (S-01) ✅ - #563
- **功能**: 缓冲池 LRU (S-02) ✅ - #564
- **功能**: WAL 预写日志 (S-03) ✅ - #566
- **功能**: 表堆存储 (S-04) ✅ - #569

#### 索引 (P1)
- **功能**: B+Tree 索引 (I-01) ✅ - #565
- **功能**: IndexScan 算子 (I-02) ✅ - #568

#### 表达式优化 (P1)
- **功能**: 常量折叠增强 (E-01) ✅ - #574
- **功能**: 表达式简化 (E-02) ✅ - #576

#### 统计信息 (P1)
- **功能**: 基础统计信息 (ST-01) ✅ - #580

#### 性能基准
- **功能**: PB-02 索引性能基准测试 ✅ - #587
- **功能**: PB-03 WAL 性能基准测试 ✅ - #582
- **功能**: PB-04 页读写吞吐量 ✅ - #583
- **功能**: PB-05 TPC-H SF=1 ✅ - #585

#### 集成测试
- **功能**: IT-01 存储引擎集成测试 ✅ - #589
- **功能**: IT-02 索引集成测试 ✅ - #590
- **功能**: IT-03 端到端查询测试 ✅ - #592

### Changed

- **增强**: 存储层从内存扩展到持久化
- **增强**: 缓冲池 LRU 淘汰策略
- **增强**: 表达式优化覆盖 Float/String/DateTime

### Fixed

- **修复**: Clippy warnings and format issues
- **修复**: debug 模式吞吐量阈值调整

---

## [已发布] v1.4.0

> **发布日期**: 2026-03-18
> **代号**: CBO & Vectorization Ready

### Added

- **架构**: CBO 成本优化器
- **功能**: SortMergeJoin 算子
- **功能**: NestedLoopJoin 算子
- **功能**: Prometheus /metrics 端点
- **功能**: TPC-H SF=1 基准测试

---

## [已发布] v1.3.0

> **发布日期**: 2026-03-15
> **代号**: Enterprise Ready

### Added

- **架构**: Volcano Model 统一 Executor
- **功能**: Metrics + Health 端点
- **功能**: 81.26% 测试覆盖率

---

## [已发布] v1.2.0

> **发布日期**: 2026-03-13
> **代号**: Vector Engine

### Added

- **架构**: 矢量化执行引擎
- **功能**: CBO 成本优化器基础
- **功能**: HashJoinExec, 聚合函数

---

## [已发布] v1.1.0

> **发布日期**: 2026-03-03
> **代号**: L3 产品级

### Added

- **架构**: LogicalPlan/PhysicalPlan 分离
- **架构**: ExecutionEngine 插件化
- **功能**: Client-Server 网络

---

## 版本历史

| 版本 | 发布日期 | 代号 | 状态 |
|------|----------|------|------|
| v1.5.0 | 2026-03-18 | Storage & Expression | ✅ |
| v1.4.0 | 2026-03-18 | CBO & Vectorization | ✅ |
| v1.3.0 | 2026-03-15 | Enterprise Ready | ✅ |
| v1.2.0 | 2026-03-13 | Vector Engine | ✅ |
| v1.1.0 | 2026-03-03 | L3 产品级 | ✅ |
| v1.0.0 | 2026-02-22 | MVP | ✅ |

---

## 相关文档

- [RELEASE_NOTES.md](./RELEASE_NOTES.md) - 发布说明
- [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) - 开发计划
- [TEST_PLAN.md](./TEST_PLAN.md) - 测试计划

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）