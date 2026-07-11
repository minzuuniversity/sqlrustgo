# Design — P2-1: Multi-Thread Server Runtime

> **Issue**: #3175 follow-up
> **日期**: 2026-06-27

## D1: 架构决策

### 方案对比

| 方案 | 优点 | 缺点 | 决策 |
|------|------|------|------|
| **A. rayon + work-stealing** | 已有 rayon 依赖，work-stealing 自动负载均衡 | 需改写 `handle_connection` 为 async | ❌ |
| **B. std::thread pool** | 最小改动，立即可用 | 固定 worker pool | ✅ |
| **C. tokio multi-thread** | 成熟生态 | 需重构 async，引入 tokio 依赖 | ❌ |

**决策**：方案 B — `std::thread` 固定 worker pool。

**原因**：
- 已有 `rayon` 依赖但用于 executor 内，非 server 级别
- `std::thread` pool 改动最小：约 30 行
- Storage 层已线程安全（`Arc<RwLock<WalStorage>>`）

## D2: CLI 参数

```rust
// crates/mysql-server/src/lib.rs
pub fn run_server_v2(
    host: &str,
    port: u16,
    data_dir: &str,
    max_connections: usize,   // 已存在
    auth_mode: &str,
    worker_threads: usize,     // 新增，默认 1
) -> MySqlResult<()>
```

- `--worker-threads=N`：可配置 1-16，默认 1
- 二进制入口 `src/main.rs` 新增 `clap` 参数

## D3: Worker Pool 实现

```rust
// 新增：WorkerPool 结构体
struct WorkerPool {
    sender: std::sync::mpsc::Sender<ClientTask>,
    handles: Vec<std::thread::JoinHandle<()>>,
}

struct ClientTask {
    stream: TcpStream,
    addr: SocketAddr,
    storage: Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>>,
    tls_config: Arc<rustls::ServerConfig>,
    user_store: UserStore,
}

impl WorkerPool {
    fn new(size: usize, storage: Arc<_>, tls: Arc<_>, users: UserStore) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let handles = (0..size).map(|_| {
            let rx = rx.clone();
            std::thread::spawn(move || {
                while let Ok(task) = rx.recv() {
                    handle_connection(task.stream, task.addr,
                                     task.storage.clone(), task.tls_config.clone(),
                                     task.user_store.clone());
                }
            })
        }).collect();
        Self { sender: tx, handles }
    }

    fn submit(&self, task: ClientTask) { self.sender.send(task).ok(); }
}
```

**接受循环**：

```rust
while !shutdown.load(Ordering::SeqCst) {
    match listener.accept() {
        Ok((stream, addr)) => {
            if worker_threads == 1 {
                // 向后兼容：直接 spawn
                thread::spawn(move || handle_connection(stream, addr, st, tc, us));
            } else {
                // Worker pool 模式
                pool.submit(ClientTask { stream, addr, storage: st, tls_config: tc, user_store: us });
            }
        }
        ...
    }
}
```

## D4: 关键文件修改

| 文件 | 修改 |
|------|------|
| `crates/mysql-server/src/lib.rs` | 新增 `WorkerPool` 结构体，修改 `run_server_v2` 签名和 accept loop |
| `crates/mysql-server/src/main.rs` | 新增 `--worker-threads` CLI 参数 |

**总修改量**：约 80 行新增，0 行删除。

## D5: 并发安全审计

### Storage
- `WalStorage<S, T>`：`Arc<RwLock<WalStorage<...>>>` — `Send + Sync` ✅
- WAL write：`FileBackedWalManager` 内部有锁 ✅

### Per-connection 状态
- `handle_connection` 中的 `ps_manager`（`PreparedStatementManager`）：**per-connection**，无竞争 ✅
- `engine`（`Arc<RwLock<ExecutionEngine>>`）：read-only，RwLock 只在获取 write 时竞争 ✅
- `scramble`（随机数）：本地变量，无竞争 ✅

### 需要关注的竞争点
- `UserStore::verify_password`：内部有 `RwLock`，可并发 ✅
- `PreparedStatementManager`：per-connection 内部 `HashMap<u32, Statement>`，无竞争 ✅

## D6: 性能模型

假设：
- 单查询耗时 T，平均分布
- N 个 worker，K 个并发连接

**理论 QPS**：
- 当前（1 thread）：QPS = 1/T
- N workers：QPS ≈ N/T（理想线性加速）

**实测预期**（SF=0.01，单查询 ~53ms）：

| Worker 线程 | 理论 QPS | 预期 QPS |
|------------|---------|---------|
| 1 | 18.9 | ~20 |
| 4 | 75.5 | ~60 |
| 8 | 151 | ~120 |
| 16 | 302 | ~240 |

**实际 QPS < 理论**，因为：
- 连接建立开销
- Storage 内部锁竞争（buffer pool、wal）
- 数据局部性
