# v1.3.0 集成状态

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta

---

## 一、Workspace 组件矩阵

```
gmp-platform (workspace)
├── gmp-core          ✅ GA    — 类型系统、共享类型
├── gmp-storage       ✅ GA    — 存储层（含 Event Bus）
├── gmp-retrieval     ✅ GA    — 四通道检索
├── gmp-reranker      ✅ GA    — 重排
├── gmp-llm           ✅ GA    — LLM 客户端
├── gmp-cli           🔄 DEP   — CLI（含 audit 子命令）
├── gmp-server        ✅ GA    — MySQL 协议服务
├── gmp-eval          ✅ GA    — 评估工具
├── gmp-api           🔄 开发中 — REST API
└── gmp-event-bus     ✅ 新增   — 事件驱动架构
```

---

## 二、gmp-event-bus 集成

### 2.1 模块导出

```rust
// crates/gmp-event-bus/src/lib.rs
pub mod events;   // GmpEvent, AuditEvent, AuditAction, MqttEvent
pub mod bus;      // InMemoryBus, EventBus trait
pub mod handlers; // AuditHandler
pub mod mqtt;     // MqttGateway

pub use events::{GmpEvent, AuditEvent, AuditAction, MqttEvent, WorkflowStatus};
pub use bus::{EventBus, InMemoryBus};
pub use handlers::AuditHandler;
pub use mqtt::MqttGateway;
```

### 2.2 gmp-storage 集成 Event Bus

`crates/gmp-storage/src/lib.rs` 中 `gmp-event-bus` 已作为依赖引入：

```toml
# Cargo.toml
[dependencies]
gmp-event-bus = { path = "../gmp-event-bus" }
```

`AuditHandler` 订阅 `GmpEventType::All`，所有通过 `InMemoryBus.publish()` 发送的 `AuditEvent` 自动记录到审计日志。

### 2.3 MQTT Gateway 集成

`MqttGateway` 发射 `MqttEvent`，通过 `EventBus` 路由到 `AuditHandler`，实现设备事件的端到端审计追踪。

---

## 三、API 兼容性

| API 组件 | 版本变化 | 兼容性 | 说明 |
|----------|----------|--------|------|
| `GmpStorage` trait | 无变更 | ✅ 兼容 | 未破坏 |
| `SearchEngine` trait | 无变更 | ✅ 兼容 | 未破坏 |
| `Reranker` trait | 无变更 | ✅ 兼容 | 未破坏 |
| `LlmClient` API | 无变更 | ✅ 兼容 | 未破坏 |
| gmp-api REST | 🔄 增强中 | 🔄 | API v2 规划中 |

---

## 四、数据层兼容性

| 数据 | v1.2.0 | v1.3.0 | 兼容性 |
|------|--------|---------|--------|
| gmp_documents | ✅ | ✅ | ✅ 兼容 |
| gmp_document_chunks | ✅ | ✅ | ✅ 兼容 |
| gmp_embeddings (binary BLOB) | ❌ 空 | 🔄 待填充 | ✅ Schema 兼容 |
| gmp_graph_nodes | ✅ | ✅ | ✅ 兼容 |
| gmp_graph_edges | ✅ | ✅ | ✅ 兼容 |
| gmp_audit_chain | ✅ | ✅ | ✅ 兼容（WAL 增强中）|
| HNSW 索引 | ❌ 缺失 | ✅ 自动构建 | ✅ 修复 |

---

## 五、Event Bus 事件流

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ MQTT Device │────▶│ MqttGateway │────▶│ InMemoryBus │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                                │
                    ┌─────────────┐              │
                    │ AuditHandler│◀─────────────┘
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │gmp_audit_chain│
                    └─────────────┘
```

### 事件类型

| 事件 | 来源 | 处理器 | 状态 |
|------|------|--------|------|
| `AuditEvent` | CLI / API / MQTT | `AuditHandler` | ✅ |
| `MqttEvent` | MQTT Gateway | `AuditHandler` | ✅ |
| `WorkflowEvent` | 待实现 | 待实现 | 🔄 规划中 |

---

## 六、依赖关系

```
gmp-event-bus (无外部依赖)
       ▲
       │ re-exports
       │
gmp-storage ──depends on──▶ gmp-event-bus
       │
       └──depends on──▶ gmp-core
                              ▲
                              │ re-exports
                              │
gmp-retrieval ──depends on──▶ gmp-storage, gmp-core
gmp-reranker  ──depends on──▶ gmp-core
gmp-llm       ──depends on──▶ gmp-core
gmp-cli       ──depends on──▶ gmp-storage, gmp-llm, gmp-retrieval, gmp-reranker, gmp-event-bus
gmp-api       ──depends on──▶ gmp-storage, gmp-event-bus
```

---

## 七、已知集成风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Binary BLOB 向量存储后向量生成流水线尚未实现 | 中 | HNSW 索引自动构建就绪，13809 embeddings 填充后可立即使用 |
| MQTT Gateway 需要实际 MQTT Broker | 中 | 模拟测试可行 |
| DiskGraphStore WAL 持久化仍在 feature 分支 | 中 | 核心功能已合入，完整功能在 feature/issue-123-disk-graph-wal |
