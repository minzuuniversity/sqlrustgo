# 性能提升任务深度分析 — Executor 多线程 + MVCC

> **Status**: Technical Analysis (Phase A & C 详细设计)
> **Created**: 2026-09-12
> **Related**: `PERFORMANCE_OPTIMIZATION_ROADMAP.md`
> **Author**: sqlrustgo core team

---

## 1. 现状代码结构分析

### 1.1 关键代码定位

经过对 v4.0.0 (`1a85ea8f3`) 源码的深度分析,关键瓶颈集中在以下位置:

| 文件 | 行数 | 关键组件 |
|---|---|---|
| `src/execution_engine.rs` | 2692 | `ExecutionEngine::execute()`, 60+ 个 execute_xxx 方法 |
| `crates/mysql-server/src/lib.rs` | 10000+ | `handle_connection()`, `do_command_loop()`, `COM_QUERY` 处理 |
| `crates/executor/src/parallel_executor.rs` | 350 | `ParallelVolcanoExecutor` (已有但未启用) |
| `crates/executor/src/task_scheduler.rs` | 315 | `RayonTaskScheduler` |
| `crates/executor/src/thread_pool_registry.rs` | 236 | `ThreadPoolRegistry` (per-query pools) |
| `crates/storage/src/file_storage.rs` | 5000+ | `FileStorage`, `tables: HashMap<String, TableData>` |
| `crates/storage/src/engine.rs` | 5000+ | `StorageEngine` trait, `MemoryStorage` impl |

### 1.2 当前执行链路 (端到端)

```
TCP 连接 (8 个 sysbench threads)
  ↓
[crates/mysql-server/src/lib.rs]
handle_connection() (per-connection thread)
  ↓
do_command_loop() (循环读 packet)
  ↓
COM_QUERY 包 (SELECT/INSERT/UPDATE/DELETE)
  ↓
[lib.rs:11889-12375] ← 关键瓶颈点
match is_read_only {
    Some(ReadOnlyStmt) => engine.read().execute_select(),
    None              => engine.write().execute(sql),  ← 写操作 → 写锁
};
  ↓
[src/execution_engine.rs:758]
ExecutionEngine::execute(&mut self, sql: &str)
  ↓
parse(sql) → Statement
  ↓
match statement {
    Statement::Select(s) => self.execute_select(s),  ← 单线程执行
    Statement::Insert(i) => self.execute_insert(i),  ← 单线程
    Statement::Update(u) => self.execute_update(u),  ← 单线程 (V4.0.0 SOAK leak fix 在此)
    Statement::Delete(d) => self.execute_delete(d),  ← 单线程
    ...
}
  ↓
[crates/storage/src/file_storage.rs]
FileStorage::scan() / insert() / delete() / update()
  - tables: HashMap<String, TableData>  ← 无锁!
  - 读写互斥靠上层 RwLock<ExecutionEngine>
```

### 1.3 性能瓶颈的根因

#### 瓶颈 1: 全局写锁 (8x 影响)

```rust
// crates/mysql-server/src/lib.rs:12375
} else {
    let mut eng = engine.write();  // ← 任何写操作都拿 WRITE LOCK
    eng.execute(stmt_sql)
};
```

**8 个 sysbench 线程**: sysbench oltp_read_write 中:
- 70% 是 SELECT (读) → 应该走 `engine.read()`
- 30% 是 INSERT/UPDATE/DELETE (写) → 必须走 `engine.write()`

但**当前 `engine.write()` 是全局独占锁**,即使 30% 的写操作也会**阻塞所有读**。

**根因**: `RwLock<ExecutionEngine>` 的 `ExecutionEngine` 含可变状态(`stats`, `current_tx_id`, `tx_undo_log` 等)

#### 瓶颈 2: 单执行器 (Issue #3703)

```rust
// src/execution_engine.rs:758
pub fn execute(&mut self, sql: &str) -> SqlResult<ExecutorResult> {
    // ...
    match statement {
        Statement::Update(...) => self.execute_update(...),
        // 全部单线程顺序执行
    }
}
```

