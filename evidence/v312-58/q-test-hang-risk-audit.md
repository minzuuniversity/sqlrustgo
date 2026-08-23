# v312-58 / Q 测试挂起风险审计 — PR #4408 修复后扫描

**Branch**: `fix/v312-58-tpch-sf1-7x`
**Date**: 2026-08-23
**Author**: openclaw
**Verdict**: **无新增高风险挂起点**。PR #4408 修复的是唯一的真正挂起向量。

---

## 1. 审计范围

PR #4408 合并后 (`src/engine_select.rs` 中 5 处 `[Q7_TRACE]` 漏门控)，
系统性扫描所有 TPC-H 相关代码位置，确认是否还存在类似"无条件
eprintln 在热路径"的死锁源。

扫描对象：
1. `src/` 与 `crates/` 下全部 Rust 文件中的 `eprintln!` / `println!` / `dbg!`
2. `tests/integration/oracle/` 下全部 `q*_*.rs` 与 `diag_q*.rs` 文件
3. `crates/executor/src/expr/mod.rs` 中 EXTRACT 函数路径

挂起复现条件（必须同时满足）：
1. `eprintln!` 在热路径（per-row / per-column / `.map()` / `.inspect()` / `.for_each()`）
2. **无条件**执行（无 `if env::var().is_ok()` / `if _xxx_trace` 门控）
3. 喷出量 > PTY 缓冲（Linux PTY 通常 4-64 KB）
4. 测试用 `--nocapture`（stderr 直连 PTY）

任何一条不满足 → 不挂起。

---

## 2. 引擎代码（src/, crates/）扫描结果

### 2.1 `src/engine_select.rs`

| 行号 | 内容 | 门控 | 状态 |
|------|------|------|------|
| 634, 636 | `[Q7_TRACE] group_exprs` | `if _q7_trace` | ✅ PR #4408 已修 |
| 640, 642, 644, 646, 648 | `[Q7_TRACE] table_info.columns/rows` | `if _q7_trace` | ✅ PR #4408 已修 |
| 659 | `[Q7_TRACE] distinct key` | `if _q7_trace && _q7_keys_seen.insert(...)` | ✅ 原本正确 |
| 664, 665 | `[Q7_TRACE] distinct groups / total rows` | `if _q7_trace` | ✅ PR #4408 已修 |
| 953, 955, 957, 958, 960 | `[Q7_TRACE] select.columns / agg_result_rows` | `if _q7_trace` | ✅ PR #4408 已修 |
| 1145 | `[Q7_TRACE] UNMATCHED col key` | `if _q7_trace`（嵌套 `.map()` 内） | ✅ PR #4408 已修 |
| 1153 | `[Q7_TRACE] FINAL row` | `if _q7_trace`（`.map()` 内） | ✅ PR #4408 已修 |
| 5712 | `Q21 chain from start_idx` | `#[cfg(test)]` + `chain.len() == join_tables.len()` 守卫；最多 4 表 × 1 次 | ✅ 冷路径，bounded |

**门控变量定义**：`let _q7_trace = std::env::var("Q7_TRACE").is_ok();` 在每个 GROUP BY
分支顶部一次性求值，所有 eprintln 都受其控制。

### 2.2 `crates/executor/src/expr/mod.rs`

| 行号 | 内容 | 门控 | 状态 |
|------|------|------|------|
| 1452 | `[Q7_TRACE] EXTRACT field=... source=...` | `if _q7_trace` | ✅ 原本正确 |

EXTRACT 函数每行仅 1 次 eprintln（不进 `.map()`），且门控正确。

### 2.3 其他 crates/

全部为错误日志 / CLI 用法 / 审计失败告警，均不在热路径：

| Crate | 路径 | 触发条件 |
|-------|------|----------|
| `gmp` | `sql_api.rs`, `backup.rs` | 审计日志写失败（罕见）|
| `sqlrustgo-cli` | `lib.rs`, `sqlite_mode.rs` | CLI 错误信息（一次性）|
| `security/audit` | `audit.rs` | 审计写失败（罕见）|
| `soak-client` | `main.rs` | 连接/预热失败（罕见）|
| `transaction` | `savepoint.rs` | Savepoint undo 失败（罕见）|
| `sqlancer` | `bin/sqlancer.rs` | 错误/启动信息（一次性）|
| `admin` | `main.rs` | KILL/processlist 错误（罕见）|
| `mysql-server` | `main.rs` | REPL/查询错误（罕见）|

**结论**：全部冷路径，无 hang 风险。

---

## 3. Q 测试文件扫描结果

### 3.1 真 TPC-H Q 查询回归测试

| 文件 | 喷出模式 | 状态 |
|------|----------|------|
| `q2_5way_comma_limit_regression.rs` | `#[ignore]` + `.take(5)` 限 5 行 | ✅ 双重保护 |
| `q8_8way_date_range_regression.rs` | 无 `eprintln` / 无 per-row `iter` | ✅ |
| `q16_notin_subquery_regression.rs` | 无 `eprintln` / 无 per-row `iter` | ✅ |

### 3.2 diag_q7_*（5 个文件）

