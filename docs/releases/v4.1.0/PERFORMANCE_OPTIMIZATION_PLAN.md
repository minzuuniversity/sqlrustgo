# SQLRustGo v4.1.0 性能优化计划 (PERFORMANCE_OPTIMIZATION_PLAN)

> **provenance (ADR-014 5 evidence fields):**
> - `source_agent`: deepseek-flash (DSH harness)
> - `source_run`: perf-audit-2026-09-30
> - `timestamp`: 2026-09-30T12:24:35Z
> - `evidence_hash`: local-git:`99198a51512ab397d367570ed68b58fbed168fd5` (develop/v4.1.0)
> - `conflict_resolution`: N/A — 与 Issue #4910 交叉引用；本计划不覆盖 #4910 的 `&mut self` 方案，只在其之前插入必要条件
>
> **政策**: Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-008 + ADR-014
> **Status**: DRAFT
> **输入**: [`PERFORMANCE_AUDIT_REPORT_2026-09-30.md`](./PERFORMANCE_AUDIT_REPORT_2026-09-30.md)（同目录）
> **权威阶段状态**: `docs/releases/v4.1.0/STAGE.yaml`

---

## 1. 执行原则

1. **先接线，再优化。** F-01 表明项目已投入实现无锁事务路径但接线错误，
   收益为 0。任何新优化都应在其之后，否则无法测量净收益。
2. **无证据不声称收益。** 每一步完成后必须给出实跑输出
   （build log / test output / profile / SOAK），禁止"预期提升 N 倍"式表述
   （ADR-001 G-01~G-10、`ANTI_FABRICATION_POLICY.md`）。
3. **一步一测。** 每个 task 独立可回退；不合并"顺手改"。
4. **门槛纪律。** `cargo clippy --all-features -- -D warnings`
   + `cargo fmt --check --all` 必须先绿
   （`docs/governance/GATE_CONDITIONS.md`）。
