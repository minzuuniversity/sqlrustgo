# v1.3.0 测试体系设计

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **Gate**: L0-Alpha / L0-Beta / L0-RC / L0-GA

---

## 一、测试分层架构

```
┌─────────────────────────────────────────────────────┐
│         391 场景测试 (手动/CI)                       │  ← 真实 GMP 场景，端到端
├─────────────────────────────────────────────────────┤
│         集成测试 (tests/)                            │  ← 多 crate 协作，Event Bus
├─────────────────────────────────────────────────────┤
│         单元测试 (#[test])                          │  ← 单个函数/模块
├─────────────────────────────────────────────────────┤
│         性能压测 (scripts/stress_test_72h.py)        │  ← 72h 连续压测
├─────────────────────────────────────────────────────┤
│         MQTT 模拟测试                                 │  ← IoT 设备事件模拟
└─────────────────────────────────────────────────────┘
```

### 与 v1.2.0 差异

| 层级 | v1.2.0 | v1.3.0 变更 |
|------|---------|-------------|
| 事件总线 | 无 | **gmp-event-bus 新增集成测试** |
| 审计中间件 | 无 | **AuditMiddleware 单元测试** |
| MQTT | 无 | **MqttGateway 模拟测试** |
| 向量存储 | 分行存储 | **Binary BLOB 存储** |
| 图持久化 | 基础实现 | **WAL 持久化（开发中）** |

---

## 二、单元测试

### 2.1 测试范围

| Crate | 测试文件 | 覆盖范围 | 状态 |
|-------|---------|---------|------|
| gmp-event-bus | `src/lib.rs #[cfg(test)]` | AuditContext, AuditEmitter, hash_event, MqttGateway | ✅ |
| gmp-storage | `src/lib.rs #[cfg(test)]` | StorageEngine, SqlrustgoAdapter, Binary BLOB | ✅ |
| gmp-retrieval | `src/tests.rs` | Channel engine, RRF fusion | 🔄 |
| gmp-reranker | `src/lib.rs #[cfg(test)]` | NoopReranker, OllamaReranker | ✅ |
| gmp-llm | `src/tests.rs` | LlmClient, Intent Classification | 🔄 |

### 2.2 运行命令

```bash
# 运行所有单元测试
cargo test --lib

# 运行特定 crate 测试
cargo test -p gmp-event-bus
cargo test -p gmp-storage

# 带特征运行
cargo test --all-features
```

### 2.3 Event Bus 测试用例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_context_new() {
        let ctx = AuditContext::new("alice");
        assert_eq!(ctx.actor, "alice");
        assert!(!ctx.trace_id.is_empty());
        assert!(ctx.session_id.is_none());
    }

    #[test]
    fn test_audit_emitter_publish() {
        let bus = InMemoryBus::new();
        bus.subscribe(Box::new(AuditHandler::new())).unwrap();
        let emitter = AuditEmitter::new(Arc::new(bus));
        let ctx = AuditContext::new("test");
        emitter.emit_search(&ctx, "SELECT * FROM", 10).unwrap();
    }

    #[test]
    fn test_hash_event_deterministic() {
        let h1 = hash_event("trace", "entity", "Insert");
        let h2 = hash_event("trace", "entity", "Insert");
        assert_eq!(h1, h2);
        let h3 = hash_event("trace", "entity", "Delete");
        assert_ne!(h1, h3);
    }
}
```

---

## 三、集成测试

### 3.1 Event Bus 集成测试

| 测试用例 | 描述 | 预期结果 |
|---------|------|----------|
| `event_bus_publish_subscribe` | 发布事件后订阅者收到 | 事件正确路由 |
| `audit_handler_records` | AuditEvent 发布后查询 gmp_audit_chain | 记录存在 |
| `mqtt_gateway_broadcast` | MQTT 事件通过 Event Bus 广播 | 多订阅者收到 |
| `audit_emitter_full_flow` | AuditEmitter emit_search 端到端 | 审计链记录完整 |

### 3.2 运行命令

```bash
cargo test --test '*integration*'
```

---

## 四、MQTT 模拟测试

### 4.1 测试场景

| 测试用例 | 描述 | 预期结果 |
|---------|------|----------|
| `mqtt_weight_device` | 模拟称重设备上报 | MqttEvent 进入 Event Bus |
| `mqtt_temp_sensor` | 模拟温湿度传感器上报 | MqttEvent 包含正确字段 |
| `mqtt_reconnect` | 模拟 MQTT Broker 重连 | Gateway 恢复订阅 |

### 4.2 运行命令

```bash
# 需要 MQTT Broker (localhost:1883)
cargo test -p gmp-event-bus -- mqtt
```

---

## 五、回归测试

### 5.1 391 场景回归

```bash
# 391 场景评估（回归基线）
gmp-eval --test-set test-data/391_scenarios.json --min-pass-rate 0.9608

# 期望: ≥ 96.08% (无退化)
```

### 5.2 向量通道回归

```bash
# 向量查询功能验证
cargo test -p gmp-storage -- vector

# Binary BLOB 读取验证
cargo test -p gmp-storage -- blob
```

---

## 六、性能测试

### 6.1 测试场景

| 指标 | v1.2.0 基线 | v1.3.0 目标 | 状态 |
|------|-------------|-------------|------|
| 391 场景通过率 | 96.08% | ≥ 96.08% | 🔄 |
| MRR | 0.497 | ≥ 0.497 | 🔄 |
| R8 QPS | 8.3 | ≥ 8.3 | 🔄 |
| R8 P95 | 1366ms | < 1500ms | 🔄 |
| Event Bus 吞吐 | N/A | 待测量 | 🔄 |
| MQTT 事件延迟 | N/A | 待测量 | 🔄 |

### 6.2 压测命令

```bash
# 单次查询压测
cargo run --release -- bin gmp-retrieve bench --single

# 并发压测
cargo run --release -- bin gmp-retrieve bench --concurrent 10 --rounds 100

# 72h 连续压测
python3 scripts/stress_test_72h.py
```
