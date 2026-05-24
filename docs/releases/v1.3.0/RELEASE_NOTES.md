# v1.3.0 发布说明

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **阶段状态**: Beta（RC 前需补充 P0 功能验证数据）
> **基于版本**: v1.2.0

---

## 概述

GMP-Platform v1.3.0 是面向 v2.0 综合管理平台的过渡版本，主要交付**事件驱动架构**（Event Bus + Audit Middleware + MQTT）和**向量存储增强**（Binary BLOB + HNSW auto-build）。

**重要提示**: P0 功能（向量索引生成流水线、Layer 0 知识抽取管线）仍在开发中。本版本为 Beta 阶段，RC/GA 需待 P0 功能完成后发布。

---

## 核心变更

### 1. Event Bus Crate (gmp-event-bus)

**功能描述**:
- `GmpEvent` 类型系统：AuditEvent, MqttEvent, WorkflowEvent
- `InMemoryBus` 发布订阅实现
- `AuditHandler` 审计处理器，自动记录所有事件到 `gmp_audit_chain`

**技术实现**:
- 独立 `gmp-event-bus` crate
- `#[derive(Clone)]` Event Bus trait，支持多订阅者
- 零外部依赖，纯 Rust 实现

**影响**: 系统从直接写数据库升级到统一事件总线，支持多事件源透明接入。

### 2. Audit Middleware

**功能描述**:
- `AuditContext` — 请求级审计上下文（trace_id + actor）
- `AuditEmitter` — 便捷 API（emit_search, emit_insert, emit_update, emit_delete, emit_sign, emit_workflow）
- `hash_event` — 事件完整性校验工具

**技术实现**:
- 使用 std `DefaultHasher`（无外部依赖）
- UUID v4 作为 event_id
- trace_id 全局追踪

### 3. Binary BLOB Embeddings + HNSW Auto-build

**功能描述**:
- 向量数据以 binary BLOB 存储，替代分行文本
- `init()` 时自动检测并构建 HNSW 索引
- 无需手动触发索引重建

**影响**: 解决了长期遗留的向量索引手动重建问题，提升运维体验。

### 4. MQTT Gateway

**功能描述**:
- `MqttGateway` 设备事件发射器
- 接入称重设备、温湿度传感器等 IoT 设备
- 通过 Event Bus 路由到 AuditHandler，实现端到端设备审计

### 5. DiskGraphStore WAL Persistence

**功能描述**:
- 图数据 WAL 预写日志持久化
- 支持 CypherEngine 重启后恢复

**状态**: WAL 基础架构已就绪，完整 recovery 逻辑在 feature 分支开发中。

---

## 性能基准

> ⚠️ **Beta 阶段**: 以下数据为 v1.2.0 基线。v1.3.0 向量索引性能数据待 P0-1 完成填充后更新。

| 指标 | v1.2.0 基线 | v1.3.0 目标 | 状态 |
|------|-------------|-------------|------|
| 391/408 场景通过率 | 96.08% | ≥ 96.08% | 🔄 |
| MRR | 0.497 | ≥ 0.497 | 🔄 |
| R8 QPS | 8.3 | ≥ 8.3 | 🔄 |
| R8 P95 | 1366ms | < 1500ms | 🔄 |
| 向量索引覆盖率 | 0% | 100% (13809) | 🔄 |

---

## 数据集

| 数据集 | 规模 | 状态 |
|--------|------|------|
| GMP 统一文档 | 1696 份 MD | ✅ |
| 向量嵌入 | 待生成 | 🔄 |
| 图节点 | ~700 | 🔄 待扩展 |
| 图关系边 | ~1500 | 🔄 待扩展 |

---

## 新增 CLI 命令

| 命令 | 功能 | 状态 |
|------|------|------|
| `gmp audit` | 查询 gmp_audit_chain | ✅ |
| `gmp mqtt-server` | MQTT Broker 服务 | 🔄 |
| `gmp populate` | (DEPRECATED) | ⚠️ 废弃 |

---

## 安全变更

### 新增安全特性

- Event Bus 统一审计：所有操作通过 AuditHandler 记录
- Audit Middleware：trace_id 端到端追踪
- hash_event：事件完整性校验

---

## 已知限制

1. **向量索引未填充**: Binary BLOB 存储就绪，13809 embeddings 待生成流水线填充
2. **Layer 0 管线未开发**: P0-3~7 仍在规划中
3. **MQTT 设备需实际 Broker**: 测试需要 MQTT Broker (localhost:1883)

---

## 致谢

- v1.0.0~v1.2.0 贡献者: Hermes + Claude + DeepSeek
- v1.3.0 贡献者: Hermes Agent
- SQLRustGo 项目提供统一数据底座