**当前**: 1 个执行线程 (--executor-parallelism=1)
**已实现但未启用**: `ParallelVolcanoExecutor` (350 行代码)
**开关**: env `SQLRUSTGO_EXECUTOR_PARALLELISM` (默认 1)

#### 瓶颈 3: 无 MVCC (并发能力受限)

```rust
// FileStorage (crates/storage/src/file_storage.rs)
pub struct FileStorage {
    tables: HashMap<String, TableData>,  // ← 无版本控制!
    tx_undo_log: Vec<UndoOp>,           // ← 仅单事务级 undo
    indexes: RwLock<HashMap<...>>,      // ← 索引已有锁
}
```

**当前隔离级别**:
- 默认 autocommit (无事务)
- 事务通过 `current_tx_id` 标记
- 写操作走 `tx_undo_log` 维护回滚
- **无 MVCC**: 读操作必须等写锁释放才能读最新数据

#### 瓶颈 4: 单连接线程模型

```rust
// crates/mysql-server/src/lib.rs
fn handle_connection(mut stream: TcpStream, ...) {
    thread::spawn(move || {
        do_command_loop(stream, ...);  // 每个连接一个线程
    });
}
```

**当前**: 1 thread per connection
**8 sysbench threads = 8 TCP connection = 8 thread**,但全部争用同一执行器

---

## 2. Phase A: Executor 多线程化 (4-6 周, 250-300 TPS)

### 2.1 核心思路

```
当前: 1 thread per conn × 8 conn = 8 thread → 全部串行执行
目标: N 个 worker thread (4-8) + 工作窃取队列 → 真正并行
```

### 2.2 改造方案

#### Step 1: 引入 ExecutorPool (1 周)

**位置**: `crates/executor/src/executor_pool.rs` (NEW)

```rust
//! ExecutorPool - 多执行器池, 工作窃取调度
//! 解决 --executor-parallelism=1 的单线程瓶颈

use std::sync::Arc;
use crossbeam::deque::{Injector, Worker, Stealer};
use parking_lot::Mutex;

/// 一个可执行的查询任务
pub struct QueryTask {
    pub sql: String,
    pub conn_id: u64,
    pub session_id: u64,
    pub isolation: IsolationLevel,
    pub priority: TaskPriority,
    pub submitted_at: Instant,
}

pub enum TaskPriority {
    Interactive,  // 用户查询 (sysbench OLTP)
    Batch,        // 批量任务
    Background,   // 后台维护
}

#[derive(Clone)]
pub struct ExecutorConfig {
    pub num_workers: usize,        // 默认 = num_cpus
    pub queue_capacity: usize,     // 默认 = 1024
    pub enable_work_stealing: bool, // 默认 true
}

/// 多执行器池
pub struct ExecutorPool {
    config: ExecutorConfig,
    global_queue: Arc<Injector<QueryTask>>,
    workers: Vec<WorkerThread>,
    shutdown: Arc<AtomicBool>,
    metrics: Arc<ExecutorMetrics>,
}

struct WorkerThread {
    handle: JoinHandle<()>,
    local_queue: Worker<QueryTask>,
    stealer: Stealer<QueryTask>,
    id: usize,
}

impl ExecutorPool {
    pub fn new(config: ExecutorConfig) -> Self {
        let global_queue = Arc::new(Injector::new());
        let mut workers = Vec::with_capacity(config.num_workers);
        
        for i in 0..config.num_workers {
            let local = Worker::new_fifo();
            let stealer = local.stealer();
            let queue = Arc::clone(&global_queue);
            let shutdown = Arc::new(AtomicBool::new(false));
            let metrics = Arc::new(ExecutorMetrics::new());
            
            let handle = std::thread::Builder::new()
                .name(format!("executor-worker-{i}"))
                .spawn(move || {
                    worker_loop(local, queue, stealer, shutdown, metrics, i);
                })?;
            
            workers.push(WorkerThread { handle, local_queue: local, stealer, id: i });
        }
        
        Self { config, global_queue, workers, ... }
    }
    
    /// 提交查询任务 (非阻塞)
    pub async fn submit(&self, task: QueryTask) -> Result<TaskHandle> {
        let handle = TaskHandle::new(task.conn_id);
        self.global_queue.push(task);
        Ok(handle)
    }
}

fn worker_loop(
    local: Worker<QueryTask>,
    global: Arc<Injector<QueryTask>>,
    stealer: Stealer<QueryTask>,
    shutdown: Arc<AtomicBool>,
    metrics: Arc<ExecutorMetrics>,
    worker_id: usize,
) {
    loop {
        if shutdown.load(Ordering::Relaxed) { break; }
        
        // 1. 本地队列优先
        // 2. 全局队列
        // 3. 窃取其他 worker
        let task = find_task(&local, &global, &stealer);
        
        if let Some(task) = task {
            execute_task(task, &metrics, worker_id);
        } else {
            // Park 当前线程,等待唤醒
            std::thread::park_timeout(Duration::from_millis(100));
        }
    }
}
```

