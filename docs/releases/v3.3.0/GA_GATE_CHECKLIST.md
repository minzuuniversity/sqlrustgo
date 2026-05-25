# v3.3.0 GA Gate Checklist

> **版本**: v3.3.0-ga-gate
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **阶段**: GA (General Availability)
> **分支**: `origin/release/v3.3.0`
> **起点**: `v3.3.0 RC release point`
> **关联门禁规范**: `docs/governance/GATE_SPEC_MASTER.md`

---

## 一、门禁信息

### 1.1 版本概述

**v3.3.0 战略定位**: Industrial Trust Platform（工业级可信闭环平台）

**核心价值**: 在 v3.2.x 内核修复基础上，构建完整的 Trust Infrastructure 四层架构，实现可信性工业闭环。

**版本演进**:
```
v3.2.x: 内核修复（覆盖率/性能/MySQL协议）
    ↓
v3.3.0: Industrial Trust Platform（可信性工业闭环）← 当前
    ↓
v3.4.0: GMP Management Suite
    ↓
v3.5.0: AI Native GMP Platform
```

### 1.2 入口条件

- [x] v3.2.0 GA 通过
- [x] Alpha Gate 通过
- [x] Beta Gate 通过
- [x] RC Gate 通过

### 1.3 Trust Infrastructure 四层架构

| 层级 | 组件 | 状态 |
|------|------|------|
| L4 | Trust Visualization | ⏳ |
| L3 | Compliance Evidence Engine | ⏳ |
| L2 | Provenance Graph | ⏳ |
| L1 | Trust Infrastructure Kernel | ⏳ |

---

## 二、Pre-Gate 自检清单

### 2.1 代码质量检查

| 检查项 | 命令 | 期望结果 | 状态 |
|--------|------|----------|------|
| cargo build | `cargo build --release --workspace` | 编译成功 | ⏳ |
| cargo clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ⏳ |
| cargo fmt | `cargo fmt --check` | 通过 | ⏳ |
| cargo test | `cargo test --lib` | 全部通过 | ⏳ |

### 2.2 覆盖率检查

| 检查项 | 命令 | 期望结果 | 实际结果 | 状态 |
|--------|------|----------|----------|------|
| L1 覆盖率 | `cargo llvm-cov test L1_CRATES --lib` | ≥85% | ⏳ | ⏳ |

---

## 三、正式门禁检查 (GA)

### 3.1 核心检查 (G1-G12)

| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| G1 | Build | `cargo build --release` | 成功 | ⏳ |
| G2 | Test | `cargo test --lib` | 全部通过 | ⏳ |
| G3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ⏳ |
| G4 | Format | `cargo fmt --check` | 通过 | ⏳ |
| G5 | Coverage | `cargo llvm-cov test L1_CRATES --lib` | ≥85% | ⏳ |
| G6 | Security | `cargo audit` | 无漏洞 | ⏳ |
| G7 | MySQL Protocol | mysql client 连接 | 连接成功 | ⏳ |
| G8 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 | ⏳ |
| G9 | Proofs | TLA+ model check | 无 violation | ⏳ |
| G10 | Docs | All OO docs | 全部存在 | ⏳ |

### 3.2 Trust Infrastructure 四层验证 (G-TI1~TI4)

| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| G-TI1 | Trust Infrastructure Kernel | perf-baseline + crash-sim + wal-verification | 通过 | ⏳ |
| G-TI2 | Provenance Graph | `cargo test -p sqlrustgo-provenance-graph --lib` | 通过 | ⏳ |
| G-TI3 | Compliance Evidence Engine | `cargo test -p sqlrustgo-compliance-engine --lib` + evidence-engine | 通过 | ⏳ |
| G-TI4 | Trust Visualization | `cargo test -p sqlrustgo-trust-viz --lib` | 通过 | ⏳ |

### 3.3 GMP 核心测试 (G-GMP1~G-GMP8)

