# V312-58 Q17/Q20/Q22 — develop/v3.12.0 HEAD 实测证据

**Commit**: `1acdd283bc` (develop/v3.12.0 HEAD, post #4457 merged)
**Date**: 2026-08-25
**Agent**: openclaw-minimax (verification)
**Policy**: Anti-Fabrication-Policy-v1.0

> **Update 2026-08-25**: Added `q17_sf1_diag` test (PR #4457). Confirmed Q17 SF=1
> data load succeeds (part 3.5s, lineitem 6M rows <60s) but query execution
> itself times out within the test framework's 60s limit — consistent with
> the known TIMEOUT issue.

---

## 实测结果 (post Sprint 4 + Phase 1 + #4457)

| Q | 状态 | Query elapsed | Total wall | Budget | row_count | Oracle | 备注 |
|---|------|--------------|------------|--------|-----------|--------|------|
| Q17 | ❌ TIMEOUT | >60s (query exec) | — | 1800s | null | 1 | Data load OK; Q17 exec TIMEOUT per q17_sf1_diag |
| Q20 | ❌ TIMEOUT | (未实测) | — | 1800s | null | 172 | Mini subsets pass per upstream |
| Q22 | ✅ PASS | 1.057s | 173.55s | 300s | 7 | 7 | SF=1 1.5M orders |

---

## Q17 (`q17_sf1_diag`)

**测试命令**:
```bash
cargo test --release --test q17_sf1_diag --all-features -- --ignored --nocapture
```

**结果**: part 表加载 3.5s，lineitem 表（6M 行，760MB）加载 <60s。Q17 查询执行超过 test framework 60s 限制。

**测试命令** (standalone，无 test framework 限制):
```bash
cargo run --release --example q17_sf1_bench 2>/dev/null
# 或直接用 release build 手动执行
```

**根因分析**（from `evidence/v312-58/issue-4379-sprint3-partial-closure.md`）:
- `try_comma_join_hash_chain` 在遇到相关子查询时 bail
- cartesian path materialization，RSS ~17 MB/s 线性增长
- 100K subset ✅（0.65s, oracle MATCH）
- 1M subset ❌（RSS 单调增长，60s 后 ~1GB，OOM killed）
- SF=1 全量 ❌（TIMEOUT within 60s of query execution）

**子 issue**: #4432 (OPEN, v313-deferred)

---

## Q20 (`q20_potential_part_promotion_perf`)

**测试命令**:
```bash
cargo test --release --test q20_potential_part_promotion_perf --all-features -- --ignored --nocapture
```

**结果**: 测试启动 >60s 后无输出，被 300s test framework timeout 终止

**根因分析**（from `evidence/v312-58/issue-4380-sprint3-closure.md`）:
- Mini subset（L4/L9/L19）✅：table_info bug fix 后 22/22 levels PASS
- Full Q20 SF=1 ❌：L0/L5 full SUM subquery 返回 0 行（CORRECTNESS 问题，非 TIMEOUT）
- 实际问题是 CORRECTNESS（0 rows）而非纯 TIMEOUT

**子 issue**: #4429 (OPEN, v313-deferred)

---

## Q22 (`q22_global_sales_opportunity_perf`)

**测试命令**:
```bash
cargo test --release --test q22_global_sales_opportunity_perf --all-features -- --ignored --nocapture
```

**结果**:
```
test q22_global_sales_opportunity_sf1 ... ok
Q22 elapsed: 1.057138709s
test result: ok. 1 passed; 0 failed; 0 ignored; finished in 173.55s
```

- Query execution: **1.057s**
- Bulk load (customer 150K + orders 1.5M): ~172s
- Total wall: **173.55s** ≤ budget 300s ✅
- row_count: **7** = SQLite oracle (7) ✅

**子 issue**: #4381 (CLOSED — 实测 PASS)