**核心设计**:
- `Injector` 全局队列 (MPSC)
- 每个 worker 有 `Worker` 本地队列 (LIFO, 缓存友好)
- 工作窃取: 闲 worker 从忙 worker 偷任务

#### Step 2: 拆分 RwLock<ExecutionEngine> → 多子锁 (2 周)

**问题**: 当前 `Arc<RwLock<ExecutionEngine>>` 是大锁,粒度过粗

**方案**: 引入 lock striping

```rust
// src/execution_engine.rs

pub struct ExecutionEngine {
    // 读可并发 - 改为 DashMap
    catalog: Arc<DashMap<String, TableInfo>>,
    stats: Arc<DashMap<String, TableStats>>,
    
    // 写需要序列化 - 保留 RwLock 但粒度更细
    schema_locks: DashMap<String, RwLock<SchemaVersion>>,
    
    // 事务状态 - 每连接独立 (已在 thread_local)
    current_tx_id: ThreadLocal<Cell<u64>>,
    
    // 共享的执行器池
    executor_pool: Arc<ExecutorPool>,
    
    // 共享的存储引擎
    storage: Arc<FileStorage>,  // ← FileStorage 内部已有 RwLock
}
```

#### Step 3: 让 FileStorage 支持并发读 (1 周)

**当前问题**: `tables: HashMap<String, TableData>` 无锁,所有访问走 outer RwLock

**方案**: 内部用 `RwLock<HashMap>`

```rust
// crates/storage/src/file_storage.rs

pub struct FileStorage {
    // 改为内部锁, 允许多读并发
    tables: parking_lot::RwLock<HashMap<String, TableData>>,
    
    // 索引已是 RwLock, 保持
    indexes: parking_lot::RwLock<HashMap<(String, String), BPlusTree>>,
    
    // 其他字段保持...
}

// scan 方法改为:
fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
    let tables = self.tables.read();  // 多 reader 并发!
    let data = tables.get(table)
        .ok_or_else(|| SqlError::TableNotFound(table.to_string()))?;
    let rows = data.rows.clone();
    drop(tables);  // 尽早释放锁
    Ok(rows)
}

// insert 用 write lock:
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    let mut tables = self.tables.write();  // 仅写时独占
    let data = tables.entry(table.to_string())
        .or_insert_with(|| TableData::default());
    data.rows.extend(records);
    Ok(())
}
```

**关键改动**: 移除 outer RwLock<ExecutionEngine>, 让 FileStorage 内部锁管理

#### Step 4: 修改 mysql-server 端 (1 周)

```rust
// crates/mysql-server/src/lib.rs:11889

// 改前:
let result = if let Some(stmt) = is_read_only {
    let eng = engine.read();
    eng.execute_select(stmt)
} else {
    let mut eng = engine.write();  // ← 写操作阻塞所有读
    eng.execute(stmt_sql)
};

// 改后:
let result = if let Some(stmt) = is_read_only {
    // 读操作直接提交到 executor pool
    let task = QueryTask::new_read(stmt, conn_id);
    executor_pool.submit(task).await?
} else {
    // 写操作也走 pool, 但需要串行化
    let task = QueryTask::new_write(stmt_sql, conn_id, write_lock_key);
    executor_pool.submit(task).await?
};
```