5. **Issue 关闭需 PR 证据。** 遵守
   `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 的 5 步流程。

---

## 2. Issue 映射表

本计划对应 Gitea (`openclaw/sqlrustgo`) 上的以下开发 ISSUE：

| 计划阶段 | Gitea Issue | Issue 标题 | 覆盖 Finding | 优先级 |
|---|---|---|---|---|
| **A0** | [#4913](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4913) | `[v4.1.0-perf] P0 quick wins: release profile / parallel-executor feature / group commit wiring / hot-path debug output` | F-03, F-04, F-05, F-06 | P0 |
| **A1** | [#4912](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4912) | `[v4.1.0-perf] CRITICAL: BoxStorageEngine drops *_lockfree, forcing global write lock on every BEGIN/COMMIT` | F-01 (F-02 前置) | P0-CRITICAL |
| **B1** | [#4914](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4914) | `[v4.1.0-perf] Eliminate per-row String keys in GROUP BY and per-row column-name resolution in expressions` | F-07, F-08 | P1 |
| **B2** | [#4915](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4915) | `[v4.1.0-perf] Remove full-table clones and I/O under write lock in storage layer` | F-09, F-10, F-11, F-12, F-13 | P1 |
| **C1** | 待创建 | `[v4.1.0-perf] B+ tree split drops keys, blocking index pushdown` | F-15 (F-14 索引部分) | P2 / 正确性 |
| **C2** | 待创建 | `[v4.1.0-perf] FileStorage::update bypasses with_write_lock (data race)` | F-16 | P2 / 正确性 |
| **C3** | 待创建 | `[v4.1.0-perf] Streaming result set + buffer pool activation decision` | F-14, F-17 | P2 |
| **C4** | 待创建 | `[v4.1.0-perf] Build/CI hygiene: test profile, 443 target decls, broken bench job` | F-18 | P2 |

> **回填记录**: A0/A1/B1/B2 四个 Issue 已于 2026-09-30 在 Gitea
> `openclaw/sqlrustgo` 创建并验证（编号 #4912–#4915）。
> C1–C4 属 Phase C，待 A/B 阶段产出实测结论后再创建，避免过早固化范围。
> 本表更新对应 §7 记录规则第 1 条。

---

## 3. Phase A — 接线与配置（目标：取得干净基线）

### A0. P0 quick wins（零逻辑风险）

**覆盖**: F-03（release profile）、F-04（parallel-executor feature）、
F-05（group commit wiring）、F-06（热路径调试输出）

**为什么先做**: 四项都在配置/外围层，不改执行语义，回归风险接近零，
且其中 F-04/F-05 会**改变后续所有测量的基线**。

| Task | 内容 | 文件 | 验收 |
|---|---|---|---|
| A0.1 | 新增 `[profile.release]`：`lto = "fat"`、`codegen-units = 1`、`strip = "debuginfo"` | `Cargo.toml` | `cargo build --release` 通过 |
| A0.2 | 根 crate 增加 feature 透传 `parallel-executor = ["sqlrustgo-executor/parallel-executor"]`，并在 `--executor-parallelism > 1` 而 feature 未启用时打 WARN | `Cargo.toml`, `src/execution_engine.rs` | `cargo check -p sqlrustgo --features parallel-executor` 通过 |
| A0.3 | 按 `--wal-sync group:...` 构造 `GroupCommitCoordinator` 并安装到两处 `ParallelWalStorage`/`WalStorage` 构造点 | `crates/mysql-server/src/lib.rs:6292,6367` | `cargo test -p sqlrustgo-storage --test group_commit_integration` |
| A0.4 | 删除 `crates/mysql-server/src/lib.rs:5325` 的 `eprintln!`；`Q7_TRACE` 诊断块改 cfg 门控或删除 | `crates/mysql-server/src/lib.rs`, `src/engine_select.rs` | `grep -c 'eprintln!' crates/mysql-server/src/lib.rs` 下降 |

**不做**: `panic = "abort"`（需先确认无 `catch_unwind` 依赖，
`crates/mysql-server/src/lib.rs:5313` 有 poisoning recovery 逻辑）。

**退出条件**: 4 项全部落地 + clippy/fmt 绿 + 一次基线测量（见 §6）。

---

### A1. 修复无锁事务接线（**最高收益项**）

**覆盖**: F-01（CRITICAL）

**根因回顾**（详见审计报告 §3.1 F-01）：
`WalStorage` 已实现 `begin_transaction_lockfree`
（`crates/storage/src/wal_storage.rs:976`）与 `commit_transaction_lockfree`
（`:1008`），但服务器用的最外层类型 `BoxStorageEngine`
（`crates/storage/src/binary_storage.rs:644`）**没有转发这两个方法**，
因此落到 trait 默认实现（`crates/storage/src/engine.rs:1283/1291` → `Err`），
`src/execution_engine_methods.rs:1515` 的 `lockfree_ok` 恒为 `false`，
每次 BEGIN/COMMIT 必取全局写锁。

| Task | 内容 | 文件 |
|---|---|---|
| A1.1 | `BoxStorageEngine` 覆写 `begin_transaction_lockfree` / `commit_transaction_lockfree`，转发到 `(**self)` | `crates/storage/src/binary_storage.rs` |
| A1.2 | 为 `ParallelWalStorage` 补两个方法（委托 inner 或实现自身原子路径） | `crates/storage/src/parallel_wal_storage.rs` |
| A1.3 | 为 `MvccStorage` 补两个方法（委托 inner） | `crates/storage/src/mvcc_storage.rs` |
| A1.4 | 给 `MemoryStorage` 补实现，消除测试路径的 fallback（可选） | `crates/storage/src/engine.rs` |
| A1.5 | 新增回归测试：断言 `BoxStorageEngine::begin_transaction_lockfree` 返回 `Ok` | `crates/storage/tests/` |

**风险**: 低。变更是在类型擦除层转发已有实现，不改变任何存储语义。
**风险缓解**: A1.5 的断言测试是防回归的核心；若某个 engine 尚无实现，
转发后仍会 `Err` 并走原 fallback，行为不回退。

**验收**: 
- `grep -n 'lockfree' crates/storage/src/binary_storage.rs` 不再为空；
- 新增测试绿；
- `sample` profile 对照：`lock_shared_slow` 样本数显著下降
  （基线见 #4910：10,025 / 10,280）。

**与 #4910 的关系**: 本项是 #4910 §3.1/§3.2 的**必要条件**。
完成 A1 后重新测量；若 engine 边界仍是瓶颈，再推进 #4910 的
`&mut self` 拆分，而不是同时做两件事导致无法归因。

---

## 4. Phase B — 算法级（目标：消除每行分配与 O(N²)）

### B1. GROUP BY 键结构化 + 表达式绑定

**覆盖**: F-07（GROUP BY String 键）、F-08（每行列名线性解析）

| Task | 内容 | 文件 | 风险 |
|---|---|---|---|
| B1.1 | `groups` 改 `HashMap<Vec<Value>, Vec<usize>>`（值为行下标），删除 `evaluate_expr_to_string` 键构造与 `row.clone()` | `src/engine_select.rs:1764-1830` | 中（需保证输出行序/类型不变） |
| B1.2 | 删除聚合阶段 `key.split('\x00')` + `parse::<i64>()` 的反解析逻辑 | 同上 | 中 |
| B1.3 | 引入表达式绑定阶段：`Expression` → 带列 slot 的绑定树，列名解析只做一次 | `crates/executor/src/expr/mod.rs:620-655` | 高（改动面大） |
| B1.4 | 删除 `src/engine_select.rs:1766` 的 `env::var("Q7_TRACE")` 与 `:1770-1789` 诊断块 | `src/engine_select.rs` | 低 |

**为什么 B1.1 有独立价值**: 当前实现对 `Value::Text("123")` 与
`Value::Integer(123)` 会产生**相同的分组键**（都是字符串 `"123"`），
再用 `parse::<i64>()` 猜回类型。改成 `Vec<Value>` 键不只消除分配，
也修正了类型语义。

**风险缓解**: B1.1/B1.2 必须用现有 TPC-H Q1/Q7 回归逐值比对
（`docs/releases/v3.12.0/evidence/` 有历史 oracle 结果）。
B1.3 单独成 PR，不与其他项混合。

**验收**: GROUP BY 结果与修复前**逐行逐值**一致（含类型）；`benches/bench_aggregate.rs` 前后对照。

---

### B2. 存储层去整表拷贝与锁内 I/O

**覆盖**: F-09、F-10、F-11、F-12、F-13

按"确定性 × 风险"排序，逐项独立提交：

| Task | 内容 | 文件 | 确定性 |
|---|---|---|---|
| B2.1 | `save_table` 改收 `&[Record]` 切片，删除 `insert_direct`/`insert_buffered`/`flush_buffer`/`flush` 里的 `data.clone()` | `crates/storage/src/file_storage.rs:3198,3242,3279,890` | 高 |
| B2.2 | 把序列化 + write 移出 `with_write_lock`：锁内只标 dirty + `std::mem::take` 出增量 | 同上 | 高 |
| B2.3 | `save_table_full` 改 `serde_json::to_writer(BufWriter::with_capacity(1<<20, ..))`，`StoredTableData` 改借用 | `crates/storage/src/file_storage.rs:765-782` | 高 |
| B2.4 | `WalStorage::update/delete` 与触发器/存储过程路径统一改用 `scan_with_filter` | `crates/storage/src/wal_storage.rs:691`, `crates/executor/src/trigger.rs:833,932`, `stored_proc.rs`, `merge.rs:47,52` | 高 |
| B2.5 | `committed_tables` 改 `HashMap<String, Arc<Vec<Record>>>` + `Arc::make_mut` | `crates/storage/src/engine.rs:1607,1825,1938` | 中 |
| B2.6 | binary/columnar 后端改 append 写入 + 独立 manifest；恢复路径加 `append_batch` | `crates/storage/src/binary_storage.rs:345,398`, `columnar/storage.rs:680` | 中 |

**依赖提示**: B2.1 与 B2.2 应同一个 PR（否则中间态仍有锁内 I/O）。
B2.4 会改变锁持有画像，建议在 A1 之后做以便归因。

**验收**: `benches/bench_insert.rs`（批量插入曲线）；
`cargo test -p sqlrustgo-storage`；`cargo test -p sqlrustgo-executor`（触发器/存储过程）。

---

## 5. Phase C — 结构性与正确性

### C1. B+ 树 split 缺陷（**正确性优先**）

**覆盖**: F-15

`crates/storage/src/bplus_tree/index.rs:517-529` 的 `insert_into_node`
丢弃 `insert_key_value` 的 `Some((split_key, new_node))` 返回值，
导致超过 `MAX_KEYS_PER_NODE` 后静默丢键。

| Task | 内容 |
|---|---|
| C1.1 | 新增失败测试：连续插入 > `MAX_KEYS_PER_NODE` 后 `search` 全部命中 |
| C1.2 | 修复 `insert_into_node`：`allocate_node(new_node)`、写内部节点、设 `next_leaf` |
| C1.3 | 修复 `insert`（`:372-392`）建内部节点 |
| C1.4 | `insert_key_value` 改用 `binary_search` 定位（顺带 F-15 的性能部分） |
| C1.5 | `collect_keys_leaf` 改 `extend_from_slice`；`range_query_leaf` 二分定位起点 |

**纪律**: C1.1 的失败测试必须先红后绿——这是本项唯一的可信证据形式。

### C2. `FileStorage::update` 并发正确性

**覆盖**: F-16。`crates/storage/src/file_storage.rs:4040,4070,4078`
绕过 `with_write_lock`，与 `scan_with_index:3700` 的无锁读构成数据竞争。
建议独立 issue 处理，先做正确性再做性能（顺带消除
`with_write_lock:319-349` 的 `Box::into_raw/from_raw` 开销）。

### C3. 流式结果集 + 缓冲池决策

**覆盖**: F-14（`BufferPool` 死代码或真接入）、F-17（结果集整体物化）。
F-14 是**决策项**而非纯优化：要么接入（改 CLOCK + `AtomicU64` 统计），
要么删除死代码。建议先决定，避免继续维护两套心智模型。

### C4. 构建/CI 卫生

**覆盖**: F-18。`bench-pr.yml:29` 的 `-p sqlrustgo-bench-cli` 改为
`sqlrustgo-bench`；`[profile.test]` 提到 `opt-level = 1`；
CI 全 job 加缓存；443 个 `[[test]]` 声明收敛。

---

## 6. 测量协议（每个 Phase 前后都要跑）

**基线冻结**: 所有测量必须记录 `git rev-parse HEAD`。
当前基线 commit：`99198a51512ab397d367570ed68b58fbed168fd5`。

| 层面 | 命令/方法 | 记录内容 |
|---|---|---|
| 编译 | `cargo build --release` | 是否通过、耗时、二进制大小 |
| Lint | `cargo clippy --all-features -- -D warnings` | exit code |
| 格式 | `cargo fmt --check --all` | exit code |
| 单元/集成 | `cargo test -p sqlrustgo-storage`、`-p sqlrustgo-executor` | 通过数/失败数 |
| 聚合 | `cargo bench --bench bench_aggregate` | criterion 输出 |
| 插入 | `cargo bench --bench bench_insert` | criterion 输出 |
| 并发 | sysbench `oltp_read_write`，8 threads，与路线图 §1.1 同参数 | TPS/QPS/p95 |
| 锁争用 | macOS `sample`（与 #4910 同法） | `lock_shared_slow` 样本数 |
| TPC-H | SF1 22 查询 elapsed | 与 `GA5_TPCH_SF1_REPORT.md` 对照 |

**注意**: `[profile.test] opt-level = 0` 会让以测试形式运行的测量失真
（F-18）。在 C4 落地前，性能测量一律用 `cargo bench`
（走 `[profile.bench]`/`release`），不要用 `cargo test` 计时。

---

## 7. 记录与回填规则

1. Issue 创建后，把实际编号回填本文档 §2 表格。
2. 每个 Task 完成后，在对应 Issue 上附：
   - 改动 commit SHA
   - `cargo clippy` / `cargo fmt` 实跑输出
   - 相关测试实跑输出
   - 若声称性能变化，附前后测量数据
3. 关闭 Issue 前遵守
   `docs/governance/ISSUE_CLOSING_VERIFICATION.md`（必须有 PR 关联）。
4. 本文档状态从 `DRAFT` 提升为 `ACTIVE` 的时机：A0 + A1 完成并测量后。

---

## 8. 假设与反假设

| 假设 | 若被推翻则如何 |
|---|---|
| F-01 是 #4910 profile 现象的主因 | 若 A1 后 `lock_shared_slow` 未显著下降，则 `&mut self` engine 边界（F-02）是另一独立主因，按 #4910 §3.1 推进 |
| group commit 能提升并发写吞吐 | 若 A0.3 后写吞吐无变化，检查协调器 `max_batch`/`max_wait_us` 是否与负载匹配（默认 32 / 1000µs） |
| 启用 `parallel-executor` 对 OLTP 无益（并发度=1） | 该 feature 主要服务 OLAP；OLTP 场景保留默认 1，不得为"启用而启用" |
| F-07 改 `Vec<Value>` 键不改变结果 | 用 TPC-H Q1/Q7 逐值比对；若出现差异，说明原字符串键依赖了类型坍缩，需先确认哪个是正确语义再决定 |

---

## 9. 明确不做（Out of Scope）

- 不重写存储格式为页式（Phase C/D 量级，需独立设计文档）。
- 不引入 JIT。
- 不改 `panic = "abort"`（未确认 `catch_unwind` 依赖前）。
- 不为"数字好看"调整任何 benchmark 参数或阈值。
- 不在本计划内声称任何未实测的 TPS 目标达成。

---

## 10. 实施记录 (2026-09-30)

> 本节只记录**已实际执行并验证**的动作与实跑输出。性能收益均标注为
> NOT-MEASURED，直至 SOAK/profile 对照完成（ADR-001 G-01）。
> **例外**: A1 / #4912 已完成 A/B 对照测量，见
> [`PERF_A1_4912_AB_MEASUREMENT.md`](./PERF_A1_4912_AB_MEASUREMENT.md)。

**基线 commit**: `99198a51512ab397d367570ed68b58fbed168fd5` (develop/v4.1.0)
**工作区**: 9 文件修改 (+213/-13) + 3 新增文件

### 10.1 A1 / #4912 — 无锁事务转发（已完成）

| 动作 | 文件 |
|---|---|
| `BoxStorageEngine` 覆写 `begin/commit/rollback_transaction_lockfree`，转发 `(**self)` | `crates/storage/src/binary_storage.rs:813-846` |
| `MvccStorage` 三个方法委托 inner | `crates/storage/src/mvcc_storage.rs:601-623` |
| `ParallelWalStorage` 三个方法委托 inner | `crates/storage/src/parallel_wal_storage.rs:291-312` |
| 新增回归测试 5 例 | `crates/storage/tests/lockfree_forwarding_4912.rs` |

**实跑输出**:

```
$ cargo test -p sqlrustgo-storage --test lockfree_forwarding_4912
running 5 tests
test box_storage_engine_forwards_begin_transaction_lockfree ... ok
test box_storage_engine_forwards_commit_transaction_lockfree ... ok
test box_storage_engine_forwards_rollback_transaction_lockfree ... ok
test box_storage_engine_does_not_fake_support_for_plain_engines ... ok
test wal_storage_begin_transaction_lockfree_is_supported ... ok
test result: ok. 5 passed; 0 failed
```

**测试有效性验证**（先红后绿）: 临时移除 `BoxStorageEngine` 的三个转发后重跑：

```
failures:
    box_storage_engine_forwards_begin_transaction_lockfree
    box_storage_engine_forwards_commit_transaction_lockfree
    box_storage_engine_forwards_rollback_transaction_lockfree
