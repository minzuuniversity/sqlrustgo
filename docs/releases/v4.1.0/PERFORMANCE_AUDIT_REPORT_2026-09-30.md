# SQLRustGo v4.1.0 性能审计报告 (PERFORMANCE_AUDIT_REPORT)

> **provenance (ADR-014 5 evidence fields):**
> - `source_agent`: deepseek-flash (DSH harness)
> - `source_run`: perf-audit-2026-09-30
> - `timestamp`: 2026-09-30T12:24:35Z
> - `evidence_hash`: local-git:`99198a51512ab397d367570ed68b58fbed168fd5` (develop/v4.1.0)
> - `conflict_resolution`: N/A — single-agent scope; 与 Issue #4910 交叉引用，不覆盖其结论
>
> **政策**: Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-008 + ADR-014
> **Status**: DRAFT
> **Branch**: `develop/v4.1.0`
> **权威阶段状态**: `docs/releases/v4.1.0/STAGE.yaml`

---

## 0. 证据边界声明（先读）

本报告的方法论是**静态代码审计**（read/grep 逐处确认），**不是** profiling 报告。

| 本报告声称 | 本报告**不**声称 |
|---|---|
| 每一条 finding 都有 `文件:行号` 与真实代码片段 | 任何未测量的性能数字（TPS/QPS/延迟） |
| 复杂度和分配模式可由代码结构推断 | 各 finding 的**实际**收益倍数 |
| 与既有 profile 证据（#4910）的机制一致性 | 修复后达到某个具体 TPS 目标 |
| 引用行号在 `99198a5151` 上可复现 | 运行过 benchmark 或 profiler |

**未执行**：`cargo bench`、`perf`/`sample`、SOAK。所有"预期收益"均为**待验证假设**，验证方式见第 6 节。

既有实测证据来自 Issue #4910 引用的 `docs/releases/v4.0.0/V400_04_20MIN_SOAK.md` §3
（sysbench oltp_read_only，4 threads，5s `sample`）——本报告只引用，不重测。

---

## 1. 审计范围与方法

| 项 | 值 |
|---|---|
| 仓库 | `openclaw/sqlrustgo` |
| 分支 / commit | `develop/v4.1.0` @ `99198a5151` |
| 规模 | 699 个 `.rs`，约 312,000 行（`crates/` + `src/`） |
| 最大 crate | `storage` 55.2k / `executor` 50.4k / `parser` 34.3k |
| 最大单文件 | `crates/parser/src/parser.rs` 20,119 行 |
| 构建 profile | 根 `Cargo.toml` 1758 行，**仅 1 个 profile**（`[profile.test]`，第 928 行） |
| 命令 | `read` / `grep` / `awk` / `git log` / `tea issues` |

**审计面的选择依据**：按"每次查询/每行必经"筛选热点路径，而非按文件大小。
覆盖 6 条路径：存储 I/O、页缓存、WAL/事务、查询执行、表达式求值、网络协议。

---

## 2. 结论摘要

共 **18** 条 finding，分 4 级：

| 级别 | 条数 | 含义 | 处理建议 |
|---|---|---|---|
| **P0-CRITICAL** | 2 | 已实现的优化被错误连接**完全旁路**；契约违宪 | 立即修，收益最大且风险最低 |
| **P0-QUICKWIN** | 4 | 配置/调试残留，零逻辑风险 | 1 周内 |
| **P1-ALGORITHMIC** | 7 | 每行分配 / O(N²) / 整表拷贝 | 2-6 周 |
| **P2-STRUCTURAL** | 5 | 需要架构改造 | 与 Phase B/C 合并 |

**最高优先级的单一发现**（F-01）：项目**已经投入实现了无锁事务路径**
（`wal_storage.rs:976`、`wal_storage.rs:1008`，对应 #4910 §3.2 方向），
但在服务器实际使用的类型上，该路径**永远返回 Err 并回落到全局写锁**。
即：优化已写好，接线错了，收益为 0。

---

## 3. 发现清单

### 3.1 P0-CRITICAL

---

#### F-01 `BoxStorageEngine` 不转发 `*_lockfree`，无锁事务路径在生产上恒不生效

**严重度**: P0-CRITICAL（**最高**）
**位置**: `crates/storage/src/binary_storage.rs:644-680`、`crates/storage/src/engine.rs:1283-1295`、`src/execution_engine_methods.rs:1513-1527`

`StorageEngine` trait 为锁占用优化提供了两个方法，trait 默认实现是**报错**
（`crates/storage/src/engine.rs:1283`）：

```rust
fn begin_transaction_lockfree(&self, _tx_id: u64) -> SqlResult<()> {
    Err(SqlError::ExecutionError(
        "begin_transaction_lockfree not supported".to_string(),
    ))
}
```