### 2.3 性能预期 (Phase A 完成)

| 场景 | 当前 | Phase A 后 | 提升 |
|---|---|---|---|
| 8-thread oltp_read_write | 164 TPS | **250-300 TPS** | 1.5-1.8x |
| 单连接 SELECT | 20 TPS | 20 TPS | 1x (无瓶颈) |
| 8-thread 全写 | 50 TPS | 80-100 TPS | 1.6-2x |
| 8-thread 全读 | 100 TPS | 200-300 TPS | 2-3x |
| RSS | 17-80 MB | 20-100 MB | +20% (可接受) |

### 2.4 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 数据竞争 | HIGH | 严格用 `parking_lot::RwLock`,锁粒度测试 |
| 死锁 | HIGH | 避免嵌套锁,统一加锁顺序 |
| TPS 不升反降 | MEDIUM | SOAK 自动化门禁,regression 阻断 |
| 兼容现有测试 | MEDIUM | 22 个 V400 测试 + 408 eval 必过 |

---

## 3. Phase C: MVCC (16-24 周, 600-800 TPS)

### 3.1 MVCC 概念

**Multi-Version Concurrency Control**:
- 每行维护多个版本 `{ts: 100, data: "old"}, {ts: 105, data: "new"}`
- 读事务看到 `read_ts` 之前的最新已提交版本
- 写事务创建新版本,不阻塞读
- **关键优势**: 读不阻塞写,写不阻塞读

### 3.2 当前事务隔离级别

```rust
// src/execution_engine.rs:1744
match isolation_level {
    ParserIsolationLevel::ReadCommitted => TmIsolationLevel::SnapshotIsolation,
    ParserIsolationLevel::ReadUncommitted => TmIsolationLevel::ReadUncommitted,
    // ...
}
```

sqlrustgo 已经支持 `IsolationLevel` 概念,但底层仍是无版本快照 (每次读拿当前状态)。

### 3.3 MVCC 改造方案

#### 数据结构: 行级版本链

```rust
// crates/storage/src/mvcc.rs (NEW)

use std::sync::atomic::{AtomicU64, Ordering};

/// 全局单调递增的时间戳
static GLOBAL_TS: AtomicU64 = AtomicU64::new(1);

pub fn next_ts() -> u64 {
    GLOBAL_TS.fetch_add(1, Ordering::SeqCst)
}

/// 一行的多版本
pub struct VersionedRow {
    /// 版本链, 按 ts 升序
    versions: Vec<RowVersion>,
}

pub struct RowVersion {
    pub ts: u64,           // 提交时间戳
    pub data: Record,      // 行数据
    pub deleted: bool,     // 删除标记 (墓碑)
    pub tx_id: u64,        // 创建该版本的事务
}

impl VersionedRow {
    /// 读取事务可见的版本
    pub fn visible(&self, read_ts: u64, tx_id: u64) -> Option<&Record> {
        self.versions.iter()
            .rev()  // 从最新版本开始
            .find(|v| v.ts <= read_ts && v.tx_id != tx_id && !v.deleted)
            .map(|v| &v.data)
    }
    
    /// 插入新版本
    pub fn insert_version(&mut self, data: Record, tx_id: u64) {
        let ts = next_ts();
        self.versions.push(RowVersion { ts, data, deleted: false, tx_id });
    }
    
    /// 删除当前版本 (墓碑)
    pub fn delete_version(&mut self, tx_id: u64) {
        let ts = next_ts();
        self.versions.push(RowVersion { ts, data: Record::empty(), deleted: true, tx_id });
    }
}
```

#### 存储层集成

