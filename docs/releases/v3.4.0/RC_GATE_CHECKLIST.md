# v3.4.0 RC Gate Checklist

> **版本**: v3.4.0-rc-gate
> **创建日期**: 2026-05-23
> **维护人**: hermes-agent
> **阶段**: RC (Release Candidate)
|> **分支**: `origin/rc/v3.4.0` (commit `9dd799f2` from `origin/develop/v3.4.0` head `bd39e787`)
|> **起点**: `b72c640a` (v3.4.0 Beta release point)

---

## 一、RC Gate 入口条件

- [x] Alpha Gate 通过 (Alpha 16/16 PASS)
- [x] Beta Gate 通过 (Beta 14/14 PASS)
- [x] 创建 `rc/v3.4.0` 分支
- [x] 合并 Beta 代码到 RC 分支

---

## 二、RC Gate 检查

### 2.1 代码质量检查 (R1-R4)

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| R1 | Build | `cargo build --release --workspace` | 成功 | ⏳ |
| R2 | Test | `cargo test --all-features -- --test-threads=4` | 全部通过 | ⏳ |
| R3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 | ⏳ |
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
| R8 | TPC-H SF=1 | Z6G4 执行 | 22/22 | ⏳ Z6G4 |

### 2.4 Beta 门禁复核 (B1-B14)

| # | 检查项 | 命令 | 标准 | Beta 状态 | RC 状态 |
|---|--------|------|------|-----------|---------|
| B1 | Build | `cargo build --release --workspace` | 成功 | ✅ PASS | ⏳ |
| B2 | Unit tests | `cargo test --lib` | 全部通过 | ✅ PASS | ⏳ |
| B3 | Clippy | `cargo clippy --all-features` | 零警告 | ✅ PASS | ⏳ |
| B4 | Format | `cargo fmt --all -- --check` | 通过 | ✅ PASS | ⏳ |
| B5 | GMP build | `cargo build -p sqlrustgo-gmp` | 成功 | ✅ PASS | ⏳ |
| B6 | GMP lib test | `cargo test -p sqlrustgo-gmp --lib` | 通过 | ✅ PASS | ⏳ |
| B7 | Batch API | `cargo test -p sqlrustgo-gmp -- batch` | 通过 | ✅ PASS | ⏳ |
| B8 | Audit API | `cargo test -p sqlrustgo-gmp -- audit` | 通过 | ✅ PASS | ⏳ |
| B9 | Retrieval build | `cargo build -p sqlrustgo-gmp-retrieval` | 成功 | ✅ PASS | ⏳ |
| B10 | BM25 search | `cargo test -p sqlrustgo-gmp-retrieval --lib -- bm25` | 通过 | ✅ PASS | ⏳ |
| B11 | RRF fusion | `cargo test -p sqlrustgo-gmp-retrieval --lib -- rrf` | 通过 | ✅ PASS | ⏳ |
| B12 | Evidence engine | `cargo test -p sqlrustgo-evidence-engine --lib` | 通过 | ✅ PASS | ⏳ |
| B13 | Workflow v2 | `cargo test -p sqlrustgo-workflow-v2 --lib` | 通过 | ✅ PASS | ⏳ |
| B14 | Trust viz | `cargo test -p sqlrustgo-trust-viz --lib` | 通过 | ✅ PASS | ⏳ |

### 2.5 RC 稳定性测试 (R-S1~S4) — Z6G4

| # | 检查项 | 说明 | 状态 |
|---|--------|------|-------|
| R-S1 | 16h Write | Z6G4 执行 | ⏳ Z6G4 |
| R-S2 | 16h Read/Write | Z6G4 执行 | ⏳ Z6G4 |
| R-S3 | 24h Write | Z6G4 执行 | ⏳ Z6G4 |
| R-S4 | 24h Read/Write | Z6G4 执行 | ⏳ Z6G4 |

### 2.6 Alpha 门禁复核 (A6, A7)

| # | 检查项 | 命令 | Alpha 状态 | RC 状态 |
|---|--------|------|-----------|---------|
| A6 | MySQL handshake | `cargo test -p sqlrustgo-mysql-server --test mysql_protocol_handshake_test` | ✅ PASS | ⏳ |
| A7 | TPC-H SF=1 | `sqlrustgo-bench-cli tpch --scale 1` | ✅ PASS | ✅ 已确认 |

---

## 三、结果汇总

| 类别 | PASS | FAIL | SKIP |
|------|------|------|------|
| 代码质量 (R1-R4) | 0 | 0 | 0 |
| 覆盖率 (R5) | 0 | 0 | 0 |
| 安全兼容 (R6-R8) | 0 | 0 | 0 |
| Beta 复核 (B1-B14) | 0 | 0 | 0 |
| 稳定性 (R-S1~S4) | 0 | 0 | 0 |
| Alpha 复核 (A6, A7) | 0 | 0 | 0 |
| **总计** | **0** | **0** | **0** |

**RC Gate 结果**: ⏳ 待执行
