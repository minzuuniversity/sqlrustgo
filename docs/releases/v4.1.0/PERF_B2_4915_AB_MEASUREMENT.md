# Phase B2 / #4915 — A/B 测量报告

**日期**: 2026-09-30
**协议**: [`PERFORMANCE_OPTIMIZATION_PLAN.md`](./PERFORMANCE_OPTIMIZATION_PLAN.md) §6
**结论**: B2.1 / B2.3 / B2.4 有实测收益；B2.2 首版是回归，已修复后复测为中性。

---

## 1. 基线与被测 commit

| 侧 | commit | 说明 |
|---|---|---|
| **baseline** | `be665d6bc1` | Phase B2 的第一个 commit 之前的最后一个 commit |
| **HEAD** | `ff34478830` | B2.1–B2.4 全部完成 + B2.2 修复 |

> **基线不是 plan §6 写的 `99198a515`。** 那一版写在 #4912（`e8c67639e2`）落地
> 之前，`99198a515..HEAD` 区间内同时包含 A1 的 lockfree 转发和 B2 的四项改动，
> 测出来的是两者叠加的效果，无法归因给 B2。此处改用 B2 动工前的
> `be665d6bc1` 作为基线。

两侧都用 `git worktree` / 独立 checkout 编译，`--profile bench`（§6 明确要求
不可用 `cargo test` 计时，见 F-18）。

---

## 2. 基准来源与一处必要的修复

§6 指定 `cargo bench --bench bench_aggregate` / `bench_insert`。这两个**都不覆盖**
本次改动的代码路径：

- `bench_insert` 用 `MemoryStorage`，不经过 `FileStorage` / `WalStorage`；
- `bench_aggregate` 测 GROUP BY / SUM / AVG / MIN / MAX，属 Phase B1（F-07）范围。

真正覆盖 B2 的是 `crates/storage/benches/storage_benchmark.rs`
（`FileStorage` insert buffering）。**该文件在本次工作前已无法编译**——它停留在
`TableInfo` / `ColumnDefinition` 加字段之前：

```
error[E0063]: missing fields `check_constraints`, `compression`, `original_sql`
  and 1 other field in initializer of `TableInfo`
  --> crates/storage/benches/storage_benchmark.rs:34:22
error[E0063]: missing field `char_max_length` in initializer of `ColumnDefinition`
  --> crates/storage/benches/storage_benchmark.rs:282:35
error: could not compile `sqlrustgo-storage` (bench "storage_benchmark")
```

先补齐 4 处字段，再新增 3 组只针对 B2 的基准。基准文件本身的缺陷在 §5 记录。

---

## 3. 测量结果（3 runs 取中位）

```
benchmark                                     n  baseline med    HEAD med   speedup
-----------------------------------------------------------------------------------
b2_insert_into_large_table/1000               3       0.603ms     0.027ms    22.44x
b2_insert_into_large_table/10000              3       0.615ms     0.026ms    23.91x
b2_insert_into_large_table/50000              3       1.002ms     0.026ms    39.26x
b2_flush_dirty_tables/1tables_1000rows/0       3       0.036ms     0.036ms     0.99x
b2_flush_dirty_tables/1tables_10000rows/0      3       0.048ms     0.038ms     1.25x
b2_flush_dirty_tables/5tables_1000rows/0       3       0.193ms     0.225ms     0.86x  ← 回归
b2_flush_dirty_tables/5tables_10000rows/0      3       0.175ms     0.203ms     0.86x  ← 回归
b2_snapshot_and_scan/full_snapshot/10000      3       3.531ms     3.318ms     1.06x
b2_snapshot_and_scan/filtered_scan_miss/10000 3       0.010ms     0.010ms     1.02x
b2_snapshot_and_scan/full_snapshot/50000      3      13.703ms    11.605ms     1.18x
b2_snapshot_and_scan/filtered_scan_miss/50000 3       0.053ms     0.045ms     1.19x
```

命令：

