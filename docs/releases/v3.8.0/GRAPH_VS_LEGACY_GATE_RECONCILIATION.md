# Graph Gate vs Legacy Gate 第一次真实对账报告

> **日期**: 2026-05-30 20:41
> **分支**: develop/v3.8.0
> **Auditor**: Hermes Agent

---

## 1. 对账来源

| | Graph Gate | Legacy Gate |
|---|---|---|
| **文件** | `.gitea/workflows/gate.yml` (v3.7.0/v3.8.0) | `.gitea/workflows/pr-gate.yml` (v3.0.0/v3.1.0/v2.9.0) |
| **版本** | Evidence Graph Gate v4.1 | Legacy Shell Gate |
| **工作流名** | Evidence Graph Gate v4.1 | PR Gate Pipeline |

---

## 2. 关键差异

| 维度 | Graph Gate | Legacy Gate |
|------|-----------|-------------|
| 触发分支 | develop, main, develop/v3.7.0, develop/v3.8.0 | develop/v3.1.0, develop/v3.0.0, develop/v2.9.0 |
| Runner | hp-z6g4 | hp-z6g4 |
| 检查方式 | Graph-native: 实体→关系→证据链→evaluate | Shell scripts: hermes_gate.sh / check_plan_integrity.sh |
| 证据存储 | SQLite evidence DB (`/tmp/evidence-{ts}.db`) | 无持久化，仅 stdout |
| Gate 评估 | `./target/release/gate evaluate` | bash script exit code |
| 执行策略 | 并行: evidence-graph-gate + legacy-gate | 单一: lint-build → test |
| AST routing 检查 | L4-1~L4-5 (GA_GATE_CHECKLIST.md) | 无 |
| L3 ACID 验证 | isolation_test_suite.py (5项) + crash_sim.py (5项) | 无 |
| L2 执行一致性 | execution_consistency_harness.py + ddl_parity_check.sh | 无 |
| L5 性能门禁 | TPC-H SF=1 + QPS regression + VTU perf + 24h stress | TPC-H SF=0.1（仅部分） |
| L6 文档门禁 | changelog + migration guide + API reference + SSOT cross-check | 无 |

---

## 3. 分层对比

### L1 — Unit Correctness

| | Graph Gate | Legacy Gate |
|---|---|---|
| **检查** | Parser/Executor/Storage/Transaction/WAL 分crate + clippy + fmt | `cargo test --all-features` (整体) + clippy + fmt |
| **winner** | Graph Gate（更精细） | |

### L2 — Execution Consistency

| | Graph Gate | Legacy Gate |
|---|---|---|
| **检查** | execution_consistency_harness.py + ddl_parity_check.sh + E2E tests | 无 |
| **winner** | Graph Gate | |

### L3 — ACID Verification

| | Graph Gate | Legacy Gate |
|---|---|---|
| **检查** | isolation_test_suite.py (5项) + crash_sim.py (5项) + execution_divergence.py (4项) | 无 |
| **winner** | Graph Gate | |

### L4 — Architecture

| | Graph Gate | Legacy Gate |
|---|---|---|
| **检查** | AST routing + execution path + VTU + execution_engine.rs <1500行 | 无 |
| **winner** | Graph Gate | |

### L5 — Performance

| | Graph Gate | Legacy Gate |
|---|---|---|
| **检查** | TPC-H SF=1 + QPS regression + VTU perf + 24h stress + Coverage delta | TPC-H SF=0.1 |
| **winner** | Graph Gate | |

### L6 — Documentation

| | Graph Gate | Legacy Gate |
|---|---|---|
| **检查** | GA Gate Report + changelog + migration guide + API reference + SSOT cross-check | 无 |
| **winner** | Graph Gate | |

---

## 4. 发现的问题

### 🔴 CRITICAL

**Legacy Gate 无法发现 ACID 问题**

L3 ACID 验证（脏读、丢失更新、crash recovery 等）在 legacy PR gate 中不存在，可能导致未发现的事务 bug 进入主线。

**双路径执行一致性仅 Graph Gate 检查**

execution_consistency_harness.py 检查 mysql-server / bench-cli / direct 三路径一致性，legacy gate 无对应检查。

### 🟡 MEDIUM

**Graph Gate 与 Legacy Gate 覆盖不同分支**

v3.7.0/v3.8.0 用 Graph Gate，v3.0.0/v3.1.0 用 Legacy Gate，无一次 push 同时验证两者。

**gate evaluate 结果依赖 graph-cli binary**

Graph Gate 依赖 `./target/release/gate evaluate`，但该 binary 需要先 `cargo build -p graph-cli`，CI 中若 build 失败则无 gate 结果。

**legacy-gate 使用 hermes_gate.sh 后备**

hermes_gate.sh 只做 clippy/fmt/Python syntax/shell syntax 检查，比 legacy pr-gate 原有检查更少（无 test/coverage），降级不等于旧版功能。

---

## 5. 建议

| 优先级 | 行动 | 理由 |
|--------|------|------|
| **P0** | 在 legacy pr-gate.yml 中补充 L3 ACID 验证 | ACID 测试是核心质量关卡，缺失可能导致事务 bug 进入主线 |
| **P0** | 统一 coverage 测量工具链 | Z6G4 81.97% vs Z440 32.59% 差异过大，需统一 cargo-llvm-cov 或 cargo-tarpaulin |
| **P1** | 在所有分支上并行运行 Graph Gate + Legacy Gate | 目前只有 v3.7.0/v3.8.0 有并行 |
| **P1** | 解决 graph-cli build 依赖 | 建议在 CI 中先 build graph-cli 或使用预编译 artifact |
| **P2** | Graph Gate L4-4 execution_engine.rs < 1500 行 | 当前约 6829 行，与目标差距 >450%，需分阶段治理 |

---

## 6. 结论

**Graph Gate v4.1 是 Legacy Gate 的超集。**

| | 覆盖 | 发现的问题 |
|---|---|---|
| Legacy Gate | 3项基础检查（build、test、clippy/fmt） | 无法发现 ACID 问题、执行路径不一致、架构债务、性能退化 |
| Graph Gate | 6层门禁（L1~L6） | 全部覆盖 |

**关键差距**: Legacy Gate 缺少的 L3 ACID 验证和 L2 执行一致性检查属于核心质量关卡。

**建议**: 所有分支统一使用 Graph Gate v4.1 作为唯一门禁标准。

---

*本报告由 Hermes Agent 生成，基于 .gitea/workflows/gate.yml (v3.7.0) 和 .gitea/workflows/pr-gate.yml 源码对比分析。*