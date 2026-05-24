# GMP-Platform v1.3.0 发布摘要

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **阶段状态**: Beta（开发中）
> **分支**: `develop/v1.3.0`
> **基于版本**: v1.2.0 (2026-05-23)

---

## 发布类型

- [ ] 正式版本 (GA)
- [ ] 候选版本 (RC)
- [x] 测试版本 (Beta/Alpha)

## 发布状态

- [x] 已完成开发（Beta 门禁前）
- [ ] 候选版本 (RC)
- [ ] 正式版本 (GA)

## 发布流程

1. **v1.0.0 GA**: 四通道检索 + Reranker + LLM 集成 ✅ (2026-05-21)
2. **v1.1.0 GA**: BM25 修复 + 集成测试 + 向量通道优化 ✅ (2026-05-22)
3. **v1.2.0 Beta**: Agent 编排层 + JWT 认证 + E2E 测试 + 移动端 ✅ (2026-05-23)
4. **v1.3.0 开发**: Event Bus + Audit Middleware + MQTT + 向量索引增强 + WAL ✅ (2026-05-24)
5. **v1.3.0 门禁**: L0-Alpha → L0-Beta → L0-RC → L0-GA — **规划中**

## 阶段目标

v1.3.0 聚焦三大核心能力，为 v2.0 综合管理平台奠定基础：

1. **数据完备**: Binary BLOB embeddings + HNSW 索引自动构建
2. **平台基座**: Event Bus + Audit Middleware + MQTT 设备集成
3. **图持久化**: DiskGraphStore WAL 持久化，支持重启恢复

## 构建信息

| 属性 | 值 |
|------|-----|
| 构建环境 | macOS / Linux x86_64 |
| Rust 版本 | ≥ 1.80 |
| 编译状态 | ✅ `cargo build --release --workspace` clean |
| Clippy | ✅ 0 errors (`-D warnings`) |
| Cargo Audit | ✅ 0 vulnerabilities |
| 测试状态 | 🔄 待完整门禁验证 |

## v1.2.0 基线数据

| 指标 | 值 |
|------|-----|
| 391/408 场景通过率 | 96.08% |
| MRR | 0.497 |
| R8 QPS | 8.3 |
| R8 P95 | 1366ms |

## v1.3.0 Beta 里程碑

```
2026-05-24  Event Bus + Audit Middleware 合入 ✅
2026-05-24  Binary Embeddings + HNSW auto-build 合入 ✅
2026-05-24  DiskGraphStore WAL 持久化合入 ✅
2026-05-24  MQTT Gateway 合入 ✅
2026-05-24  clippy + audit 技术债清理合入 ✅
TBD         P0-1 向量索引流水线上线
TBD         P0-2 DiskGraphStore WAL 持久化完整测试
TBD         P0-3~7 Layer 0 管线完成
TBD         v1.3.0 L0-Alpha Gate
TBD         v1.3.0 L0-Beta Gate
TBD         v1.3.0 L0-RC Gate
TBD         v1.3.0 L0-GA 发布
```

## Beta 版本已合并功能

| 功能 | Commit | 状态 |
|------|--------|------|
| Event Bus Crate (GmpEvent, InMemoryBus, AuditHandler) | `0104cb9` | ✅ |
| Audit Middleware (AuditContext, AuditEmitter, hash utils) | `7380869` | ✅ |
| Binary BLOB Embeddings + HNSW auto-build on init | `3aeccf0` | ✅ |
| DiskGraphStore WAL persistence | `cf65908` | ✅ |
| MQTT Gateway for device integration | `28d1af5` | ✅ |
| clippy warnings cleanup | `021f075`, `4dbd8a3` | ✅ |
| cargo audit 0 vulnerabilities | (PR #135) | ✅ |

## 后续 P0 功能（开发中）

| 功能 | 分支 | 状态 |
|------|------|------|
| 向量索引生成流水线 (13809 embeddings) | `feature/issue-122-vector-embeddings` | 🔄 开发中 |
| Layer 0 知识抽取管线 (P0-3~7) | 待创建 | 🔄 规划中 |
| DiskGraphStore WAL 完整功能 | `feature/issue-123-disk-graph-wal` | 🔄 开发中 |

## 核心提交记录

| SHA (7位) | 描述 | Issue |
|-----------|------|-------|
| `0104cb9` | feat(gmp-event-bus): implement Event Bus crate | #131 |
| `7380869` | feat(gmp-event-bus): add Audit Middleware | #132 |
| `3aeccf0` | feat(storage): binary BLOB embeddings + HNSW auto-build | #122 |
| `cf65908` | feat(graph): DiskGraphStore WAL persistence | #123 |
| `28d1af5` | feat(gmp-event-bus): add MqttGateway | #125 |
| `021f075` | fix(gmp-cli): simplify Commands::Populate to DEPRECATED | #126 |
| `4dbd8a3` | fix(gmp-storage): TempDir::keep() clippy fix | #126 |

---

> **说明**: 本摘要反映 v1.3.0 Beta 阶段状态。P0 功能（向量索引、Layer 0 管线）仍在开发中，需完成 L0-Alpha 门禁后推进。