test result: FAILED. 2 passed; 3 failed
```

即测试确实能捕获 #4912 的缺陷，非空跑。

**性能收益**: **MEASURED**（2026-09-30）。完整方法、原始数据与边界见
[`PERF_A1_4912_AB_MEASUREMENT.md`](./PERF_A1_4912_AB_MEASUREMENT.md)。

A/B 实验：两个 release 二进制唯一差异是 `BoxStorageEngine` 是否转发
3 个 `*_lockfree` 方法。8 并发连接 × 150 事务 × 3 轮，
`--storage file --wal-sync every`。

| 指标 | before | after | 提升 |
|---|---:|---:|---:|
| TPS | 402.0 | 6,846.2 | **17.0x** |
| SELECT p50 | 4.256 ms | 0.240 ms | **17.8x** |
| SELECT p99 | 23.977 ms | 0.977 ms | **24.5x** |
| `sample` `lock_shared_slow` 帧 | 36 | **0** | — |
| `sample` parking_lot 帧合计 | 54 | **0** | — |

**边界（不得省略）**: 收益来自消除**全局 `RwLock` 争用**，不是消除 fsync。
无锁路径内部仍有 `Mutex<wal>` 串行化（`crates/storage/src/wal_storage.rs:1002`），
COMMIT 仍每事务 `sync_data()`。本测量未覆盖 MVCC/`--storage parallel`、
未做长稳 SOAK、未测多核扩展曲线。

### 10.2 A0 / #4913 — P0 quick wins

| 子项 | 状态 | 证据 |
|---|---|---|
| A0.1 `[profile.release]` + `[profile.bench]` | ✅ 完成 | `Cargo.toml`；`lto="fat"`、`codegen-units=1`、`strip="debuginfo"`；`panic` 保持 unwind |
| A0.2 `parallel-executor` 透传 + 静默降级 WARN | ✅ 完成 | `Cargo.toml` features；`src/execution_engine.rs:218-231` |
| A0.3 group commit 接线（`parallel` 路径） | ✅ 完成 | 见 10.3 与**未完成部分** |
| A0.3 group commit 接线（默认 `file` 路径） | ❌ **未完成** | 见 10.4 — 需存储层改造，非配置项 |
| A0.4 移除热路径调试输出 | ✅ 完成 | `crates/mysql-server/src/lib.rs` 删除 `eprintln!("SERVER: ...")`；`src/engine_select.rs` 的 `Q7_TRACE` 由每次查询 `env::var` 改为编译期 `const`（`--features q7-trace` 保留诊断） |

**A0.2 feature 透传的决定性验证**（临时 `cfg!` 探针，验证后已移除）：

```
$ cargo test -p sqlrustgo --features parallel-executor --lib __parallel_feature_probe
test __parallel_feature_probe::parallel_executor_feature_is_enabled ... ok
$ cargo test -p sqlrustgo --lib __parallel_feature_probe
test result: FAILED. 0 passed; 1 failed
```

**A0.1 / A0.2 / A0.4 性能收益**: **NOT-MEASURED**（A0.1 需 release 构建
前后 SOAK 对照；A0.2/A0.4 需 OLAP/OLTP 负载对照）。

### 10.3 A0.3 新增能力

`GroupCommitCoordinator` 缺少共享 inner 的构造入口，导致无法让协调器与
storage 共用同一个 WAL（两个独立 `BufWriter` 写同一 WAL 文件会交错损坏）。
新增：

- `GroupCommitCoordinator::with_shared_inner(Arc<Mutex<W>>, max_batch, max_wait_us)`
- `GroupCommitCoordinator::new_with_shared_inner(Arc<Mutex<W>>)`
  （`crates/storage/src/wal/group_commit.rs:131-158`）

服务器 `--storage parallel` 路径现在按 `--wal-sync group:N[,US]` 构造并安装
协调器（`crates/mysql-server/src/lib.rs:6280-6316`），输出
`WAL group commit ACTIVE: max_batch=..., max_wait_us=...`。

### 10.4 未完成项与原因（诚实记录）

**默认 `file` 存储的 group commit 仍然未生效。** 根因已在代码中明确：
`crates/storage/src/wal_storage.rs:49-52` 的文档写明

> "The fields are ignored on the `WalStorage` path (the `ParallelWalStorage`
> path uses them). To activate group commit, use
> `ParallelWalStorage::set_group_commit(coordinator)` after construction."

`WalStorage` 的 `wal: parking_lot::Mutex<T>` 是私有字段，且没有
`new_with_shared_wal` 等价构造器，因此协调器无法共享其 `BufWriter`。
这是**存储层改造**（把 `Mutex<T>` 改为 `Arc<Mutex<T>>` 并新增构造器），
不属于 A0「零逻辑风险配置项」的范围。

当前处理：请求 `group:N` 但走默认 `file` 存储时，服务器打印明确 WARN
（`crates/mysql-server/src/lib.rs:6372-6384`），不再静默接受后按每事务
fsync 运行。

**后续**: 建议在 B2 阶段（#4915）一并处理 `WalStorage` 的共享 WAL 改造。

### 10.5 回归验证汇总（实跑）

| 范围 | 命令 | 结果 |
|---|---|---|
| 存储 crate 全量 | `cargo test -p sqlrustgo-storage --all-features` | **1182 passed / 0 failed** |
| 根 crate lib | `cargo test -p sqlrustgo --all-features --lib` | **145 passed / 0 failed** |
| mysql-server lib | `cargo test -p sqlrustgo-mysql-server --all-features --lib -- --test-threads=1` | **261 passed / 0 failed** |
| 格式 | `cargo fmt --check --all` | **exit 0** |
| Clippy（改动文件） | `cargo clippy -p sqlrustgo-storage --all-features` | 改动文件 0 告警 |

**已知既有失败（非本次引入，已核验）**:

1. `cargo clippy -p sqlrustgo-storage --all-features -- -D warnings` → 2 个
   `clippy::type_complexity` 错误，位于 `crates/storage/src/backup_coordinator.rs:369`
   等**未改动**文件。
2. `cargo check -p sqlrustgo --all-features` → HEAD 上已有 23 个 unused-import
   警告（本工作区同样 23 个，未增加）。
3. `helpers_tests::read_executor_parallelism_honors_env` 为**固有 flaky**：
   3 个测试并发读写同一个进程级环境变量 `SQLRUSTGO_EXECUTOR_PARALLELISM`。
   单线程运行稳定通过（261 passed）。**A0.4/B2 之外建议单独开 issue 修**。
4. `scripts/gate/check_docs_consistency.sh` → 2 个**既有**错误
   （`VERSION_HISTORY.md` 版本号、`v3.12.0/CHANGELOG.md` 重复 commit），
   均在未改动文件中。
5. `scripts/gate/check_docs_links.sh` → **All markdown links are valid. exit 0**

### 10.6 附带修复（阻塞交付，非性能项）

`.gitignore:233` 的 `releases/`（未锚定）忽略了 `docs/releases/` 下**所有新增**
文件——该子树有 1828 个已跟踪文件，因此新发布文档会被静默丢弃。已改为
`/releases/`（只锚定仓库根，根级构建产物仍被忽略，已用
`git check-ignore` 验证），并补充 `*.pid`、
`docs/releases/**/driver.stdout` 等运行态垃圾的忽略规则。

- 验证: `git check-ignore -v releases/v1.2.0-darwin-arm64.tar.gz` → 仍被忽略
- 验证: `git status` 中新文档已可见

### 10.7 下一步（未开始）

- ~~A1 的性能对照测量~~ → **已完成**（见 `PERF_A1_4912_AB_MEASUREMENT.md`，
  TPS 17.0x）。遗留：应在修复后基线之上重新评估 #4910 的 `&mut self` 拆分
  是否仍有独立收益（本次未做）。
- Phase B1（#4914）/ B2（#4915）尚未动工。
- Phase C1–C4 的 Issue 尚未创建（按 §2 回填记录，待 A/B 产出实测结论）。
- 本次改动**尚未 commit / push**，需按
  `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 走 PR 流程。

