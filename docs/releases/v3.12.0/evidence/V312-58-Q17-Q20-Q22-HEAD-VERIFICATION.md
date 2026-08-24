# V312-58 Q17/Q20/Q22 — develop/v3.12.0 HEAD 实测证据

**Commit**: `3172bb0f62` (develop/v3.12.0 HEAD, post #4436 merged)
**Date**: 2026-08-25
**Agent**: claude-code (minimax-m2.7)
**Policy**: Anti-Fabrication-Policy-v1.0

---

## 实测结果

| Q | 状态 | Query elapsed | Total wall | Budget | row_count | Oracle | 备注 |
|---|------|--------------|------------|--------|-----------|--------|------|
| Q17 | ❌ TIMEOUT | >1950s | — | 1800s | null | 1 | SF=1 6M lineitem |
| Q20 | ❌ TIMEOUT | >300s | — | 1800s | null | 172 | SF=1 |
| Q22 | ✅ PASS | 1.057s | 173.55s | 300s | 7 | 7 | SF=1 1.5M orders |

---

## Q17 (`q17_small_order_shortage_perf`)

**测试命令**:
```bash
cargo test --release --test q17_small_order_shortage_perf --all-features -- --ignored --nocapture q17_small_order_shortage_sf1
```

**结果**: 测试运行 >1950s 后被外部 timeout 终止，未见 PASS 输出

**根因分析**（from `evidence/v312-58/issue-4379-sprint3-partial-closure.md`）:
- `try_comma_join_hash_chain` 在遇到相关子查询时 bail
- cartesian path materialization，RSS ~17 MB/s 线性增长
- 100K subset ✅（0.65s, oracle MATCH）
- 1M subset ❌（RSS 单调增长，60s 后 ~1GB，OOM killed）
- SF=1 全量 ❌（TIMEOUT >1950s）

**子 issue**: #4379 (OPEN, v313-deferred)

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

**子 issue**: #4380 (OPEN, v313-deferred)

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