真正的实现只存在于 `WalStorage`：`crates/storage/src/wal_storage.rs:976`（begin）
与 `crates/storage/src/wal_storage.rs:1008`（commit）。

调用方在 BEGIN 时**优先尝试**无锁路径
（`src/execution_engine_methods.rs:1513-1527`）：

```rust
let lockfree_ok = {
    let storage = self.storage.read();
    storage.begin_transaction_lockfree(tx_id.as_u64()).is_ok()
};
if !lockfree_ok {
    // Fallback: lockfree not supported by this storage engine.
    let mut storage = self.storage.write();          // ← 全局写锁
    storage.set_current_tx_id(tx_id.as_u64());
    let _ = storage.begin_transaction();
}
```

但服务器创建的存储对象被包在 `BoxStorageEngine` 里
（`crates/mysql-server/src/lib.rs:6292`、`:6367`）：

```rust
Arc::new(parking_lot::RwLock::new(BoxStorageEngine::new(wal_storage)))
```

而 `BoxStorageEngine` 对 `StorageEngine` 的实现
（`crates/storage/src/binary_storage.rs:679` 起）**只逐个转发普通方法**，
全文 `grep lockfree crates/storage/src/binary_storage.rs` → **0 命中**。
因此类型擦除后调用落到 trait 默认实现，恒为 `Err`。

**验证矩阵**（代码级，已 grep 确认）：

| 类型 | `begin_transaction_lockfree` | 是否被服务器使用 |
|---|---|---|
| `WalStorage` | ✅ 实现（`wal_storage.rs:976`） | 是（作为 inner） |
| `MvccStorage` | ❌ 未实现 | 是（作为 inner） |
| `ParallelWalStorage` | ❌ 未实现 | 是（`--storage parallel`） |
| `BoxStorageEngine` | ❌ **未转发** | **是（最外层）** |
| `FileStorage` / `MemoryStorage` | ❌ 未实现 | 是（作为 inner） |

**影响**: BEGIN 与 COMMIT 每次都取全局 `Arc<RwLock<storage>>` 写锁；
持锁期间所有其他连接的 SELECT 读数（`storage.read()`）阻塞。
机制上完全解释 #4910 记录的 profile：`lock_shared_slow` 10,025 样本 / `execute_select` 10,280 样本
（**97.5% 的读路径样本落在锁等待中**）。这把"已实现"的优化变成了死代码。

**建议**: `BoxStorageEngine` 覆写两个方法转发到 `(**self)`
（`begin_transaction_lockfree` / `commit_transaction_lockfree`）。
这是 4 行改动，风险极低，且能让既有 `WalStorage` 实现真正生效。
同时给 `ParallelWalStorage`、`MvccStorage` 补实现（或显式委托 inner），
否则 `--storage parallel` 仍走写锁。

**关联**: #4910（master tracking issue，§3.1/§3.2 提出拆分 `execute` 的 `&mut self`，
方向正确；本 finding 指出**在那之前有一处接线 bug 让收益为 0**，
应先做本项以取得可测量基线，再评估是否需要 `&mut self` 拆分）。

---

#### F-02 `ExecutionEngine::execute(&mut self)` + 每连接独占 engine 锁，构成吞吐上限

**严重度**: P0-CRITICAL
**位置**: `crates/mysql-server/src/lib.rs:5314-5327`

```rust
let result = if let Some(stmt) = is_read_only {
    let eng = engine.read();                       // 共享读
    match rstmt {
        Some(ReadOnlyStmt::Select(s)) => eng.execute_select(s),
        ...
    }
} else {
    let mut eng = engine.write();                  // 独占
    eprintln!("SERVER: eng.execute(sql={})", stmt_sql);
    eng.execute(stmt_sql)
};
```

engine 本身是**每连接独立**的（`crates/mysql-server/src/lib.rs:5927`，
`Arc::new(RwLock::new(ExecutionEngine::new(storage.clone())))`），
因此这层锁不是跨连接争用点；真正的跨连接串行化来自 F-01 的共享 `storage` 写锁。

但 `execute(&mut self)` 使得**同一连接内**任何语句（含纯 SELECT 之外的 DML/DDL）
都必须拿 `engine.write()`，且 `&mut self` 在 Rust 借用层面
阻止了单连接内的语句级并行。这是 #4910 的核心论点，机制成立。

**建议**: 优先级低于 F-01。先修 F-01 取得干净基线；
若基线显示 engine 边界仍是瓶颈，再按 #4910 §3.1 拆分
`plan(&self)` / `execute_plan(&mut self)`。

---

### 3.2 P0-QUICKWIN（配置类，零逻辑风险）

---

