# v3.6.0 Beta Gate Checklist

## 入口条件
- [✅] Alpha Gate CONDITIONAL PASS
- [✅] ALPHA_GATE_REPORT.md exists
- [✅] DEVELOPMENT_PLAN.md exists
- [✅] TEST_PLAN.md exists
- [✅] COVERAGE_ANALYSIS_REPORT.md exists

## 正式门禁

| ID | Check | Standard | Status |
|----|-------|----------|--------|
| B1 | Build (release) | cargo build --release --workspace | ❌ PENDING |
| B2 | Workspace test | cargo test --workspace ≥90% PASS | ❌ PENDING |
| B3 | Clippy zero | cargo clippy --all-features -- -D warnings | ❌ PENDING |
| B4 | Format | cargo fmt --all -- --check | ❌ PENDING |
| B5 | Coverage | L1 avg ≥85% (current: 81.97%) | ❌ PENDING |
| B6 | TPC-H SF=1 | 22/22 PASS | ❌ PENDING |
| B7 | Security | cargo audit | ❌ PENDING |
| B8 | SQL compat | SQL Corpus ≥85% | ❌ PENDING |

## Alpha 延续任务
- [❌] Parser coverage: 47.16% → ≥75%
- [❌] Executor coverage: 72.04% → ≥75%
- [❌] mysql-server tests2: 43 errors → 0

## 结果汇总
PASS: 1/8 + 0/3 延续
FAIL: 0/8 + 3/3 延续
