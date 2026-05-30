# v3.7.0 RC Gate Checklist

> **版本**: v3.7.0  
> **分支**: `develop/v3.7.0` (commit `66d13cf1`)  
> **日期**: 2026-05-30  
> **Gate Type**: RC — TPC-H + Security + Performance  
> **Auditor**: Hermes Agent

---

## 0. RC Gate 入口条件

根据 governance-execution skill，Beta PASS 后进入 RC 必须满足：

| 条件 | 状态 | 来源 |
|------|------|------|
| Beta Gate PASS | ✅ | BETA_GATE_REPORT.md |
| RC_GATE_CHECKLIST.md 存在 | ✅ | 本文档 |
| SECURITY_REPORT.md 存在 | 🔲 待创建 | 入口条件 |
| PERFORMANCE_TARGETS.md 已更新 | 🔲 待验证 | 入口条件 |
| TPC-H SF=1 已验证 | 🔲 待执行 | 入口条件 |

---

## 1. RC Gate 检查项 (R1-R8)

| ID | 检查项 | 检查命令 | 通过标准 | 状态 |
|----|--------|----------|----------|------|
| R1 | Build (release) | `cargo build --release --workspace` | 0 errors | 🔲 待检 |
| R2 | Clippy zero | `cargo clippy --all-features -- -D warnings` | 0 errors | 🔲 待检 |
| R3 | Format | `cargo fmt --all -- --check` | 0 failures | 🔲 待检 |
| R4 | Cargo audit | `cargo audit` | 0 vulnerabilities | 🔲 待检 |
| R5 | TPC-H SF=1 | 22/22 queries | 22/22 PASS | 🔲 待检 |
| R6 | SQL compat | SQL Corpus ≥85% | ≥85% PASS | 🔲 待检 |
| R7 | Security | Security audit + pen test | 0 critical/high | 🔲 待检 |
| R8 | Performance | QPS benchmarks | ≥ targets | 🔲 待检 |

---

## 2. RC Gate 执行规则

**执行方式**: 必须实际运行命令，不能只检查文档存在。

**SSOT 参考**: `Governance-Gate-Phases.md`  
**RC 阈值**: 平均 ≥85%（每 crate ≥75%）

---

## 3. 日志存档

RC Gate 每次执行必须存档日志：

```
docs/releases/v3.7.0/logs/
├── rc_r1_build_{commit}_{timestamp}.log
├── rc_r2_clippy_{commit}_{timestamp}.log
├── rc_r3_fmt_{commit}_{timestamp}.log
├── rc_r4_audit_{commit}_{timestamp}.log
├── rc_r5_tpch_{commit}_{timestamp}.log
├── rc_r6_compat_{commit}_{timestamp}.log
├── rc_r7_security_{commit}_{timestamp}.log
└── rc_r8_perf_{commit}_{timestamp}.log
```

---

## 4. RC Gate 结论模板

执行完成后填入：

| ID | 检查项 | 实际结果 | 状态 |
|----|--------|----------|------|
| R1 | Build | | |
| R2 | Clippy | | |
| R3 | Format | | |
| R4 | Audit | | |
| R5 | TPC-H | | |
| R6 | SQL Compat | | |
| R7 | Security | | |
| R8 | Performance | | |

**RC Gate 判定**: 

---

## 5. RC→GA 入口条件

RC PASS 后进入 GA 前必须满足：

| 条件 | 状态 |
|------|------|
| RC Gate 全项 PASS | 🔲 |
| RC_GATE_REPORT.md 已创建 | 🔲 |
| GA_GATE_CHECKLIST.md 已创建 | 🔲 |
| USER_MANUAL.md 已创建 | 🔲 |
| API_REFERENCE.md 已创建 | 🔲 |
| RELEASE_NOTES.md 已更新 | 🔲 |
| CHANGELOG.md 已更新 | 🔲 |
| BENCHMARK.md 已创建 | 🔲 |
| TEST_REPORT.md 已创建 | 🔲 |
| SECURITY_ANALYSIS.md 已创建 | 🔲 |