#### F-03 `[profile.release]` 完全缺失，LTO 关闭、`codegen-units` 默认 16

**严重度**: P0-QUICKWIN
**位置**: `Cargo.toml`（1758 行中唯一的 profile 是第 928 行 `[profile.test]`）

```
$ grep -n '^\[profile' Cargo.toml
928:[profile.test]
```

即：无 `lto`、无 `codegen-units = 1`、无 `panic`、无 `strip`。
发布二进制（`target/release`，635MB）以默认 thin-local 内联构建，
跨 crate（storage ↔ executor ↔ mysql-server）热点无法内联。

**建议**: 新增（详见优化计划 A2）：

```toml
[profile.release]
lto = "fat"
codegen-units = 1
strip = "debuginfo"
```

`panic = "abort"` **不建议**首轮启用：需先确认无 `catch_unwind` 依赖
（`crates/mysql-server/src/lib.rs` 存在 poisoning recovery 注释，见 `:5313`）。

---

#### F-04 `parallel-executor` feature 从未启用，并行开关是空开关

**严重度**: P0-QUICKWIN
**位置**: `crates/executor/Cargo.toml:43`、`Cargo.toml:66,160`、`crates/mysql-server/src/main.rs:241`

```toml
# crates/executor/Cargo.toml:43
parallel-executor = []
```

全仓搜索该 feature 的**启用点**：

```
$ grep -rn 'parallel-executor' --include='*.toml' . | grep -v '^./target'
./crates/executor/Cargo.toml:32:# See openspec/changes/issue-3703-intra-query-parallel-executor/.
./crates/executor/Cargo.toml:43:parallel-executor = []
```

`crates/executor` 在根 `Cargo.toml:66` 以 `{ path = "crates/executor" }` 引入
（无 `features`），`:160` 亦然。CI 与 `scripts/` 亦无启用点。

但用户可见的开关确实存在（`crates/mysql-server/src/main.rs:241` 起）：
`--executor-parallelism`（默认 1）、`SQLRUSTGO_EXECUTOR_PARALLELISM`
（`src/execution_engine.rs:217`）。开启后 `src/execution_engine.rs:224-229`
还会真的去建 rayon 线程池。

被 feature 门控的并行实现位于
`crates/executor/src/parallel_group_by.rs:475-485`、`crates/executor/src/task_scheduler.rs:128`；
未启用时走 `cfg(not(feature = "parallel-executor"))` 串行分支。

**影响**: 用户传 `--executor-parallelism=8` 会得到"看似并行、实际串行"的
静默降级，且构建 `--all-features` 也无法覆盖（`--all-features` 只作用于
当前包自身的 features，不会启用依赖 crate 的 feature）。

**建议**: 在根 `Cargo.toml` 做 feature 透传
（`parallel-executor = ["sqlrustgo-executor/parallel-executor"]`）
并把该 feature 加入默认构建；或至少在 `--executor-parallelism > 1`
且 feature 未启用时打印 WARN，而不是静默降级。

---

#### F-05 WAL 每事务 fsync；`GroupCommitCoordinator` 已实现但服务器未接线

**严重度**: P0-QUICKWIN
**位置**: `crates/mysql-server/src/lib.rs:6285-6291`、`crates/mysql-server/src/main.rs:248`、`crates/storage/src/wal/group_commit.rs`

服务器自己的注释已经承认这件事（`crates/mysql-server/src/lib.rs:6285`）：

```rust
// does not yet install a `GroupCommitCoordinator` here. To
// use group commit, construct a `ParallelWalStorage` with
// a `GroupCommitCoordinator` programmatically (see
// `crates/storage/tests/group_commit_integration.rs` for an
// example). The storage layer's commit path already routes
// through the coordinator when one is installed; this
// server just hasn't been wired up to construct one yet.
```

`--wal-sync` 默认 `"every"`（`crates/mysql-server/src/main.rs:248`），
即每事务 `flush() + sync_data()`。协调器实现完整
（`crates/storage/src/wal/group_commit.rs`，含 `with_limits`、
`commit_lsn`、`sync_now`），且 `WalStorage` 的 commit 路径已支持
"装了协调器就走协调器"（`crates/storage/src/parallel_wal_storage.rs:173-180`）。
设计文档 `docs/releases/v4.0.0/PHASE_B_GROUP_COMMIT.md` 状态栏原文即
"**POC shipped** ...; server-side wiring is the follow-up."

**影响**: 每 commit 一次 fsync 是写路径的硬延迟地板；协调器把 N 个并发
commit 合并为一次 fsync（InnoDB 语义），是并发写吞吐的直接乘数。

**建议**: 在 `crates/mysql-server/src/lib.rs:6292` 与 `:6367` 两处
按 `--wal-sync group:...` 构造并安装协调器。

