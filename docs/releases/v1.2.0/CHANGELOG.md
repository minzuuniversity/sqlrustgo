# v1.2.0 变更日志 (CHANGELOG.md)

> **版本**: v1.2.0
> **发布日期**: 2026-03-13
> **代号**: Architecture Refactoring
> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
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

## [已发布] v1.2.0

> **发布日期**: 2026-03-13
> **代号**: Architecture Refactoring
> **发布类型**: GA (正式发布)

### Added

#### 架构重构 (P0)
- **功能**: Optimizer 框架层 ✅ - #305
- **功能**: Statistics 子系统 ✅
- **功能**: Catalog 正式落地 ✅
- **功能**: 执行层接口化 (Executor trait) ✅
- **功能**: 错误域重构 ✅
- **功能**: Pipeline 执行模型 ✅

#### CBO 成本优化 (P1)
- **功能**: CostModel 结构设计 ✅
- **功能**: 基础成本估算 ✅
- **功能**: Join 选择优化 ✅
- **功能**: 索引选择优化 ✅
- **功能**: 优化器集成 ✅

#### 向量化基础 (P1)
- **功能**: RecordBatch 结构定义 ✅
- **功能**: ColumnarArray trait 实现 ✅
- **功能**: 向量化表达式 ✅

### Changed

- **增强**: 目录重构 (crates/ workspace)
- **增强**: 所有核心 trait 已定义，为 2.0 预留接口

### Fixed

- **修复**: Clippy warnings (15 个)
- **修复**: 测试编译错误
- **修复**: Benchmark 编译错误
- **修复**: Cargo.toml 重复定义
- **修复**: 格式化问题

---

## 版本历史

| 版本 | 发布日期 | 代号 | 状态 |
|------|----------|------|------|
| v1.2.0 | 2026-03-13 | Architecture Refactoring | ✅ |
| v1.1.0 | 2026-03-03 | L3 产品级 | ✅ |
| v1.0.0 | 2026-02-22 | MVP | ✅ |

---

## 相关文档

- [ARCHITECTURE_REFACTORING_PLAN.md](./ARCHITECTURE_REFACTORING_PLAN.md) - 架构重构计划
- [VERSION_PLAN.md](./VERSION_PLAN.md) - 版本计划
- [DEV_PLAN.md](./DEV_PLAN.md) - 开发计划
- [RELEASE_GATE_CHECKLIST.md](./RELEASE_GATE_CHECKLIST.md) - 门禁清单

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）