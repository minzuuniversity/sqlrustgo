# v3.0.0 Alpha Gate 检查报告（历史版本 — 当前 HEAD 测试）

> **日期**: 2026-05-25（测试时间）
> **HEAD**: `c047c371` (v3.4.0)
> **版本**: v3.0.0（历史版本）
> **状态**: ❌ FAIL — 
> **Truthfulness Score**: 60% — 在 v3.4.0 HEAD 测试，非 v3.0.0 实际 commit

---

## 一、执行摘要

| 类别 | PASS | FAIL | SKIP |
|------|------|------|------|
| 代码质量 A1-A4 | 4 | 0 | 0 |
| 覆盖率 A5 | 0 | 1 | 0 |
| 协议/性能 A6-A7 | 0 | 0 | 2 |
| Trust Infra | 0 | 0 | 6 |
| SSOT 规范 | 0 | 0 | 3 |
| **总计** | **4** | **1** | **11** |

---

## 二、详细结果（在 v3.4.0 HEAD 测试）

| 检查项 | 命令 | 结果 |
|--------|------|------|
| A1 Build | `cargo build --workspace` | ✅ PASS |
| A2 Test | `cargo test --workspace` | ✅ PASS |
| A3 Clippy | `cargo clippy --all-features -- -D warnings` | ✅ PASS |
| A4 Format | `cargo fmt --all -- --check` | ✅ PASS |
| A5 Coverage | 脚本无输出（bug） | ❌ FAIL |

### A6-A7 ⏭️ SKIP（Z6G4）

---

## 三、结论

❌ FAIL — A5 覆盖率脚本 bug

**重要说明**: 本报告在 v3.4.0 HEAD 测试，非 v3.0.0 实际 commit 结果。

*最后更新: 2026-05-25 | Truthfulness Score: 60%*
