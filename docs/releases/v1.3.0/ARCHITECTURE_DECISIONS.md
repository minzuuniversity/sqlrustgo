# v1.3.0 架构决策记录

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`

---

## ADR-001: 引入 gmp-event-bus 独立 Crate

### 背景

v1.2.0 中审计功能直接嵌入 `gmp-storage`，无统一事件总线。新增 MQTT 设备集成需求后，事件路由逻辑会变得混乱。

### 决策

新建 `gmp-event-bus` crate（不依赖外部消息队列），实现内存发布订阅：

```
EventBus trait ──▶ InMemoryBus 实现
                     ├── AuditHandler (订阅所有事件)
                     ├── MqttGateway (发射 MQTT 事件)
                     └── (未来) WorkflowHandler
```

### 后果

- ✅ 事件路由逻辑集中，MQTT/Audit/Workflow 可独立扩展
- ✅ 无外部依赖，编译和部署简单
- ⚠️ 内存总线不支持跨进程/跨机器（未来可通过 MQTT Broker 网桥解决）

### 决策日期

2026-05-24

---

## ADR-002: Binary BLOB 存储向量数据

### 背景

v1.2.0 中 `gmp_embeddings` 表以分行文本存储向量（每行一个 f32 值），HNSW 索引需要手动触发重建。

### 决策

改用 SQLRustGo binary 模式存储向量为单个 BLOB，在 `init()` 时自动构建 HNSW 索引。

### 后果

- ✅ 存储效率提升（binary vs 文本）
- ✅ HNSW 索引自动就绪，无需手动触发
- ✅ 与 SQLRustGo 原生 binary 模式兼容
- ⚠️ 13809 embeddings 待生成流水线填充

### 决策日期

2026-05-24

---

## ADR-003: Audit Middleware 使用 std DefaultHasher

### 背景

审计事件需要完整性校验，方案包括：MD5/SHA256（需外部 crate）、或使用 Rust 标准库。

### 决策

使用 `std::collections::hash_map::DefaultHasher`（不引入外部 crate），提供 `hash_event(trace_id, entity_id, action)` API。

### 后果

- ✅ 零外部依赖
- ✅ 与 Rust runtime 兼容
- ⚠️ DefaultHasher 不是加密哈希（不适用于安全敏感场景，仅用于事件完整性追踪）

### 决策日期

2026-05-24

---

## ADR-004: Commands::Populate 废弃

### 背景

`gmp-cli` 的 `Populate` 命令引用了未定义的变量（`ollama_url`, `model`, `stage`, `db`），导致 E0425 编译错误。

### 决策

将 `Commands::Populate` 整个 body 替换为 DEPRECATED 提示信息，注释原有导入。

### 后果

- ✅ 消除 E0425 编译错误
- ✅ `gmp index` 命令已覆盖文档导入功能
- ⚠️ 废弃命令保留在 CLI（向后兼容）

### 决策日期

2026-05-24