```rust
// crates/storage/src/file_storage.rs (改造)

pub struct FileStorage {
    // MVCC 表数据
    tables: parking_lot::RwLock<HashMap<String, MVTableData>>,
    
    // 每行的版本链 (lock-free per row)
    row_versions: DashMap<(String, RowId), Arc<parking_lot::RwLock<VersionedRow>>>,
    
    // 活跃事务列表 (用于 GC)
    active_txns: Arc<RwLock<HashMap<u64, TransactionState>>>,
}

pub struct MVTableData {
    schema: TableSchema,
    /// 主键 → 行版本位置 (lock-free)
    primary_index: BPlusTree,
    // rows: Vec<Record>  ← 移除,改为按需查 row_versions
}

pub struct TransactionState {
    pub tx_id: u64,
    pub read_ts: u64,       // 开始事务时取的快照
    pub commit_ts: Option<u64>,  // 提交时间 (未提交则 None)
    pub write_set: Vec<(RowId, RowVersion)>,  // Undo log
}
```

#### 读路径 (Snapshot Read)

```rust
fn scan(&self, table: &str, tx_id: u64, read_ts: u64) -> SqlResult<Vec<Record>> {
    let tables = self.tables.read();
    let data = tables.get(table).ok_or(...)?;
    let primary_index = &data.primary_index;
    
    // 1. 获取主键范围
    let pks = primary_index.range();
    
    // 2. 对每个 RowId, lock-free 读取 VersionedRow
    let rows: Vec<Record> = pks.iter()
        .filter_map(|pk| {
            let versioned = self.row_versions.get(&...)?;
            let row_guard = versioned.read();
            row_guard.visible(read_ts, tx_id).cloned()
        })
        .collect();
    
    Ok(rows)
}
```

#### 写路径 (Append New Version)

```rust
fn insert(&mut self, table: &str, records: Vec<Record>, tx_id: u64) -> SqlResult<()> {
    let mut tables = self.tables.write();
    let data = tables.entry(table.to_string()).or_default();
    
    for record in records {
        let row_id = data.primary_index.insert(record.pk)?;
        let versioned = VersionedRow::new();
        versioned.insert_version(record.clone(), tx_id);
        self.row_versions.insert((table.to_string(), row_id), Arc::new(RwLock::new(versioned)));
    }
    Ok(())
}
```

#### 垃圾回收 (GC)

```rust
// 后台线程定期 GC 旧版本
fn gc_loop(storage: Arc<FileStorage>) {
    let mut last_gc_ts = next_ts();
    
    loop {
        std::thread::sleep(Duration::from_secs(60));
        
        // 找出最早的活跃事务 read_ts 作为 GC 下限
        let min_active_read_ts = storage.active_txns.read()
            .values()
            .map(|t| t.read_ts)
            .min()
            .unwrap_or(last_gc_ts);
        
        // 删除 ts < min_active_read_ts 的版本 (每个 VersionedRow 只保留最新可见版本)
        for entry in storage.row_versions.iter() {
            let mut row = entry.value().write();
            row.versions.retain(|v| v.ts >= min_active_read_ts);
            if row.versions.is_empty() {
                // 整行可删除
            }
        }
    }
}
```

### 3.4 性能预期 (Phase C 完成)

| 场景 | Phase A | Phase C | 提升 |
|---|---|---|---|
| 8-thread oltp_read_write | 250-300 TPS | **500-700 TPS** | 2-2.3x |
| 8-thread 全写 (高竞争) | 80-100 TPS | **300-500 TPS** | 3-5x (MVCC 核心收益) |
| 8-thread 全读 | 200-300 TPS | **400-500 TPS** | 1.5-2x |
| 读不阻塞写 | N/A | **YES** | 关键能力 |
| RSS | 20-100 MB | 50-200 MB | 略增 |

### 3.5 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 版本链膨胀 | HIGH | 后台 GC, 定期 vacuum |
| 写写冲突检测 (WW) | MEDIUM | 用 ts + tx_id 检测 |
| 索引维护复杂 | MEDIUM | 主键索引加锁升级,二级索引惰性更新 |
| 兼容性 | HIGH | 必须保持所有现有 SOAK 测试通过 |