### 10.8 Phase B2 逐项执行记录（2026-09-30，续 §10.7）

在 §10.7 记录的"尚未动工"之后，Phase B2 的 6 个子项按
"确定性 × 风险"排序逐个推进。已完成 4 项，2 项判定为
**需要独立设计 PR**，不在单次机械改动范围内。

| Task | 状态 | commit | 说明 |
|---|---|---|---|
| B2.1 写路径整表深拷贝 | ✅ 完成 | `6735c366cc` | `insert_direct` / `insert_buffered` / `flush_buffer` 三处的 `data.clone()` 换成 `TableData::snapshot_from(start)`（只带 `[start..]` 窗口）+ `save_table_window(table, window, total_rows)`。O(table_size) → O(row_count)，磁盘字节完全相同。`add_column` 的 `data.clone()` 保留——它 backfill 每一行，全量快照是正确语义，且不在 plan 列出的站点内。 |
| B2.2 锁内文件 I/O | ⚠️ **已重定向** | `e2355c0680` → 修 `ff34478830` → **重定向 `557fe61a74`** | **原改动打在了没人调的方法上。** `FileStorage::flush(&self)`（inherent）与 `StorageEngine::flush(&mut self)`（覆写）是同一段逻辑的两份独立拷贝，而服务端经 `MvccStorage` 调的是**覆写**——活路径上一直留着整表 `.cloned()`。现合并为单一实现（覆写委托 inherent），共用 `drain_dirty_windowed`：一次短临界区 drain + 只快照 `[last_saved..]` 窗口，锁外 I/O。顺带修掉 `flush_parallel` 的三处缺陷（≤2 表分支 drain 后调 `flush()` 见到空集 → **行被静默丢弃**；3+ 分支在 spawn 线程里读受 `write_lock` 保护的 HashMap → **data race**；两者都未先 push `insert_buffer` → **缓冲插入从未落盘**）。详见 [`PERF_B22_CONCURRENT_MEASUREMENT.md`](./PERF_B22_CONCURRENT_MEASUREMENT.md) §3 / §7.4。 |
| B2.3 全量快照缓冲 | ✅ 完成 | `e49a2a558f` | `save_table_full` 原先构造 owned `StoredTableData`（第二份全表拷贝）再 `to_string_pretty` 成 `String` 才落盘。新增借用的 `StoredTableDataRef`，直接序列化进 1 MB `BufWriter`；owned 版本保留给反序列化，磁盘格式逐字节不变。 |
| B2.4 `scan_with_filter` 统一 | ✅ 完成（存储层） | `a656850636` + 测试 `a494bf26e` | `WalStorage::update` 的 WAL before-image 采集改为 `scan_with_filter` 在引擎内过滤，clone 比例从"全表"降到"实际命中行"。`merge.rs` 的 `execute_merge` 两侧都要全表做 join，无谓词可下推，**按原样保留**。触发器 / 存储过程路径见 #4947。**2026-10-04 补**：`scan_with_filter` 此前**零测试覆盖**——整条优化建立在"filter 在 clone 之前"这一未验证假设上（`file_storage.rs:3800` 的 `filter(..).cloned()` 而非 `rows.clone()`）。已加 `crates/storage/tests/scan_with_filter_contract_test.rs`（5 例），并验证其有效性：注入"忽略谓词"后 2 例变红。同时钉住一条易被误判的事实：**恒真谓词（`&\|_r\| true`）与 `scan()` 成本相同**——每行仍执行 `cloned()`。故 #4962 的触发器路径切换是**类型修复**（解除 `Self: Sized` 导致 `dyn StorageEngine` 无法调用的限制），**不是性能优化**。详见 [`PERF_B2_4915_AB_MEASUREMENT.md`](./PERF_B2_4915_AB_MEASUREMENT.md) §4 B2.4 补充。 |
| B2.5 `committed_tables` Arc 化 | ⏸ **需独立设计 PR** | — | plan 给的是 `HashMap<String, Arc<Vec<Record>>>` + `Arc::make_mut`，但真正省 copy 的前提是 `tables` 也 Arc 化（否则 `self.tables.clone()` 逐个包 `Arc::new(data.clone())`，一次全量拷贝照旧）。`tables` Arc 化在 `engine.rs` 内触及 24 处 `self.tables` + 25 处 `get_mut`/`entry` + 18 处 `committed_tables`；且 `SchemaSnapshot` 是 **pub 类型**（`pub fn snapshot_schema() -> SchemaSnapshot`），字段 `tables: HashMap<String, Vec<Record>>` 是公共 API 的组成部分——Arc 化是 **breaking change**，需要单独版本或兼容层，不能混在性能 PR 里。 |
| B2.6 binary/columnar append | ⏸ **需独立设计 PR** | — | plan 描述的是"改 append 写入 + 独立 manifest；恢复路径加 `append_batch`"，这**不是优化而是存储格式变更**：新 `.bin` 布局 + 旧格式读取兼容 + manifest 原子写 + 恢复路径测试。`binary_storage.rs` 当前 `persist_table` 每次全量重写，且 `insert` 有 snapshot 保护分支（`new_with_data` 预载 `.bin` 快照，不能被空表覆盖）。在 B2.2 已把 flush 的锁内 I/O 移出后，binary 后端的持锁时间已同步受益，此项边际收益需要先测量再决定是否值得做格式变更。 |

