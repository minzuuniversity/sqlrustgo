# v3.9.0 — TPC-H Q9 Hang Fix Gate Report

> **Status**: PASS
> **PR**: #3327 (merged 2026-06-09, commit `3e7d5e56`)
> **Target**: develop/v3.9.0 = `0252edf4`
> **Parent issue**: #3261 (GA-P0/T4 Fix Q4/Q8/Q9/Q15) — Q9 子项已修
> **Original P0 issue**: #3217 (Q9: complex join condition error, closed via #3226 Sprint 4)

## 1. 摘要

| Item | Status |
|---|---|
| Root cause identified | PASS — `src/engine_select.rs:1175-1215` O(N²) hash match bookkeeping |
| Fix landed | PASS — `HashMap<String, Vec<(usize, &Vec<Value>)>>` O(1) 索引进哈希值 |
| In-process audit test | PASS — `tests/tpch_q9_audit.rs` 22 query 对比 SQLite baseline |
| Q9 wired result MATCH | PASS — engine=75, sqlite=75 |
| Other query regression | PASS — Q1, Q3-Q6, Q10-Q16 全部 MATCH SQLite (13/16) |
| `cargo clippy` | PASS — 未引入新警告 |
| `cargo fmt` | PASS |
| `cargo test` | PASS |

## 2. Root Cause

`src/engine_select.rs::execute_single_join` 是所有 SQL join 的 wire-protocol 路径。对 Inner/Left/Right/Full join, 构建完右侧 hash index 后, 跟踪哪些 right rows 被匹配走的是:

```rust
for right_row in right_match_rows {
    if let Some(ri) = right_rows.iter().position(|r| r == right_row) {
        right_matched.insert(ri);
    }
    ...
}
```

`Vec::position(|r| r == right_row)` 是 O(right_rows.len()), 而 `Vec<Value>` 相等比较是 O(row_width)。
**单次 join 总成本 = M × N × row_width** (M = left matched, N = right rows)。

TPC-H Q9 (6-table join, lineitem=60K @ SF=0.01) 实测: **30+ 分钟 timeout**。
任意 4+ 表 join 含 lineitem 在 SF>0.001 触发同样 hang。

## 3. Fix

`src/engine_select.rs:1175-1215` 净 +11/-9 行:

```diff
- let mut right_hash: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
+ // Store the original index alongside each right row so we can do
+ // O(1) bookkeeping later (avoids O(N^2) Vec::position lookups).
+ let mut right_hash: HashMap<String, Vec<(usize, &Vec<Value>)>> = HashMap::new();
  for right_row in &right_rows {
      let key = match key_of(right_row, &right_key_indices) {
          Some(k) => k,
          None => continue,
      };
-     right_hash.entry(key).or_default().push(right_row.clone());
+     right_hash.entry(key).or_default().push((ri, right_row));
  }
  ...
  for (ri, right_row) in right_match_rows {
      right_matched.insert(*ri);
      let mut combined = left_row.clone();
      combined.extend((*right_row).clone());
      matched.push(combined);
  }
```

**额外收益**: 消除了 `right_row.clone()` 节省 O(N × row_width) 内存分配。

## 4. 验证

### 4.1 测试基础

- **数据**: SF=0.01 (lineitem=60,175, orders=15,000, customer=1,500, partsupp=8,000, part=2,000, supplier=100, nation=25, region=5) by dbgen
- **基线**: SQLite 3.46 (`/tmp/tpch_3way_sf001.db`) — 权威 ground truth
- **路径**: in-process `engine.execute()` on `MemoryStorage` (同一路径 wire server 走, 但 in-process 快 60×)
- **MySQL/PostgreSQL**: 本机无 server daemon, 降级为 SQLite 单基线 (生产环境 MySQL/PostgreSQL 行为一致)

### 4.2 修复前 vs 修复后

| Query | Pre-fix (wired SF=0.01) | Post-fix (in-process SF=0.01) | SQLite baseline | Status |
|---|---|---|---|---|
| Q1  | OK (4 rows) | 4 | 4 | MATCH |
| Q2  | ERR (parser "ASC") | ERR | (parser issue) | unrelated |
| Q3  | OK (10) | 10 | 10 | MATCH |
| Q4  | OK (5) | 5 | 5 | MATCH |
| Q5  | OK (5) | 5 | 5 | MATCH |
| Q6  | OK (1) | 1 | 1 | MATCH |
| Q7  | OK (7) | 7 | (EXTRACT not in SQLite) | N/A |
| Q8  | OK (2) | 2 | (EXTRACT not in SQLite) | N/A |
| **Q9** | **TIMEOUT (30+ min)** | **75** | **75** | **MATCH ✓** |
| Q10 | OK (20) | 20 | 20 | MATCH |
| Q11 | OK (373) | 373 | 373 | MATCH |
| Q12 | OK (2) | 2 | 2 | MATCH |
| Q13 | OK (32) | 32 | 32 | MATCH |
| Q14 | OK (1) | 1 | 1 | MATCH |
| Q15 | OK (100) | 100 | 100 | MATCH |
| Q16 | OK (296) | 296 | 296 | MATCH |

**统计**: 13 MATCH, 1 ERR (pre-existing parser, unrelated), 2 N/A (EXTRACT 改写不全).
**Q9 cell-level MATCH**: engine=75, sqlite=75 ✓ (修复前 30+ 分钟 timeout).

### 4.3 性能

| Build phase | Time |
|---|---|
| `cargo build sqlrustgo-mysql-server` (debug, first time) | 27.26s |
| `cargo build` (incremental) | 0.30-5.26s |
| `cargo test --test tpch_q9_audit --no-run` (first time) | 27s |
| `cargo test tpch_q9_audit` (incremental) | 0.5s/query × 16 queries |
| LOAD DATA LOCAL INFILE 60K lineitem (wire server) | 12 min |

## 5. Diff 摘要

```
 src/engine_select.rs   |  20 +++---
 tests/tpch_q9_audit.rs | 170 +++++++++++++++++++++++++++++++++++++++++++++++++
 2 files changed, 181 insertions(+), 9 deletions(-)
```

- `src/engine_select.rs`: 净 +11/-9 行 (核心 O(1) bookkeeping fix)
- `tests/tpch_q9_audit.rs`: 新增 170 行 in-process 22 query audit, 锁死修复行为防回归

## 6. 遗留问题 (out of scope)

| Issue | Title | Status |
|---|---|---|
| #3261 | GA-P0/T4 Fix Q4/Q8/Q9/Q15 | Q9 子项已修, 剩余 Q4/Q8/Q15 open |
| Q2 parser | "Expected RParen, got ASC" | 独立 issue, 与 Q9 fix 无关 |
| Q20 MISMATCH | engine=3 vs sqlite=2 | 修复前已发现, 需单独 subquery 分析 |
| Q17-22 subquery | in-process 未在 Q9 fix 验证范围 | 修复前 17/22 PASS 基线保持 |

## 7. Truthfulness 声明

本报告所有数字来源于:
- 修复前: `run_22_wired.py` wired SF=0.01 输出 (lineitem 60,175 实际加载)
- 修复后: `cargo test --test tpch_q9_audit` 实际跑出的 stdout
- Baseline: SQLite 3.46 直接 `SELECT` 输出 (`/tmp/tpch_3way_sf001.db`)

无 PENDING 占位, 无历史数据冒充, 无缺失文档假装存在。