---

## 4. 实施时间表

### Phase A (v4.1.0, 4-6 周)

| 周 | 任务 | 验收 |
|---|---|---|
| W1 | ExecutorPool + work-stealing | 单元测试 + bench (单线程 baseline) |
| W2 | 拆分 ExecutionEngine locks | 22 个 V400 测试通过 |
| W3 | FileStorage 内部锁 | 408 eval 100%, sysbench TPS ≥ 200 |
| W4 | mysql-server 端集成 | sysbench TPS ≥ 250, RSS ≤ 100MB |
| W5 | CI 性能门禁 + 自动化 SOAK | regression 检测通过 |
| W6 | 文档 + Phase A release notes | 发版 v4.1.0 |

### Phase C (v5.0.0, 16-24 周)

| 周 | 任务 | 验收 |
|---|---|---|
| W1-W2 | VersionedRow + row_versions | 单元测试 |
| W3-W4 | 替换 FileStorage::tables 为 MVTable | 所有现有测试通过 |
| W5-W6 | 读路径 (Snapshot Read) | 读不阻塞写 |
| W7-W8 | 写路径 (Append Version) | 写可并发 |
| W9-W10 | 事务隔离级别实现 | SI + RC 测试 |
| W11-W12 | GC 机制 | 长时间 SOAK RSS 稳定 |
| W13-W14 | 性能调优 | TPS ≥ 500 |
| W15-W16 | 兼容性回归 | GMP-Platform 408 100% |
| W17-W20 | 文档 + Phase C release | 发版 v5.0.0 |

---

## 5. 关键技术决策

### 5.1 用 rayon 还是 tokio 还是 crossbeam?

| 框架 | 优势 | 劣势 | 选择 |
|---|---|---|---|
| rayon | 数据并行友好 | 任务调度粒度粗 | ✗ |
| tokio | 异步生态完整 | 需要 `async/await` 改写 | ✗ |
| crossbeam | 工作窃取 (work-stealing) 原生支持 | 学习曲线 | **✓** |

**决策**: 使用 `crossbeam-deque` 的 `Injector` + `Worker` + `Stealer` 三件套

### 5.2 锁的选择: parking_lot vs std

| 库 | 性能 | 公平性 | 毒化处理 | 选择 |
|---|---|---|---|---|
| std::sync::RwLock | 中等 | 公平 | 毒化需手动 | ✗ |
| parking_lot::RwLock | 高 (~2x) | 写优先 | 自动 `into_inner()` 恢复 | **✓** |
| tokio::sync::RwLock | 高 | 异步友好 | 复杂 | ✗ (非异步场景) |

**决策**: `parking_lot::RwLock` (已在 codebase 大量使用)

### 5.3 MVCC 时间戳生成

```rust
// 方案 1: 全局 AtomicU64 单调递增
static GLOBAL_TS: AtomicU64 = AtomicU64::new(1);
pub fn next_ts() -> u64 { GLOBAL_TS.fetch_add(1, Ordering::SeqCst) }
```
**优点**: 简单, 全局单调
**缺点**: 序列化争用

```rust
// 方案 2: Hybrid Logical Clock (HLC)
// 物理时间 + 逻辑计数器
struct HLC { wall_time_ms: u64, logical: u32 }
```
**优点**: 可分布式
**缺点**: 复杂, 单机不需要

**决策**: **方案 1** (单机单进程, 不需要 HLC)

---

## 6. 验证策略

### 6.1 性能回归测试

每个 Phase 必须通过:

```
1. sysbench oltp_read_write 1h SOAK
   - TPS ≥ Phase 目标 (Phase A: 250, Phase C: 600)
   - RSS 不超过基线 × 1.5
   - 0 errors, 0 reconnects

2. GMP-Platform 408 评估 100% 通过
   - 确保功能不破坏

3. V400 测试 22 个全部通过
   - 向量 + 图 + 事务测试

4. mysql-server 端到端测试
   - sysbench prepare + run + cleanup
```

