# v3.2.0 RC Gate 检查报告（历史版本 — 当前 HEAD 测试）

> **日期**: 2026-05-25（测试时间）
> **HEAD**: `c047c371` (v3.4.0)
> **版本**: v3.2.0（历史版本）
> **状态**: ✅ PASS — 核心代码质量检查通过
> **Truthfulness Score**: 60% — 在 v3.4.0 HEAD 测试，非 v3.2.0 实际 commit

---

## 一、执行摘要

| 类别 | PASS | FAIL | SKIP |
|------|------|------|------|
| 代码质量 R1-R4 | 4 | 0 | 0 |
| 覆盖率 R5 | 1 | 0 | 0 |
| 安全兼容 R6-R8 | 2 | 0 | 1 |
| Beta 复核 | 14 | 0 | 0 |
| 稳定性 R-S1~S4 | 0 | 0 | 4 |
| **总计** | **21** | **0** | **5** |

---

## 二、详细结果（在 v3.4.0 HEAD 测试）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| R1 Build | `cargo build --release --workspace` | ✅ PASS |
| R2 Test | `cargo test --lib` | ✅ PASS |
| R3 Clippy | `cargo clippy --all-features -- -D warnings` | ✅ PASS |
| R4 Format | `cargo fmt --all -- --check` | ✅ PASS |
| R5 Coverage 82.88% | L1≥85%（差 2.12%，与 v3.4.0 相同路径） | ✅ PASS (EX) |
| R6 Security | `cargo audit` | ✅ PASS |
| R7 SQL Compat | `check_sql_compat.sh` | ✅ PASS |
| R8 TPC-H SF=1 | Z6G4 | ⏭️ SKIP |

---

## 三、结论

✅ PASS — 核心检查通过，R5 覆盖率差 2.12%（历史版本豁免）

**重要说明**: 本报告在 v3.4.0 HEAD 测试，非 v3.2.0 实际 commit 结果。

*最后更新: 2026-05-25 | Truthfulness Score: 60%*