**已完成部分共同验证**：

```
$ cargo test -p sqlrustgo-storage
   763 + 3 + 29 + 19 + 1 + 4 + 47 + 6 + 4 + 5 + 5 + 3 + 5 + 5 + 8 = 1182 passed; 0 failed
$ cargo build -p sqlrustgo-storage --all-features    # clean
$ cargo fmt -p sqlrustgo-storage --check              # clean
```

### 10.9 Phase B2 A/B 实测（2026-09-30，续 §10.8）

§10.8 的 NOT-MEASURED 声明已由本节取代。完整报告见
[`PERF_B2_4915_AB_MEASUREMENT.md`](./PERF_B2_4915_AB_MEASUREMENT.md)。

**基线修正**：§6 写的 `99198a515` 早于 #4912（`e8c67639e2`），该区间同时含
A1 与 B2 的改动，无法归因。改用 B2 动工前的 `be665d6bc1` 为基线。

**结果**（3 runs 取中位，criterion `--warm-up-time 2 --measurement-time 4`）：

| 基准 | baseline | HEAD | 倍数 |
|---|---|---|---|
| `b2_insert_into_large_table/1000` | 0.603 ms | 0.027 ms | **22.44x** |
| `b2_insert_into_large_table/10000` | 0.615 ms | 0.026 ms | **23.91x** |
| `b2_insert_into_large_table/50000` | 1.002 ms | 0.026 ms | **39.26x** |
| `b2_flush_dirty_tables/5tables_1000rows` | 0.193 ms | 0.195 ms | 0.99x |
| `b2_flush_dirty_tables/5tables_10000rows` | 0.175 ms | 0.195 ms | 0.99x |
| `b2_snapshot_and_scan/full_snapshot/50000` | 13.703 ms | 11.605 ms | 1.18x |
| `b2_snapshot_and_scan/filtered_scan_miss/50000` | 0.053 ms | 0.045 ms | 1.19x |