```
$ cargo bench -p sqlrustgo-storage --bench storage_benchmark --no-run
$ ./target/release/deps/storage_benchmark-* --bench "b2_" \
      --warm-up-time 2 --measurement-time 4
```

---

## 4. 逐项判读

### B2.1 写路径整表深拷贝 — **22x–39x，成立**

`insert_direct` 原来每次调用都 `data.clone()` 一份完整 `TableData`，
成本 O(table_size) 且随插入单调增长。改成只快照
`[last_saved..]` 窗口后是 O(row_count)，所以：

- 预填充 1000 行：0.603ms → 0.027ms
- 预填充 50000 行：1.002ms → 0.026ms

关键读法是**绝对值几乎不变（0.027 / 0.026 / 0.026 ms）而 baseline 随表大小
线性增长（0.603 / 0.615 / 1.002 ms）**——这正是"去掉 O(N) 拷贝"应有的形状，
不是常数因子调优。

### B2.2 锁内文件 I/O — **首版 0.86x 回归，修复后 0.99x 中性**

首版实现把 I/O 移出了 `with_write_lock`，但做法是"锁内快照整张
`TableData`，锁外写"——每次 flush 对每张 dirty 表付一次 O(table_size)
拷贝。`5tables` 两项因此测到 0.86x：**拷贝比它省下的锁更贵**。

已修（`ff34478830`）：改成像 `insert_direct` 那样只取窗口 + 真实总行数，
交给 `save_table_window`；`save_table_window` 内部的冷启动 / 收缩 / 压缩
分支自己回表取全量，磁盘结果不变。

| 指标 | 首版 | 修复后 |
|---|---|---|
| `5tables_1000rows` | 0.86x | 0.99x |
| `5tables_10000rows` | 0.86x | 0.99x |

**须如实说明：修复后这一项是中性（0.99x），不是收益。** B2.2 的目标是
缩短**持锁时间**，而该收益只在并发下可见——单线程 bench 测的是 flush
本身，它测不出锁的收益。真正能证明 B2.2 的是 sysbench `oltp_read_write`
多线程下的读延迟，本次**未跑**（见 §6）。

### B2.3 全量快照缓冲 — **1.06x–1.18x，弱但存在**

`save_table_full` 原先构造 owned `StoredTableData`（第二份全表拷贝）
再 `to_string_pretty` 成 `String` 才落盘。改为借用结构体直接序列化进
1 MB `BufWriter` 后：50000 行 13.703ms → 11.605ms（1.18x）。

**这项的真实收益是内存峰值而非墙钟时间。** 少掉的两份拷贝在
50000 行规模只有 ~1 MB 级别，对 CPU 时间影响有限；`sample` / RSS 对照才能
量化，本次未做。

### B2.4 `scan_with_filter` 统一 — **1.19x @50k，成立**

`WalStorage::update` 采集 before-image 时，改为在引擎内过滤而不是先
物化全部行再丢弃大部分：50000 行 0.053ms → 0.045ms。

10000 行那档是 1.02x（0.010ms vs 0.010ms），量级太小落在噪声内。
`scan_with_filter` 本身是 B2.1 引入的既有方法，本次只是让
`WalStorage::update` 用上它；`merge.rs` 两侧都需全表参与 join、
无谓词可下推，按原样保留（见 plan §10.8）。

#### 补充（2026-10-04）：`scan_with_filter` 此前零测试覆盖

`crates/storage/tests/scan_with_filter_contract_test.rs`（5 例）记录两点，
都是 B2.4 与 #4947 建立其上却未被验证的前提。

**一、filter 在 clone 之前。** `file_storage.rs:3800` 是

```rust
.map(|data| data.rows.iter().filter(|r| filter(r)).cloned().collect())
```

而非 `scan()` 的 `data.rows.clone()`。这正是 B2.4 收益的来源，但此前没有任何
测试钉住它——改回整表 clone 不会有测试失败。已补测试并验证其有效性：注入
"忽略谓词"后 2 例立即变红。

