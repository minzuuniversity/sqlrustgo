# v1.3.0 测试覆盖率报告

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta

---

## 一、覆盖率状态

> ⚠️ **Beta 阶段说明**: 覆盖率数据需在 L0-Alpha Gate 完成后生成。以下为计划指标。

---

## 二、Crate 覆盖率目标

| Crate | 单元测试目标 | 集成测试目标 | 状态 |
|-------|-------------|-------------|------|
| gmp-core | ≥ 80% | N/A | 🔄 |
| gmp-storage | ≥ 80% | ≥ 60% | 🔄 |
| gmp-retrieval | ≥ 80% | ≥ 60% | 🔄 |
| gmp-reranker | ≥ 80% | N/A | 🔄 |
| gmp-llm | ≥ 70% | N/A | 🔄 |
| gmp-cli | ≥ 60% | ≥ 50% | 🔄 |
| gmp-server | ≥ 70% | ≥ 50% | 🔄 |
| gmp-eval | ≥ 80% | N/A | 🔄 |
| gmp-api | ≥ 70% | ≥ 60% | 🔄 |
| gmp-event-bus | ≥ 90% | ≥ 80% | 🔄 |

---

## 三、覆盖率测量命令

### 3.1 生成覆盖率报告

```bash
# 安装 cargo-llvm-cov（如未安装）
cargo install cargo-llvm-cov

# 生成覆盖率报告
cargo llvm-cov --all-features --workspace --html --open

# 文本报告
cargo llvm-cov --all-features --workspace --text
```

### 3.2 单 Crate 覆盖率

```bash
cargo llvm-cov -p gmp-event-bus --html --open
cargo llvm-cov -p gmp-storage --html --open
```

---

## 四、覆盖范围说明

### 4.1 单元测试覆盖

- ✅ gmp-event-bus: `AuditContext`, `AuditEmitter`, `hash_event`, `InMemoryBus`
- 🔄 gmp-storage: `StorageEngine`, `SqlrustgoAdapter`, Binary BLOB
- 🔄 gmp-retrieval: `ChannelEngine`, `RRF fusion`
- 🔄 gmp-reranker: `NoopReranker`, `OllamaReranker`

### 4.2 集成测试覆盖

- 🔄 Event Bus E2E: 发布订阅端到端
- 🔄 MQTT 模拟: 设备事件上报审计链
- 🔄 391 场景: 全链路检索回归

---

## 五、目标指标

| 指标 | 当前 | Beta 目标 | RC 目标 |
|------|------|-----------|---------|
| Workspace 平均行覆盖率 | 🔄 | ≥ 70% | ≥ 80% |
| gmp-event-bus 行覆盖率 | 🔄 | ≥ 90% | ≥ 95% |
| gmp-storage 行覆盖率 | 🔄 | ≥ 70% | ≥ 80% |
| 集成测试覆盖率 | 🔄 | ≥ 60% | ≥ 75% |
