# v1.4.0 变更日志 (CHANGELOG.md)

> **版本**: v1.4.0
> **发布日期**: 2026-03-18
> **代号**: CBO & Vectorization Ready
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

## [已发布] v1.4.0

> **发布日期**: 2026-03-18
> **代号**: CBO & Vectorization Ready (性能增强版)
> **发布类型**: GA (正式发布)

### Added

#### CBO 优化器
- **功能**: 代价模型基础 (CostModel) ✅
- **功能**: 统计信息集成 (CBO-02) ✅
- **功能**: 索引选择优化 (CBO-04) ✅
- **功能**: Join 顺序优化 (CBO-03) ✅
- **功能**: IndexSelect 规则实现 ✅

#### Join 算法增强
- **功能**: SortMergeJoin 算子 (SMJ-01) ✅
- **功能**: SortMergeJoin 测试 (SMJ-02) ✅
- **功能**: NestedLoopJoin 算子 (NLJ-01) ✅
- **功能**: Cross Join 支持 ✅
- **功能**: Left/Right/Full Outer Join 支持 ✅
- **功能**: Semi/Anti Join 支持 ✅

#### 向量化基础
- **功能**: SIMD 基础设施 (vectorization.rs) ✅
- **功能**: Vector 数据结构 ✅
- **功能**: BatchIterator trait ✅

#### 可观测性增强
- **功能**: /metrics 端点 (M-004) ✅
- **功能**: Prometheus 格式支持 (M-003) ✅
- **功能**: Grafana Dashboard 模板 (M-005) ✅
- **功能**: actix-web HTTP 服务器 ✅

#### 性能基准
- **功能**: TPC-H 基准测试 (PB-01) ✅
- **功能**: CBO 基准测试 ✅
- **功能**: v1.4.0 性能报告 ✅

### Changed

- **增强**: HashJoin 支持更多 Join 类型
- **增强**: Optimizer 规则稳定性提升

### Fixed

- **修复**: 优化器规则编译问题
- **修复**: SortMergeJoinVolcanoExecutor 测试重复定义
- **修复**: Clippy warnings and formatting

---

## [已发布] v1.3.0

> **发布日期**: 2026-03-15
> **代号**: Enterprise Ready (架构稳定版)

### Added

- **架构**: Volcano Model 统一 Executor trait
- **功能**: TableScan/Projection/Filter/HashJoin 算子
- **功能**: Metrics trait + 指标收集
- **功能**: /health/live, /health/ready, /health 端点
- **功能**: /metrics 端点 + Prometheus 格式

---

## [已发布] v1.2.0

> **发布日期**: 2026-03-13
> **代号**: Vector Engine

### Added

- **架构**: 矢量化执行引擎 (RecordBatch)
- **架构**: StorageEngine 特征
- **功能**: ANALYZE 命令
- **功能**: HashJoinExec, 聚合函数
- **功能**: CBO 成本优化器

---

## [已发布] v1.1.0

> **发布日期**: 2026-03-03

### Added

- **架构**: LogicalPlan/PhysicalPlan 分离
- **架构**: ExecutionEngine 插件化
- **架构**: Client-Server 网络架构

---

## 版本历史

| 版本 | 发布日期 | 代号 | 状态 |
|------|----------|------|------|
| v1.4.0 | 2026-03-18 | CBO & Vectorization Ready | ✅ |
| v1.3.0 | 2026-03-15 | Enterprise Ready | ✅ |
| v1.2.0 | 2026-03-13 | Vector Engine | ✅ |
| v1.1.0 | 2026-03-03 | L3 产品级 | ✅ |
| v1.0.0 | 2026-02-22 | MVP | ✅ |

---

## 相关文档

- [RELEASE_NOTES.md](./RELEASE_NOTES.md) - 发布说明
- [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) - 开发计划
- [TEST_PLAN.md](./TEST_PLAN.md) - 测试计划
- [TEST_MANUAL.md](./TEST_MANUAL.md) - 测试手册

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）