---

#### F-06 热路径调试输出未清理

**严重度**: P0-QUICKWIN
**位置**: `crates/mysql-server/src/lib.rs:5325`；`src/engine_select.rs:1766,1770-1789`

```rust
// crates/mysql-server/src/lib.rs:5325 — 每条非只读语句执行一次
let mut eng = engine.write();
eprintln!("SERVER: eng.execute(sql={})", stmt_sql);
```

```rust
// src/engine_select.rs:1766 — 每个 GROUP BY 查询一次
let _q7_trace = std::env::var("Q7_TRACE").is_ok();
// src/engine_select.rs:1770-1789 — _q7_trace 为真时每行 eprintln
```

`eprintln!` 持久持有 stderr 锁并做行缓冲，在 QPS 路径上是可观开销；
`std::env::var` 每次调用都走环境块查找。`src/engine_select.rs` 全文
18 处 `println!/eprintln!`。

**建议**: 删除 `lib.rs:5325` 的 `eprintln!`；把 `Q7_TRACE` 诊断块改为
`#[cfg(feature = "q7-trace")]` 门控或直接删除。

---

### 3.3 P1-ALGORITHMIC（每行分配 / O(N²) / 整表拷贝）

---

#### F-07 GROUP BY 用「String 拼串 + 反解析」做分组键

**严重度**: P1
**位置**: `src/engine_select.rs:1764-1770`（键类型）、`:1791-1800`（构造）、`:1810-1830`（反解析）

```rust
// :1764
let mut groups: std::collections::HashMap<String, Vec<Vec<Value>>> =
    std::collections::HashMap::new();
...
// :1791
for row in &rows {
    let key = group_exprs
        .iter()
        .map(|expr| evaluate_expr_to_string(expr, row, &table_info))  // 每分组列一次 String
        .collect::<Vec<_>>()
        .join("\x00");                                               // 再一次 String
    groups.entry(key).or_default().push(row.clone());                // 整行再次深拷贝
}
```

聚合时又反向拆解（`:1810` 起）：`key.split('\x00')` 后逐个
`s.parse::<i64>()` / `s.parse::<f64>()` **猜回类型**。

三重代价：每行 `O(分组列数)` 次堆分配；每行整行深拷贝；
**类型信息在字符串化中丢失**（`Value::Text("123")` 与
`Value::Integer(123)` 会归入同一组，反解析结果的类型也可能与原值不符）。

`Value` 已实现 `Hash + Eq + Ord`（`crates/types/src/value.rs:41-81`），
无需字符串中转。`evaluate_expr_to_string` 本身还先做一次完整
`evaluate_expression`（`src/expr_utils.rs:1443-1453`）。

**建议**: 键改 `HashMap<Vec<Value>, Vec<usize>>`（值为行下标，
聚合时按需索引 `rows[i]`，消除整行 clone）。

---

#### F-08 表达式求值无绑定阶段：每行每表达式重做列名线性解析

**严重度**: P1
**位置**: `crates/executor/src/expr/mod.rs:620-655`

```rust
if let Some(idx) = columns.iter().position(|c| c.name == col_name) { return Some(idx); }
if let Some(idx) = columns
    .iter()
    .position(|c| c.name.eq_ignore_ascii_case(col_name)) { return Some(idx); }
...
let user_segments: Vec<&str> = col_name.split('.').collect();
for (i, c) in columns.iter().enumerate() {
    let col_segments: Vec<&str> = c.name.split('.').collect();   // 每行每列一次 Vec 分配
```

求值器没有"编译/绑定"阶段：`Identifier` 每次求值都按**字符串**重新解析列位置。
总代价 `O(行数 × 列数 × 表达式数)`，且多 join 分支为每行每列分配
`Vec<&str>`。这是纯标量解释执行的典型形态。

**建议**: 引入绑定阶段，把 `Expression` 编译为带列下标（slot）的树，
解析只做一次；`Expression::Literal` 也应在绑定时解析为 `Value`
（现状见 `src/engine_select.rs:3292` 的 `eval_literal_from_str` 调用）。

---

#### F-09 FileStorage 写路径在**持全局写锁**时整表深拷贝 + 执行文件 I/O

**严重度**: P1
**位置**: `crates/storage/src/file_storage.rs:3198-3218`（`insert_direct`）、
`:3242-3245`（`insert_buffered`）、`:3279-3282`（`flush_buffer`）、`:890-901`（`flush`）

```rust
// :3198
Self::with_write_lock(self.as_mut_self(), |s| -> Option<...> {
    if let Some(ref mut data) = s.tables.get_mut(table) {
        start_row_id = data.rows.len() as u32;
        data.rows.extend(records.iter().cloned());
        let table_data = data.clone();               // ← 克隆全部历史行
        let cols = data.info.columns.clone();
        if s.save_table(table, &table_data).is_ok() { // ← 锁内 JSON 序列化 + write syscall
```