| # | 检查项 | 命令 | 期望结果 | 状态 |
|---|--------|------|----------|------|
| G-GMP1 | gmp lib tests | `cargo test -p sqlrustgo-gmp --lib` | 全部通过 | ⏳ |
| G-GMP2 | Audit Chain | `cargo test -p sqlrustgo-gmp --test gmp_audit_chain_verify_test` | 通过 | ⏳ |
| G-GMP3 | Digital Signature | `cargo test -p sqlrustgo-gmp --test gmp_digital_signature_test` | 通过 | ⏳ |
| G-GMP4 | Electronic Signature | `cargo test -p sqlrustgo-gmp --test gmp_electronic_signature_test` | 通过 | ⏳ |
| G-GMP5 | Workflow V2 | `cargo test -p sqlrustgo-gmp --test gmp_workflow_v2_test` | 通过 | ⏳ |
| G-GMP6 | Evidence Engine | `cargo test -p sqlrustgo-gmp --test evidence_export_test` | 通过 | ⏳ |
| G-GMP7 | Immutable Record | `cargo test -p sqlrustgo-gmp --test gmp_immutable_record_test` | 通过 | ⏳ |
| G-GMP8 | Provenance | `cargo test -p sqlrustgo-gmp --test gmp_provenance_test` | 通过 | ⏳ |

### 3.4 稳定性与性能 (G-SP1~SP4)

| # | 检查项 | 说明 | 状态 |
|---|--------|------|-------|
| G-SP1 | 72h Write | 长时间写入稳定性 | ⏳ |
| G-SP2 | 72h Read/Write | 混合负载稳定性 | ⏳ |
| G-SP3 | Performance regression | <5% vs baseline | ⏳ |
| G-SP4 | Crash recovery | 10,000次注入无损坏 | ⏳ |

---

## 四、检查结果汇总

| 类别 | 通过数 | 总数 | 通过率 | 状态 |
|------|--------|------|--------|------|
| 核心检查 G1-G10 | 0 | 10 | 0% | ⏳ |
| Trust Infrastructure G-TI1~TI4 | 0 | 4 | 0% | ⏳ |
| GMP 核心 G-GMP1~G-GMP8 | 0 | 8 | 0% | ⏳ |
| 稳定性与性能 G-SP1~SP4 | 0 | 4 | 0% | ⏳ |
| **总计** | **0** | **26** | **0%** | **⏳** |

---

## 五、Issue 状态

### 5.1 待完成 Issue

| Issue | 标题 | 优先级 |
|-------|------|--------|
| #1196/#1197 | executor 覆盖率 <85% | P0 |
| #1201 | MySQL Protocol 握手失败 | P0 |
| #1198 | TPC-H SF=1 + 72h 稳定性 | P1 |
| #1236 | Crash Simulation Framework | Beta |
| #1237 | WAL Formal Verification | Beta |
| #1238 | Compliance-as-Code Engine | Beta |
| #1239 | Evidence Engine | RC |
| #1240 | Provenance Knowledge Graph | RC |
| #1241 | Workflow V2 | RC |
| #1242 | Trust Visualization | GA |

---

## 六、附录

### A.1 相关文档

- 门禁规范: `docs/governance/GATE_SPEC_MASTER.md`
- 开发计划: `docs/releases/v3.3.0/DEV_PLAN.md`
- 综合状态报告: `docs/releases/v3.3.0/COMPREHENSIVE_STATUS_REPORT.md`

### A.2 关键文件路径

| 文件 | 路径 |
|------|------|
| Trust Infrastructure | `crates/perf-baseline/`, `crates/crash-sim/`, `crates/wal-verification/` |
| Compliance Engine | `crates/compliance-engine/` |
| Evidence Engine | `crates/evidence-engine/` |
| Provenance Graph | `crates/provenance-graph/` |
| Trust Visualization | `crates/trust-viz/` |

---

*最后更新: 2026-05-18*
