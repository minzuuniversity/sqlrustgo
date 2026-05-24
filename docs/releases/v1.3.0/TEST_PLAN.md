# v1.3.0 测试计划

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta

---

## 一、测试范围

### 1.1 已完成功能测试范围

| 功能 | 测试类型 | 工具 | 状态 |
|------|---------|------|------|
| Event Bus (GmpEvent, InMemoryBus) | 单元测试 | `cargo test -p gmp-event-bus` | ✅ |
| Audit Middleware (AuditContext, AuditEmitter) | 单元测试 | `cargo test -p gmp-event-bus` | ✅ |
| MQTT Gateway | 模拟测试 | `cargo test -p gmp-event-bus -- mqtt` | 🔄 |
| Binary BLOB Embeddings | 单元测试 | `cargo test -p gmp-storage` | 🔄 |
| HNSW Index Auto-build | 集成测试 | `cargo test -p gmp-storage` | 🔄 |
| DiskGraphStore WAL | 集成测试 | `cargo test -p gmp-storage` | 🔄 |
| gmp-cli (不含 Populate) | CLI 测试 | `cargo test -p gmp-cli` | 🔄 |

### 1.2 待开发功能测试范围

| 功能 | 测试类型 | 工具 | 状态 |
|------|---------|------|------|
| 向量索引生成流水线 (P0-1) | E2E 测试 | `scripts/embed/generate_embeddings.sh` | 🔄 待实现 |
| Layer 0 术语库 (P0-3) | 单元测试 | `cargo test -p gmp-kg-pipeline` | 🔄 待实现 |
| Layer 0 NER (P0-4) | 单元测试 | `cargo test -p gmp-kg-pipeline` | 🔄 待实现 |
| Layer 0 关系抽取 (P0-5) | 单元测试 | `cargo test -p gmp-kg-pipeline` | 🔄 待实现 |
| Layer 0 多维分类 (P0-6) | 单元测试 | `cargo test -p gmp-kg-pipeline` | 🔄 待实现 |
| Layer 0 预评分 (P0-7) | 集成测试 | `cargo test -p gmp-kg-pipeline` | 🔄 待实现 |

---

## 二、L0-Alpha Gate 测试用例

### 2.1 Build 测试

| ID | 测试项 | 命令 | 期望结果 | 状态 |
|----|--------|------|---------|------|
| A-B1 | Workspace Build | `cargo build --release --workspace` | exit_code=0 | 🔄 |
| A-B2 | Event Bus Build | `cargo build --release -p gmp-event-bus` | exit_code=0 | 🔄 |
| A-B3 | Storage Build | `cargo build --release -p gmp-storage` | exit_code=0 | 🔄 |

### 2.2 Lint 测试

| ID | 测试项 | 命令 | 期望结果 | 状态 |
|----|--------|------|---------|------|
| A-L1 | Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors | 🔄 |
| A-L2 | Format | `cargo fmt -- --check` | 通过 | 🔄 |

### 2.3 Unit 测试

| ID | 测试项 | 命令 | 期望结果 | 状态 |
|----|--------|------|---------|------|
| A-T1 | All Lib Tests | `cargo test --lib` | ≥ 90% | 🔄 |
| A-T2 | Event Bus Tests | `cargo test -p gmp-event-bus` | 全部通过 | 🔄 |
| A-T3 | Storage Tests | `cargo test -p gmp-storage` | 全部通过 | 🔄 |

---

## 三、L0-Beta Gate 测试用例

### 3.1 391 场景回归测试

```bash
# 391 场景评估
gmp-eval --test-set test-data/391_scenarios.json --min-pass-rate 0.9608

# 期望: ≥ 96.08% (无退化)
```

### 3.2 向量通道回归测试

```bash
# 向量 HNSW 查询
cargo run --release --bin gmp-cli -- search "洁净区级别划分" --channels vector

# 期望: recall@20 ≥ 70%
```

### 3.3 Event Bus E2E 测试

```bash
# 发布订阅端到端
cargo test -p gmp-event-bus --test '*e2e*'
```

### 3.4 MQTT 模拟测试

```bash
# 需要 MQTT Broker (localhost:1883)
python3 scripts/mqtt_simulator.py --device scale-01 --weight 1250.5
cargo test -p gmp-event-bus -- mqtt
```

---

## 四、L0-RC Gate 测试用例

### 4.1 向量索引全量生成测试

| ID | 测试项 | 命令 | 期望结果 | 状态 |
|----|--------|------|---------|------|
| R-V1 | 生成计数 | `bash scripts/embed/generate_embeddings.sh` | 13809 embeddings | 🔄 |
| R-V2 | 索引验证 | `cargo run --bin gmp-cli -- verify-embeddings` | count=13809 | 🔄 |
| R-V3 | HNSW 查询 | 向量查询 recall@20 | ≥ 70% | 🔄 |

### 4.2 Layer 0 管线测试

| ID | 测试项 | 命令 | 期望结果 | 状态 |
|----|--------|------|---------|------|
| R-L0-1 | 术语库构建 | `gmp-kg-pipeline --step glossary` | ≥ 600 术语 | 🔄 |
| R-L0-2 | NER 识别 | `gmp-kg-pipeline --step ner` | 召回率 ≥ 85% | 🔄 |
| R-L0-3 | 关系抽取 | `gmp-kg-pipeline --step re` | ≥ 500 三元组 | 🔄 |
| R-L0-4 | 多维分类 | `gmp-kg-pipeline --step classify` | 8 大章全覆盖 | 🔄 |
| R-L0-5 | 图谱写入 | `gmp-kg-pipeline --step graph` | 节点 > 5000 | 🔄 |

---

## 五、性能测试用例

| ID | 指标 | 测试方法 | 基线 | 目标 | 状态 |
|----|------|---------|------|------|------|
| P1 | 391 场景通过率 | `gmp-eval` | 96.08% | ≥ 96.08% | 🔄 |
| P2 | MRR | `gmp-eval --metrics mrr` | 0.497 | ≥ 0.497 | 🔄 |
| P3 | R8 QPS | `gmp-retrieve bench` | 8.3 | ≥ 8.3 | 🔄 |
| P4 | R8 P95 | `gmp-retrieve bench` | 1366ms | < 1500ms | 🔄 |
| P5 | 向量生成耗时 | `scripts/embed/generate_embeddings.sh` | N/A | < 2h | 🔄 |
| P6 | Layer 0 管线耗时 | `gmp-kg-pipeline --all` | N/A | < 4h | 🔄 |