而 `save_table`（`crates/storage/src/file_storage.rs:744-745`）
只消费 `rows[last_saved..]` 的增量：

```rust
let new_rows = &table_data.rows[last_saved..];
self.append_table_delta(table_name, new_rows)?;
```

即那次整表 clone 是**纯浪费**。文件内注释（`:129-137`、`:3810-3814`）
自述曾靠把 threshold 从 100 提到 10000 来"摊薄"这个问题，
说明 O(N²) 是已知但未根治。

**建议**: `save_table(&self, name, rows: &[Record])` 改收切片，
直接传 `&records` 或 `&data.rows[last_saved..]`，删除 `data.clone()`；
锁内只标记 `dirty_tables` 并取出待写增量，**解锁后**再做序列化与 write。

---

#### F-10 每次 `scan` 深拷贝整表；WAL 的 update/delete 依赖它

**严重度**: P1
**位置**: `crates/storage/src/file_storage.rs:3615-3619`、`crates/storage/src/wal_storage.rs:691-696`

```rust
// file_storage.rs:3615
fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
    let mut rows: Vec<Record> = self
        .get_table(table)
        .map(|data| data.rows.clone())      // ← 每次扫描深拷贝整表
        .unwrap_or_default();
```

```rust
// wal_storage.rs:691
let all_rows = self.inner().scan(table)?;            // O(N) 深拷贝
let rows_to_update: Vec<(Vec<u8>, Vec<Value>)> = all_rows.iter()
    .filter(|r| Self::row_matches_filter(r, filters))
    .map(|r| (Self::record_key(r), r.clone()))       // 命中行再 clone
    .collect();
```

`WalStorage::update` 先整表深拷贝，再对命中行二次 clone。而
`scan_with_filter`（`crates/storage/src/file_storage.rs:3634-3641`、
`crates/storage/src/engine.rs:1753-1761`）已经实现了"锁内过滤、只 clone 命中行"，
只是调用点没统一。生产调用者包括
`crates/executor/src/trigger.rs:833,932`（触发器）、`crates/executor/src/stored_proc.rs`
多处、`crates/executor/src/merge.rs:47,52`。

另：`src/engine_select.rs:3430` 的 `scan_with_ahi` 把
`storage: &parking_lot::RwLockReadGuard<'_, S>` 作为参数传入，
意味着**全局存储读锁在整个扫描期间持有**（读读可并行，但 DDL/写阻塞）。

**建议**: `WalStorage::update/delete` 与触发器/存储过程路径统一改用
`scan_with_filter`；`tables` 改 `HashMap<String, Arc<Vec<Record>>>`，
提供 `scan_ref` 返回借用迭代器或 `Arc<[Record]>`。

---

#### F-11 `committed_tables` 每次 commit 深拷贝整个数据库

**严重度**: P1
**位置**: `crates/storage/src/engine.rs:1607`、`:1825`、`:1938-1946`

```rust
// :1607 (commit_transaction_with_log) 与 :1825 同样写法
self.committed_tables = self.tables.clone();
```

autocommit 插入路径（`:1938-1946`）额外逐行 clone 一份：

```rust
} else {
    self.committed_tables.entry(table_key.clone()).or_default()
        .extend(padded.iter().cloned());       // 每行再克隆
}
self.tables.entry(table_key).or_default().extend(padded);
```

用途仅是"晚加入的连接继承已提交状态"（`:1444-1449`）。
代价：常驻内存 ≈ 2×，每次 COMMIT 一次 `O(数据库总大小)` 停顿。

**建议**: `committed_tables: HashMap<String, Arc<Vec<Record>>>` +
`Arc::make_mut`（无其他引用时零拷贝）。

---

#### F-12 `save_table_full` 一次落盘产生 3 份全量副本，且用 `to_string_pretty`

**严重度**: P1
**位置**: `crates/storage/src/file_storage.rs:765-782`、`:653`（`save_index` 同型）

```rust
let stored = StoredTableData {
    ...
    rows: table_data.rows.clone(),                 // 副本 1
};
let json = serde_json::to_string_pretty(&stored)   // 副本 2（缩进额外 +20~30% 字节）
    .map_err(...)?;
writer.write_all(json.as_bytes())?;                // 副本 3
```

落盘 JSON 是机器读取，缩进无收益。索引同理：
`save_index` 用 `to_string_pretty` 序列化整棵 B+ 树。

**建议**: `serde_json::to_writer(&mut BufWriter::with_capacity(1<<20, file), &stored)`；
`StoredTableData` 改借用字段 `rows: &'a [Record]`。

