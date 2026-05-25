# v3.3.0 Beta Gate Checklist

> **版本**: v3.3.0-beta-gate
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **阶段**: Beta
> **分支**: `origin/develop/v3.3.0`
> **起点**: `v3.2.0 GA release point`
> **关联门禁规范**: `docs/governance/GATE_SPEC_MASTER.md`

---

## 一、Beta Gate 入口条件

- [x] Alpha Gate 通过
- [x] 所有 P0 功能完成
- [x] 单元测试覆盖率 ≥75%
- [ ] MySQL Protocol 修复完成
- [ ] TPC-H SF=1 22/22 通过

---

## 二、Beta Gate 检查

### 2.1 代码质量

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B1 | Build | `cargo build --release --workspace` | 成功 | ⏳ |
| B2 | 单元测试 | `cargo test --lib` | ≥90% 通过 | ⏳ |
| B3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ⏳ |
| B4 | 格式化 | `cargo fmt --all -- --check` | 通过 | ⏳ |

### 2.2 Trust Infrastructure 四层

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B5 | perf-baseline | `cargo test -p sqlrustgo-perf-baseline --lib` | 通过 | ⏳ |
| B6 | crash-sim | `cargo test -p sqlrustgo-crash-sim --lib` | 1000次注入无损坏 | ⏳ |
| B7 | wal-verification | `cargo test -p sqlrustgo-wal-verification --lib` | TLA+ model check | ⏳ |
| B8 | compliance-engine | `cargo test -p sqlrustgo-compliance-engine --lib` | 规则引擎测试 | ⏳ |
| B9 | evidence-engine | `cargo test -p sqlrustgo-evidence-engine --lib` | 审计包验证 | ⏳ |
| B10 | provenance-graph | `cargo test -p sqlrustgo-provenance-graph --lib` | Cypher 查询测试 | ⏳ |
| B11 | workflow-v2 | `cargo test -p sqlrustgo-workflow-v2 --lib` | 集成测试 | ⏳ |
| B12 | trust-viz | `cargo test -p sqlrustgo-trust-viz --lib` | 可视化组件 | ⏳ |

### 2.3 GMP 核心

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B13 | gmp lib tests | `cargo test -p sqlrustgo-gmp --lib` | 全部通过 | ⏳ |
| B14 | Audit Chain | `cargo test -p sqlrustgo-gmp --test gmp_audit_chain_verify_test` | 通过 | ⏳ |

---

## 三、结果汇总

| 类别 | 通过数 | 总数 | 通过率 | 状态 |
|------|--------|------|--------|------|
| 代码质量 B1-B4 | 0 | 4 | 0% | ⏳ |
| Trust Infrastructure B5-B12 | 0 | 8 | 0% | ⏳ |
| GMP 核心 B13-B14 | 0 | 2 | 0% | ⏳ |
| **总计** | **0** | **14** | **0%** | **⏳** |

---

## 四、待完成项

| ID | 检查项 | 说明 |
|----|--------|------|
| P0 | MySQL Protocol 修复 | Issue #1201 |
| P0 | 覆盖率提升至 85% | Issue #1196/#1197 |
| P1 | TPC-H SF=1 | Issue #1198 |
| P1 | 72h 稳定性测试 | Issue #1198 |

---

## 五、关联 Issue

| Issue | 标题 | 优先级 | 状态 |
|-------|------|--------|------|
| #1201 | MySQL Protocol 握手失败 | P0 | 🔴 Open |
| #1196/#1197 | executor 覆盖率 <85% | P0 | 🔴 Open |
| #1198 | TPC-H SF=1 + 72h 稳定性 | P1 | 🔴 Open |
| #1235 | Performance Governance System | — | ✅ 已合并 |
| #1236 | Crash Simulation Framework | Beta | 🔴 Open |
| #1237 | WAL Formal Verification | Beta | 🔴 Open |

---

*最后更新: 2026-05-18*
