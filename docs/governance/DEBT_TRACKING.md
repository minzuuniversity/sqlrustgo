# Cross-Version Debt Tracking

## 概述

本文档定义跨版本债务的追踪机制，确保历史遗留问题不被遗忘或重复发现。

---

## 问题分类

### 1. 技术债务 (Technical Debt)

| 类型 | 说明 | 示例 |
|------|------|------|
| 代码债务 | 低质量代码需要重构 | DML 双轨执行 |
| 测试债务 | 测试覆盖率不足或测试缺失 | mysql-server 编译错误 |
| 文档债务 | 文档缺失或过时 | API reference 不完整 |
| 架构债务 | 设计缺陷，需要较大改动 | ExecutionEngine 单体架构 |

### 2. 治理债务 (Governance Debt)

| 类型 | 说明 | 示例 |
|------|------|------|
| 流程债务 | 流程缺失或执行不严格 | Beta 入口检查形同虚设 |
| 追踪债务 | 问题未创建 Issue | Parser 47% 无 Issue |
| 标准债务 | 标准被随意调整 | Alpha 阈值凭记忆填写 |

---

## Issue 命名规范

### 跨版本延续问题

```
格式: [debt:<来源版本>] <问题描述>
Example: [debt:v3.5.0] Parser coverage 47% structural deficiency
```

### Issue 标签

| 标签 | 用途 |
|------|------|
| debt | 跨版本债务追踪 |
| structural | 结构性缺陷，需要较大改动 |
| compile-error | 编译错误 |
| coverage | 覆盖率问题 |
| architecture | 架构相关 |
| governance | 治理相关 |

---

## Issue 模板

### 跨版本债务 Issue

```markdown
## 问题描述
<详细描述问题>

## 根本原因
<分析根本原因>

## 影响版本
- <版本>: <状态>
- <版本>: <状态>
- <版本>: <状态>

## 修复策略
<计划>

## 关联文档
- <文档路径>

## 状态
- 创建时间: <日期>
- 来源版本: <版本>
- 目标版本: <版本>
- 状态: OPEN | IN_PROGRESS | CLOSED
```

---

## 追踪流程

### 1. 问题发现

当发现跨版本延续问题时：

1. 创建 Issue，标题使用 `[debt:<来源版本>]` 格式
2. 添加 `debt` 标签
3. 在本文档的 "当前追踪问题" 节添加条目

### 2. 问题解决

当 Issue 关闭时：

1. 验证修复有效
2. 更新本文档 "已关闭问题" 节
3. 确保有回归测试防止问题重现

### 3. 季度审查

每季度末审查所有 OPEN 的跨版本债务 Issue：

1. 检查是否有进展
2. 评估是否需要调整目标版本
3. 识别是否有新产生的债务

---

## 当前追踪问题

### OPEN

| Issue | 标题 | 来源版本 | 创建时间 | 目标版本 |
|-------|------|----------|----------|----------|
| I#2580 | Parser coverage structural deficiency | v3.5.0 | 2026-05-30 | v3.7.0 |
| I#2581 | mysql-server 30+ compile errors | v2.5.0 | 2026-05-30 | v3.6.0 |
| I#2582 | Executor coverage below GA threshold | v3.5.0 | 2026-05-30 | v3.6.0 |
| I#2583 | DML execution path not unified | v3.0.0 | 2026-05-30 | v3.7.0 |
| I#2584 | Alpha CONDITIONAL PASS semantics unclear | v3.6.0 | 2026-05-30 | v3.6.0 |
| I#2585 | Cross-version debt tracking missing | v3.6.0 | 2026-05-30 | v3.6.0 |

### IN_PROGRESS

| Issue | 标题 | 来源版本 | 创建时间 | 目标版本 |
|-------|------|----------|----------|----------|

### CLOSED

| Issue | 标题 | 来源版本 | 关闭时间 | 解决方案 |
|-------|------|----------|----------|----------|

---

## 健康度指标

### 债务率

```
债务率 = OPEN Issue 数 / 总 Issue 数
目标: <= 10%
警告: > 10%
危险: > 20%
```

### 债务解决率 (季度)

```
债务解决率 = CLOSED Issue 数 / 新增 Issue 数 (季度)
目标: >= 100%
警告: 50-99%
危险: < 50%
```

---

**最后更新**: 2026-05-30
**维护者**: hermes-agent
**关联 Issue**: I#2585