---

#### F-13 binary / columnar 后端每次 `insert` 全量重写整表文件（O(N²)）

**严重度**: P1
**位置**: `crates/storage/src/binary_storage.rs:345-353`、`:374-375`、`:398-399`；
`crates/storage/src/columnar/storage.rs:680-688`、`:169-228`

```rust
// binary_storage.rs
existing.rows.extend(records);
self.persist_table(table)?;   // 每次 insert 从头重写整个 .bin
```

`force_insert`（`:398-399`）在 WAL 恢复时**逐行**调用，使恢复时间随数据量
平方增长。columnar 每次 insert 后 `store.serialize(&path)`
（`columnar/storage.rs:680-688`），而 `serialize`（`:169-228`）
重写所有 `column_*.bin` + `metadata.json`。

**建议**: append 写入；行数/统计放独立 manifest/footer；全量重写只在
`compact()` 触发；恢复路径提供 `append_batch(&[Record])` 批量接口。

---

### 3.4 P2-STRUCTURAL

---

#### F-14 页缓存事实上不存在（`BufferPool` 零生产调用者）

**严重度**: P2（架构决策）
**位置**: `crates/storage/src/buffer_pool.rs:8`、`crates/storage/src/lib.rs:55`

`BufferPool` 仅被 re-export，全仓无生产调用者。同类死代码：
`BufferPoolMetrics`、`PageGuard`、`MemoryPool`（`crates/storage/src/page_guard.rs`、
`buffer_pool_metrics.rs`）。两处注释声称已集成，但被引用的 API 不存在：

- `crates/storage/src/change_buffer.rs:40` — "Integrated into `BufferPool::read_page()`"
- `crates/storage/src/page_guard.rs:65` — `buffer_pool.fetch_page(page_id)?`

`BufferPool::get` / `fetch_page` / `read_page` 在 `buffer_pool.rs` 中均不存在。

即便接入，`BufferPool::get` 的实现也有缺陷
（`crates/storage/src/buffer_pool.rs:81-101`）：

```rust
let pages = self.pages.lock().unwrap();
if let Some(page) = pages.get(&page_id).cloned() {
    let mut lru = self.lru.lock().unwrap();
    lru.retain(|&id| id != page_id);            // O(capacity) 线性扫描，且在 pages 锁内
    lru.push_front(page_id);
    let mut stats = self.stats.write().unwrap(); // 每次命中抢一次全局写锁
```

**建议**: 二选一，不要维持现状 —— (a) 真正接入并改用
`crates/storage/src/clock_replacer.rs` 已有的 CLOCK 算法、
统计改 `AtomicU64`；(b) 删除这批死代码，避免"有缓存"的假象。

---

#### F-15 B+ 树索引存在 split 丢键缺陷，是索引下推无法启用的根因

**严重度**: P2（**正确性缺陷**，非纯性能）
**位置**: `crates/storage/src/bplus_tree/index.rs:214-252`、`:510-529`

```rust
// :214 insert_key_value → insert_at 在节点满时 split 并返回
Some((split_key, new_node))
```

但唯一调用者 `insert_into_node`（`:517-529`）**丢弃返回值**：

```rust
if node.is_leaf {
    node.insert_key_value(key, value);   // Option<(i64, BTreeNode)> 被忽略
```

而 `insert_at`（`:228-247`）已经 `split_off` 把后半 keys 移入 `new_node`
——该 `new_node` 从未被 `allocate_node` 注册进 `self.nodes`。
即：**插入超过 `MAX_KEYS_PER_NODE` 后索引静默丢键，`search` 返回错误结果**。
另 `insert`（`:372-392`）只建叶子根、从不建内部节点。

**影响**: 这解释了 `src/engine_select.rs:3464-3503`（`USE INDEX` 路径）
为何写成半成品（以 `Value::Null` 当 key 去"测试索引支持"、
把索引名当列名匹配）。**索引下推不是"还没做"，而是被这个缺陷挡住。**

**建议**: 先修 split（分配并链接 `new_node`、写内部节点、设 `next_leaf`），
再启用索引扫描。本项必须先于任何索引性能工作。

---

#### F-16 `FileStorage::update` 绕过 `with_write_lock`，与读路径假设冲突

**严重度**: P2（**并发正确性**）
**位置**: `crates/storage/src/file_storage.rs:4040`、`:4070`、`:4078`、`:3700`

`update` 直接 `self.tables.get_mut(table)`、原地改行、直写 `dirty_tables`，
而 `insert`/`delete` 都走 `with_write_lock`（`:3823`、`:3842`）。
同一把锁出现两套纪律。与此并行的是"读路径无锁"假设
（`scan_with_index:3700` 无锁读 `self.tables`）——两者组合构成数据竞争。

