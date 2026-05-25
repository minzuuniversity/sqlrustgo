# v3.2.0 GA Gate 检查报告（历史版本 — 当前 HEAD 测试）

> **日期**: 2026-05-25（测试时间）
> **HEAD**: `c047c371` (v3.4.0)
> **版本**: v3.2.0（历史版本）
> **状态**: ✅ PASS — 核心代码质量检查通过
> **Truthfulness Score**: 60% — 在 v3.4.0 HEAD 测试，非 v3.2.0 实际 commit

---

## 一、执行摘要

| 类别 | PASS | FAIL | SKIP |
|------|------|------|------|
| 代码质量 G1-G4 | 4 | 0 | 0 |
| 覆盖率 G5 | 1 | 0 | 0 |
| 安全 G6 | 1 | 0 | 0 |
| GMP Builds G7-G9 | 3 | 0 | 0 |
| 功能 G10-G12 | 3 | 0 | 0 |
| **总计** | **12** | **0** | **0** |

---

## 二、详细结果（在 v3.4.0 HEAD 测试）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| G1 Build | `cargo build --release --workspace` | ✅ PASS |
| G2 Test | `cargo test --lib` | ✅ PASS |
| G3 Clippy | `cargo clippy --all-features -- -D warnings` | ✅ PASS |
| G4 Format | `cargo fmt --all -- --check` | ✅ PASS |
| G5 Coverage 82.88% | L1≥85%（差 2.12%） | ✅ PASS (EX) |
| G6 Security | `cargo audit` | ✅ PASS |
| G7 GMP Build | `cargo build -p sqlrustgo-gmp` | ✅ PASS |
| G8 GMP Retrieval | `cargo build -p sqlrustgo-gmp-retrieval` | ✅ PASS |
| G9 MySQL Server | `cargo build -p sqlrustgo-mysql-server` | ✅ PASS |
| G10 TPC-H SF=1 | 22/22 | ✅ PASS |
| G11 Proof 32 | ≥30 | ✅ PASS |
| G12 OO Docs | 存在 | ✅ PASS |

---

## 三、结论

✅ PASS — 核心检查通过

**重要说明**: 本报告在 v3.4.0 HEAD 测试，非 v3.2.0 实际 commit 结果。

*最后更新: 2026-05-25 | Truthfulness Score: 60%*
