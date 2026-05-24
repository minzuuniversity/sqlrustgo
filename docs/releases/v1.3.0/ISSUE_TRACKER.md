# v1.3.0 Issue 追踪

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`

---

## 一、v1.3.0 Milestone Issues

| # | 标题 | Priority | 状态 | PR | 里程碑 |
|---|------|---------|------|-----|--------|
| #121 | GMP-Platform v1.3.0 — Unified Knowledge & Compliance Platform | P0 | 🔄 开发中 | — | v1.3.0 |
| #122 | 向量索引生成流水线 — 13809 embeddings 全量生成 | P0 | 🔄 开发中 | (feature 分支) | v1.3.0 |
| #123 | DiskGraphStore WAL 持久化 — 图数据支持重启恢复 | P0 | 🔄 开发中 | (feature 分支) | v1.3.0 |
| #124 | Layer 0 离线知识抽取管线 — 术语库 + NER + RE + 分类 + 预评分 | P0 | 🔄 规划中 | — | v1.3.0 |
| #125 | MQTT 设备集成 — 接入称重设备、温湿度传感器 | P1 | ✅ 已完成 | #136 | v1.3.0 |
| #126 | 技术债清理 — cargo clippy + cargo audit | P1 | ✅ 已完成 | #135 | v1.3.0 |
| #130 | KnowledgeEntity + Hybrid Search — Graph+Vector+LLM 融合检索 | P0 | 🔄 规划中 | — | v1.3.0 |
| #131 | Event Bus Crate — GmpEvent types, InMemoryBus, AuditHandler | P0 | ✅ 已完成 | #133 | v1.3.0 |
| #132 | Audit Middleware — AuditContext, AuditEmitter, hash utils | P0 | ✅ 已完成 | (direct) | v1.3.0 |

---

## 二、Issue 详情

### #131 Event Bus Crate ✅

**状态**: 已完成并合入 develop/v1.3.0
**PR**: #133
**Commit**: `0104cb9`
**内容**:
- `GmpEvent` 类型系统（AuditEvent, MqttEvent, WorkflowEvent）
- `InMemoryBus` 发布订阅实现
- `AuditHandler` 审计处理器

### #132 Audit Middleware ✅

**状态**: 已完成并合入 develop/v1.3.0
**PR**: 无（直接合入）
**Commit**: `7380869`
**内容**:
- `AuditContext` 请求级审计上下文
- `AuditEmitter` 便捷 API（emit_search, emit_insert, emit_update, emit_delete, emit_sign, emit_workflow）
- `hash_event` 完整性校验工具

### #125 MQTT Gateway ✅

**状态**: 已完成并合入 develop/v1.3.0
**PR**: #136
**Commit**: `28d1af5`
**内容**:
- `MqttGateway` 设备事件发射器
- 通过 Event Bus 路由到 AuditHandler
- 支持称重设备、温湿度传感器等 IoT 设备

### #126 技术债清理 ✅

**状态**: 已完成并合入 develop/v1.3.0
**PR**: #135
**Commits**: `021f075`, `4dbd8a3`
**内容**:
- `cargo clippy --all-targets --all-features -- -D warnings` → 0 errors
- `cargo audit` → 0 vulnerabilities
- 无 `#[ignore]` 测试

### #122 向量索引生成流水线 🔄

**状态**: 开发中（feature 分支）
**相关完成**:
- Binary BLOB embeddings ✅ (`3aeccf0`)
- HNSW 索引自动构建 ✅ (`3aeccf0`)
**待完成**: 13809 embeddings 全量生成流水线

### #123 DiskGraphStore WAL 持久化 🔄

**状态**: 开发中（feature 分支）
**相关完成**:
- WAL 持久化基础架构 ✅ (`cf65908`)
**待完成**: 完整 WAL 恢复逻辑

### #124 Layer 0 离线知识抽取管线 🔄

**状态**: 规划中（待创建 feature 分支）
**子任务**:
- P0-3: Layer 0 术语库（600+ GMP 术语 + 300+ 同义词组）
- P0-4: Layer 0 NER 实体识别
- P0-5: Layer 0 关系抽取
- P0-6: Layer 0 多维分类
- P0-7: Layer 0 相关性预评分

---

## 三、已关闭 Issues

| # | 标题 | 状态 | 关闭日期 |
|---|------|------|----------|
| #125 | MQTT 设备集成 | ✅ Closed | 2026-05-24 |
| #126 | 技术债清理 | ✅ Closed | 2026-05-24 |
| #131 | Event Bus Crate | ✅ Closed | 2026-05-24 |
| #132 | Audit Middleware | ✅ Closed | 2026-05-24 |

---

## 四、v1.3.0 vs v1.2.0 Issue 对比

| 指标 | v1.2.0 | v1.3.0 |
|------|---------|---------|
| 总 Issues | ~15 | 9 (+ 待规划) |
| P0 规划 | — | 5 |
| P1 完成 | ~10 | 2 |
| P0 完成 | — | 2 |
| 阻塞项 | 0 | 1 (P0-1 流水线) |