另：`with_write_lock`（`:319-349`）每次调用一次
`Box::into_raw`/`from_raw` + unsafe 类型擦除；`FileStorage::insert`
（`:3815-3825`）单次 insert 独立获取 write_lock 两次。

**建议**: 独立 issue 处理（正确性优先于性能）；裸 `tables` 字段改字段级
锁（文件内注释的 Phase C.2 计划）。

---

#### F-17 结果集整体物化后才发送

**严重度**: P2
**位置**: `crates/mysql-server/src/lib.rs:3170-3193`（`send_result_set` /
`send_result_set_with_more`）

```rust
fn send_result_set<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],      // ← 已完全物化的行集
```

内存峰值 = 全结果集；首字节延迟 = 全查询耗时。对 OLAP/导出场景是硬伤。

**建议**: 执行器改流式迭代器 + 协议层边算边发。改动横跨
executor/engine/network，建议与 Phase C 合并评估。

---

#### F-18 编译与 CI 层：13327 个测试、443 个显式 target、bench job 引用不存在的包

**严重度**: P2
**位置**: `Cargo.toml`、`.github/workflows/bench-pr.yml:29`、`crates/bench/Cargo.toml:2`

| 项 | 实测 |
|---|---|
| `#[test]` / `#[tokio::test]` | 13,327 |
| 测试文件数 | 659 |
| `[[test]]`/`[[bench]]`/`[[bin]]` 声明 | 443 |
| `target/` 占用 | 3.6 GB（debug 3.0 GB + release 635 MB） |
| `bench-pr.yml` 引用 | `-p sqlrustgo-bench-cli` — **该包不存在**（实际 `sqlrustgo-bench`） |

`[profile.test] opt-level = 0`（`Cargo.toml:929`）使任何以测试形式运行的
测量失真，也让数值/加密类测试显著变慢。

**建议**: `bench-pr.yml` 改包名；CI 全 job 加缓存
（`ci-pr.yml` 的 fmt/docs job 目前无缓存）；
`[profile.test]` 提到 `opt-level = 1`；443 个声明收敛为目录自动发现。

---

## 4. 与既有工作的关系（避免重复劳动）

| 既有工作 | 状态 | 本报告的关系 |
|---|---|---|
| **Issue #4910** `execute(&mut self)` 单线程吞吐 | OPEN | 本报告 **F-01 指出其前置接线 bug**：`BoxStorageEngine` 未转发 lockfree，导致已有优化收益为 0。建议先做 F-01 取得干净基线，再评估 #4910 的 `&mut self` 拆分是否必要 |
| `PERFORMANCE_OPTIMIZATION_ROADMAP.md` (v4.0.0) | 长期路线 | 其 B2「无 prepared statement cache」**已过期**（`crates/cache/src/lib.rs` 已实现，`src/engine_ddl.rs:1202` 在用）；B5「无缓冲池」**部分过期**（`BufferPool` 存在但未接线 → 本报告 F-14） |
| `PHASE_B_GROUP_COMMIT.md` (v4.0.0) | POC shipped，server wiring 为 follow-up | 本报告 **F-05 即该 follow-up** |
| `PERFORMANCE_PLAN.md` (v4.0.0, 2026-09-13) | profile 已定位 storage 写锁 | 本报告 F-01 给出**该 profile 现象的确切代码机制** |
| `V400_04_20MIN_SOAK.md` | 实测 TPS 3,284（20 min） | 本报告引用其数据，不重测 |

**本报告新增、路线图未覆盖的项**: F-01、F-03、F-04、F-06、F-07、F-08、
F-09、F-10、F-11、F-12、F-13、F-15、F-16。

---

## 5. 未声称事项（Anti-Fabrication 边界）

- 未运行任何 benchmark / profiler / SOAK。
- 未测量任何 finding 的实际收益倍数。
- 未验证 F-15（B+ 树丢键）在真实 SQL 负载下的可达路径
  （`insert_with_index` 目前只有测试调用者，
  `crates/storage/tests/file_storage_direct_v3_12.rs:362-380,476-480`）。
- 未验证 F-16 的数据竞争在并发下可实际触发。
- 未评估各修复之间的相互作用（例如 F-01 生效后 F-10 的锁争用画像会改变）。
- 与 ADR-008（test-claim transparency）一致：本报告**无**任何
  "X/Y PASS" 形式的测试声明。

---

## 6. 验证要求（每项修复的验收方式）