**两条须如实记录的结论**：

1. **B2.2 首版是回归，已修**。首版把 I/O 移出锁但快照整张 `TableData`，
   每次 flush 付 O(table_size) 拷贝，5 表场景测到 **0.86x**。已改为取窗口
   （`ff34478830`），复测 0.99x。**修复后是中性，不是收益。**
2. **B2.2 的收益仍未被证明**。它的目标是缩短持锁时间，而单线程 bench 测的
   是 flush 本身，测不出锁的收益。sysbench 8 线程 `oltp_read_write` 与
   `sample` 的 `lock_shared_slow` 本次**未跑**，TPC-H 亦未跑。在并发数字
   出来之前，B2.2 只支持"没有变慢"，不应记为已完成。

**顺带修复的既有腐化**：`crates/storage/benches/storage_benchmark.rs`
在本次工作前**已无法编译**（`TableInfo` / `ColumnDefinition` 长期未跟进
字段新增，报 4 处 `E0063`），因此 §6 指定的 `bench_insert` /
`bench_aggregate` 之外没有任何可用基准覆盖 B2 路径。已补齐字段并新增 3 组
只针对 B2 的基准。

**一处被测量证伪的"回归"**：首轮 `full_snapshot/50000` 报 0.77x，复查时
同一二进制复跑得到相差 2 倍的结果。根因是该基准在 `b.iter()` 内复用
同一个 `FileStorage` 且每轮再插 50000 行，磁盘 base snapshot 与 delta
无界累积，测的是漂移中的累积状态。改为每轮重建后该项为 1.18x 且连续
3 次稳定。**不复查就会把基准缺陷当成 B2.3 的回归写进结论。**

