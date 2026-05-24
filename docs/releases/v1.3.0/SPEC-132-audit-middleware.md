# SPEC: GMP-Platform Audit Middleware — Issue #132

> **版本**: 1.0
> **日期**: 2026-05-24
> **分支**: `issue/132-audit-middleware` → `develop/v1.3.0`
> **Issue**: #132
> **前置**: #131 (Event Bus)

---

## 一、目标

在 `gmp-event-bus` 基础上实现 Audit Middleware，提供：
1. `AuditContext` — 请求级审计上下文（trace_id + actor）
2. `AuditEmitter` — 便捷 API 供 CLI/API 层发射审计事件
3. Hash + trace integrity 机制

---

## 二、实现位置

```
crates/gmp-event-bus/src/
├── lib.rs          # 新增: AuditContext, AuditEmitter, hash utils
├── events.rs       # 已有: AuditEvent, AuditAction (from #131)
├── bus.rs          # 已有: InMemoryBus, EventBus (from #131)
└── handlers.rs     # 已有: AuditHandler (from #131)
```

不新建 crate，复用 `gmp-event-bus`。

---

## 三、API 设计

```rust
// AuditContext — 请求级审计上下文
#[derive(Debug, Clone)]
pub struct AuditContext {
    pub trace_id: String,       // 全局 trace ID
    pub actor: String,          // 操作人
    pub session_id: Option<String>,
}

impl AuditContext {
    pub fn new(actor: &str) -> Self
    pub fn with_session(actor: &str, session: &str) -> Self
}

// AuditEmitter — 审计事件发射器
pub struct AuditEmitter {
    bus: std::sync::Arc<dyn EventBus>,
}

impl AuditEmitter {
    pub fn new(bus: std::sync::Arc<dyn EventBus>) -> Self

    pub fn emit_search(&self, ctx: &AuditContext, query: &str, result_count: usize) -> Result<()>
    pub fn emit_insert(&self, ctx: &AuditContext, entity_id: &str, entity_type: &str) -> Result<()>
    pub fn emit_update(&self, ctx: &AuditContext, entity_id: &str, entity_type: &str) -> Result<()>
    pub fn emit_delete(&self, ctx: &AuditContext, entity_id: &str, entity_type: &str) -> Result<()>
    pub fn emit_sign(&self, ctx: &AuditContext, entity_id: &str, signature: &str) -> Result<()>
    pub fn emit_workflow(&self, ctx: &AuditContext, workflow_id: &str, step: &str, status: WorkflowStatus) -> Result<()>
}

// hash_utils — 审计 hash 工具
pub fn hash_event(trace_id: &str, entity_id: &str, action: &str) -> String
pub fn unix_timestamp() -> i64
```

---

## 四、关键实现细节

### 4.1 Hash 算法

使用 std `DefaultHasher`（不引入外部 crate）：
```rust
fn hash_event(trace_id: &str, entity_id: &str, action: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    trace_id.hash(&mut h);
    entity_id.hash(&mut h);
    action.hash(&mut h);
    format!("{:016x}", h.finish())
}
```

### 4.2 GmpEvent 构造

```rust
fn make_audit_event(
    action: AuditAction,
    entity_id: &str,
    ctx: &AuditContext,
    metadata: Option<String>,
) -> GmpEvent {
    GmpEvent::Audit(AuditEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        action,
        entity_id: entity_id.to_string(),
        timestamp: unix_timestamp(),
        actor: ctx.actor.clone(),
        hash: hash_event(&ctx.trace_id, entity_id, &format!("{:?}", action)),
        signature: None,
        trace_id: ctx.trace_id.clone(),
        metadata,
    })
}
```

### 4.3 集成现有 AuditHandler

`AuditHandler` 已订阅 `GmpEventType::All`，会自动 log 所有通过 `bus.publish()` 发送的 `AuditEvent`。

---

## 五、测试计划

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_context_new() {
        let ctx = AuditContext::new("alice");
        assert_eq!(ctx.actor, "alice");
        assert!(ctx.trace_id.len() > 0);
        assert!(ctx.session_id.is_none());
    }

    #[test]
    fn test_audit_context_with_session() {
        let ctx = AuditContext::with_session("alice", "sess-123");
        assert_eq!(ctx.session_id, Some("sess-123".to_string()));
    }

    #[test]
    fn test_audit_emitter_publish() {
        let bus = InMemoryBus::new();
        bus.subscribe(Box::new(AuditHandler::new())).unwrap();
        let emitter = AuditEmitter::new(std::sync::Arc::new(bus));
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

## 六、验证标准

- [x] `cargo test -p gmp-event-bus` ✅
- [x] `cargo clippy -p gmp-event-bus -- -D warnings` ✅
- [x] `cargo check --workspace` ✅
- [x] 所有新 API 有文档注释 ✅
- [x] 集成到 `gmp-event-bus` lib.rs 导出 ✅
