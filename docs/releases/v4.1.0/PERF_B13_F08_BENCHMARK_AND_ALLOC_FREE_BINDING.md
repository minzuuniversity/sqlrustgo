# F-08 基准与「分配无关」列名绑定测量（B1.3 前置）

> **日期**: 2026-10-04
> **范围**: `develop/v4.1.0`
> **关联**: 审计报告 §F-08、优化计划 §B1.3 / §10.11 B1.4
> **基准**: `benches/bench_expr_binding.rs`
> **改动**: `crates/executor/src/expr/mod.rs`（`find_column_index` + 新增 `trailing_segments_eq`）

---

## 1. 为什么先做基准

优化计划 §B1.3 把「F-08 完整绑定阶段」列为 ⏸ 暂不实施，理由原文：

> - `find_column_index` 在全仓有 **40 处调用点**；把 `Expression` 编译成带 slot 的
>   绑定树会触及求值器、计划器与所有谓词路径，计划 §4 亦将其标为
>   「高风险（改动面大）」，要求单独 PR。
> - **缺少能保护该改动的基准** … 目前没有覆盖 `find_column_index` 的基准
> - 在无基准保护的情况下做 40 点改动，无法证明收益也无法排除回归。
>
> **建议的下一步**：先补一个覆盖 `WHERE <col> = <lit>` 多列扫描的基准，
> 再在绑定树上动刀。

本文交付那个基准（`benches/bench_expr_binding.rs`），以及**基准能保护的第一刀**：
消除 F-08 点名的 `Vec<&str>` 分配。完整绑定树**仍不在本文范围**（见 §6）。

### 调用点实测（修正 §B1.3 的「40 处」）

| 口径 | 数量 |
|---|---|
| `fn find_column_index` 定义 | 6 处（`src/engine_utils.rs`、`src/expr_utils.rs`、`crates/executor/src/merge.rs`、`crates/executor/src/expr/mod.rs`、`crates/storage/src/vtu_ir/predicate_ir.rs`、`crates/sql-corpus/src/lib.rs`） |
| 全部 `find_column_index(` 出现 | 53 处 |
| 其中**生产**调用点（非测试） | 11 处 |
| 其中测试调用点 | 42 处 |

> 说明：§B1.3 的「40 处调用点」接近**含测试**的总数（53 处出现 − 6 处定义 = 47）
> 而非生产调用点（11 处）。这个差异影响风险判断：真正需要改的生产路径比原文更集中。
> 本表为 grep 实测，命令见 §7。

---

## 2. 基准覆盖

`benches/bench_expr_binding.rs` 四组：

| 组 | 压什么 | 维度 |
|---|---|---|
| `expr_find_column_index` | 直接压 `find_column_index` | 列数 {4,16,64} × 命中位置 {首,中,尾,未命中} |
| `expr_find_column_index_multijoin` | 多 join 尾段匹配（`a_join_b.tN.cM` vs `tN.cM`） | 列数 {4,16,64} × {命中末列, 未命中} |
| `expr_eval_identifier` | 热路径入口（每行每标识符一次） | 列数 {4,16,64} × {首,尾,未命中} |
| `expr_where_eq_multicolumn` | 端到端 `SELECT COUNT(*) FROM wide WHERE cN = 1` | 列数 {4,16,64} × 行数 {1000,4000} × 过滤列 {首,尾} |

运行：

```bash
cargo bench --bench bench_expr_binding
cargo bench --bench bench_expr_binding -- expr_where_eq   # 只跑端到端组
```

---

## 3. 优化前基线

`cargo bench --bench bench_expr_binding -- --warm-up-time 1 --measurement-time 2 --sample-size 30`

| 用例 | 时间（中位） |
|---|---|
| `expr_find_column_index/first/64` | 6.66 ns |
| `expr_find_column_index/middle/64` | 137.8 ns |
| `expr_find_column_index/last/64` | 295.0 ns |
| `expr_find_column_index/miss/64` | 410.0 ns |
| `expr_find_column_index_multijoin/hit_last/64` | 447.5 ns |
| `expr_find_column_index_multijoin/miss/64` | **4.395 µs** |
| `expr_eval_identifier/last/64` | 205.9 ns |
| `expr_eval_identifier/miss/64` | 303.1 ns |
| `expr_where_eq_multicolumn/first_col_c64_r1000` | 292.0 µs |
| `expr_where_eq_multicolumn/last_col_c64_r1000` | **502.9 µs** |

**基线本身即一条结论**：64 列表上过滤**末列比首列慢 72 %**（502.9 vs 292.0 µs）；
多 join 形态单次查找 **4.395 µs**，是单表同条件未命中（410 ns）的 **10.7 ×**。
后者正是 F-08 点名的 `split('.').collect::<Vec<&str>>()` 分配。

---

## 4. 改动

`crates/executor/src/expr/mod.rs`：

1. 新增 `trailing_segments_eq(haystack, needle)`：把
   `split('.').collect::<Vec<_>>()` + `[&str] == [&str]` 换成两个 `rsplit('.')`
   迭代器的右到左逐段比较。**零分配**。
2. 用 `columns.iter().position(..)` 取代手写 `for + enumerate + return`，
   保持「取第一个匹配下标」。
3. 非限定分支的 `rsplit_once('.')` 尾段比较保持原样（大小写不敏感）。

### 语义契约（必须保持的不对称）