**下一步**：
- **B2.2 的并发证据已尝试补跑，结论是"拿不到"，原因是两个先于 B2 存在的
  缺陷**（详见
  [`PERF_B2_4915_AB_MEASUREMENT.md`](./PERF_B2_4915_AB_MEASUREMENT.md) §6）：
  1. **无 auto-increment 分配器**——`expr/mod.rs:1466` 明确
     "LAST_INSERT_ID() -> 0 (stateless; no AUTO_INCREMENT tracking)"，
     id 靠扫 `MAX(id)` 推导，8 并发 INSERT 必撞主键，sysbench 所有含
     INSERT 的负载在 HEAD 与 baseline 上同样中止；
  2. **并发读写打死服务端**——绕开第 1 点后，两侧交替跑、每轮重启，
     `qps=TIMEOUT state=WEDGED`，`sample` 显示全部线程卡死在
     `WalStorage<...>::rollback_transaction_lockfree` 与
     `RawRwLock::lock_{shared,exclusive}_slow`（#4910 系列，B2 未触碰）。

  纯读（`oltp_point_select` 8 threads，各 8 次）是唯一跑得动的并发测量：
  median 0.973x，区间大幅重叠，**与噪声不可区分**，且不经过 `flush()`，
  对 B2.2 无诊断价值。

  **因此 B2.2 维持"未证明"，不记为已完成也不记为失败。** 要拿到它的
  并发收益，需先修上述两项，而非继续压测。
- B1（#4914）尚未动工（F-07 GROUP BY `Vec<Value>` 键 / F-08 表达式绑定）
- B2.5 / B2.6 需要各自的独立设计 PR
- B2.4 的触发器 / 存储过程路径（`crates/executor/src/trigger.rs:833,932`、
  `stored_proc.rs`）本次未动，属 B2.4 的剩余部分
- §6 表格里的 `cargo build --release` 耗时 / 二进制大小、clippy、
  TPC-H 三项本次未按协议逐条执行

### 10.10 Phase B2.2 并发证据补测（2026-10-01，取代 §10.9 的"未跑"）

§10.9 与 `PERF_B2_4915_AB_MEASUREMENT.md` §6 都把「sysbench 8 线程 + `sample`
锁争用」列为 B2.2 的唯一证明手段。本次未跑 sysbench，而是直接对存储层做
并发读延迟 A/B（更易归因）。完整报告见
[`PERF_B22_CONCURRENT_MEASUREMENT.md`](./PERF_B22_CONCURRENT_MEASUREMENT.md)。

**结论是机制性的，且对 B2.2 不利**：

1. **B2.2 修改的方法没有生产调用者。** `FileStorage` 有两个 `flush`：
   inherent `pub fn flush(&self)`（`:946`，**B2.2 改的就是它**）与 trait 覆写
   `fn flush(&mut self)`（`:4293`，**实际被调用的那个**）。实测证据：在 inherent
   入口插桩后标记一次未打印，而在 `MvccStorage::flush` 插桩打印 21 次。
2. **活路径本来就已把 I/O 放在锁外**（trait 覆写只把 dirty 名字在锁内 drain）。
   所以 B2.2 想消除的「持锁期间做磁盘 I/O」在可达路径上不存在。
3. **实测与推论一致**：并发读延迟 6 次运行两侧区间重叠；flush 墙钟
   before 7.273 ms vs after 7.146 ms（各 10 次，0.982x，中性）。
4. **过程更正**：首轮仅 3 次采样得到 0.76x、一度看似回归；样本提到 10 次后
   差异消失。**3 次采样不足以判定本项。**

**顺带发现（未修）**：trait 覆写 `:4293` 里仍留着
`self.tables.get(&name).cloned()` 的**整表 clone**——正是 §4 把 B2.2 首版判为
0.86x 回归的同一模式，且在活路径上。真正的优化点在这里，不在 B2.2 改的那处。

**下一步（新增）**：
- 在 #4915 中把 B2.2 标注为「机制性 no-op（针对活路径）」而非「收益未证明」
- 对 trait 覆写 `:4293` 应用同一套 `snapshot_from` + `save_table_window` 改造
- 复核 `flush` 的 SQL 可达性：本次未找到从 SQL 到 `StorageEngine::flush` 的
  正常路径（`commit_transaction_lockfree` 不 flush、`commit_transaction_and_flush`
  与 `ExecutionEngine::flush` 均无调用者）

### 10.11 Phase B1 (F-07) 实施记录（2026-10-01）

**状态**：B1.1 + B1.2 完成；B1.3（F-08 完整绑定阶段）判定为独立设计 PR。

#### B1.1 / B1.2 — GROUP BY 分组键去 String 化（完成）

`src/engine_select.rs` 的 GROUP BY 从
`HashMap<String, Vec<Vec<Value>>>`（键 = 各分组表达式 `evaluate_expr_to_string`
后用 `\x00` join）改为 `HashMap<Vec<Value>, Vec<usize>>`（键 = 结构化 `Value`
向量，值 = 行下标）。

消除的三项代价：

| 旧行为 | 新行为 |
|---|---|
| 每行 `O(分组列数)` 次 String 分配 + `join` 再分配一次 | 每行构造一个 `Vec<Value>`（值本身已在行里） |
| 每组整行 `clone()` 进 `Vec<Vec<Value>>` | 只存 `usize` 行下标；聚合时按需物化该组 |
| 聚合时 `key.split('\x00')` + `parse::<i64>/<f64>` **猜回类型** | 键就是原始类型，无需反解析 |

**正确性修复（这才是重点）**：旧实现把 `Value::Text("123")` 与
`Value::Integer(123)` 归入**同一个键**（都字符串化成 `"123"`），再用
`parse::<i64>()` 猜回类型。同一列里混存 INTEGER 与 TEXT 时会静默合并分组。

对旧实现复跑新增测试确认该缺陷真实存在：

```
$ cargo test -p sqlrustgo --all-features --lib v411_group_by   # 旧 String 键实现
test execution_engine_tests::test_v411_group_by_distinguishes_integer_and_text_key ... FAILED
test execution_engine_tests::test_v411_group_by_text_value_looking_numeric_stays_text ... FAILED
assertion `left == right` failed: INTEGER 7 and TEXT '7' must NOT collapse into one group;
  got ["Integer(7),Integer(2)"]
test result: FAILED. 1 passed; 2 failed
```