| Finding | 验收证据 | 工具 |
|---|---|---|
| F-01 | `BoxStorageEngine` 上 `begin_transaction_lockfree` 返回 `Ok`；并发 BEGIN/SELECT 不再阻塞 | 单元测试 + `sample` profile 对照 |
| F-02 | 单连接内 SELECT 不取 `engine.write()` | 代码审查 + bench |
| F-03 | release 二进制可用，`cargo build --release` 通过；1h SOAK TPS 对照 | `cargo build --release` + SOAK |
| F-04 | `--executor-parallelism=8` 时 `parallel_group_by` 走 rayon 分支 | `cargo test -p sqlrustgo-executor --features parallel-executor` |
| F-05 | `--wal-sync group:32` 下并发 commit 只触发 ~1/N 次 fsync | `group_commit` 集成测试 + strace 计数 |
| F-06 | `grep -c 'eprintln!' crates/mysql-server/src/lib.rs` 下降 | grep |
| F-07 | GROUP BY 输出与修复前逐行一致（含类型） | 现有 TPC-H Q1/Q7 回归 |
| F-08 | 表达式求值结果等价 | parser/executor 全量测试 |
| F-09 | `insert_direct` 无整表 clone；bulk load 时间亚二次增长 | `benches/bench_insert.rs` |
| F-10 | `WalStorage::update` 走 `scan_with_filter` | 单元测试 + `cargo test -p sqlrustgo-storage` |
| F-11 | COMMIT 无全库 clone | 代码审查 + 内存曲线 |
| F-15 | 插入 > `MAX_KEYS_PER_NODE` 后 `search` 仍正确 | 新增 B+ 树测试 |
| F-18 | `cargo bench` job 可运行 | CI |

**门槛纪律**（`GATE_CONDITIONS.md` + `ISSUE_CLOSING_VERIFICATION.md`）：
任何 finding 的 issue 关闭前必须附实跑输出；禁止无证据的 "PASS" 声明。

---

## 7. 后续动作

1. 本报告的优化序列见
   [`PERFORMANCE_OPTIMIZATION_PLAN.md`](./PERFORMANCE_OPTIMIZATION_PLAN.md)
   （同目录）。
2. 对应开发 ISSUE 在 Gitea `openclaw/sqlrustgo` 创建，编号见优化计划 §2。
3. 实施纪律：每项改动前跑影响分析，改动后跑
   `cargo clippy --all-features -- -D warnings` + `cargo fmt --check --all`
   + 相关 crate 测试。

---

## 附录 A — 引用文件索引

| 文件 | Finding |
|---|---|
| `Cargo.toml` | F-03, F-04, F-18 |
| `crates/executor/Cargo.toml` | F-04 |
| `crates/mysql-server/src/main.rs` | F-04, F-05 |
| `crates/mysql-server/src/lib.rs` | F-02, F-05, F-06, F-17 |
| `src/execution_engine_methods.rs` | F-01, F-02 |
| `src/execution_engine.rs` | F-01, F-04 |
| `src/engine_select.rs` | F-06, F-07, F-10 |
| `src/expr_utils.rs` | F-07 |
| `crates/executor/src/expr/mod.rs` | F-08 |
| `crates/storage/src/binary_storage.rs` | F-01, F-13 |
| `crates/storage/src/engine.rs` | F-01, F-10, F-11 |
| `crates/storage/src/wal_storage.rs` | F-01, F-10 |
| `crates/storage/src/parallel_wal_storage.rs` | F-01, F-05 |
| `crates/storage/src/file_storage.rs` | F-09, F-10, F-12, F-16 |
| `crates/storage/src/buffer_pool.rs` | F-14 |
| `crates/storage/src/bplus_tree/index.rs` | F-15 |
| `crates/storage/src/columnar/storage.rs` | F-13 |
| `.github/workflows/bench-pr.yml` | F-18 |

## 附录 B — 验证命令记录

本报告全部行号在以下 commit 上核验：

```
$ git rev-parse HEAD
99198a51512ab397d367570ed68b58fbed168fd5
$ git branch --show-current
develop/v4.1.0
```

关键核验命令：

```
$ grep -n '^\[profile' Cargo.toml
928:[profile.test]

$ grep -rn 'parallel-executor' --include='*.toml' . | grep -v '^./target'
./crates/executor/Cargo.toml:32:# See openspec/changes/issue-3703-intra-query-parallel-executor/.
./crates/executor/Cargo.toml:43:parallel-executor = []

$ grep -n 'lockfree' crates/storage/src/binary_storage.rs
(无输出)

$ grep -rn 'fn begin_transaction_lockfree|fn commit_transaction_lockfree' crates/storage/src/
crates/storage/src/engine.rs:1283    (trait 默认实现 → Err)
crates/storage/src/engine.rs:1291    (trait 默认实现 → Err)
crates/storage/src/wal_storage.rs:976  (真实实现)
crates/storage/src/wal_storage.rs:1008 (真实实现)
```
