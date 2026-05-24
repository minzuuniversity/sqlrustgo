# v1.3.0 门禁检查清单

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta
> **关联门禁规范**: [docs/governance/gate_spec_v120.md](../../governance/gate_spec_v120.md)

---

## 一、门禁信息

| 属性 | 值 |
|------|-----|
| 门禁类型 | L0-Alpha → L0-Beta → L0-RC → L0-GA |
| 关联版本 | v1.3.0 |
| 基线版本 | v1.2.0 (96.08%, MRR 0.497, QPS 8.3) |
| 规范版本 | gate_spec_v120.md v1.0 |
| 当前 Gate | L0-Alpha 🔄 待执行 |

---

## 二、L0-Alpha Gate (A)

> **入口条件**: 核心功能可运行，workspace 全量编译，clippy 零警告

### 2.1 Build 检查

| # | 检查项 | 命令 | 通过标准 | 结果 |
|---|--------|------|---------|------|
| A1 | Workspace Build | `cargo build --release --workspace` | exit_code=0 | ✅ 已验证 |
| A2 | Incremental Build | `cargo build --release -p gmp-cli` | exit_code=0 | ✅ 已验证 |
| A3 | Event Bus Build | `cargo build --release -p gmp-event-bus` | exit_code=0 | ✅ 已验证 |

### 2.2 Lint 检查

| # | 检查项 | 命令 | 通过标准 | 结果 |
|---|--------|------|---------|------|
| A4 | Clippy (workspace) | `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors | ✅ 已验证 |
| A5 | Clippy (event-bus) | `cargo clippy -p gmp-event-bus -- -D warnings` | 0 errors | ✅ 已验证 |
| A6 | Format | `cargo fmt -- --check` | 通过 | ✅ 已验证 |

### 2.3 Test 检查

| # | 检查项 | 命令 | 通过标准 | 结果 |
|---|--------|------|---------|------|
| A7 | Unit Tests | `cargo test --lib` | ≥ 90% 通过 | 🔄 待验证 |
| A8 | Event Bus Tests | `cargo test -p gmp-event-bus` | 全部通过 | 🔄 待验证 |
| A9 | Storage Tests | `cargo test -p gmp-storage` | 全部通过 | 🔄 待验证 |

### 2.4 功能检查

| # | 检查项 | 方法 | 通过标准 | 结果 |
|---|--------|------|---------|------|
| A10 | Event Bus | `cargo test -p gmp-event-bus` | GmpEvent/InMemoryBus/AuditHandler 可用 | ✅ |
| A11 | Audit Middleware | 检查 `AuditContext`, `AuditEmitter`, `hash_event` 导出 | 存在 | ✅ |
| A12 | MQTT Gateway | 检查 `MqttGateway` 导出 | 存在 | ✅ |
| A13 | Binary Embeddings | 检查 `gmp_embeddings` binary BLOB 模式 | Schema 正确 | ✅ |

### 2.5 Alpha 结果汇总

```
L0-Alpha Gate: 🔄 待执行
  PASS: 8/13 (Build + Lint + 功能)
  FAIL: 0/13
  待验证: 5/13 (Tests)
  日期: 2026-05-24
```

---

## 三、L0-Beta Gate (B)

> **入口条件**: L0-Alpha 全部 PASS，所有 crate 集成测试通过

### 3.1 编译与测试

| # | 检查项 | 命令/方法 | 通过标准 | 结果 |
|---|--------|----------|---------|------|
| B1 | Build | `cargo build --release --workspace` | exit_code=0 | 🔄 |
| B2 | All Tests | `cargo test --workspace` | 全部通过 | 🔄 |
| B3 | Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 errors | 🔄 |
| B4 | Format | `cargo fmt -- --check` | 通过 | 🔄 |
| B5 | Audit | `cargo audit` | 0 vulnerabilities | 🔄 |

### 3.2 功能门禁

| # | 检查项 | 命令/方法 | 通过标准 | 结果 |
|---|--------|----------|---------|------|
| B6 | Event Bus 集成 | 测试发布订阅流程 | 事件正确路由 | 🔄 |
| B7 | AuditHandler | 发送 AuditEvent 后查询 gmp_audit_chain | 记录存在 | 🔄 |
| B8 | MQTT Gateway | 模拟设备事件上报 | 事件进入 Event Bus | 🔄 |
| B9 | Binary BLOB HNSW | 初始化后检查 HNSW 索引状态 | 索引可用 | 🔄 |
| B10 | 向量生成流水线 | 验证 feature 分支可编译 | 可构建 | 🔄 |
| B11 | Layer 0 管线 | feature 分支规划 | 代码结构存在 | 🔄 |
| B12 | 文档完整性 | `bash docs/completeness_check.sh 1.3.0` | 21/21 docs | 🔄 |
| B13 | 391 场景回归 | `gmp-eval --test-set test-data/391_scenarios.json` | ≥ 96.08% | 🔄 |
| B14 | 向量通道回归 | 测试 HNSW 查询 | recall@20 ≥ 70% | 🔄 |

### 3.3 Beta 结果汇总

```
L0-Beta Gate: 🔄 待执行
  功能门禁 (B1-B14): 0/14 PASS
  阻塞项: P0-1 向量流水线, P0-3~7 Layer 0 管线
