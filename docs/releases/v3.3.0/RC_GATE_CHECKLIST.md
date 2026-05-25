# v3.3.0 RC Gate Checklist

> **版本**: v3.3.0-rc-gate
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **阶段**: RC (Release Candidate)
> **分支**: `origin/rc/v3.3.0`
> **起点**: `v3.3.0 Beta release point`
> **关联门禁规范**: `docs/governance/GATE_SPEC_MASTER.md`

---

## 一、RC Gate 入口条件

- [x] Alpha Gate 通过
- [x] Beta Gate 通过
- [x] 所有 Trust Infrastructure 四层架构完成
- [x] 创建 `rc/v3.3.0` 分支
- [x] 合并 Beta 代码到 RC 分支

---

## 二、RC Gate 检查

### 2.1 代码质量检查 (R1-R4)

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| R1 | Build | `cargo build --release --workspace` | 成功 | ⏳ |
| R2 | Test | `cargo test --all-features --workspace` | 全部通过 | ⏳ |
| R3 | Clippy | `cargo clippy --all-features --workspace -- -D warnings` | 零警告 | ⏳ |
| R4 | Format | `cargo fmt --all -- --check` | 通过 | ⏳ |

### 2.2 覆盖率检查 (R5)

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| R5 | L1 覆盖率 | `cargo llvm-cov test --lib L1_CRATES` | ≥85% | ⏳ |

> **L1 Crates**: sqlrustgo, sqlrustgo-executor, sqlrustgo-optimizer, sqlrustgo-storage,
> sqlrustgo-types, sqlrustgo-parser, sqlrustgo-planner, sqlrustgo-transaction, sqlrustgo-catalog

### 2.3 安全与兼容性 (R6-R8)

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| R6 | Security | `cargo audit` | 无漏洞 | ⏳ |
| R7 | SQL Compat | `bash scripts/gate/check_sql_compat.sh` | 通过 | ⏳ |
| R8 | TPC-H SF=1 | `sqlrustgo-bench-cli tpch --scale 1` | 22/22 | ⏳ |

### 2.4 Beta 门禁复核 (B1-B14)

| # | 检查项 | 命令 | Beta 状态 | RC 状态 |
|---|--------|------|-----------|---------|
| B1 | Build | `cargo build --release --workspace` | ⏳ | ⏳ |
| B2 | Unit tests | `cargo test --lib` | ⏳ | ⏳ |
| B3 | Clippy | `cargo clippy --all-features` | ⏳ | ⏳ |
| B4 | Format | `cargo fmt --all -- --check` | ⏳ | ⏳ |
| B5 | perf-baseline | `cargo test -p sqlrustgo-perf-baseline --lib` | ⏳ | ⏳ |
| B6 | crash-sim | `cargo test -p sqlrustgo-crash-sim --lib` | ⏳ | ⏳ |
| B7 | wal-verification | `cargo test -p sqlrustgo-wal-verification --lib` | ⏳ | ⏳ |
| B8 | compliance-engine | `cargo test -p sqlrustgo-compliance-engine --lib` | ⏳ | ⏳ |
| B9 | evidence-engine | `cargo test -p sqlrustgo-evidence-engine --lib` | ⏳ | ⏳ |
| B10 | provenance-graph | `cargo test -p sqlrustgo-provenance-graph --lib` | ⏳ | ⏳ |
| B11 | workflow-v2 | `cargo test -p sqlrustgo-workflow-v2 --lib` | ⏳ | ⏳ |
| B12 | trust-viz | `cargo test -p sqlrustgo-trust-viz --lib` | ⏳ | ⏳ |
| B13 | gmp lib tests | `cargo test -p sqlrustgo-gmp --lib` | ⏳ | ⏳ |
| B14 | Audit Chain | `cargo test -p sqlrustgo-gmp --test gmp_audit_chain_verify_test` | ⏳ | ⏳ |

### 2.5 RC 稳定性测试 (R-S1~S4)

| # | 检查项 | 说明 | 状态 |
|---|--------|------|-------|
| R-S1 | 16h Write | Z6G4 执行 | ⏳ |
| R-S2 | 16h Read/Write | Z6G4 执行 | ⏳ |
| R-S3 | 24h Write | Z6G4 执行 | ⏳ |
| R-S4 | 24h Read/Write | Z6G4 执行 | ⏳ |

---

## 三、结果汇总

| 类别 | PASS | FAIL | SKIP | 待执行 |
|------|------|------|------|--------|
| 代码质量 (R1-R4) | 0 | 0 | 0 | 4 |
| 覆盖率 (R5) | 0 | 0 | 0 | 1 |
| 安全兼容 (R6-R8) | 0 | 0 | 0 | 3 |
| Beta 复核 (B1-B14) | 0 | 0 | 0 | 14 |
| 稳定性 (R-S1~S4) | 0 | 0 | 0 | 4 |
| **总计** | **0** | **0** | **0** | **26** |

---

## 四、关键 Issue

| Issue | 标题 | 优先级 |
|-------|------|--------|
| #1196/#1197 | executor 覆盖率 <85% | P0 |
| #1201 | MySQL Protocol 握手失败 | P0 |
| #1198 | TPC-H SF=1 + 72h 稳定性 | P1 |

---

*最后更新: 2026-05-18*