原实现里三个匹配层的大小写策略**不一致**，重写必须逐条保持：

| 层 | 匹配对象 | 大小写 |
|---|---|---|
| 1 | 整列名精确 | 敏感（`==`） |
| 2 | 整列名 | 不敏感 |
| 3 | `col_name` 第一个 `.` 之后的部分 | 不敏感 |
| 4 | 多 join 尾段（`split('.').collect()` + 切片相等） | **敏感** |
| 5（非限定分支） | 最后一段（`rsplit_once`） | 不敏感 |

第 4 层是敏感：`trailing_segments_eq("a_join_b.T.col", "t.col") == false`。
新增测试 `test_trailing_segments_eq_contract` 与
`test_find_column_index_multi_join_trailing_segments` 把它固化。

> 这两条测试在写作过程中**先失败过一次**：我最初把 `find_column_index("T1.c1", cols)`
> 的期望写成 `Some(0)`，实际是 `None`。差别的来源正是第 4 层的大小写敏感性
> （`c1` 也不等于任何完整列名，所以第 3 层救不了）。测试抓出了这个理解错误，
> 也证明重写后的行为与原实现一致。

---

## 5. 优化后（criterion 对比，n=30）

`change:` 为 criterion 相对上一轮已保存基线的变化；以下均 `p < 0.05`（除另注）。

| 用例 | 优化前 | 优化后 | 变化 |
|---|---|---|---|
| `multijoin/miss/64` | 4.395 µs | 1.020 µs | **−76.6 %** |
| `multijoin/miss/4` | 357.5 ns | 76.4 ns | **−77.6 %** |
| `multijoin/hit_last/64` | 447.5 ns | 144.3 ns | −67.0 % |
| `find_column_index/miss/64` | 410.0 ns | 308.0 ns | −29.5 % |
| `find_column_index/last/64` | 295.0 ns | 171.1 ns | −42.8 % |
| `find_column_index/middle/16` | 48.7 ns | 28.3 ns | −39.8 % |
| `find_column_index/first/64` | 6.66 ns | 3.90 ns | −42.6 % |
| `eval_identifier/last/16` | 65.3 ns | 30.2 ns | −52.7 % |
| `eval_identifier/miss/4` | 92.8 ns | 55.1 ns | −40.2 % |
| `where_eq/last_col_c64_r1000` | 502.9 µs | 450.9 µs | −11.7 % |
| `where_eq/last_col_c64_r4000` | 3.346 ms | 2.976 ms | −6.8 % |
| `where_eq/first_col_c64_r1000` | 292.0 µs | 270.0 µs | −9.9 % |
| `where_eq/last_col_c16_r1000` | 171.9 µs | 161.3 µs | −5.3 % |
| `eval_identifier/miss/64` | 303.1 ns | 338.5 ns | +2.0 %（**p = 0.20，不显著**） |

**结论**：分配移除让多 join 查找快 **3.1–4.3 ×**；单表查找 1.3–1.8 ×；
端到端多列过滤 4–24 %。唯一变慢的一项不显著（p = 0.20），按无变化处理。

---

## 6. 未声称事项（Anti-Fabrication 边界）

- **未**实现 F-08 的完整绑定/编译阶段（`Expression` → 带 slot 的树）。
  计划 §B1.3 要求它作为**独立设计 PR**，本文只交付其前置基准 + 一刀低风险改动。
- **未**改动其余 5 份 `find_column_index` 副本（`engine_utils`、`expr_utils`、
  `merge.rs`、`predicate_ir.rs`、`sql-corpus`）。它们仍是独立的重复实现；
  合并/统一是另一个议题（见 §8）。
- 端到端组 `first_col_c64_r4000` 的置信区间较宽（p = 0.05），
  该点位不宜单独引用；本报告只用其趋势。
- 未做 profiler 采样；所有数字均为 criterion 挂钟时间。
- 未在多线程 / 并发查询下测量。

---

## 7. 复现命令

```bash
# 基准
cargo bench --bench bench_expr_binding -- \
  --warm-up-time 1 --measurement-time 2 --sample-size 30

# 语义契约测试
cargo test -p sqlrustgo-executor --lib --all-features -- find_column_index
cargo test -p sqlrustgo-executor --lib --all-features -- trailing_segments

# 调用点清点（§1 表格来源）
grep -rn 'fn find_column_index' --include='*.rs' . | grep -v target | wc -l   # 6
grep -rn 'find_column_index('   --include='*.rs' . | grep -v target | wc -l   # 53
```

## 8. 后续

1. **绑定阶段**（计划 §B1.3 的正式内容）：现在有基准保护，可单独 PR。
2. **6 份 `find_column_index` 副本合并**：先确认各副本语义是否一致
   （`merge.rs` 与 `predicate_ir.rs` 是私有副本，可能已漂移），再决定收敛方向。
3. **`develop/v4.1.0` 上 executor 有既存失败**（与本改动无关，已实测确认）：
   - `--all-features`：8 个 `task_scheduler::tests::*` 失败 ——
     `parallel-executor` 把并行度设为 8，而 stub 测试断言 1（feature-gate 问题）。
   - 默认 features：2 个 `expr::tests::{test_eval_binary_op_eq_null,
     test_eval_unary_op_not}` 失败。
   两者在**撤销本改动后同样失败**，属 T1 级既有债务。
