# SQLRustGo v1.3.0 README

> **版本**: v1.3.0
> **发布日期**: 2026-03-15
> **代号**: Enterprise Ready
> **状态**: GA (正式发布)
> **类型**: 事后补充（Retrospective）

---

## SQLRustGo v1.3.0

**L4 企业级数据库内核**

SQLRustGo v1.3.0 是企业级架构稳定版，核心目标是实现 L4 企业级成熟度。

### 核心特性

| 特性 | 说明 |
|------|------|
| **Volcano Executor** | 统一执行器接口，所有算子标准化 |
| **Metrics + Health** | 完整可观测性支持 |
| **Prometheus 格式** | 标准监控指标导出 |
| **Grafana Dashboard** | 可视化监控面板 |

---

## 快速开始

```bash
# 构建
cargo build --all-features

# 运行 REPL
cargo run --bin sqlrustgo

# 运行测试
cargo test --all-features

# 查看覆盖率
cargo llvm-cov --workspace
```

---

## 文档

| 文档 | 说明 |
|------|------|
| [RELEASE_NOTES.md](./RELEASE_NOTES.md) | 发布说明 |
| [CHANGELOG.md](./CHANGELOG.md) | 变更日志 |
| [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) | 开发计划 |
| [TEST_PLAN.md](./TEST_PLAN.md) | 测试计划 |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 架构文档 |

---

## 架构演进

```
v1.2.0 架构                    v1.3.0 架构 (企业级)
┌─────────────────────┐       ┌─────────────────────┐
│ PhysicalPlan       │       │ Volcano Executor    │
│ (旧式 execute)     │  ──→  │ Trait + 4 核心算子  │
└─────────────────────┘       └─────────────────────┘
┌─────────────────────┐       ┌─────────────────────┐
│ 内部测试框架       │       │ 标准化测试框架      │
│ (不一致)           │  ──→  │ (mock + generator)  │
└─────────────────────┘       └─────────────────────┘
┌─────────────────────┐       ┌─────────────────────┐
│ 无可观测性          │       │ Metrics trait       │
│                    │  ──→  │ + Health Endpoints   │
└─────────────────────┘       └─────────────────────┘
```

---

## 核心算子

| 算子 | 说明 | 状态 |
|------|------|------|
| TableScan | 完整表扫描实现 | ✅ |
| Projection | 列投影功能 | ✅ |
| Filter | 条件过滤 | ✅ |
| HashJoin | 内连接实现 | ✅ |

---

## 可观测性端点

| 端点 | 说明 | 状态 |
|------|------|------|
| /health/live | 存活探针 | ✅ |
| /health/ready | 就绪探针 | ✅ |
| /health | 综合健康检查 | ✅ |
| /metrics | Prometheus 指标 | ✅ |

---

## 测试覆盖率

| 指标 | 目标值 | 实际值 | 状态 |
|------|--------|--------|------|
| 整体行覆盖率 | ≥65% | **81.26%** | ✅ |
| Executor 行覆盖率 | ≥60% | **87%** | ✅ |
| Planner 行覆盖率 | ≥60% | **76%** | ✅ |
| Optimizer 行覆盖率 | ≥40% | **82%** | ✅ |

---

## 质量门禁

| 检查项 | 状态 |
|--------|------|
| 编译通过 | ✅ |
| 测试通过 | ✅ |
| Clippy 检查 | ✅ |
| 格式检查 | ✅ |
| 无 unwrap/panic | ✅ |

---

## 资源

- **代码**: https://github.com/minzuuniversity/sqlrustgo
- **讨论**: https://github.com/minzuuniversity/sqlrustgo/discussions
- **问题**: https://github.com/minzuuniversity/sqlrustgo/issues

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）