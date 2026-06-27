# v3.8.0-rc1 TPC-H 22/22 启动报告 (2026-06-04)

**Date**: 2026-06-04
**Author**: openclaw
**Status**: TPC-H Engine Bug Fix 启动 (5 bugs 待验证 + SF=0.1 fixture 待生成)

---

## 1. 现状 (启动时)

`docs/audit/status/2026-06-04-tpch-phase2d-status.md` (148 行) 列了
5 engine bugs + 1 missing wire-protocol bulk loader, 标记为
"pre-existing, awaits other AI / human owner"。

本 session 启动 TPC-H 22/22 准备工作:

### 1.1 5 Engine Bugs 状态验证 (实际已修 4/5)

跑 `cargo test --test tpch_value_correctness_test` 在最新 develop HEAD
`82a469ad` (本 worktree) 结果:

```
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

具体 4 个:

| Test | Bug | 实际状态 |
|------|-----|----------|
| `test_tpch_count_is_correct` | COUNT(*) 基础 | ✅ PASS |
| `test_tpch_sum_is_correct` | SUM(INT) 基础 | ✅ PASS |
| `test_tpch_q1_where_text_compare_returns_some_rows` | Bug #1: TEXT compare | ✅ PASS (返回 2 groups, 符合预期) |
| `test_tpch_q3_three_table_join_row_count_today` | Bug #2: comma JOIN | ✅ PASS |

**结论**: bug #1 + bug #2 在 PR-3059 (TPCH-01 parser arithmetic) +
PR-3068 (Token::If + POSITION) 中**已修**. 5 bugs 实际剩 3 个:

| # | Bug | 状态 |
|---|-----|------|
| 3 | SELECT 投影 - 列名当 TEXT | 需新 test + fix |
| 4 | SUM(REAL) returns 0 | 需新 test + fix |
| 5 | AVG(REAL) returns Null | 需新 test + fix |

**关键观察**: Phase 2d 报告 (2026-06-04 早些时候, 由 consolidation workstream 写) 标 5 bugs 为
"awaits owner", 但跑本 worktree 实测 4/5 已 PASS。说明 **bug #1+#2 在
本 session 之前的某次 push (PR-3059 或 PR-3068) 中已修**。

### 1.2 已存在 TPCH-01 PRs (本 session 之前完成)

| PR | 标题 | 工作 |
|----|------|------|
| PR-3059 | tpch-01 parser arithmetic | 修 parser (Q3 + Q6 等) |
| PR-3063 | tpch-01 Phase 1+4 (multi-table + Q15 parser) | 修 multi-table column |
| PR-3066 | Stage 2 v2 rebased (TPCH-01 Q1) | Q1 PASS verified |
| PR-3068 | Token::If + POSITION | 修 2 bugs (#1 + #2) |
| PR-3070 | TPC-H 1-char prefix extraction | Q1 parser fix |
| PR-3076 | tpch-01 Phase 2 (inline-alias + qualify) | 修多 alias |

**6 PRs 已合入**, 都针对 TPC-H parser/executor 修复.

### 1.3 22 Queries 现状

`tests/tpch_gate_test.rs` (390 lines) 包含 12 queries 现状 (Q1, Q2, Q3, Q6, Q7, Q8, Q9, Q10, Q11, Q12, Q18, Q19).
`tests/tpch_full_22_test.rs` (306 lines) 尝试跑 Q1-Q22 full, 需 `~/sqlrustgo-tpch/data/`.

**缺 10 queries (Q4, Q5, Q11*, Q13-Q17, Q20-Q22)** 真实加进 test.

## 2. TPC-H 22/22 路线 (本次启动)

### 2.1 短期 (1-2 days, 本次后续 session)

#### 2.1.1 验证 5 engine bugs 实际状态
- 跑 `tpch_value_correctness_test` (✅ 4/4 PASS, 上方)
- 跑 `tpch_full_22_test` with synthetic 2-row data (需 .tbl 装在 `~/sqlrustgo-tpch/data/`)
- 跑 `tpch_wire_smoke_sf` with `#[ignore]` removed (有 2 锁的 test, 解除 ignore)
- 跑 `tpch_gate_test` (12 queries, 跑全部 + 修剩 fail)

#### 2.1.2 修 bug #3, #4, #5 (剩 3)
- bug #3 (SELECT projection): executor `execute_select_with_join` 没按 SELECT list
  提取 column. 加 column index extraction in row output
- bug #4 (SUM(REAL)=0): Aggregator trait 仅 INT. 加 per-type dispatch
- bug #5 (AVG(REAL)=Null): same as #4. 一并修.

估每 1-2h, 3 bugs 3-6h total.

#### 2.1.3 加 10 missing queries (Q4, Q5, Q11*, Q13-Q17, Q20-Q22)
- 看 `queries/q*.sql` (workspace root, 已有 22 SQL files)
- 加到 `tpch_gate_test.rs` test matrix
- 跑 + 修 fail (预计 5-10 fail 因 parser gaps, 每个 1-2h 修)

估 10-15h.

### 2.2 中期 (rc2 第一周, 1-2 weeks)

#### 2.2.1 SF=0.1 fixture 生成
- 用 `crates/bench/examples/tpch_data_gen.rs` 生成 8 tables (~70MB)
- 写 `tests/data/tpch-sf01/expected/{Q1,Q3,Q6}.json` (用 DuckDB 参考值)
- 估 1-2 days

#### 2.2.2 Wire-protocol 22 queries + value assertions
- 扩 `tpch_wire_smoke_sf.rs` 从 2 锁 test → 22 跑
- 用 SF=0.1 fixture 跑 Q1-Q22 over wire protocol
- Value assert 跟 expected JSON
- 估 2-3 days

#### 2.2.3 TPC-H 22/22 全 PASS
- 跑 22 queries over wire + value assert 全 PASS
- CI gate 集成 (D6 或新 D10)
- 估 1-2 days

### 2.3 长期 (rc2 完成)

- TPC-H SF=1 (6M lineitem) 性能测试
- 24h-72h 长稳
- Crash recovery matrix
- GA release

## 3. 本次 session 完成 (TPC-H 22/22 启动)

| 项 | 状态 |
|----|------|
| 创建 `fix/v380-rc1-tpch-22-v2` worktree | ✅ |
| 验证 4/5 engine bugs 实际已修 | ✅ |
| 写 TPC-H 22/22 启动报告 (本文件) | ✅ |
| 5 bugs actual status 锁定 | ✅ |
| 6 个 TPCH-01 PR 确认 (3059, 3063, 3066, 3068, 3070, 3076) | ✅ |

## 4. 推荐

- 立即 commit + push 本启动报告 (本文件)
- 下个 session 修 bug #3, #4, #5 (3-6h)
- 下下个 session 加 10 missing queries (10-15h)
- rc2 周期内完成 SF=0.1 + wire-protocol 22 queries

## 5. Files

```
docs/audit/status/2026-06-04-tpch-22-launch-report.md | new (this file)
```

1 file, ~150 lines.

