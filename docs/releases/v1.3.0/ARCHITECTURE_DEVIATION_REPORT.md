# GMP-Platform v1.3.0 — 架构偏离报告

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **分支**: `develop/v1.3.0`
> **状态**: Beta — 开发中

---

## 一、概述

本报告追踪 v1.3.0 开发过程中的架构偏离事项，对比 v1.2.0 基线，评估偏离影响，并记录修复状态。

### v1.3.0 主要变更

| 变更类型 | 内容 | 状态 |
|----------|------|------|
| 新增功能 | gmp-event-bus crate (Event Bus + Audit Middleware) | ✅ 已完成 |
| 新增功能 | MQTT Gateway 设备集成 | ✅ 已完成 |
| 性能优化 | Binary BLOB embeddings + HNSW auto-build | ✅ 已完成 |
| 可靠性增强 | DiskGraphStore WAL 持久化（基础） | 🔄 开发中 |
| 技术债 | clippy 清理 + cargo audit | ✅ 已完成 |

---

## 二、架构偏离追踪

### 2.1 P0 级偏离

#### v1.3.0-Z1: Binary BLOB 存储格式变更

| 属性 | 值 |
|------|-----|
| 问题 ID | v1.3.0-Z1 |
| 优先级 | P0 |
| 发现日期 | 2026-05-24 |
| 状态 | ✅ **已修复** |

**背景**: v1.2.0 中 `gmp_embeddings` 以分行文本格式存储向量，HNSW 索引需要手动重建。

**决策**: 改用 Binary BLOB 存储向量数据（commit `3aeccf0`），`init()` 时自动构建 HNSW 索引。

**后果**:
- ✅ 向量查询性能提升（binary 直接加载）
- ✅ HNSW 索引自动就绪，无需手动触发
- ✅ 与 SQLRustGo binary 存储原生支持兼容

**修复验证**:
```bash
cargo build --release -p gmp-storage
cargo test -p gmp-storage
```

---

### v1.3.0-Z2: 新增 gmp-event-bus Crate

| 属性 | 值 |
|------|-----|
| 问题 ID | v1.3.0-Z2 |
| 优先级 | P0 |
| 发现日期 | 2026-05-24 |
| 状态 | ✅ **设计完成** |

**背景**: v1.2.0 中审计功能直接写入数据库，无统一事件总线。

**决策**: 新建 `gmp-event-bus` crate，实现：
- `GmpEvent` 类型系统（AuditEvent, MqttEvent, WorkflowEvent）
- `InMemoryBus` 发布订阅实现
- `AuditHandler` 审计处理器

**后果**:
- ✅ 审计事件统一通过 Event Bus 路由
- ✅ MQTT 设备事件可透明接入审计链
- ✅ 未来 WorkflowEvent 可扩展
- ⚠️ 引入新 crate，workspace 依赖关系增加

---

### v1.3.0-Z3: DiskGraphStore WAL 持久化范围

| 属性 | 值 |
|------|-----|
| 问题 ID | v1.3.0-Z3 |
| 优先级 | P0 |
| 发现日期 | 2026-05-24 |
| 状态 | 🔄 **开发中** (feature 分支) |

**背景**: v1.2.0 中 DiskGraphStore 无 WAL，图数据重启后可能丢失。

**决策**: 实现 WAL 预写日志（commit `cf65908`），完整 WAL recovery 逻辑在 `feature/issue-123-disk-graph-wal` 分支开发中。

**后果**:
- ✅ WAL 基础架构已就绪
- 🔄 完整 recovery 逻辑待 feature 分支完成后可用
- ⚠️ WAL 可能影响写入性能（需 benchmark 验证）

---

## 三、架构增强追踪

### 3.1 新架构引入

#### MQTT Gateway 集成

**决策**: `MqttGateway` 作为 Event Bus 的事件源，发射 `MqttEvent` 到 `InMemoryBus`，由 `AuditHandler` 自动记录到审计链。

```
设备 → MqttGateway → InMemoryBus → AuditHandler → gmp_audit_chain
```

**无偏离**: 此为新增功能，不影响现有架构。

---

## 四、偏离汇总

| ID | 偏离项 | Priority | 状态 | 影响评估 |
|----|--------|----------|------|----------|
| v1.3.0-Z1 | Binary BLOB 存储变更 | P0 | ✅ 已修复 | 正向（性能） |
| v1.3.0-Z2 | 新增 gmp-event-bus Crate | P0 | ✅ 设计完成 | 正向（可扩展性） |
| v1.3.0-Z3 | DiskGraphStore WAL 范围 | P0 | 🔄 开发中 | 正向（可靠性） |

---

## 五、v1.3.0 与 v1.2.0 架构对比

| 架构层面 | v1.2.0 | v1.3.0 | 变更 |
|----------|---------|---------|------|
| 数据存储 | SQLRustGo + 分行向量 | SQLRustGo + Binary BLOB | ✅ 优化 |
| 事件总线 | 无 | gmp-event-bus | ✅ 新增 |
| 审计系统 | 直接写 DB | Event Bus + AuditHandler | ✅ 增强 |
| MQTT | 无 | MqttGateway | ✅ 新增 |
| HNSW 索引 | 手动构建 | 自动构建 | ✅ UX 改善 |
| 图持久化 | 基础 | WAL 持久化 | 🔄 增强中 |
| workspace crates | 9 | **11** | ✅ 新增 2 个 |