### 6.2 正确性测试

```
1. 并发读不阻塞写
2. 写写冲突正确检测 (WW conflict)
3. 隔离级别语义 (RC / SI / RR)
4. GC 后数据正确性
5. 崩溃恢复 (WAL + MVCC 兼容性)
```

---

## 7. 替代方案

### 7.1 不做 MVCC,只做多线程

如果资源有限, **Phase A 已足够显著提升**:

| 方案 | TPS 提升 | 工作量 | 收益/投入比 |
|---|---|---|---|
| 只做 Phase A | 1.5-1.8x | 4-6 周 | **HIGH** |
| Phase A + C | 3-4x | 20-30 周 | MEDIUM |
| Phase A + B + C | 5-10x | 36-46 周 | LOW |

**建议**: **先实施 Phase A** (4-6 周达到 250-300 TPS),再决定是否继续 Phase C。

### 7.2 第三方库方案

考虑用 `sqlx` / `diesel` 替代自研执行器:
- ✗ 失去 MySQL 协议兼容
- ✗ 重写工作量大 (估计 6+ 个月)
- ✓ 性能可能更好

---

## 8. 总结

### 8.1 推荐路径

```
现在          4-6 周后       16-24 周后
│              │               │
v4.0.0       v4.1.0         v5.0.0
164 TPS      250-300 TPS    600-800 TPS
              Phase A         Phase C
              (多线程)        (MVCC)
```

### 8.2 优先级

1. **P0**: Phase A.1 (ExecutorPool + work-stealing) — 最大单点优化
2. **P0**: Phase A.3 (FileStorage 内部锁) — 解锁读并发
3. **P1**: Phase C (MVCC) — 长期目标,等 Phase A 收益验证后启动
4. **P2**: Phase B (查询优化器) — 单独 Phase

### 8.3 立即可启动的工作

- [x] Phase A 详细设计完成 (本文档)
- [ ] 在 v4.1.0-mvp 分支启动 ExecutorPool 实现
- [ ] 添加 `crates/executor/src/executor_pool.rs`
- [ ] 编写 work-stealing 单元测试
- [ ] 集成到 ExecutionEngine

### 8.4 不在本任务范围内

- 分布式扩展 (v6.0+ 考虑)
- 实时复制 / CDC
- 图查询优化 (V400-04 单独推进)
- 时序数据

---

## 9. 附录

### 9.1 关键文件路径

```
crates/mysql-server/src/lib.rs       # COM_QUERY 入口
crates/mysql-server/src/main.rs      # --executor-parallelism flag
src/execution_engine.rs              # 60+ execute_xxx 方法
crates/executor/src/parallel_executor.rs  # 已存在的并行框架 (待启用)
crates/executor/src/task_scheduler.rs     # RayonTaskScheduler (待替换)
crates/executor/src/thread_pool_registry.rs  # per-query pool cache
crates/storage/src/file_storage.rs   # 存储层 (需要 MVCC 改造)
crates/storage/src/engine.rs         # StorageEngine trait
src/engine_builder.rs                # isolation level 集成
```

### 9.2 参考资料

- **MySQL InnoDB MVCC**: https://dev.mysql.com/doc/refman/8.0/en/innodb-multi-versioning.html
- **PostgreSQL MVCC**: https://www.postgresql.org/docs/current/mvcc.html
- **TiDB MVCC**: https://docs.pingcap.com/tidb/stable/mvcc
- **crossbeam 工作窃取**: https://github.com/crossbeam-rs/crossbeam

### 9.3 监控指标 (Phase A/C 都需)

```bash
# 每 10s 采样
- RSS, VSZ, threads, CPU%
- executor_pool.queue_depth (待提交)
- executor_pool.active_workers
- storage.row_versions.size
- storage.gc_last_run_seconds_ago

# sysbench 每 60s
- TPS, QPS, lat p95, err/s, reconn/s
```

---

**Last Updated**: 2026-09-12
**Owner**: sqlrustgo core team
**Next Review**: After Phase A implementation (estimated 2026-10-15)