| 文件 | 喷出模式 | 状态 |
|------|----------|------|
| `diag_q7_mini.rs` | 1 行 fixture，断言通过 | ✅ |
| `diag_q7_mini_columns.rs` | 1 行 × 4 col ≈ 5 行输出 | ✅ 远小于 PTY 缓冲 |
| `diag_q7_sf1_real_count.rs` | 仅 `rows.len()` 单值 | ✅ |
| `diag_q7_subset_columns.rs` | DE/FR subset ≈ 7 行 × 4 col ≈ 36 行 | ✅ 远小于 PTY 缓冲 |
| `diag_q7_extract.rs` | `#[ignore]` × 2 tests；`take(10)` | ✅ |

### 3.3 其他 diag_q*.rs（按 Q 编号）

| 文件 | 喷出模式 | 状态 |
|------|----------|------|
| `diag_q6_filter.rs` | COUNT/SUM 标量（5 个 Step）| ✅ |
| `diag_q6_where_parsed.rs` | `for r5.rows`（`LIMIT 3` 限定 3 行）| ✅ |
| `diag_q11.rs` | 3 个 step × 标量 | ✅ |
| `diag_q11_3way.rs` | 标量 + first row | ✅ |
| `diag_q11_having.rs` | 3 个 step × 标量 | ✅ |
| `diag_q11_steps.rs` | 4 个 step × 标量 | ✅ |
| `diag_q11_where.rs` | 3 个 step × 标量 | ✅ |
| `diag_q12.rs` | 4 个 step × 标量 | ✅ |
| `diag_q12_deep.rs` | 6 个 step × 标量 | ✅ |
| `diag_q14_full.rs` | first/second row（`.first()` / `.get(1)`）| ✅ |
| `diag_q14_only.rs` | 2 个 step × first row | ✅ |
| `diag_q14_q16.rs` | 5 个 step × 标量 | ✅ |
| `diag_q22_cell_level.rs` | 2 行 bounded by `assert_eq!(result.rows.len(), 2)` | ✅ |
| `diag_q22_minimal.rs` | 3 个 SUBSTR 错误测试 × 标量 | ✅ |
| `diag_q22_steps.rs` | 3 个 step × `.err()` | ✅ |
| `diag_q22_substr.rs` | 3 个 SUBSTR 错误测试 × 标量 | ✅ |
| `diag_compare.rs` | 1-2 个标量比较 | ✅ |
| `diag_simple_text_ge.rs` | 4 行小 fixture × 4 查询 | ✅ |
| `diag_shipdate_type.rs` | 标量（first 5 shipdates 是 1 个 eprintln 包含 5 行）| ✅ |

### 3.4 其他 oracle_*.rs

| 文件模式 | 喷出模式 | 状态 |
|----------|----------|------|
| `oracle_g12_sysbench.rs`, `oracle_g11_qps.rs`, `oracle_g2_int2.rs` | 仅在 `Ok(())` 或 `Err(e)` 一次性打印 | ✅ |
| `oracle_p23_hash_chain.rs`, `oracle_p34_parallel_executor.rs` | 无 `eprintln`（仅 `iter` 用作断言）| ✅ |

---

## 4. 分类汇总

| 类别 | 数量 | 风险 |
|------|------|------|
| 热路径无条件 + 大结果集（**挂起风险**）| 0 | ✅ 0 个 |
| 热路径有门控（冷启动时被门掉）| 5 处（PR #4408 修复） | ✅ 0 个 |
| 热路径有 `#[ignore]` 保护 | 3 处（`diag_q7_extract.rs` × 2、`q2_5way_comma_limit_regression.rs`） | ✅ |
| 热路径有 `.take(N)` 限行 | 2 处（`q2_5way_comma_limit_regression.rs`、`diag_q7_extract.rs`）| ✅ |
| 小 fixture + bounded 输出 | 全部其他 diag_* | ✅ |
| 冷路径 / 错误日志 / 一次性 | crates/* 中全部 | ✅ |

**总挂起风险点：0**

---

## 5. 结论

PR #4408 修复后，**未发现其他 Q 测试会导致系统挂起的代码**。
所有 TPC-H 相关测试的 `eprintln!` 用法均满足以下至少一条防御：

1. **冷路径**：仅在错误、启动、审计失败时触发（crates/cli/admin 等）
2. **小 fixture**：in-memory fixture 行数有限（diag_q7_mini, diag_q22 等）
3. **`.take(N)` 限行**：即使 fixture 大也只喷 N 行（q2、diag_q7_extract）
4. **`#[ignore]` 标记**：默认不跑，仅显式 opt-in
5. **只打标量**：`r.rows.len()`、`first()`、`take(1)` 而非 per-row

### 5.1 建议

未来添加新的诊断测试时，建议遵循：
- 默认加 `#[ignore]`，仅在排查时 opt-in
- 优先用 `r.rows.first()` / `r.rows.iter().take(10)` 而非 `for r in r.rows`
- 用 `assert_eq!(r.rows.len(), N)` 在打印前限制
- eprintln 必须用 `if std::env::var("DEBUG_XXX").is_ok() { ... }` 包裹

---

## 6. 关联

- 修复 PR：#4408（commit 2fbdf3f0c3f8）
- 证据：`evidence/v312-58/engine-select-q7trace-gate-fix.md`
- Issue：#4376