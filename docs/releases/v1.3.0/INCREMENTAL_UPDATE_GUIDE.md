# v1.3.0 增量更新指南

> **版本**: v1.3.0
> **日期**: 2026-05-24
> **维护人**: Hermes Agent
> **从**: v1.2.0 | **到**: v1.3.0

---

## 1. 更新概述

### 1.1 主要变更

| 组件 | v1.2.0 | v1.3.0 | 变更类型 |
|------|---------|---------|---------|
| 事件总线 | 无 | **gmp-event-bus crate** | 新增功能 |
| 审计中间件 | 基础 | **AuditMiddleware** | 功能增强 |
| 向量存储 | 分行存储 | **Binary BLOB** | 性能优化 |
| HNSW 索引 | 手动构建 | **自动构建** | UX 改善 |
| 图持久化 | 基础 | **WAL 持久化** | 可靠性增强 |
| MQTT | 无 | **MqttGateway** | 新增功能 |
| CLI | gmp populate | **DEPRECATED** | 废弃 |

### 1.2 数据库兼容性

v1.3.0 **完全兼容** v1.2.0 的数据库 schema，无需数据迁移。

| 表 | 兼容性 | 说明 |
|----|--------|------|
| gmp_documents | ✅ 兼容 | 无变更 |
| gmp_document_chunks | ✅ 兼容 | 无变更 |
| gmp_embeddings | ✅ 兼容 | Binary BLOB 模式，写入新数据 |
| gmp_graph_nodes | ✅ 兼容 | 无变更 |
| gmp_graph_edges | ✅ 兼容 | 无变更 |
| gmp_audit_chain | ✅ 兼容 | WAL 增强，兼容旧数据 |

---

## 2. 代码更新

### 2.1 获取更新

```bash
# 切换到 v1.3.0 分支
git fetch origin
git checkout develop/v1.3.0

# 确认分支
git log --oneline -1
# 输出: 0912bcd Merge pull request 'fix: tech-debt cleanup (Issue #126)' (#135)
```

### 2.2 编译

```bash
# 增量编译 (推荐)
cargo build --release

# 清理后编译 (如遇编译错误)
cargo clean
cargo build --release

# 验证编译成功
cargo build --release --workspace && echo "BUILD SUCCESS"
```

---

## 3. 向量索引重建

### 3.1 Binary BLOB + HNSW 自动构建

v1.3.0 中 `gmp-storage` 的 `init()` 会在每次调用时自动检测 HNSW 索引状态，如索引缺失则自动重建：

```bash
# 触发 HNSW 索引自动重建
cargo run --release --bin gmp-cli -- init

# 验证索引状态
cargo test -p gmp-storage -- hnsw
```

### 3.2 向量数据填充（待实现）

向量索引生成流水线（Issue #122）仍在开发中。待 feature 分支合入后，执行：

```bash
# 生成 13809 embeddings（需要 Ollama bge-m3）
bash scripts/embed/generate_embeddings.sh

# 验证
cargo run --release --bin gmp-cli -- verify-embeddings
```

---

## 4. Event Bus 使用

### 4.1 新增依赖

如项目代码中使用了 `gmp-storage`，升级后自动获得 Event Bus 功能：

```toml
# Cargo.toml
[dependencies]
gmp-event-bus = { version = "0.1.0" }  # 从 workspace 成员引入
```

### 4.2 API 变更

无破坏性变更。`gmp-storage` 内部已集成 `AuditHandler`，审计链自动生效。

**新增便捷 API**（可选使用）：

```rust
use gmp_event_bus::{AuditContext, AuditEmitter, InMemoryBus};

let bus = InMemoryBus::new();
let emitter = AuditEmitter::new(std::sync::Arc::new(bus));

let ctx = AuditContext::new("user@example.com");
emitter.emit_search(&ctx, "洁净区级别", 10)?;
```

---

## 5. MQTT 设备集成

### 5.1 MQTT Broker 要求

- MQTT Broker (Mosquitto 或类似)
- 默认端口: `1883`
- 默认主题: `gmp/events/#`

### 5.2 配置

```bash
# 设置 MQTT Broker URL
export GMP_MQTT_BROKER="tcp://localhost:1883"

# 运行 CLI
cargo run --release --bin gmp-cli -- mqtt-server
```

### 5.3 验证

```bash
# 订阅事件
mosquitto_sub -t "gmp/events/#"

# 模拟称重设备上报
mosquitto_pub -t "gmp/events/weight" -m '{"device":"scale-01","weight":1250.5,"unit":"g"}'
```

---

## 6. 验证清单

```bash
# 1. 编译验证
cargo build --release --workspace && echo "✅ BUILD OK"

# 2. Lint 验证
cargo clippy --all-targets --all-features -- -D warnings && echo "✅ CLIPPY OK"

# 3. 安全审计
cargo audit && echo "✅ AUDIT OK"

# 4. 单元测试
cargo test --lib && echo "✅ TESTS OK"

# 5. Event Bus 测试
cargo test -p gmp-event-bus && echo "✅ EVENT-BUS OK"

# 6. 向量索引状态（如已填充）
cargo run --release --bin gmp-cli -- verify-embeddings && echo "✅ EMBEDDINGS OK"
```

---

## 7. 回滚方案

### 7.1 代码回滚

```bash
# 回滚到 v1.2.0
git checkout develop/v1.2.0
cargo clean
cargo build --release
```

### 7.2 数据库回滚

无需回滚。v1.3.0 的 schema 变更完全向后兼容旧数据。

---

## 8. 已知变更

| 变更 | 影响 | 处理方式 |
|------|------|----------|
| `Commands::Populate` 废弃 | CLI 用户 | 使用新的 `gmp index` 命令 |
| Binary BLOB 存储格式 | 无 | 透明兼容，init 时自动转换 |
| 新增 `gmp-event-bus` crate | 编译时间 | 首次编译略慢 |
