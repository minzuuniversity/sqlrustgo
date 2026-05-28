# v1.7.0 变更日志 (CHANGELOG.md)

> **版本**: v1.7.0
> **发布日期**: 2026-03-20
> **代号**: Explainable Database Core
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

## [已发布] v1.7.0

> **发布日期**: 2026-03-20
> **代号**: Explainable Database Core
> **发布类型**: GA (正式发布)

### Added

#### SQL 补完 (P0)
- **功能**: UNION / UNION ALL 支持 ✅ - #701
- **功能**: VIEW 创建和查询 ✅ - #703
- **功能**: BOOLEAN 类型补齐 ✅ - #704

#### 可观测性增强 (P0)
- **功能**: EXPLAIN 执行计划 ✅ - #702
- **功能**: EXPLAIN ANALYZE ✅ - #702
- **功能**: 算子级 Profiling ✅ - #702
- **功能**: 格式化输出 ✅ - #702

#### Benchmark 完善 (P1)
- **功能**: 完整 TPC-H 支持 ✅ - #705
- **功能**: 性能回归检测 ✅ - #706
- **功能**: 报告生成自动化 ✅ - #706

#### 错误系统 (P1)
- **功能**: 统一错误码 ✅ - #707
- **功能**: 错误消息国际化 ✅ - #707
- **功能**: 错误恢复机制 ✅ - #707

### Changed

- **增强**: EXPLAIN 输出格式标准化
- **增强**: 错误消息可读性提升

### Fixed

- **修复**: SQL 解析边界情况处理
- **修复**: UNION ALL 结果顺序问题

---

## 版本历史

| 版本 | 发布日期 | 代号 | 状态 |
|------|----------|------|------|
| v1.7.0 | 2026-03-20 | Explainable Database | ✅ |
| v1.6.0 | 2026-03-20 | Transaction Isolation | ✅ |
| v1.5.0 | 2026-03-18 | Storage & Expression | ✅ |
| v1.4.0 | 2026-03-18 | CBO & Vectorization | ✅ |
| v1.3.0 | 2026-03-15 | Enterprise Ready | ✅ |
| v1.2.0 | 2026-03-13 | Architecture Refactoring | ✅ |

---

## 相关文档

- [GOALS_AND_PLANNING.md](./GOALS_AND_PLANNING.md) - 开发规划
- [RELEASE_NOTES.md](./RELEASE_NOTES.md) - 发布说明
- [TEST_PLAN.md](./TEST_PLAN.md) - 测试计划
- [RELEASE_GATE_CHECKLIST.md](./RELEASE_GATE_CHECKLIST.md) - 门禁清单

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）