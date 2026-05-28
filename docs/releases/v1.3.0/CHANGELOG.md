# v1.3.0 变更日志 (CHANGELOG.md)

> **版本**: v1.3.0
> **发布日期**: 2026-03-15
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

## [已发布] v1.3.0

> **发布日期**: 2026-03-15
> **代号**: Enterprise Ready (架构稳定版)
> **目标成熟度**: L4 企业级

### Added

#### Executor 模块
- **架构**: Volcano Model 统一 Executor trait
- **功能**: TableScan 算子完善
- **功能**: Projection 算子实现
- **功能**: Filter 算子实现
- **功能**: HashJoin 算子实现 (内连接)
- **测试**: Executor 测试框架 (mock storage + 数据生成器)

#### Planner 模块
- **测试**: Planner 测试套件 (76.44% 覆盖率)

#### Optimizer 模块
- **测试**: Optimizer 测试补充 (82.12% 覆盖率)

#### 可观测性
- **功能**: Metrics trait 定义 (M-001) ✅
- **功能**: BufferPool 指标收集 (M-002) ✅
- **功能**: ExecutorMetrics 执行器指标收集 (M-003) ✅
- **功能**: 指标聚合器和 Prometheus 格式 (M-005) ✅
- **功能**: /health/live 端点 (H-001) ✅
- **功能**: /health/ready 端点 (H-002) ✅
- **功能**: /health 综合端点 (H-004) ✅
- **功能**: /metrics 端点 (E-001) ✅
- **功能**: Grafana Dashboard 模板 ✅

### Changed

- **重构**: 逻辑计划/物理计划分离
- **重构**: Executor 接口标准化 (Volcano Model)
- **增强**: 整体覆盖率 78.88% → 81.26%

### Fixed

- **修复**: 性能基准测试编译错误
- **修复**: HashJoin 测试解析问题
- **修复**: v1.3.0 性能基准测试

### Security

- 依赖审核已通过 (无高严重性漏洞)

---

## [已发布] v1.2.0

> **发布日期**: 2026-03-13
> **代号**: Vector Engine

### Added

- **架构**: 带有 RecordBatch 的矢量化执行引擎
- **架构**: 可插拔存储后端的 StorageEngine 特征
- **架构**: 文件存储和内存存储实现
- **架构**: 带有统计信息的基于成本的优化器 (CBO)
- **功能**: ANALYZE 命令
- **功能**: HashJoinExec 表连接实现
- **功能**: 聚合函数 (COUNT, SUM, AVG, MIN, MAX)
- **功能**: ProjectionExec 列投影
- **功能**: FilterExec 谓词过滤
- **功能**: SeqScanExec 全表扫描

---

## [已发布] v1.1.0

> **发布日期**: 2026-03-03

### Added

- **架构**: LogicalPlan/PhysicalPlan 分离
- **架构**: ExecutionEngine 插件化
- **架构**: Client-Server 网络架构
- **功能**: HashJoin 执行器
- **功能**: 连接池支持
- **测试**: Criterion 性能基准框架

---

## 版本历史

| 版本 | 发布日期 | 代号 | 状态 |
|------|----------|------|------|
| v1.3.0 | 2026-03-15 | Enterprise Ready | ✅ |
| v1.2.0 | 2026-03-13 | Vector Engine | ✅ |
| v1.1.0 | 2026-03-03 | L3 产品级 | ✅ |
| v1.0.0 | 2026-02-22 | MVP | ✅ |

---

## 贡献者

- @yinglichina8848
- @TRAE人工智能助手

---

## 相关文档

- [RELEASE_NOTES.md](./RELEASE_NOTES.md) - 发布说明
- [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) - 开发计划
- [TEST_PLAN.md](./TEST_PLAN.md) - 测试计划

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）