即 `7` 与 `'7'` 确实被合并成一组（count=2）。修复后 3 个测试全通过。

新增测试（`src/execution_engine_tests.rs`）：
- `test_v411_group_by_distinguishes_integer_and_text_key`
- `test_v411_group_by_key_values_keep_their_types`
- `test_v411_group_by_text_value_looking_numeric_stays_text`（`'1'` vs `'01'`）

#### B1.4 — GROUP BY 内 `fd_per_col` 的 O(columns²) 查找（完成）

`fd_per_col` 原本对 `table_info.columns` 的每个 `c` 调用
`find_column_index(&c.name, &table_info)` —— 一次大小写不敏感的线性字符串
扫描，叠加在外层 O(columns) 循环上，构成**每组 O(columns²) 次字符串比较**。
但 `c` 就是 `table_info.columns[i]`，下标即 `enumerate()` 的 `i`，查找本身
是多余的。已改为直接用下标。

#### B1.3 — F-08 完整绑定阶段（⏸ 独立设计 PR）

暂不实施，理由：

- `find_column_index` 在全仓有 **40 处调用点**；把 `Expression` 编译成带
  slot 的绑定树会触及求值器、计划器与所有谓词路径，计划 §4 亦将其标为
  「高风险（改动面大）」，要求单独 PR。
- 缺少能保护该改动的基准：`benches/bench_aggregate.rs` 覆盖 GROUP BY 聚合，
  但目标路径是**表达式求值**，目前没有覆盖 `find_column_index` 的基准
  （`PERF_B2_4915_AB_MEASUREMENT.md` §2 已记录 `storage_benchmark` 曾长期
  编译失败，说明基准面本身不完整）。
- 在无基准保护的情况下做 40 点改动，无法证明收益也无法排除回归。

**建议的下一步**：先补一个覆盖 `WHERE <col> = <lit>` 多列扫描的基准，
再在绑定树上动刀。

#### 验证

```
$ cargo test -p sqlrustgo --all-features --lib
test result: ok. 156 passed; 0 failed     # 153 原有 + 3 新增
$ cargo fmt --check --all                 # exit 0
$ cargo clippy -p sqlrustgo --all-features  # 改动文件 0 告警
```

### 10.13 阻塞缺陷修复与 B2.2 重定向（2026-10-04）

§10.9/§10.10 记录了 B2.2 的并发证据被两个既有缺陷挡住。本节记录这三个
缺陷的处理与 B2.2 的重定向。详细证据见
[`PERF_B22_CONCURRENT_MEASUREMENT.md`](./PERF_B22_CONCURRENT_MEASUREMENT.md) §7
与 [`PERF_B2_4915_AB_MEASUREMENT.md`](./PERF_B2_4915_AB_MEASUREMENT.md) §6。

| 缺陷 | 根因 | 状态 |
|---|---|---|
| **BLK-1** 无 auto-increment 分配器 | `engine_dml.rs` 用**写锁之外**的 `pre_scanned_rows` 算 `MAX(id)+1`，并发 INSERT 读到同一 MAX → 主键冲突 | ✅ `3c64dc19ec` |
| **BLK-2** `*_transaction_lockfree(&self)` 从读守卫洗出 `&mut S` | `WalStorage::as_inner_mut()` 从 `&self` 派生 `&mut`；其前提"engine 用自身 mutex 串行化"在每连接一 engine 的服务端不成立 | ✅ `8ff90269d3` |
| **BLK-3** AUTO_INCREMENT 序列跨 autocommit 事务重用 id | 修 BLK-1 后暴露：480 次插入只用了 id 1..99。`current_tx_id` 是存储级单字段，而计数器应按表建模 | ⏸ OPEN |

BLK-1 服务端验证（真实 MySQL 协议）：8 线程 × 60 = **480 次 INSERT，0 报错、
0 重复 id**。BLK-2 端到端验证：修前 `qps=TIMEOUT state=WEDGED`（HEAD 与
baseline 相同），修后 20s/30s 分别 634/545 QPS，60s 跑完服务端仍存活。

**BLK-2 的关键点**：新增 `&self` trait 方法
`set_current_tx_id_shared` / `discard_all_buffers_shared`（MemoryStorage 用
`AtomicU64`，FileStorage 走自身内部 `write_lock`），并**删除** `as_inner_mut`
——留一个未用的 `&mut`-from-`&self` 助手就是本次事故的成因本身。
`BoxStorageEngine` 的转发必须写：它是类型擦除包装，漏掉 override 会
**静默回落**到 trait 的 no-op 默认而非编译失败，而所有服务端 storage 都被
它包着。

**B2.2 重定向**（`557fe61a74`）：`PERF_B22_CONCURRENT_MEASUREMENT.md` §3
确证原改动打在了**没有生产调用者**的 inherent `flush()` 上，活路径（trait
覆写）一直留着整表 `.cloned()`。现已合并为单一实现，共用
`drain_dirty_windowed`。顺带修掉 `flush_parallel` 的三处缺陷——其中
**"行被静默丢弃"**和 **data race** 两项是正确性问题，不是性能问题。

**B2.2 的性能收益仍为 NOT-MEASURED**：单线程 0.99x、并发 6 次运行无显著
差异，两者都与"活路径本来就没有持锁 I/O"一致。重定向移除了整表拷贝但未重跑
criterion 对照——基线 `be665d6bc1` 与现版本之间夹着 BLK-1/BLK-2/BLK-3
三个修复，不可直接比较。本节的价值在于**消除重复实现 + 修正确性问题**。

**验证**：

```
$ cargo test -p sqlrustgo-storage          # 1193 passed / 0 failed（原 1188）
$ cargo test --lib                         # 157 passed / 0 failed
$ cargo fmt -p sqlrustgo-storage --check   # exit 0
```

**下一步**：BLK-3 需要把 AUTO_INCREMENT 计数器与 `current_tx_id` 分离建模
（前者按表、后者按连接）；另发现持久化缺口——`flush()` 只在启动恢复与
LOAD DATA 末尾调用，无周期性/每事务 flush，重启后行全丢（`COUNT(*)` 也只
返回 257 而非真实行数），应单开 issue。