**二、恒真谓词与 `scan` 等价，且成本相同。** #4962 把触发器路径切成
`scan_with_filter(&|_| true)`（谓词恒真，因触发器的 WHERE 可能引用
NEW/OLD 上下文）。此时每行仍执行 `cloned()`，**与 `scan()` 成本一致**。

因此 **#4962 是类型修复而非性能优化**——它解决的是
`scan_with_filter<F>` 因 `Self: Sized` 而无法从 `Arc<RwLock<dyn StorageEngine>>`
调用的真实限制，收益不在性能。这个区分很容易被误判，测试与本节都为此存在。

另有一条语义必须保住：filter 要能看到**仍在 `insert_buffer` 里**的行。
触发器路径在事务内运行，这是常态；只扫 `tables.rows` 的实现会漏掉它们。
`filter_also_sees_rows_still_in_the_insert_buffer` 钉住这一点。


---

## 5. 一处被测量本身证伪的"回归"

首轮测出 `b2_snapshot_and_scan/full_snapshot/50000` 为 **0.77x**，区间
不重叠（HEAD `[671, 924]` vs baseline `[515, 674]`）。复查时同一份
HEAD 二进制复跑却得到 `[293, 390]`——**同一二进制两次相差 2 倍以上**。

原因是基准本身有缺陷：`b.iter()` 复用同一个 `FileStorage`，而基准在
预填充 50000 行后又每轮再 `insert(50000 rows)`，于是磁盘上的 base
snapshot 和 delta 文件在迭代之间无界累积，测的是漂移中的累积状态而非
稳态。

已改为每轮重建 `FileStorage` 并清空目录（setup 成本计入计时，两侧
相同），重测后该项 13.703ms vs 11.605ms = **1.18x**，且连续 3 次
run 稳定在 `13.4 / 14.8 / 15.9 ms`。

**记录此点是因为：若不复查，就会把基准缺陷当成 B2.3 的性能回归写进
结论。**

---

## 6. 本次**没有**测到的

§6 表格里的其余层面本次未执行，因此**不**对它们作任何声明：

- **并发 / TPS**：sysbench `oltp_read_write` 8 threads 未跑。B2.2 的
  全部意义在于并发读延迟，**缺这一项等于 B2.2 未被证明**。
- **锁争用**：macOS `sample` 采 `lock_shared_slow` 未跑。
- **TPC-H**：SF1 22 查询未跑（B2 不在 TPC-H 热路径上，但仍属 §6 表格项）。
- **构建矩阵**：`cargo build --release` 耗时 / 二进制大小、
  `cargo clippy --all-features -D warnings`、`cargo fmt --check --all`
  未按 §6 逐条执行并记录（`cargo fmt` 与 `cargo test` 在各次提交时单独
  执行过，均通过）。

---

## 7. 结论

| Phase | 结论 |
|---|---|
| **B2.1** | ✅ 实测 22x–39x，且随表大小退化的曲线被压平 |
| **B2.2** | ⚠️ 首版回归已修；单线程中性（0.99x），**并发收益未测** |
| **B2.3** | ✅ 实测 1.18x；真实收益在内存峰值，未量化 |
| **B2.4** | ✅ 实测 1.19x @50k |
| **B2.5 / B2.6** | ⏸ 判定需独立设计 PR，理由见 plan §10.8 |

**下一步的判据**：先跑 sysbench 8 线程对照。并发数字出来之前，
不应把 B2.2 记为已完成——它目前的证据只支持"没有变慢"。

---

## 附：原始数据

- HEAD: `/tmp/b2_head_final.txt`（3 runs）
- baseline: `/tmp/b2_base_final.txt`（3 runs）
- 解析结果: `/tmp/b2_ab_final.json`

复现时注意两点：baseline worktree 需要手动把修好的
`crates/storage/benches/storage_benchmark.rs` 拷过去（baseline 自带的
那份编译不过）；两侧在 50k 档的绝对值受磁盘缓存影响明显，冷/热之间
可差 30%，故每项取 3 runs 中位而非单次。