```

---

## 四、L0-RC Gate (R)

> **入口条件**: L0-Beta 全部 PASS，391 场景无退化，文档齐全

### 4.1 RC 门禁

| # | 检查项 | 命令/方法 | 通过标准 | 结果 |
|---|--------|----------|---------|------|
| R1 | Build | `cargo build --release --workspace` | exit_code=0 | 🔄 |
| R2 | All Tests | `cargo test --workspace` | 100% | 🔄 |
| R3 | Clippy | `cargo clippy --all-targets -- -D warnings` | 0 errors | 🔄 |
| R4 | Audit | `cargo audit` | 0 vulnerabilities | 🔄 |
| R5 | 391 场景 | `gmp-eval --test-set test-data/391_scenarios.json` | ≥ 96.08% | 🔄 |
| R6 | 向量生成 | 13809 embeddings 全部生成 | 计数 = 13809 | 🔄 |
| R7 | Layer 0 管线 | 全量 5 步执行完成 | 节点/边统计达标 | 🔄 |
| R8 | 性能 P50 | `gmp-retrieve bench --single` | < 5s | 🔄 |
| R9 | 并发压测 | `gmp-retrieve bench --concurrent 10 --rounds 100` | QPS ≥ 8.3 | 🔄 |
| R10 | 文档门禁 | VERSION_DOCS_SPEC | 21/21 docs | 🔄 |
| R11 | 架构偏离 | ARCHITECTURE_DEVIATION_REPORT | 无 P0 偏离 | 🔄 |
| R12 | Issue 清理 | 全部 v1.3.0 P0/P1 Issue 已关闭 | 100% | 🔄 |

### 4.2 RC 结果汇总

```
L0-RC Gate: 🔄 待执行
  功能门禁 (R1-R12): 0/12 PASS
```

---

## 五、L0-GA Gate (G)

> **入口条件**: L0-RC 全部 PASS，无阻塞性 OPEN Issue

### 5.1 GA 门禁

| # | 检查项 | 命令/方法 | 通过标准 | 结果 |
|---|--------|----------|---------|------|
| G1 | Build | `cargo build --release --workspace` | exit_code=0 | 🔄 |
| G2 | 391 回归 | `gmp-eval --test-set test-data/391_scenarios.json` | ≥ 96.08% | 🔄 |
| G3 | 文档完整性 | VERSION_DOCS_SPEC | 21/21 docs | 🔄 |
| G4 | 版本一致性 | grep 版本号 | 全部 v1.3.0 | 🔄 |
| G5 | 安全 | `cargo audit` | 无高危 | 🔄 |
| G6 | Git Tag | `git tag v1.3.0` | 正确打标 | 🔄 |
| G7 | Gitea Release | Release 页面创建 | 完整 Release | 🔄 |
| G8 | develop/v1.3.0 保护 | 分支保护规则 | 1 approval required | 🔄 |

### 5.2 GA 结果汇总

```
L0-GA Gate: 🔄 待执行
  功能门禁 (G1-G8): 0/8 PASS
```

---

## 六、失败项处理

| # | 检查项 | 失败原因 | Issue | 状态 |
|---|--------|----------|-------|------|
| — | — | — | — | — |

---

## 七、门禁历史

| 日期 | 门禁 | 结果 | 执行人 |
|------|------|------|--------|
| 2026-05-21 | v1.0.0 L0-GA | PASS | Hermes |
| 2026-05-22 | v1.1.0 L0-GA | PASS | Hermes + Claude |
| 2026-05-23 | v1.2.0 L0-Beta | 🔄 | — |
| 2026-05-24 | v1.3.0 L0-Alpha | 🔄 待执行 | — |

---

## 八、性能基准

| 指标 | v1.2.0 基线 | v1.3.0 目标 | 状态 |
|------|-------------|-------------|------|
| 391/408 场景通过率 | 96.08% | ≥ 96.08% | 🔄 |
| MRR | 0.497 | ≥ 0.497 | 🔄 |
| R8 QPS | 8.3 | ≥ 8.3 | 🔄 |
| R8 P95 | 1366ms | < 1500ms | 🔄 |
| 向量索引覆盖率 | 0% | 100% (13809) | 🔄 |
| 图谱节点数 | ~700 | > 5000 | 🔄 |

---

> **说明**: L0-Beta Gate 需在 P0 功能（向量索引、Layer 0 管线）完成后方可执行。当前 develop/v1.3.0 合入的 Event Bus、Audit Middleware、Binary Embeddings、MQTT Gateway 已满足 L0-Alpha 功能要求。
